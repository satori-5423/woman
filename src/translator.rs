use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthChar;

use crate::config::Config;
use crate::prompts;

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

/// Check whether a roff source string already has a no-adjust / no-fill directive.
pub fn has_no_adjust(roff: &str) -> bool {
    roff.contains("\n.na") || roff.contains("\n.ad l")
}

/// Paragraph macros in the man macro package (an.tmac) that reset adjustment
/// to `.ad b` (full justification). We insert `.na` after each of these to
/// prevent groff from adding excessive spaces in translated text.
const PARAGRAPH_MACROS: &[&str] = &[".P", ".PP", ".TP", ".IP", ".HP", ".RS"];

/// Check if a line is a paragraph macro (one that resets adjustment to `.ad b`).
fn is_paragraph_macro(line: &str) -> bool {
    let trimmed = line.trim();
    PARAGRAPH_MACROS.iter().any(|&m| {
        trimmed == m || trimmed.starts_with(m) && trimmed.as_bytes().get(m.len()) == Some(&b' ')
    })
}

/// Check if a line is a "tag" macro (e.g., `.B`, `.I`, `.BR`, `.IR`, `.RI`, `.RB`)
/// that typically follows `.TP`/`.IP` as the tag line.
fn is_tag_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let tag_macros = [".B ", ".I ", ".BR ", ".IR ", ".RI ", ".RB ", ".BI ", ".IB "];
    // Also match macros without arguments like ".B" at end of line
    let tag_macros_bare = [".B", ".I", ".BR", ".IR", ".RI", ".RB", ".BI", ".IB"];
    tag_macros.iter().any(|&m| trimmed.starts_with(m)) || tag_macros_bare.contains(&trimmed)
}

/// Check if a line (already trimmed) is a no-adjust / no-fill directive.
fn is_na_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == ".na" || trimmed == ".ad l"
}

/// Insert `.na` (no fill, no adjust) after `.TH` and after every
/// paragraph macro (.P, .PP, .TP, .IP, .HP, .RS) in a roff man page.
/// For .TP/.IP/.HP, the next line is the tag; `.na` goes after the tag line.
///
/// `.na` disables both filling and justification. This stops groff from
/// adding excessive spaces (visible in CJK) AND from breaking lines at
/// awkward positions (the few space characters in Chinese text).
///
/// We pair this with `break_long_cjk_lines()` which manually breaks long
/// non-macro lines at CJK-friendly positions (after punctuation).
pub fn insert_no_adjust(roff: &str) -> String {
    let lines: Vec<&str> = roff.lines().collect();
    let mut result = String::with_capacity(roff.len() + 256);
    let mut th_seen = false;
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        result.push_str(line);
        result.push('\n');

        // Insert `.na` after the `.TH` header line if not already there.
        if !th_seen && line.trim().starts_with(".TH ") {
            th_seen = true;
            if i + 1 >= lines.len() || !is_na_line(lines[i + 1]) {
                result.push_str(".na\n");
            }
        }

        // Insert `.na` after paragraph macros that reset adjustment.
        if is_paragraph_macro(line) {
            let is_tagged = line.trim() == ".TP"
                || line.trim() == ".IP"
                || line.trim() == ".HP"
                || line.trim().starts_with(".TP ")
                || line.trim().starts_with(".IP ")
                || line.trim().starts_with(".HP ");

            if is_tagged && i + 1 < lines.len() && is_tag_line(lines[i + 1]) {
                // For tagged paragraphs, the next line is the tag.
                // Insert `.na` after the tag line if not already there.
                i += 1;
                let tag_line = lines[i];
                result.push_str(tag_line);
                result.push('\n');
                if i + 1 >= lines.len() || !is_na_line(lines[i + 1]) {
                    result.push_str(".na\n");
                }
            } else {
                // For .P, .PP, .RS, or .TP/.IP/.HP without a tag line,
                // insert `.na` immediately if not already there.
                if i + 1 >= lines.len() || !is_na_line(lines[i + 1]) {
                    result.push_str(".na\n");
                }
            }
        }

        i += 1;
    }

    result
}

/// Maximum display width for text lines before they are broken.
const MAX_LINE_WIDTH: usize = 76;

/// CJK punctuation characters after which we prefer to break long lines.
fn is_cjk_break_char(c: char) -> bool {
    matches!(
        c,
        '\u{3002}' | // 。 ideographic full stop
        '\u{FF0C}' | // ， fullwidth comma
        '\u{3001}' | // 、 ideographic comma
        '\u{FF1B}' | // ； fullwidth semicolon
        '\u{FF1A}' | // ： fullwidth colon
        '\u{3009}' | // 》 right double angle bracket
        '\u{300B}' | // 》
        '\u{300D}' | // 」 right corner bracket
        '\u{300F}' | // 』 right white corner bracket
        '\u{3011}' | // 】 right black lenticular bracket
        '\u{FF01}' | // ！ fullwidth exclamation
        '\u{FF1F}' | // ？ fullwidth question mark
        '\u{FF0E}' | // ． fullwidth full stop
        '\u{FF05}' // ％ fullwidth percent
    )
}

/// Insert line breaks into long non-macro text lines at CJK-friendly positions.
/// This is needed because `.na` disables groff's own line filling, so we must
/// ensure source lines are a reasonable display width.
///
/// Rules:
/// - Lines starting with `.` or `'` are roff macros/control lines → keep as-is.
/// - Empty lines → keep.
/// - Other lines longer than `MAX_LINE_WIDTH` display columns are broken at
///   CJK punctuation marks (。，、；：）》 etc.) or at spaces.
pub fn break_long_cjk_lines(roff: &str) -> String {
    let lines: Vec<&str> = roff.lines().collect();
    let mut result = String::with_capacity(roff.len() + roff.len() / 10);

    for line in &lines {
        let trimmed = line.trim();

        // Keep roff macros and empty lines as-is.
        if trimmed.is_empty() || trimmed.starts_with('.') || trimmed.starts_with('\'') {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Calculate display width of the entire line.
        let total_width: usize = line.chars().map(|c| c.width().unwrap_or(1)).sum();
        if total_width <= MAX_LINE_WIDTH {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Need to break. Walk through characters tracking width and last
        // good break position.
        let mut out_line = String::new();
        let mut out_width: usize = 0;
        // Byte offset of the last good break point within `out_line`.
        let mut last_break_byte: usize = 0;

        for ch in line.chars() {
            let ch_width = ch.width().unwrap_or(1);
            out_width += ch_width;
            out_line.push(ch);

            // Is this a good break point?
            if ch == ' ' || is_cjk_break_char(ch) {
                last_break_byte = out_line.len();
            }

            // If we exceed max width, break at the last good point.
            if out_width > MAX_LINE_WIDTH && last_break_byte > 0 {
                // Break: output up to last_break_byte, keep the rest.
                let kept = out_line[..last_break_byte].trim_end().to_string();
                let rest = out_line[last_break_byte..].to_string();
                if !kept.is_empty() {
                    result.push_str(&kept);
                    result.push('\n');
                }
                out_line = rest;
                out_width = out_line.chars().map(|c| c.width().unwrap_or(1)).sum();
                last_break_byte = 0;

                // Re-scan the remainder for break points.
                let mut byte_pos = 0usize;
                for rc in out_line.chars() {
                    if rc == ' ' || is_cjk_break_char(rc) {
                        last_break_byte = byte_pos + rc.len_utf8();
                        // We don't track last_break_width here — it's not
                        // needed after the break since we recalculate width.
                    }
                    byte_pos += rc.len_utf8();
                }
            }
        }

        // Output remaining text.
        if !out_line.is_empty() {
            result.push_str(out_line.trim_end());
            result.push('\n');
        }
    }

    result
}

/// Read a cached gzip translation file, ensure it has `.na` (insert if missing)
/// and that long lines are properly broken, then write it back.
pub fn ensure_cached_file_has_no_adjust(path: &std::path::Path) -> Result<(), String> {
    let content = crate::gzip::read_gzip_file(path)?;
    let with_na = insert_no_adjust(&content);
    let fixed = break_long_cjk_lines(&with_na);
    if fixed != content {
        crate::gzip::write_gzip_file(path, &fixed)?;
    }
    Ok(())
}

/// Strip Markdown code block markers (like ```groff ... ```) from the AI response.
/// Handles two cases:
///   1. The entire response is wrapped in a code block.
///   2. A code block is embedded inside conversational text.
pub fn strip_markdown_code_blocks(content: &str) -> String {
    let trimmed = content.trim();

    // Case 1: The response starts with a code block and ends with it.
    if let (true, Some(nl)) = (trimmed.starts_with("```"), trimmed.find('\n'))
        && trimmed.ends_with("```")
    {
        let code = &trimmed[nl + 1..trimmed.len() - 3];
        return code.trim().to_string();
    }

    // Case 2: The code block is embedded inside text (e.g., introductory text exists).
    if let Some(start_pos) = trimmed.find("```") {
        let after_ticks = &trimmed[start_pos..];
        if let Some(first_newline) = after_ticks.find('\n') {
            let code_start = start_pos + first_newline + 1;
            let remaining = &trimmed[code_start..];
            if let Some(end_pos) = remaining.rfind("```") {
                return remaining[..end_pos].trim().to_string();
            }
        }
    }

    trimmed.to_string()
}

/// Translates the given roff man page content into the target locale.
/// `api_url` should be the resolved API base URL (from config or provider default).
///
/// Uses streaming mode (`stream: true`) to keep the connection alive during
/// long-running translation requests, preventing CDN/proxy timeouts.
pub fn translate_man_page(
    content: &str,
    locale: &str,
    config: &Config,
    api_url: &str,
) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let system_prompt = prompts::build_system_prompt(locale);

    let request_body = ChatRequest {
        model: config.active_model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Here is the original English man page content:\n\n{}",
                    content
                ),
            },
        ],
        temperature: 0.0,
        stream: true,
    };

    let url = format!("{}/chat/completions", api_url.trim_end_matches('/'));

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&request_body)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().unwrap_or_default();
        return Err(format!(
            "API returned error status ({}): {}",
            status, error_text
        ));
    }

    // Parse the SSE streaming response.
    let body_bytes = response
        .bytes()
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    let body_text = String::from_utf8_lossy(&body_bytes);

    // If the response is JSON (non-streaming fallback), parse it directly.
    if let Ok(parsed) = serde_json::from_str::<ChatResponse>(&body_text) {
        if parsed.choices.is_empty() {
            return Err("API response contained no choices".to_string());
        }
        let translated_content = &parsed.choices[0].message.content;
        let stripped = strip_markdown_code_blocks(translated_content);
        let with_na = insert_no_adjust(&stripped);
        return Ok(break_long_cjk_lines(&with_na));
    }

    // Otherwise, parse as SSE stream.
    let translated = parse_sse_stream(&body_text)?;
    if translated.is_empty() {
        return Err("API streaming response contained no content".to_string());
    }

    let stripped = strip_markdown_code_blocks(&translated);
    let with_na = insert_no_adjust(&stripped);
    Ok(break_long_cjk_lines(&with_na))
}

/// Parse a Server-Sent Events (SSE) stream and extract the concatenated content
/// from chat completion chunks.
fn parse_sse_stream(body: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct StreamChoice {
        delta: StreamDelta,
    }

    #[derive(Deserialize)]
    struct StreamDelta {
        #[serde(default)]
        content: String,
    }

    #[derive(Deserialize)]
    struct StreamChunk {
        choices: Vec<StreamChoice>,
    }

    let mut full_content = String::new();

    for event in body.split("\n\n") {
        let event = event.trim();
        if event.is_empty() {
            continue;
        }

        // Each event may have multiple lines; extract the "data:" line.
        for line in event.lines() {
            let line = line.trim();
            if line == "data: [DONE]" {
                continue;
            }
            if let Some(data) = line.strip_prefix("data: ")
                && let Ok(chunk) = serde_json::from_str::<StreamChunk>(data)
                && let Some(choice) = chunk.choices.first()
            {
                full_content.push_str(&choice.delta.content);
            }
        }
    }

    Ok(full_content)
}

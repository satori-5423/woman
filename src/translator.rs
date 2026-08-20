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
    /// DeepSeek V4 thinking-mode toggle (OpenAI format: `{"thinking": {"type": ...}}`).
    /// `type: "disabled"` turns off chain-of-thought reasoning, which otherwise
    /// streams a large `reasoning_content` payload before the answer and makes
    /// translation take minutes for small pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<Thinking>,
    /// Reasoning effort ("low"/"high"/"max"), only meaningful when thinking is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
}

#[derive(Serialize)]
struct Thinking {
    #[serde(rename = "type")]
    kind: String,
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
/// to `.ad b` (full justification) via `an*reset-adjustment-mode` — including
/// `.SH`/`.SS` section headings, which start a new paragraph. We insert `.na`
/// after each of these to prevent groff from adding excessive spaces in
/// translated (especially CJK) text.
const PARAGRAPH_MACROS: &[&str] = &[
    ".P", ".PP", ".LP", ".TP", ".IP", ".HP", ".RS", ".SH", ".SS",
];

/// Check if a line is a paragraph macro (one that resets adjustment to `.ad b`).
fn is_paragraph_macro(line: &str) -> bool {
    let trimmed = line.trim();
    PARAGRAPH_MACROS.iter().any(|&m| {
        trimmed == m || trimmed.starts_with(m) && trimmed.as_bytes().get(m.len()) == Some(&b' ')
    })
}

/// True if `trimmed` is exactly `name` or `name` followed by a space or tab.
fn is_macro_with_boundary(trimmed: &str, name: &str) -> bool {
    trimmed == name
        || (trimmed.starts_with(name) && trimmed[name.len()..].starts_with([' ', '\t']))
}

/// Whether the paragraph macro takes its tag from the next input line:
/// `.TP`, `.HP`, and bare `.IP` (no marker argument). `.IP` with a marker has
/// the tag inline on the same line.
fn is_tag_next_line_macro(line: &str) -> bool {
    let trimmed = line.trim();
    is_macro_with_boundary(trimmed, ".TP")
        || is_macro_with_boundary(trimmed, ".HP")
        || trimmed == ".IP"
}

/// Check if a line (already trimmed) is a no-adjust directive.
fn is_na_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == ".na" || trimmed == ".ad l"
}

/// Check if a line is a `.TH` header line (argument separated by space or tab).
fn is_th_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == ".TH" || (trimmed.starts_with(".TH") && trimmed[3..].starts_with([' ', '\t']))
}

/// Insert `.na` (no adjust) after `.TH` and after every paragraph macro
/// (.P, .PP, .LP, .TP, .IP, .HP, .RS, .SH, .SS) in a roff man page.
///
/// groff's man macros reset adjustment to `.ad b` (full justification) at
/// every paragraph macro, so `.na` must be re-inserted after each of them.
/// Without it, groff stretches CJK lines to full width by inserting
/// excessive spaces between the few space-separated Latin tokens.
///
/// For `.TP`, `.HP` and bare `.IP`, the tag comes from the next input line;
/// groff's `.TP`/`.IP` input trap restores `.ad b` AFTER that tag, so `.na`
/// must be placed after the tag line — placing it before the tag is useless.
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

        // Insert `.na` after the `.TH` header line if not already there.
        if !th_seen && is_th_line(line) {
            th_seen = true;
            result.push_str(line);
            result.push('\n');
            if i + 1 >= lines.len() || !is_na_line(lines[i + 1]) {
                result.push_str(".na\n");
            }
            i += 1;
            continue;
        }

        if is_paragraph_macro(line) {
            if is_tag_next_line_macro(line) {
                // .TP / .HP / bare .IP: the tag is the next input line.
                // Skip any stray `.na`/`.ad l` that older woman versions
                // wrongly placed before the tag, then insert `.na` after the
                // real tag line (where it actually takes effect).
                let mut j = i + 1;
                while j < lines.len() && is_na_line(lines[j]) {
                    j += 1;
                }
                if j < lines.len() {
                    // Copy the paragraph macro line and the tag line, then
                    // add `.na` after the tag.
                    result.push_str(line);
                    result.push('\n');
                    result.push_str(lines[j]);
                    result.push('\n');
                    if j + 1 >= lines.len() || !is_na_line(lines[j + 1]) {
                        result.push_str(".na\n");
                    }
                    i = j + 1;
                    continue;
                }
                // No tag line at all — fall through and insert `.na` directly.
            }

            // Non-tagged paragraph macro (.P, .PP, .LP, .RS, .SH, .SS, or
            // .IP with an inline marker): insert `.na` immediately after it.
            result.push_str(line);
            result.push('\n');
            if i + 1 >= lines.len() || !is_na_line(lines[i + 1]) {
                result.push_str(".na\n");
            }
            i += 1;
            continue;
        }

        result.push_str(line);
        result.push('\n');
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

            // If we exceed max width, break at the last good point; if there is
            // no good break point yet (e.g. a long unbroken URL or ASCII run),
            // hard-break right before the current character to keep the line
            // width bounded.
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

                // Re-scan the remainder for break points (the loop will not
                // revisit characters already moved into `rest`).
                let mut byte_pos = 0usize;
                for rc in out_line.chars() {
                    if rc == ' ' || is_cjk_break_char(rc) {
                        last_break_byte = byte_pos + rc.len_utf8();
                    }
                    byte_pos += rc.len_utf8();
                }
            } else if out_width > MAX_LINE_WIDTH {
                // No natural break point found yet — break just before the
                // character that pushed the line over the limit.
                let break_byte = out_line.len() - ch.len_utf8();
                if break_byte > 0 {
                    let kept = out_line[..break_byte].trim_end().to_string();
                    let rest = out_line[break_byte..].to_string();
                    if !kept.is_empty() {
                        result.push_str(&kept);
                        result.push('\n');
                    }
                    out_line = rest;
                    out_width = out_line.chars().map(|c| c.width().unwrap_or(1)).sum();
                    last_break_byte = 0;
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

/// Collapse runs of 2+ ASCII spaces (and tabs) inside non-macro text lines to
/// a single space.
///
/// This is a safety net against AI-generated translations that contain
/// excessive whitespace (alignment padding, stray tabs, etc.), which groff
/// renders literally and looks especially bad in CJK text. Macro/control lines
/// (starting with `.` or `'`) and blank lines are left untouched, and leading
/// indentation of text lines is preserved.
pub fn normalize_text_spaces(roff: &str) -> String {
    let mut result = String::with_capacity(roff.len());

    for line in roff.split_inclusive('\n') {
        let has_nl = line.ends_with('\n');
        let content = if has_nl {
            &line[..line.len() - 1]
        } else {
            line
        };
        let trimmed = content.trim();

        // Keep macro/control lines and blank lines verbatim.
        if trimmed.is_empty() || trimmed.starts_with('.') || trimmed.starts_with('\'') {
            result.push_str(line);
            continue;
        }

        // Text line: preserve leading indentation, collapse internal runs of
        // 2+ spaces/tabs into a single space, and drop trailing whitespace.
        let leading_len = content.len() - content.trim_start().len();
        let body = content[leading_len..].trim_end();
        let mut collapsed = String::with_capacity(content.len());
        collapsed.push_str(&content[..leading_len]);
        let mut pending_space = false;
        for ch in body.chars() {
            if ch == ' ' || ch == '\t' {
                if !pending_space {
                    collapsed.push(' ');
                    pending_space = true;
                }
            } else {
                pending_space = false;
                collapsed.push(ch);
            }
        }
        result.push_str(&collapsed);
        if has_nl {
            result.push('\n');
        }
    }

    result
}

/// Read a cached gzip translation file, ensure it has `.na` (insert if missing)
/// and that long lines are properly broken, then write it back.
pub fn ensure_cached_file_has_no_adjust(path: &std::path::Path) -> Result<(), String> {
    let content = crate::gzip::read_gzip_file(path)?;
    let normalized = normalize_text_spaces(&content);
    let with_na = insert_no_adjust(&normalized);
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

/// Returns true when `WOMAN_DEBUG=1` is set, enabling timing diagnostics on stderr.
fn debug_enabled() -> bool {
    std::env::var("WOMAN_DEBUG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Save the raw API response body under the woman debug directory (best effort).
fn dump_response_body(body: &[u8], filename: &str) {
    if let Ok(dir) = crate::config::get_woman_dir().map(|d| d.join("debug"))
        && std::fs::create_dir_all(&dir).is_ok()
    {
        let path = dir.join(filename);
        match std::fs::write(&path, body) {
            Ok(()) => eprintln!("[woman] raw response saved to {}", path.display()),
            Err(e) => eprintln!("[woman] warning: could not save debug response: {}", e),
        }
    }
}

/// Read a blocking response body incrementally, stopping as soon as the
/// terminal SSE event `data: [DONE]` is observed (or the body ends).
///
/// With a plain `response.bytes()`, we would wait for the server to close the
/// HTTP body even after it has already sent the final `data: [DONE]` event;
/// some servers/CDNs hold the connection open after the last event, which
/// makes `bytes()` block for a long time. Stopping at `[DONE]` avoids that.
///
/// A size cap derived from the source length guards against a model that runs
/// away generating a huge amount of output (e.g. hundreds of thousands of
/// tokens for a small man page), so the user gets a clear error instead of
/// waiting several minutes.
fn read_sse_body(
    mut response: reqwest::blocking::Response,
    source_len: usize,
    debug: bool,
) -> Result<Vec<u8>, String> {
    use std::io::Read;

    // Generous cap: 50x the source size, with a 1 MiB floor. A translation of
    // a man page is normally about the same size as the source; anything far
    // beyond this indicates a runaway model.
    let max_bytes = std::cmp::max(source_len * 50, 1_000_000);

    let mut body = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = response
            .read(&mut buf)
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        if n == 0 {
            break; // body fully received
        }
        body.extend_from_slice(&buf[..n]);
        if sse_done_received(&body) {
            break;
        }
        if body.len() > max_bytes {
            if debug {
                dump_response_body(&body, "last_response_overrun.txt");
            }
            return Err(format!(
                "API response exceeded the {} byte safety cap (source was {} bytes); \
                 the model appears to be generating an abnormally large amount of output. \
                 Try a different model (e.g. `woman model set-model deepseek-chat`), or inspect \
                 the raw response with WOMAN_DEBUG=1.",
                max_bytes, source_len
            ));
        }
    }
    Ok(body)
}

/// Check whether the accumulated SSE body contains the terminal `data: [DONE]`
/// event at a line boundary, with nothing but whitespace before the line end
/// (so `data: [DONE]foo` is not mistaken for the terminal event).
fn sse_done_received(body: &[u8]) -> bool {
    const DONE: &[u8] = b"data: [DONE]";
    body.windows(DONE.len())
        .position(|w| w == DONE)
        .map(|pos| {
            // Must be at the start of a line.
            if pos != 0 && body[pos - 1] != b'\n' {
                return false;
            }
            // The rest of the line after `[DONE]` must be only whitespace.
            let after = &body[pos + DONE.len()..];
            let end = after
                .iter()
                .position(|&b| b == b'\n')
                .unwrap_or(after.len());
            after[..end].iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
        })
        .unwrap_or(false)
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

    let thinking_enabled = config.thinking.eq_ignore_ascii_case("enabled");
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
        thinking: Some(Thinking {
            kind: if thinking_enabled {
                "enabled".to_string()
            } else {
                "disabled".to_string()
            },
        }),
        reasoning_effort: thinking_enabled.then(|| config.reasoning_effort.clone()),
    };

    let url = format!("{}/chat/completions", api_url.trim_end_matches('/'));

    let debug = debug_enabled();
    let t0 = std::time::Instant::now();

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&request_body)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    if debug {
        eprintln!(
            "[woman][timing] request sent: {:.1}s",
            t0.elapsed().as_secs_f64()
        );
    }

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().unwrap_or_default();
        return Err(format!(
            "API returned error status ({}): {}",
            status, error_text
        ));
    }

    // Read the SSE streaming response body, stopping as soon as the terminal
    // `data: [DONE]` event arrives instead of waiting for the server to close
    // the connection (which some servers/CDNs hold open after the last event).
    let body_bytes = read_sse_body(response, content.len(), debug)?;
    if debug {
        dump_response_body(&body_bytes, "last_response.txt");
        eprintln!(
            "[woman][timing] response body received: {:.1}s ({} bytes)",
            t0.elapsed().as_secs_f64(),
            body_bytes.len()
        );
    }

    let body_text = String::from_utf8_lossy(&body_bytes);

    // If the response is JSON (non-streaming fallback), parse it directly.
    if let Ok(parsed) = serde_json::from_str::<ChatResponse>(&body_text) {
        if parsed.choices.is_empty() {
            return Err("API response contained no choices".to_string());
        }
        let translated_content = &parsed.choices[0].message.content;
        let stripped = strip_markdown_code_blocks(translated_content);
        let normalized = normalize_text_spaces(&stripped);
        let with_na = insert_no_adjust(&normalized);
        let fixed = break_long_cjk_lines(&with_na);
        if debug {
            eprintln!(
                "[woman][timing] translation total: {:.1}s\n",
                t0.elapsed().as_secs_f64()
            );
        }
        return Ok(fixed);
    }

    // Otherwise, parse as SSE stream.
    let translated = parse_sse_stream(&body_text)?;
    if debug {
        eprintln!(
            "[woman][timing] SSE parsed: {:.1}s",
            t0.elapsed().as_secs_f64()
        );
    }
    if translated.is_empty() {
        return Err("API streaming response contained no content".to_string());
    }

    let stripped = strip_markdown_code_blocks(&translated);
    let normalized = normalize_text_spaces(&stripped);
    let with_na = insert_no_adjust(&normalized);
    let fixed = break_long_cjk_lines(&with_na);
    if debug {
        eprintln!(
            "[woman][timing] translation total: {:.1}s\n",
            t0.elapsed().as_secs_f64()
        );
    }
    Ok(fixed)
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
        // Accept both `data: {json}` and `data:{json}` forms.
        for line in event.lines() {
            let line = line.trim();
            if line == "data: [DONE]" {
                continue;
            }
            let Some(data) = line
                .strip_prefix("data: ")
                .or_else(|| line.strip_prefix("data:"))
                .map(str::trim)
            else {
                continue;
            };
            if data.is_empty() {
                continue;
            }

            if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                if let Some(choice) = chunk.choices.first() {
                    full_content.push_str(&choice.delta.content);
                }
            } else if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                // Surface API errors sent as SSE events instead of silently
                // reporting an empty translation.
                if let Some(err) = value.get("error") {
                    return Err(format!("API streaming error: {}", err));
                }
            }
        }
    }

    Ok(full_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_done_detection() {
        assert!(sse_done_received(b"data: [DONE]\n\n"));
        assert!(sse_done_received(b"data: hello\n\ndata: [DONE]\n\n"));
        assert!(sse_done_received(b"\n\ndata: [DONE]"));
        assert!(!sse_done_received(b"data: [DONE"));
        assert!(!sse_done_received(b"xdata: [DONE]\n"));
        assert!(!sse_done_received(b""));
        assert!(!sse_done_received(b"data: [DONE]foo"));
    }

    #[test]
    fn test_chat_request_thinking_disabled_serialization() {
        // Thinking disabled → send `{"thinking":{"type":"disabled"}}`, no effort.
        let req = ChatRequest {
            model: "deepseek-v4-flash".to_string(),
            messages: vec![],
            temperature: 0.0,
            stream: true,
            thinking: Some(Thinking {
                kind: "disabled".to_string(),
            }),
            reasoning_effort: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"thinking\":{\"type\":\"disabled\"}"), "{}", json);
        assert!(!json.contains("reasoning_effort"), "{}", json);
    }

    #[test]
    fn test_chat_request_thinking_enabled_serialization() {
        let req = ChatRequest {
            model: "deepseek-v4-flash".to_string(),
            messages: vec![],
            temperature: 0.0,
            stream: true,
            thinking: Some(Thinking {
                kind: "enabled".to_string(),
            }),
            reasoning_effort: Some("low".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"thinking\":{\"type\":\"enabled\"}"), "{}", json);
        assert!(json.contains("\"reasoning_effort\":\"low\""), "{}", json);
    }

    #[test]
    fn test_read_sse_body_stops_at_done() {
        // A fake response is hard to construct without a real server, so we
        // just verify the helper's early-exit logic via `sse_done_received`
        // (already covered above) and that `parse_sse_stream` handles the
        // `data:`-without-space form.
        let body = "data:{\"choices\":[{\"delta\":{\"content\":\"你好\"}}]}\n\n\
                    data:{\"choices\":[{\"delta\":{\"content\":\"世界\"}}]}\n\n\
                    data: [DONE]\n\n";
        assert_eq!(parse_sse_stream(body).unwrap(), "你好世界");
    }
}

use woman::translator::{break_long_cjk_lines, has_no_adjust, insert_no_adjust};

#[test]
fn test_insert_na_after_th() {
    let input = ".TH FD 1\n.SH NAME\nfd - find files\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains(".TH FD 1\n.na\n"),
        "should insert .na after .TH"
    );
}

#[test]
fn test_insert_na_idempotent_after_th() {
    // .na already present right after .TH, .SH and .P — should not double-insert
    let input = ".TH FD 1\n.na\n.SH DESC\n.na\n.P\n.na\ntext\n";
    let result = insert_no_adjust(input);
    assert_eq!(
        result, input,
        "should not double-insert .na after .TH, .SH or .P"
    );
}

#[test]
fn test_insert_na_fills_missing_after_p() {
    // Has .na after .TH but MISSING after .P — should still add it after .P
    let input = ".TH FD 1\n.na\n.SH DESC\n.P\nsome paragraph text\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains("\n.P\n.na\n"),
        "should insert .na after .P even if .TH already has it"
    );
    // .TH should NOT get a second .na
    assert_eq!(
        result.matches(".TH FD 1\n.na\n").count(),
        1,
        "should not double-insert after .TH"
    );
}

#[test]
fn test_insert_na_after_p() {
    let input = ".TH FD 1\n.SH DESC\n.P\nsome paragraph text\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains(".TH FD 1\n.na\n"),
        "should insert after .TH"
    );
    assert!(result.contains("\n.P\n.na\n"), "should insert after .P");
}

#[test]
fn test_insert_na_after_tp_with_tag() {
    let input = ".TH FD 1\n.SH OPTIONS\n.TP\n.B -H, --hidden\nInclude hidden files.\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains(".TP\n"),
        ".TP macro line must be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains(".B -H, --hidden\n.na\n"),
        "should insert .na after tag line following .TP"
    );
}

#[test]
fn test_insert_na_multiple_paragraphs() {
    let input = ".TH FD 1\n.SH DESC\n.P\nfirst paragraph\n.P\nsecond paragraph\n.TP\n.B -f\nthird paragraph\n";
    let result = insert_no_adjust(input);
    let na_count = result.matches("\n.na\n").count();
    // One after .TH, one after .SH, two after .P, one after .TP tag = 5 total
    assert_eq!(
        na_count, 5,
        "should have .na after .TH, .SH and each paragraph macro"
    );
}

#[test]
fn test_insert_na_after_sh_and_ss() {
    // .SH/.SS reset adjustment to .ad b — .na must be re-inserted after them.
    let input = ".TH FD 1\n.SH NAME\nfd\n.SS Subsection\ntext\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains(".SH NAME\n.na\n"),
        "should insert .na after .SH, got:\n{}",
        result
    );
    assert!(
        result.contains(".SS Subsection\n.na\n"),
        "should insert .na after .SS, got:\n{}",
        result
    );
}

#[test]
fn test_pacat_like_description_no_big_spaces() {
    // Regression test for the reported `woman pacat` issue: a paragraph
    // directly after .SH (no .P before it) with AI-inserted excessive spaces
    // must (a) get a `.na` after .SH so groff does not justify the line, and
    // (b) have the literal multi-space runs collapsed.
    let input = ".TH pacat 1 User Manuals\n\
                 .SH DESCRIPTION\n\
                 \\fIpacat\\f1  是一个简单的工具，用于在                        PulseAudio 音频服务器上播放或捕获原始或编码的音频文件。\n";
    let normalized = woman::translator::normalize_text_spaces(input);
    let with_na = woman::translator::insert_no_adjust(&normalized);

    // .na must be present right after .SH DESCRIPTION (not just after .TH).
    assert!(
        with_na.contains(".SH DESCRIPTION\n.na\n"),
        "missing .na after .SH DESCRIPTION, got:\n{}",
        with_na
    );
    // The literal multi-space run must be collapsed to a single space.
    assert!(
        with_na.contains("用于在 PulseAudio"),
        "multi-space run not collapsed, got:\n{}",
        with_na
    );
    assert!(
        !with_na.contains("用于在                        PulseAudio"),
        "big spaces still present, got:\n{}",
        with_na
    );
}

#[test]
fn test_insert_na_after_th_with_tab() {
    let input = ".TH\tFD 1\n.SH NAME\nfd\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains(".TH\tFD 1\n.na\n"),
        "should insert .na after .TH separated by tab, got:\n{}",
        result
    );
}

#[test]
fn test_insert_na_after_tp_with_tab_tag() {
    let input = ".TH FD 1\n.SH OPTIONS\n.TP\n.B\t-H, --hidden\nInclude hidden files.\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains(".TP\n"),
        ".TP macro line must be preserved, got:\n{}",
        result
    );
    assert!(
        result.contains(".B\t-H, --hidden\n.na\n"),
        "should insert .na after tag line separated by tab, got:\n{}",
        result
    );
}

#[test]
fn test_insert_na_after_tp_with_font_escape_tag() {
    // Tags that are NOT roff macros (font escapes / plain text) must still get
    // `.na` AFTER them — groff's .TP trap restores `.ad b` after the tag, so a
    // `.na` placed before the tag is useless (this was the big-space bug).
    let input = ".TH FD 1\n.SH OPTIONS\n.TP\n\\fB--format\\f1=格式\nbody text\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains("\\fB--format\\f1=格式\n.na\n"),
        "should insert .na after the non-macro tag line, got:\n{}",
        result
    );
    assert!(
        !result.contains(".TP\n.na\n"),
        "must not insert .na before the tag, got:\n{}",
        result
    );
}

#[test]
fn test_insert_na_after_bare_ip_tag() {
    // Bare .IP takes the next line as its tag, like .TP.
    let input = ".TH FD 1\n.SH OPTIONS\n.IP\n\\fBfoo\\f1\nbody text\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains("\\fBfoo\\f1\n.na\n"),
        "should insert .na after the bare .IP tag, got:\n{}",
        result
    );
}

#[test]
fn test_insert_na_after_ip_with_marker() {
    // .IP with an inline marker: .na goes right after the .IP line.
    let input = ".TH FD 1\n.SH OPTIONS\n.IP \\fB-f\\f1 2\nbody text\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains(".IP \\fB-f\\f1 2\n.na\n"),
        "should insert .na right after .IP with marker, got:\n{}",
        result
    );
}

#[test]
fn test_insert_na_after_hp_tag() {
    let input = ".TH FD 1\n.SH OPTIONS\n.HP\nHPtag line\nbody text\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains("HPtag line\n.na\n"),
        "should insert .na after the .HP tag line, got:\n{}",
        result
    );
}

#[test]
fn test_migrates_stray_na_before_tag() {
    // Older woman versions inserted .na BEFORE the tag for non-macro tags.
    // Re-processing must drop that stray .na and move it after the tag line.
    let input = ".TH FD 1\n.SH OPTIONS\n.TP\n.na\n\\fB--format\\f1=格式\nbody text\n";
    let result = insert_no_adjust(input);
    assert_eq!(
        result,
        ".TH FD 1\n.na\n.SH OPTIONS\n.na\n.TP\n\\fB--format\\f1=格式\n.na\nbody text\n"
    );
}

#[test]
fn test_break_hard_long_unbreakable_line() {
    // A long line with no spaces or CJK punctuation should still be hard-broken
    // so the output stays bounded in width.
    let long = "a".repeat(200);
    let input = format!(".TH FD 1\n.SH NAME\n{}\n", long);
    let result = break_long_cjk_lines(&input);
    let text_lines: Vec<&str> = result
        .lines()
        .filter(|l| !l.starts_with('.') && !l.is_empty())
        .collect();
    assert!(
        text_lines.len() >= 2,
        "200-char run should be broken into multiple lines, got:\n{}",
        result
    );
    for line in &text_lines {
        assert!(
            line.chars().count() <= 77,
            "broken line too long: {} chars: {:?}",
            line.chars().count(),
            line
        );
    }
}

#[test]
fn test_has_no_adjust_true() {
    assert!(has_no_adjust(".TH FD 1\n.na\n.P\ntext\n"));
    assert!(has_no_adjust(".TH FD 1\n.ad l\n.P\ntext\n"));
}

#[test]
fn test_has_no_adjust_false() {
    assert!(!has_no_adjust(".TH FD 1\n.SH NAME\n.P\ntext\n"));
}

#[test]
fn test_insert_na_without_th() {
    let input = ".SH NAME\n.P\nsome text\n";
    let result = insert_no_adjust(input);
    assert!(
        result.contains("\n.P\n.na\n"),
        "should insert after .P even without .TH"
    );
}

#[test]
fn test_break_short_line_unchanged() {
    let input = ".TH FD 1\n.SH NAME\nshort line\n";
    let result = break_long_cjk_lines(input);
    assert_eq!(result, input, "short line should be unchanged");
}

#[test]
fn test_break_long_cjk_line() {
    // A long Chinese line that should be broken at punctuation
    let input = ".TH FD 1\n.SH DESCRIPTION\n.P\n这是一个非常长的中文句子，它包含了逗号和句号。用来测试断行功能是否正常工作。\n";
    let result = break_long_cjk_lines(input);
    // Should have at least one line break inserted inside the long text line
    let body_start = result.find("这是一个").unwrap();
    let body = &result[body_start..];
    assert!(
        body.contains('\n'),
        "long CJK line should be broken: got:\n{}",
        result
    );
}

#[test]
fn test_break_preserves_macros() {
    let input = ".TH FD 1\n.na\n.SH OPTIONS\n.TP\n.B -H, --hidden\nThis is a description that might be somewhat long but is in English so it has spaces for breaking naturally okay.\n";
    let result = break_long_cjk_lines(input);
    // Macro lines (.TH, .na, .SH, .TP, .B) should be preserved
    assert!(result.contains(".TH FD 1\n"), ".TH should be preserved");
    assert!(result.contains(".na\n"), ".na should be preserved");
    assert!(result.contains(".SH OPTIONS\n"), ".SH should be preserved");
    assert!(result.contains(".TP\n"), ".TP should be preserved");
    assert!(
        result.contains(".B -H, --hidden\n"),
        ".B should be preserved"
    );
}

#[test]
fn test_break_empty_and_macro_only() {
    let input = ".TH FD 1\n.na\n.SH NAME\n.P\n.na\n";
    let result = break_long_cjk_lines(input);
    assert_eq!(result, input);
}

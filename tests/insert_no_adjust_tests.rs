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
    // .na already present right after .TH and .P — should not double-insert
    let input = ".TH FD 1\n.na\n.SH DESC\n.P\n.na\ntext\n";
    let result = insert_no_adjust(input);
    assert_eq!(
        result, input,
        "should not double-insert .na after .TH or .P"
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
        result.contains(".B -H, --hidden\n.na\n"),
        "should insert .na after tag line following .TP"
    );
}

#[test]
fn test_insert_na_multiple_paragraphs() {
    let input = ".TH FD 1\n.SH DESC\n.P\nfirst paragraph\n.P\nsecond paragraph\n.TP\n.B -f\nthird paragraph\n";
    let result = insert_no_adjust(input);
    let na_count = result.matches("\n.na\n").count();
    // One after .TH, two after .P, one after .TP tag = 4 total
    assert_eq!(
        na_count, 4,
        "should have .na after .TH and each paragraph macro"
    );
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

use woman::translator::normalize_text_spaces;

#[test]
fn test_collapse_internal_spaces() {
    let input = ".SH DESC\n这是  一个    有  很多 空格 的  句子。\n";
    let expected = ".SH DESC\n这是 一个 有 很多 空格 的 句子。\n";
    assert_eq!(normalize_text_spaces(input), expected);
}

#[test]
fn test_preserve_macros_and_blank_lines() {
    let input = ".SH DESC\n\n.TP\n.B  -H,  --hidden\nText   here.\n";
    // Macro lines (.SH, .TP, .B) and blank lines are preserved verbatim;
    // only plain text lines are normalized.
    let expected = ".SH DESC\n\n.TP\n.B  -H,  --hidden\nText here.\n";
    assert_eq!(normalize_text_spaces(input), expected);
}

#[test]
fn test_collapse_tabs() {
    let input = "text\t\twith\ttabs   and\t spaces\n";
    let expected = "text with tabs and spaces\n";
    assert_eq!(normalize_text_spaces(input), expected);
}

#[test]
fn test_preserve_leading_indentation() {
    let input = "     pacat   is a   simple tool.\n";
    let expected = "     pacat is a simple tool.\n";
    assert_eq!(normalize_text_spaces(input), expected);
}

#[test]
fn test_single_spaces_unchanged() {
    let input = "这是一个普通的句子，没有多余空格。\n";
    assert_eq!(normalize_text_spaces(input), input);
}

#[test]
fn test_no_trailing_newline() {
    let input = "text  with  spaces";
    let expected = "text with spaces";
    assert_eq!(normalize_text_spaces(input), expected);
}

#[test]
fn test_control_lines_preserved() {
    let input = "'\" comment line\n.nf\ncode   with   spacing\n.fi\n";
    let expected = "'\" comment line\n.nf\ncode with spacing\n.fi\n";
    assert_eq!(normalize_text_spaces(input), expected);
}

#[test]
fn test_empty_input() {
    assert_eq!(normalize_text_spaces(""), "");
}

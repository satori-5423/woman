use woman::translator::strip_markdown_code_blocks;

#[test]
fn test_strip_markdown_code_blocks_simple() {
    let input = "```groff\n.TH LS 1\n.SH NAME\nls\n```";
    let expected = ".TH LS 1\n.SH NAME\nls";
    assert_eq!(strip_markdown_code_blocks(input), expected);
}

#[test]
fn test_strip_markdown_code_blocks_embedded() {
    let input = "Here is the translation:\n```\n.TH LS 1\n.SH NAME\nls\n```\nHope it helps!";
    let expected = ".TH LS 1\n.SH NAME\nls";
    assert_eq!(strip_markdown_code_blocks(input), expected);
}

#[test]
fn test_strip_markdown_code_blocks_none() {
    let input = ".TH LS 1\n.SH NAME\nls";
    let expected = ".TH LS 1\n.SH NAME\nls";
    assert_eq!(strip_markdown_code_blocks(input), expected);
}

#[test]
fn test_strip_markdown_code_blocks_empty() {
    let input = "";
    let expected = "";
    assert_eq!(strip_markdown_code_blocks(input), expected);
}

#[test]
fn test_strip_markdown_code_blocks_only_backticks() {
    let input = "```\n```";
    let expected = "";
    // Empty code block is stripped to empty string
    assert_eq!(strip_markdown_code_blocks(input), expected);
}

use nanachi_derive::Parser;

// ── #[grammar_inline] ──

#[derive(Parser)]
#[grammar_inline("alpha = { 'a'..'z' | 'A'..'Z' }")]
struct AlphaParser;

#[test]
fn inline_grammar_parses() {
    assert_eq!(AlphaParser::parse_alpha("x").unwrap(), "x");
}

#[test]
fn inline_grammar_rejects() {
    AlphaParser::parse_alpha("1").unwrap_err();
}

#[test]
fn inline_grammar_error_has_context() {
    let err = AlphaParser::parse_alpha("1").unwrap_err();
    assert!(err.contains("invalid alpha"));
}

// ── #[grammar] with file path ──

#[derive(Parser)]
#[grammar("../../examples/simple.nanachi")]
struct SimpleParser;

#[test]
fn file_grammar_ident() {
    assert_eq!(SimpleParser::parse_ident("hello123").unwrap(), "hello123");
}

#[test]
fn file_grammar_alpha() {
    assert_eq!(SimpleParser::parse_alpha("Z").unwrap(), "Z");
}

#[test]
fn file_grammar_rejects_digit_start() {
    SimpleParser::parse_ident("123").unwrap_err();
}

// ── Stateful grammar ──

#[derive(Parser)]
#[grammar("../../examples/markdown_bold.nanachi")]
struct BoldParser;

#[test]
fn bold_parses() {
    assert_eq!(BoldParser::parse_bold("**x**").unwrap(), "**x**");
}

#[test]
fn bold_rejects_empty() {
    BoldParser::parse_bold("").unwrap_err();
}

use logos::Logos;
use nanachi_meta::lexer::Token;

fn lex(input: &str) -> Vec<Token<'_>> {
    Token::lexer(input).filter_map(|t| t.ok()).collect()
}

#[test]
fn lex_state_declaration() {
    let tokens = lex("let flag inside_bold");
    assert_eq!(
        tokens,
        vec![Token::Let, Token::Flag, Token::Ident("inside_bold")]
    );
}

#[test]
fn lex_counter_declaration() {
    let tokens = lex("let counter section_counter");
    assert_eq!(
        tokens,
        vec![Token::Let, Token::Counter, Token::Ident("section_counter")]
    );
}

#[test]
fn lex_rule_definition() {
    let tokens = lex(r#"bold = { "**" inline+ "**" }"#);
    assert_eq!(
        tokens,
        vec![
            Token::Ident("bold"),
            Token::Eq,
            Token::LBrace,
            Token::StringLit("**"),
            Token::Ident("inline"),
            Token::Plus,
            Token::StringLit("**"),
            Token::RBrace,
        ]
    );
}

#[test]
fn lex_guard() {
    let tokens = lex("guard !inside_bold");
    assert_eq!(
        tokens,
        vec![Token::Guard, Token::Bang, Token::Ident("inside_bold")]
    );
}

#[test]
fn lex_with_block() {
    let tokens = lex("with inside_bold { }");
    assert_eq!(
        tokens,
        vec![
            Token::With,
            Token::Ident("inside_bold"),
            Token::LBrace,
            Token::RBrace
        ]
    );
}

#[test]
fn lex_with_increment() {
    let tokens = lex("with trim_brace_depth += 1 { }");
    assert_eq!(
        tokens,
        vec![
            Token::With,
            Token::Ident("trim_brace_depth"),
            Token::PlusEq,
            Token::Number(1),
            Token::LBrace,
            Token::RBrace,
        ]
    );
}

#[test]
fn lex_char_range() {
    let tokens = lex("'a'..'z'");
    assert_eq!(
        tokens,
        vec![Token::CharLit('a'), Token::DotDot, Token::CharLit('z')]
    );
}

#[test]
fn lex_repetition_bounds() {
    let tokens = lex("p{3,6}");
    assert_eq!(
        tokens,
        vec![
            Token::Ident("p"),
            Token::LBrace,
            Token::Number(3),
            Token::Comma,
            Token::Number(6),
            Token::RBrace,
        ]
    );
}

#[test]
fn lex_choice_and_lookahead() {
    let tokens = lex("a | &b | !c");
    assert_eq!(
        tokens,
        vec![
            Token::Ident("a"),
            Token::Pipe,
            Token::Amp,
            Token::Ident("b"),
            Token::Pipe,
            Token::Bang,
            Token::Ident("c"),
        ]
    );
}

#[test]
fn lex_builtins() {
    let tokens = lex("SOI EOI ANY LINE_START LINE_END");
    assert_eq!(
        tokens,
        vec![
            Token::Soi,
            Token::Eoi,
            Token::Any,
            Token::LineStart,
            Token::LineEnd
        ]
    );
}

#[test]
fn lex_depth_limit() {
    let tokens = lex("depth_limit(64) { }");
    assert_eq!(
        tokens,
        vec![
            Token::DepthLimit,
            Token::LParen,
            Token::Number(64),
            Token::RParen,
            Token::LBrace,
            Token::RBrace,
        ]
    );
}

#[test]
fn lex_comment_skipped() {
    let tokens = lex("let flag x // this is a comment\nlet flag y");
    assert_eq!(
        tokens,
        vec![
            Token::Let,
            Token::Flag,
            Token::Ident("x"),
            Token::Newline,
            Token::Let,
            Token::Flag,
            Token::Ident("y"),
        ]
    );
}

#[test]
fn lex_when_condition() {
    let tokens = lex("when trim_brace_depth > 0 { }");
    assert_eq!(
        tokens,
        vec![
            Token::When,
            Token::Ident("trim_brace_depth"),
            Token::Gt,
            Token::Number(0),
            Token::LBrace,
            Token::RBrace,
        ]
    );
}

#[test]
fn lex_emit() {
    let tokens = lex("emit section_counter");
    assert_eq!(tokens, vec![Token::Emit, Token::Ident("section_counter")]);
}

#[test]
fn lex_unicode_string_literal() {
    let tokens = lex(r#""문단""#);
    assert_eq!(tokens, vec![Token::StringLit("문단")]);
}

#[test]
fn lex_unicode_char_range() {
    let tokens = lex("'가'..'힣'");
    assert_eq!(
        tokens,
        vec![Token::CharLit('가'), Token::DotDot, Token::CharLit('힣')]
    );
}

#[test]
fn lex_emoji_string_literal() {
    let tokens = lex(r#""🎉""#);
    assert_eq!(tokens, vec![Token::StringLit("🎉")]);
}

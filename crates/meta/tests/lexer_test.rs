use logos::Logos;
use nanachi_meta::lexer::Token;

fn lex(input: &str) -> Vec<Token<'_>> {
    Token::lexer(input).filter_map(|t| t.ok()).collect()
}

fn lex_results(input: &str) -> Vec<Result<Token<'_>, ()>> {
    Token::lexer(input).collect()
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
fn lex_line_comment_skipped() {
    let tokens = lex("let flag x // this is a comment\nlet flag y");
    assert_eq!(
        tokens,
        vec![
            Token::Let,
            Token::Flag,
            Token::Ident("x"),
            Token::Let,
            Token::Flag,
            Token::Ident("y"),
        ]
    );
}

#[test]
fn lex_block_comment_skipped() {
    let tokens = lex("let /* skip this */ flag x");
    assert_eq!(tokens, vec![Token::Let, Token::Flag, Token::Ident("x")]);
}

#[test]
fn lex_multiline_block_comment() {
    let tokens = lex("let flag x\n/* this\nspans\nlines */\nlet flag y");
    assert_eq!(
        tokens,
        vec![
            Token::Let,
            Token::Flag,
            Token::Ident("x"),
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

#[test]
fn lex_keyword_prefixes_stay_identifiers() {
    let tokens = lex("letter whenish depth_limiter SOIX LINE_ENDING");
    assert_eq!(
        tokens,
        vec![
            Token::Ident("letter"),
            Token::Ident("whenish"),
            Token::Ident("depth_limiter"),
            Token::Ident("SOIX"),
            Token::Ident("LINE_ENDING"),
        ]
    );
}

#[test]
fn lex_string_literal_with_escapes() {
    let tokens = lex(r#""\n\t\\\"""#);
    assert_eq!(tokens, vec![Token::StringLit(r#"\n\t\\\""#)]);
}

#[test]
fn lex_escaped_char_literals() {
    let tokens = lex(r"'\n' '\t' '\\' '\''");
    assert_eq!(
        tokens,
        vec![
            Token::CharLit('\n'),
            Token::CharLit('\t'),
            Token::CharLit('\\'),
            Token::CharLit('\''),
        ]
    );
}

#[test]
fn lex_unexpected_character_returns_error_token() {
    let tokens = lex_results("rule $");
    assert_eq!(tokens, vec![Ok(Token::Ident("rule")), Err(())]);
}

#[test]
fn lex_at_token() {
    let tokens = lex_results(r#"rule @ "label""#);
    assert_eq!(
        tokens,
        vec![
            Ok(Token::Ident("rule")),
            Ok(Token::At),
            Ok(Token::StringLit("label")),
        ]
    );
}

#[test]
fn lex_invalid_escaped_char_literal_returns_error_token() {
    let tokens = lex_results(r"'\x'");
    assert_eq!(tokens, vec![Err(())]);
}

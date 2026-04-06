use crate::ast::*;
use crate::lexer::{self, Token};

use super::error::ParseError;
use super::expr::parse_choice;
use super::state::parse_state_decl;
use super::statement::parse_statements;
use super::tokens::TokenStream;

pub(crate) fn parse_grammar(tokens: &mut TokenStream<'_>) -> Result<Grammar, ParseError> {
    let mut items = Vec::new();

    while !tokens.at_end() {
        let item = parse_item(tokens)?;
        items.push(item);
    }

    Ok(Grammar { items })
}

fn parse_item(tokens: &mut TokenStream<'_>) -> Result<Item, ParseError> {
    match tokens.peek() {
        Some(Token::Let) => parse_state_decl(tokens).map(Item::StateDecl),
        Some(Token::Ident(_)) => parse_rule_def(tokens).map(Item::RuleDef),
        other => Err(tokens.error(format!("expected 'let' or rule name, got {other:?}"))),
    }
}

fn parse_rule_def(tokens: &mut TokenStream<'_>) -> Result<RuleDef, ParseError> {
    let name = tokens.expect_ident()?;
    tokens.expect(&Token::Eq)?;

    // Optional rule-level error label: `= @ "label" { ... }`
    let error_label = if tokens.peek() == Some(&Token::At) {
        tokens.advance();
        match tokens.advance() {
            Some(Token::StringLit(s)) => Some(lexer::unescape_str(s)),
            other => return Err(tokens.error(format!("expected string after '@', got {other:?}"))),
        }
    } else {
        None
    };

    tokens.expect(&Token::LBrace)?;

    let statements = parse_statements(tokens)?;
    let expr = parse_choice(tokens)?;

    tokens.expect(&Token::RBrace)?;

    Ok(RuleDef {
        name,
        error_label,
        body: RuleBody { statements, expr },
    })
}

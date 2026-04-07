use crate::ast::{BuiltinPredicate, CompareOp, GuardCondition, GuardStmt, IncStmt, Statement};
use crate::lexer::Token;

use super::error::ParseError;
use super::expr::parse_numeric_expr;
use super::tokens::TokenStream;

pub(crate) fn parse_statements(tokens: &mut TokenStream<'_>) -> Result<Vec<Statement>, ParseError> {
    let mut statements = Vec::new();

    loop {
        match tokens.peek() {
            Some(Token::Guard) => {
                statements.push(Statement::Guard(parse_guard(tokens)?));
            }
            Some(Token::Inc) => {
                statements.push(Statement::Inc(parse_inc(tokens)?));
            }
            _ => break,
        }
    }

    Ok(statements)
}

fn parse_guard(tokens: &mut TokenStream<'_>) -> Result<GuardStmt, ParseError> {
    tokens.expect(&Token::Guard)?;
    let condition = parse_guard_condition(tokens)?;
    Ok(GuardStmt { condition })
}

pub(crate) fn parse_guard_condition(
    tokens: &mut TokenStream<'_>,
) -> Result<GuardCondition, ParseError> {
    match tokens.peek() {
        Some(Token::Bang) => {
            tokens.advance();
            let name = tokens.expect_ident()?;
            Ok(GuardCondition::NotFlag(name))
        }
        Some(Token::LineStart) => {
            tokens.advance();
            Ok(GuardCondition::Builtin(BuiltinPredicate::LineStart))
        }
        Some(Token::LineEnd) => {
            tokens.advance();
            Ok(GuardCondition::Builtin(BuiltinPredicate::LineEnd))
        }
        Some(Token::Soi) => {
            tokens.advance();
            Ok(GuardCondition::Builtin(BuiltinPredicate::Soi))
        }
        Some(Token::Eoi) => {
            tokens.advance();
            Ok(GuardCondition::Builtin(BuiltinPredicate::Eoi))
        }
        Some(Token::Ident(_)) => {
            let name = tokens.expect_ident()?;
            match tokens.peek() {
                Some(
                    Token::Gt | Token::Lt | Token::Ge | Token::Le | Token::EqEq | Token::BangEq,
                ) => {
                    let op = parse_compare_op(tokens)?;
                    let value = parse_numeric_expr(tokens)?;
                    Ok(GuardCondition::Compare { name, op, value })
                }
                _ => Ok(GuardCondition::IsFlag(name)),
            }
        }
        other => Err(tokens.error(format!("expected guard condition, got {other:?}"))),
    }
}

fn parse_compare_op(tokens: &mut TokenStream<'_>) -> Result<CompareOp, ParseError> {
    match tokens.advance() {
        Some(Token::Gt) => Ok(CompareOp::Gt),
        Some(Token::Lt) => Ok(CompareOp::Lt),
        Some(Token::Ge) => Ok(CompareOp::Ge),
        Some(Token::Le) => Ok(CompareOp::Le),
        Some(Token::EqEq) => Ok(CompareOp::Eq),
        Some(Token::BangEq) => Ok(CompareOp::Ne),
        other => Err(tokens.error(format!("expected comparison operator, got {other:?}"))),
    }
}

fn parse_inc(tokens: &mut TokenStream<'_>) -> Result<IncStmt, ParseError> {
    tokens.expect(&Token::Inc)?;
    let counter = tokens.expect_ident()?;
    Ok(IncStmt { counter })
}

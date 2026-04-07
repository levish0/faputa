use super::error::ParseError;
use super::statement::parse_guard_condition;
use super::tokens::TokenStream;
use crate::ast::*;
use crate::lexer::{self, Token};

/// Choice: `a | b | c`
pub(crate) fn parse_choice(tokens: &mut TokenStream<'_>) -> Result<Expr, ParseError> {
    let first = parse_sequence(tokens)?;
    let mut alternatives = vec![first];

    while tokens.peek() == Some(&Token::Pipe) {
        tokens.advance();

        alternatives.push(parse_sequence(tokens)?);
    }

    if alternatives.len() == 1 {
        Ok(alternatives.pop().unwrap())
    } else {
        Ok(Expr::Choice(alternatives))
    }
}

/// Sequence: `a b c` (whitespace-separated)
fn parse_sequence(tokens: &mut TokenStream<'_>) -> Result<Expr, ParseError> {
    let mut items = Vec::new();
    items.push(parse_postfix(tokens)?);

    loop {
        if !is_at_expr_start(tokens) {
            break;
        }
        items.push(parse_postfix(tokens)?);
    }

    if items.len() == 1 {
        Ok(items.pop().unwrap())
    } else {
        Ok(Expr::Seq(items))
    }
}

/// Postfix operators: `p+`, `p*`, `p?`, `p{n,m}`, `p @ "label"`
fn parse_postfix(tokens: &mut TokenStream<'_>) -> Result<Expr, ParseError> {
    let mut expr = parse_prefix(tokens)?;

    loop {
        match tokens.peek() {
            Some(Token::Plus) => {
                tokens.advance();
                expr = Expr::Repeat {
                    expr: Box::new(expr),
                    kind: RepeatKind::OneOrMore,
                };
            }
            Some(Token::Star) => {
                tokens.advance();
                expr = Expr::Repeat {
                    expr: Box::new(expr),
                    kind: RepeatKind::ZeroOrMore,
                };
            }
            Some(Token::Question) => {
                tokens.advance();
                expr = Expr::Repeat {
                    expr: Box::new(expr),
                    kind: RepeatKind::Optional,
                };
            }
            Some(Token::LBrace) => {
                if let Some(kind) = try_parse_repeat_bounds(tokens)? {
                    expr = Expr::Repeat {
                        expr: Box::new(expr),
                        kind,
                    };
                } else {
                    break;
                }
            }
            Some(Token::At) => {
                tokens.advance();
                match tokens.advance() {
                    Some(Token::StringLit(s)) => {
                        let label = lexer::unescape_str(s);
                        expr = Expr::Labeled {
                            expr: Box::new(expr),
                            label,
                        };
                    }
                    other => {
                        return Err(
                            tokens.error(format!("expected string after '@', got {other:?}"))
                        );
                    }
                }
                // @ is terminal — don't allow chaining
                break;
            }
            _ => break,
        }
    }

    Ok(expr)
}

/// Try to parse `{n}`, `{n,}`, `{,m}`, `{n,m}`.
/// Returns None if the `{` is not a repeat bound.
fn try_parse_repeat_bounds(tokens: &mut TokenStream<'_>) -> Result<Option<RepeatKind>, ParseError> {
    let saved = tokens.save();

    tokens.advance(); // consume `{`

    match tokens.peek() {
        Some(Token::Number(_)) | Some(Token::Ident(_)) => {
            let n = parse_numeric_expr(tokens)?;
            match tokens.peek() {
                Some(Token::RBrace) => {
                    tokens.advance();
                    Ok(Some(RepeatKind::Exact(n)))
                }
                Some(Token::Comma) => {
                    tokens.advance();
                    match tokens.peek() {
                        Some(Token::RBrace) => {
                            tokens.advance();
                            Ok(Some(RepeatKind::AtLeast(n)))
                        }
                        Some(Token::Number(_)) | Some(Token::Ident(_)) => {
                            let m = parse_numeric_expr(tokens)?;
                            tokens.expect(&Token::RBrace)?;
                            Ok(Some(RepeatKind::Range(n, m)))
                        }
                        _ => {
                            tokens.restore(saved);
                            Ok(None)
                        }
                    }
                }
                _ => {
                    tokens.restore(saved);
                    Ok(None)
                }
            }
        }
        Some(Token::Comma) => {
            tokens.advance();
            let m = parse_numeric_expr(tokens)?;
            tokens.expect(&Token::RBrace)?;
            Ok(Some(RepeatKind::AtMost(m)))
        }
        _ => {
            tokens.restore(saved);
            Ok(None)
        }
    }
}

/// Prefix operators: `&p`, `!p`
fn parse_prefix(tokens: &mut TokenStream<'_>) -> Result<Expr, ParseError> {
    match tokens.peek() {
        Some(Token::Amp) => {
            tokens.advance();
            let expr = parse_atom(tokens)?;
            Ok(Expr::PosLookahead(Box::new(expr)))
        }
        Some(Token::Bang) => {
            tokens.advance();
            let expr = parse_atom(tokens)?;
            Ok(Expr::NegLookahead(Box::new(expr)))
        }
        _ => parse_atom(tokens),
    }
}

/// Atoms: literals, idents, builtins, groups, with, when, depth_limit
fn parse_atom(tokens: &mut TokenStream<'_>) -> Result<Expr, ParseError> {
    match tokens.peek() {
        Some(Token::StringLit(_)) => {
            if let Some(Token::StringLit(s)) = tokens.advance() {
                Ok(Expr::StringLit(lexer::unescape_str(s)))
            } else {
                unreachable!()
            }
        }
        Some(Token::CharLit(_)) => {
            if let Some(Token::CharLit(c)) = tokens.advance() {
                tokens.expect(&Token::DotDot)?;
                if let Some(Token::CharLit(end)) = tokens.advance() {
                    Ok(Expr::CharRange(c, end))
                } else {
                    Err(tokens.error("expected char literal after '..'".to_string()))
                }
            } else {
                unreachable!()
            }
        }
        Some(Token::Ident(_)) => {
            if let Some(Token::Ident(name)) = tokens.advance() {
                Ok(Expr::Ident(name.to_string()))
            } else {
                unreachable!()
            }
        }
        Some(Token::Soi) => {
            tokens.advance();
            Ok(Expr::Builtin(BuiltinPredicate::Soi))
        }
        Some(Token::Eoi) => {
            tokens.advance();
            Ok(Expr::Builtin(BuiltinPredicate::Eoi))
        }
        Some(Token::Any) => {
            tokens.advance();
            Ok(Expr::Builtin(BuiltinPredicate::Any))
        }
        Some(Token::LineStart) => {
            tokens.advance();
            Ok(Expr::Builtin(BuiltinPredicate::LineStart))
        }
        Some(Token::LineEnd) => {
            tokens.advance();
            Ok(Expr::Builtin(BuiltinPredicate::LineEnd))
        }
        Some(Token::LParen) => {
            tokens.advance();

            let inner = parse_choice(tokens)?;

            tokens.expect(&Token::RParen)?;
            Ok(Expr::Group(Box::new(inner)))
        }
        Some(Token::With) => parse_with_expr(tokens),
        Some(Token::When) => parse_when_expr(tokens),
        Some(Token::If) => parse_if_expr(tokens),
        Some(Token::Measure) => parse_measure_expr(tokens),
        Some(Token::DepthLimit) => parse_depth_limit_expr(tokens),
        other => Err(tokens.error(format!("expected expression, got {other:?}"))),
    }
}

pub(crate) fn parse_numeric_expr(tokens: &mut TokenStream<'_>) -> Result<NumericExpr, ParseError> {
    match tokens.advance() {
        Some(Token::Number(n)) => Ok(NumericExpr::Literal(n)),
        Some(Token::Ident(name)) => Ok(NumericExpr::Counter(name.to_string())),
        other => Err(tokens.error(format!("expected number or counter name, got {other:?}"))),
    }
}

fn parse_with_expr(tokens: &mut TokenStream<'_>) -> Result<Expr, ParseError> {
    tokens.expect(&Token::With)?;
    let name = tokens.expect_ident()?;

    match tokens.peek() {
        Some(Token::PlusEq) => {
            tokens.advance();
            let amount = parse_numeric_expr(tokens)?;
            tokens.expect(&Token::LBrace)?;

            let body = parse_choice(tokens)?;

            tokens.expect(&Token::RBrace)?;
            Ok(Expr::WithIncrement(WithIncrementExpr {
                counter: name,
                amount,
                body: Box::new(body),
            }))
        }
        Some(Token::LBrace) => {
            tokens.advance();

            let body = parse_choice(tokens)?;

            tokens.expect(&Token::RBrace)?;
            Ok(Expr::With(WithExpr {
                flag: name,
                body: Box::new(body),
            }))
        }
        other => Err(tokens.error(format!("expected '{{' or '+=' after with, got {other:?}"))),
    }
}

fn parse_when_expr(tokens: &mut TokenStream<'_>) -> Result<Expr, ParseError> {
    tokens.expect(&Token::When)?;
    let condition = parse_guard_condition(tokens)?;
    tokens.expect(&Token::LBrace)?;
    let body = parse_choice(tokens)?;
    tokens.expect(&Token::RBrace)?;
    Ok(Expr::When(WhenExpr {
        condition,
        body: Box::new(body),
    }))
}

fn parse_if_expr(tokens: &mut TokenStream<'_>) -> Result<Expr, ParseError> {
    tokens.expect(&Token::If)?;
    let condition = parse_guard_condition(tokens)?;
    tokens.expect(&Token::LBrace)?;
    let then_body = parse_choice(tokens)?;
    tokens.expect(&Token::RBrace)?;
    tokens.expect(&Token::Else)?;
    tokens.expect(&Token::LBrace)?;
    let else_body = parse_choice(tokens)?;
    tokens.expect(&Token::RBrace)?;
    Ok(Expr::If(IfExpr {
        condition,
        then_body: Box::new(then_body),
        else_body: Box::new(else_body),
    }))
}

fn parse_measure_expr(tokens: &mut TokenStream<'_>) -> Result<Expr, ParseError> {
    tokens.expect(&Token::Measure)?;
    let counter = tokens.expect_ident()?;
    tokens.expect(&Token::LBrace)?;
    let body = parse_choice(tokens)?;
    tokens.expect(&Token::RBrace)?;
    Ok(Expr::Measure(MeasureExpr {
        counter,
        body: Box::new(body),
    }))
}

fn parse_depth_limit_expr(tokens: &mut TokenStream<'_>) -> Result<Expr, ParseError> {
    tokens.expect(&Token::DepthLimit)?;
    tokens.expect(&Token::LParen)?;
    let limit = parse_numeric_expr(tokens)?;
    tokens.expect(&Token::RParen)?;
    tokens.expect(&Token::LBrace)?;
    let body = parse_choice(tokens)?;
    tokens.expect(&Token::RBrace)?;
    Ok(Expr::DepthLimit(DepthLimitExpr {
        limit,
        body: Box::new(body),
    }))
}

fn is_at_expr_start(tokens: &TokenStream<'_>) -> bool {
    matches!(
        tokens.peek(),
        Some(
            Token::StringLit(_)
                | Token::CharLit(_)
                | Token::Ident(_)
                | Token::Soi
                | Token::Eoi
                | Token::Any
                | Token::LineStart
                | Token::LineEnd
                | Token::LParen
                | Token::Amp
                | Token::Bang
                | Token::With
                | Token::When
                | Token::If
                | Token::Measure
                | Token::DepthLimit
        )
    )
}

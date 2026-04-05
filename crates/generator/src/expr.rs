use nanachi_meta::ast::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate winnow combinator code for an expression.
pub(crate) fn generate_expr(expr: &Expr) -> TokenStream {
    match expr {
        Expr::StringLit(s) => {
            quote! { literal(#s) }
        }

        Expr::CharRange(start, end) => {
            quote! { one_of(#start..=#end) }
        }

        Expr::Ident(name) => {
            let fn_name = format_ident!("{}", name);
            quote! { #fn_name }
        }

        Expr::Builtin(builtin) => generate_builtin_expr(builtin),

        Expr::Seq(exprs) => {
            let items: Vec<_> = exprs.iter().map(generate_expr).collect();
            quote! { (#(#items),*) }
        }

        Expr::Choice(exprs) => {
            let items: Vec<_> = exprs.iter().map(generate_expr).collect();
            quote! { alt((#(#items),*)) }
        }

        Expr::Repeat { expr, kind } => generate_repeat(expr, kind),

        Expr::PosLookahead(inner) => {
            let inner_code = generate_expr(inner);
            quote! { peek(#inner_code) }
        }

        Expr::NegLookahead(inner) => {
            let inner_code = generate_expr(inner);
            quote! { not(#inner_code) }
        }

        Expr::Group(inner) => generate_expr(inner),

        Expr::With(with_expr) => generate_with_flag(with_expr),
        Expr::WithIncrement(with_inc) => generate_with_increment(with_inc),
        Expr::When(when_expr) => generate_when(when_expr),
        Expr::DepthLimit(dl) => generate_depth_limit(dl),
    }
}

fn generate_builtin_expr(builtin: &BuiltinPredicate) -> TokenStream {
    match builtin {
        BuiltinPredicate::Soi => {
            // SOI as expression: succeed if at position 0
            quote! {
                winnow::combinator::trace("SOI", |input: &mut Input<'_, ParseState>| {
                    if input.input.location() == 0 {
                        Ok(())
                    } else {
                        Err(winnow::error::ErrMode::Backtrack(
                            winnow::error::ContextError::new(),
                        ))
                    }
                })
            }
        }
        BuiltinPredicate::Eoi => {
            quote! { eof.void() }
        }
        BuiltinPredicate::Any => {
            quote! { any.void() }
        }
        BuiltinPredicate::LineStart => {
            quote! {
                winnow::combinator::trace("LINE_START", |input: &mut Input<'_, ParseState>| {
                    let pos = input.input.location();
                    if input.state.is_at_line_start(pos) {
                        Ok(())
                    } else {
                        Err(winnow::error::ErrMode::Backtrack(
                            winnow::error::ContextError::new(),
                        ))
                    }
                })
            }
        }
        BuiltinPredicate::LineEnd => {
            quote! {
                winnow::combinator::trace("LINE_END", |input: &mut Input<'_, ParseState>| {
                    let pos = input.input.location();
                    if input.state.is_at_line_end(pos) {
                        Ok(())
                    } else {
                        Err(winnow::error::ErrMode::Backtrack(
                            winnow::error::ContextError::new(),
                        ))
                    }
                })
            }
        }
    }
}

fn generate_repeat(expr: &Expr, kind: &RepeatKind) -> TokenStream {
    let inner = generate_expr(expr);
    match kind {
        RepeatKind::ZeroOrMore => quote! { repeat(0.., #inner) },
        RepeatKind::OneOrMore => quote! { repeat(1.., #inner) },
        RepeatKind::Optional => quote! { opt(#inner) },
        RepeatKind::Exact(n) => {
            let n = *n as usize;
            quote! { repeat(#n, #inner) }
        }
        RepeatKind::AtLeast(n) => {
            let n = *n as usize;
            quote! { repeat(#n.., #inner) }
        }
        RepeatKind::AtMost(m) => {
            let m = *m as usize;
            quote! { repeat(..=#m, #inner) }
        }
        RepeatKind::Range(n, m) => {
            let n = *n as usize;
            let m = *m as usize;
            quote! { repeat(#n..=#m, #inner) }
        }
    }
}

fn generate_with_flag(with_expr: &WithExpr) -> TokenStream {
    let name = &with_expr.flag;
    let body = generate_expr(&with_expr.body);
    quote! {
        winnow::combinator::trace("with_flag", |input: &mut Input<'_, ParseState>| {
            let prev = input.state.get_flag(#name);
            input.state.set_flag(#name, true);
            let result = (#body).parse_next(input);
            input.state.set_flag(#name, prev);
            result
        })
    }
}

fn generate_with_increment(with_inc: &WithIncrementExpr) -> TokenStream {
    let name = &with_inc.counter;
    let amount = with_inc.amount as usize;
    let body = generate_expr(&with_inc.body);
    quote! {
        winnow::combinator::trace("with_increment", |input: &mut Input<'_, ParseState>| {
            input.state.increment_counter(#name, #amount);
            let result = (#body).parse_next(input);
            input.state.decrement_counter(#name, #amount);
            result
        })
    }
}

fn generate_when(when_expr: &WhenExpr) -> TokenStream {
    let condition_check = generate_condition_check(&when_expr.condition);
    let body = generate_expr(&when_expr.body);
    quote! {
        winnow::combinator::trace("when", |input: &mut Input<'_, ParseState>| {
            if #condition_check {
                (#body).parse_next(input)
            } else {
                Ok(Default::default())
            }
        })
    }
}

fn generate_condition_check(condition: &GuardCondition) -> TokenStream {
    match condition {
        GuardCondition::NotFlag(name) => quote! { !input.state.get_flag(#name) },
        GuardCondition::IsFlag(name) => quote! { input.state.get_flag(#name) },
        GuardCondition::Builtin(BuiltinPredicate::Soi) => quote! { input.input.location() == 0 },
        GuardCondition::Builtin(BuiltinPredicate::Eoi) => quote! { input.input.is_empty() },
        GuardCondition::Builtin(BuiltinPredicate::LineStart) => {
            quote! { input.state.is_at_line_start(input.input.location()) }
        }
        GuardCondition::Builtin(BuiltinPredicate::LineEnd) => {
            quote! { input.state.is_at_line_end(input.input.location()) }
        }
        GuardCondition::Builtin(BuiltinPredicate::Any) => quote! { true },
        GuardCondition::Compare { name, op, value } => {
            let value = *value as usize;
            let cmp = match op {
                CompareOp::Eq => quote! { == },
                CompareOp::Ne => quote! { != },
                CompareOp::Lt => quote! { < },
                CompareOp::Le => quote! { <= },
                CompareOp::Gt => quote! { > },
                CompareOp::Ge => quote! { >= },
            };
            quote! { input.state.get_counter(#name) #cmp #value }
        }
    }
}

fn generate_depth_limit(dl: &DepthLimitExpr) -> TokenStream {
    let limit = dl.limit as usize;
    let body = generate_expr(&dl.body);
    quote! {
        winnow::combinator::trace("depth_limit", |input: &mut Input<'_, ParseState>| {
            let depth = input.state.get_counter("__recursion_depth");
            if depth >= #limit {
                return Err(winnow::error::ErrMode::Backtrack(
                    winnow::error::ContextError::new(),
                ));
            }
            input.state.increment_counter("__recursion_depth", 1);
            let result = (#body).parse_next(input);
            input.state.decrement_counter("__recursion_depth", 1);
            result
        })
    }
}

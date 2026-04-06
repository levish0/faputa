use nanachi_meta::ast::{BuiltinPredicate, CompareOp, GuardCondition};
use nanachi_meta::ir::{Boundary, CharRange, IrExpr, IrProgram};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate winnow combinator code for an IR expression.
pub(crate) fn generate_expr(expr: &IrExpr, ir: &IrProgram) -> TokenStream {
    match expr {
        IrExpr::Literal(s) => {
            quote! { literal(#s) }
        }

        IrExpr::CharSet(ranges) => generate_one_of(ranges),

        IrExpr::Any => {
            quote! { any.void() }
        }

        IrExpr::Boundary(boundary) => generate_boundary(boundary),

        IrExpr::RuleRef(idx) => {
            let name = &ir.rules[*idx].name;
            let fn_name = format_ident!("{}", name);
            quote! { #fn_name }
        }

        IrExpr::Seq(items) => {
            let codes: Vec<_> = items.iter().map(|e| generate_expr(e, ir)).collect();
            if codes.len() <= 1 {
                return quote! { (#(#codes),*) };
            }
            // Interleave position tracking between sequence elements so that
            // error positions reflect the furthest point actually reached.
            let mut interleaved: Vec<TokenStream> = Vec::with_capacity(codes.len() * 2 - 1);
            for (i, code) in codes.into_iter().enumerate() {
                if i > 0 {
                    interleaved.push(quote! {
                        (|input: &mut Input<'_, ParseState>| -> ModalResult<()> {
                            input.state.track_pos(input.current_token_start());
                            Ok(())
                        })
                    });
                }
                interleaved.push(code);
            }
            // winnow tuple limit is 21 elements. With interleaving, N items
            // become 2N-1 elements, so up to N=11 stays within limits.
            // For larger sequences, use explicit sequential parsing.
            if interleaved.len() <= 21 {
                quote! { (#(#interleaved),*) }
            } else {
                quote! {
                    (|input: &mut Input<'_, ParseState>| -> ModalResult<()> {
                        #(
                            (#interleaved).void().parse_next(input)?;
                        )*
                        Ok(())
                    })
                }
            }
        }

        IrExpr::Choice(items) => {
            let codes: Vec<_> = items
                .iter()
                .map(|e| {
                    let code = generate_expr(e, ir);
                    quote! { (#code).void() }
                })
                .collect();
            generate_alt(codes)
        }

        IrExpr::Repeat { expr, min, max } => generate_repeat(expr, *min, *max, ir),

        IrExpr::PosLookahead(inner) => {
            let inner_code = generate_expr(inner, ir);
            quote! { peek(#inner_code) }
        }

        IrExpr::NegLookahead(inner) => {
            let inner_code = generate_expr(inner, ir);
            quote! { not(#inner_code) }
        }

        IrExpr::WithFlag { flag, body } => {
            let body_code = generate_expr(body, ir);
            quote! {
                (|input: &mut Input<'_, ParseState>| {
                    let prev = input.state.get_flag(#flag);
                    input.state.set_flag(#flag, true);
                    let result = (#body_code).void().parse_next(input);
                    input.state.set_flag(#flag, prev);
                    result
                })
            }
        }

        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => {
            let amount = *amount as usize;
            let body_code = generate_expr(body, ir);
            quote! {
                (|input: &mut Input<'_, ParseState>| {
                    input.state.increment_counter(#counter, #amount);
                    let result = (#body_code).void().parse_next(input);
                    input.state.decrement_counter(#counter, #amount);
                    result
                })
            }
        }

        IrExpr::When { condition, body } => {
            let condition_check = generate_condition_check(condition);
            let body_code = generate_expr(body, ir);
            quote! {
                (|input: &mut Input<'_, ParseState>| {
                    if #condition_check {
                        (#body_code).void().parse_next(input)
                    } else {
                        Ok(())
                    }
                })
            }
        }

        IrExpr::DepthLimit { limit, body } => {
            let limit = *limit as usize;
            let body_code = generate_expr(body, ir);
            quote! {
                (|input: &mut Input<'_, ParseState>| {
                    let depth = input.state.get_counter("__recursion_depth");
                    if depth >= #limit {
                        return Err(winnow::error::ErrMode::Backtrack(
                            winnow::error::ContextError::new(),
                        ));
                    }
                    input.state.increment_counter("__recursion_depth", 1);
                    let result = (#body_code).void().parse_next(input);
                    input.state.decrement_counter("__recursion_depth", 1);
                    result
                })
            }
        }

        IrExpr::TakeWhile { ranges, min, max } => generate_take_while(ranges, *min, *max),

        IrExpr::Labeled { expr, label } => {
            let inner = generate_expr(expr, ir);
            quote! { (#inner).context(StrContext::Expected(StrContextValue::Description(#label))) }
        }
    }
}

/// Generate `one_of(...)` for a set of character ranges.
fn generate_one_of(ranges: &[CharRange]) -> TokenStream {
    if ranges.len() == 1 {
        let r = &ranges[0];
        if r.start == r.end {
            let ch = r.start;
            quote! { one_of(#ch) }
        } else {
            let start = r.start;
            let end = r.end;
            quote! { one_of(#start..=#end) }
        }
    } else if ranges.len() <= 10 {
        let range_tokens = generate_range_tuple(ranges);
        quote! { one_of(#range_tokens) }
    } else {
        let closure = generate_char_match_closure(ranges);
        quote! { one_of(#closure) }
    }
}

/// Generate `take_while(range, set)` for fused char-class repeats.
fn generate_take_while(ranges: &[CharRange], min: u32, max: Option<u32>) -> TokenStream {
    let range_expr = generate_repeat_range(min, max);
    if ranges.len() <= 10 {
        let set = generate_range_tuple(ranges);
        quote! { take_while(#range_expr, #set) }
    } else {
        let closure = generate_char_match_closure(ranges);
        quote! { take_while(#range_expr, #closure) }
    }
}

/// Generate a tuple of ranges for winnow's `ContainsToken`.
/// e.g., `('a'..='z', 'A'..='Z', '_')` or just `'a'..='z'`
fn generate_range_tuple(ranges: &[CharRange]) -> TokenStream {
    let parts: Vec<_> = ranges
        .iter()
        .map(|r| {
            if r.start == r.end {
                let ch = r.start;
                quote! { #ch }
            } else {
                let start = r.start;
                let end = r.end;
                quote! { #start..=#end }
            }
        })
        .collect();
    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        quote! { (#(#parts),*) }
    }
}

/// Generate a closure for char matching when ranges exceed tuple limit.
/// e.g., `|c: char| matches!(c, 'a'..='z' | 'A'..='Z' | '_')`
fn generate_char_match_closure(ranges: &[CharRange]) -> TokenStream {
    let patterns: Vec<_> = ranges
        .iter()
        .map(|r| {
            if r.start == r.end {
                let ch = r.start;
                quote! { #ch }
            } else {
                let start = r.start;
                let end = r.end;
                quote! { #start..=#end }
            }
        })
        .collect();
    quote! { |c: char| matches!(c, #(#patterns)|*) }
}

fn generate_boundary(boundary: &Boundary) -> TokenStream {
    match boundary {
        Boundary::Soi => {
            quote! {
                (|input: &mut Input<'_, ParseState>| {
                    if input.current_token_start() == 0 {
                        Ok(())
                    } else {
                        Err(winnow::error::ErrMode::Backtrack(
                            winnow::error::ContextError::new(),
                        ))
                    }
                })
            }
        }
        Boundary::Eoi => {
            quote! { eof.void() }
        }
        Boundary::LineStart => {
            quote! {
                (|input: &mut Input<'_, ParseState>| {
                    let pos = input.current_token_start();
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
        Boundary::LineEnd => {
            quote! {
                (|input: &mut Input<'_, ParseState>| {
                    let pos = input.current_token_start();
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

fn generate_repeat(expr: &IrExpr, min: u32, max: Option<u32>, ir: &IrProgram) -> TokenStream {
    let inner = generate_expr(expr, ir);
    let fold = quote! { .fold(|| (), |(), _| ()) };
    let range = generate_repeat_range(min, max);

    if min == 0 && max == Some(1) {
        quote! { opt(#inner) }
    } else {
        quote! { repeat(#range, #inner)#fold }
    }
}

/// Generate a winnow range expression for repeat/take_while.
fn generate_repeat_range(min: u32, max: Option<u32>) -> TokenStream {
    let min = min as usize;
    match max {
        None => quote! { #min.. },
        Some(m) if m as usize == min => {
            let m = m as usize;
            quote! { #m }
        }
        Some(m) => {
            let m = m as usize;
            quote! { #min..=#m }
        }
    }
}

fn generate_condition_check(condition: &GuardCondition) -> TokenStream {
    match condition {
        GuardCondition::NotFlag(name) => quote! { !input.state.get_flag(#name) },
        GuardCondition::IsFlag(name) => quote! { input.state.get_flag(#name) },
        GuardCondition::Builtin(BuiltinPredicate::Soi) => {
            quote! { input.current_token_start() == 0 }
        }
        GuardCondition::Builtin(BuiltinPredicate::Eoi) => quote! { input.input.is_empty() },
        GuardCondition::Builtin(BuiltinPredicate::LineStart) => {
            quote! { input.state.is_at_line_start(input.current_token_start()) }
        }
        GuardCondition::Builtin(BuiltinPredicate::LineEnd) => {
            quote! { input.state.is_at_line_end(input.current_token_start()) }
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

/// Generate nested `alt()` calls, chunking into groups of 21 to stay within
/// winnow's tuple size limit.
fn generate_alt(items: Vec<TokenStream>) -> TokenStream {
    const MAX: usize = 21;
    if items.len() <= MAX {
        quote! { alt((#(#items),*)) }
    } else {
        let chunks: Vec<_> = items
            .chunks(MAX)
            .map(|chunk| {
                let chunk = chunk.to_vec();
                quote! { alt((#(#chunk),*)) }
            })
            .collect();
        generate_alt(chunks)
    }
}

use faputa_meta::ast::{BuiltinPredicate, CompareOp, GuardCondition};
use faputa_meta::hir::{Boundary, CharRange};
use faputa_meta::mir::{DispatchArm, MirExpr, MirProgram};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate winnow combinator code for an IR expression.
pub(crate) fn generate_expr(expr: &MirExpr, ir: &MirProgram) -> TokenStream {
    match expr {
        MirExpr::Literal(s) => {
            quote! { literal(#s) }
        }

        MirExpr::CharSet(ranges) => generate_one_of(ranges),

        MirExpr::Any => {
            quote! { any.void() }
        }

        MirExpr::Boundary(boundary) => generate_boundary(boundary),

        MirExpr::RuleRef(idx) => {
            let name = &ir.rules[*idx].name;
            let fn_name = format_ident!("{}", name);
            quote! { #fn_name }
        }

        MirExpr::Seq(items) => {
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
                        (|input: &mut Input<'i, ParseState<'i>>| -> ModalResult<()> {
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
                    (|input: &mut Input<'i, ParseState<'i>>| -> ModalResult<()> {
                        #(
                            (#interleaved).void().parse_next(input)?;
                        )*
                        Ok(())
                    })
                }
            }
        }

        MirExpr::Choice(items) => {
            let codes: Vec<_> = items
                .iter()
                .map(|e| {
                    let code = generate_expr(e, ir);
                    quote! { (#code).void() }
                })
                .collect();
            generate_alt(codes)
        }

        MirExpr::Dispatch(arms) => generate_dispatch(arms, ir),

        MirExpr::Repeat { expr, min, max } => generate_repeat(expr, *min, *max, ir),

        MirExpr::Scan {
            plain_ranges,
            specials,
            min,
        } => generate_scan(plain_ranges, specials, *min, ir),

        MirExpr::PosLookahead(inner) => {
            let inner_code = generate_expr(inner, ir);
            quote! { peek(#inner_code) }
        }

        MirExpr::NegLookahead(inner) => {
            let inner_code = generate_expr(inner, ir);
            quote! { not(#inner_code) }
        }

        MirExpr::WithFlag { flag, body } => {
            let body_code = generate_expr(body, ir);
            quote! {
                (|input: &mut Input<'i, ParseState<'i>>| {
                    let prev = input.state.get_flag(#flag);
                    input.state.set_flag(#flag, true);
                    let result = (#body_code).void().parse_next(input);
                    input.state.set_flag(#flag, prev);
                    result
                })
            }
        }

        MirExpr::WithCounter {
            counter,
            amount,
            body,
        } => {
            let amount = *amount as usize;
            let body_code = generate_expr(body, ir);
            quote! {
                (|input: &mut Input<'i, ParseState<'i>>| {
                    input.state.increment_counter(#counter, #amount);
                    let result = (#body_code).void().parse_next(input);
                    input.state.decrement_counter(#counter, #amount);
                    result
                })
            }
        }

        MirExpr::When { condition, body } => {
            let condition_check = generate_condition_check(condition);
            let body_code = generate_expr(body, ir);
            quote! {
                (|input: &mut Input<'i, ParseState<'i>>| {
                    if #condition_check {
                        (#body_code).void().parse_next(input)
                    } else {
                        Ok(())
                    }
                })
            }
        }

        MirExpr::DepthLimit { limit, body } => {
            let limit = *limit as usize;
            let body_code = generate_expr(body, ir);
            quote! {
                (|input: &mut Input<'i, ParseState<'i>>| {
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

        MirExpr::TakeWhile { ranges, min, max } => generate_take_while(ranges, *min, *max),

        MirExpr::SeparatedList { first, rest } => generate_separated_list(first, rest, ir),

        MirExpr::Labeled { expr, label } => {
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
    let patterns = generate_match_patterns(ranges);
    quote! { |c: char| matches!(c, #(#patterns)|*) }
}

fn generate_match_patterns(ranges: &[CharRange]) -> Vec<TokenStream> {
    ranges
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
        .collect()
}

fn generate_boundary(boundary: &Boundary) -> TokenStream {
    match boundary {
        Boundary::Soi => {
            quote! {
                (|input: &mut Input<'i, ParseState<'i>>| {
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
                (|input: &mut Input<'i, ParseState<'i>>| {
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
                (|input: &mut Input<'i, ParseState<'i>>| {
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

fn generate_repeat(expr: &MirExpr, min: u32, max: Option<u32>, ir: &MirProgram) -> TokenStream {
    let inner = generate_expr(expr, ir);
    let fold = quote! { .fold(|| (), |(), _| ()) };
    let range = generate_repeat_range(min, max);

    if min == 0 && max == Some(1) {
        quote! { opt(#inner) }
    } else {
        quote! { repeat(#range, #inner)#fold }
    }
}

fn generate_dispatch(arms: &[DispatchArm], ir: &MirProgram) -> TokenStream {
    let arms: Vec<_> = arms
        .iter()
        .map(|arm| {
            let patterns = generate_match_patterns(&arm.ranges);
            let body = generate_expr(&arm.expr, ir);
            quote! {
                Some(ch) if matches!(ch, #(#patterns)|*) => (#body).void().parse_next(input),
            }
        })
        .collect();

    quote! {
        (|input: &mut Input<'i, ParseState<'i>>| -> ModalResult<()> {
            match input.input.chars().next() {
                #(#arms)*
                _ => Err(winnow::error::ErrMode::Backtrack(
                    winnow::error::ContextError::new(),
                )),
            }
        })
    }
}

fn generate_scan(
    plain_ranges: &[CharRange],
    specials: &[DispatchArm],
    min: u32,
    ir: &MirProgram,
) -> TokenStream {
    let bulk = generate_scan_bulk_parser(plain_ranges);
    let arms: Vec<_> = specials
        .iter()
        .map(|arm| {
            let patterns = generate_match_patterns(&arm.ranges);
            let body = generate_expr(&arm.expr, ir);
            quote! {
                Some(ch) if matches!(ch, #(#patterns)|*) => {
                    (#body).void().parse_next(input)?;
                }
            }
        })
        .collect();

    match min {
        0 => quote! {
            (|input: &mut Input<'i, ParseState<'i>>| -> ModalResult<()> {
                loop {
                    let chunk = (#bulk).parse_next(input)?;
                    if !chunk.is_empty() {
                        continue;
                    }
                    match input.input.chars().next() {
                        #(#arms)*
                        _ => return Ok(()),
                    }
                }
            })
        },
        1 => quote! {
            (|input: &mut Input<'i, ParseState<'i>>| -> ModalResult<()> {
                let mut matched_any = false;
                loop {
                    let chunk = (#bulk).parse_next(input)?;
                    if !chunk.is_empty() {
                        matched_any = true;
                        continue;
                    }
                    match input.input.chars().next() {
                        #(#arms)*
                        _ => {
                            return if matched_any {
                                Ok(())
                            } else {
                                Err(winnow::error::ErrMode::Backtrack(
                                    winnow::error::ContextError::new(),
                                ))
                            };
                        }
                    }
                    matched_any = true;
                }
            })
        },
        _ => {
            let min = min as usize;
            quote! {
                (|input: &mut Input<'i, ParseState<'i>>| -> ModalResult<()> {
                    let mut matched = 0usize;
                    loop {
                        let chunk = (#bulk).parse_next(input)?;
                        if !chunk.is_empty() {
                            matched += chunk.chars().count();
                            continue;
                        }
                        match input.input.chars().next() {
                            #(#arms)*
                            _ => {
                                return if matched >= #min {
                                    Ok(())
                                } else {
                                    Err(winnow::error::ErrMode::Backtrack(
                                        winnow::error::ContextError::new(),
                                    ))
                                };
                            }
                        }
                        matched += 1;
                    }
                })
            }
        }
    }
}

fn generate_separated_list(first: &MirExpr, rest: &MirExpr, ir: &MirProgram) -> TokenStream {
    let first_code = generate_expr(first, ir);
    let rest_code = generate_expr(rest, ir);
    quote! {
        (|input: &mut Input<'i, ParseState<'i>>| -> ModalResult<()> {
            (#first_code).void().parse_next(input)?;
            loop {
                let checkpoint = input.checkpoint();
                match (#rest_code).void().parse_next(input) {
                    Ok(()) => {}
                    Err(winnow::error::ErrMode::Backtrack(_)) => {
                        input.reset(&checkpoint);
                        return Ok(());
                    }
                    Err(err) => return Err(err),
                }
            }
        })
    }
}

fn generate_scan_bulk_parser(plain_ranges: &[CharRange]) -> TokenStream {
    let stop_ranges = invert_ranges(plain_ranges);
    if !stop_ranges.is_empty() && stop_ranges.len() <= 10 {
        let set = generate_range_tuple(&stop_ranges);
        quote! { take_till(0.., #set) }
    } else {
        let range_expr = generate_repeat_range(0, None);
        if plain_ranges.len() <= 10 {
            let set = generate_range_tuple(plain_ranges);
            quote! { take_while(#range_expr, #set) }
        } else {
            let closure = generate_char_match_closure(plain_ranges);
            quote! { take_while(#range_expr, #closure) }
        }
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

fn invert_ranges(ranges: &[CharRange]) -> Vec<CharRange> {
    if ranges.is_empty() {
        return vec![CharRange::new(char::MIN, char::MAX)];
    }

    let mut result = Vec::new();
    let mut cursor = char::MIN;

    for range in ranges {
        if cursor < range.start {
            if let Some(end) = prev_scalar(range.start) {
                if cursor <= end {
                    result.push(CharRange::new(cursor, end));
                }
            }
        }

        cursor = match next_scalar(range.end) {
            Some(next) => next,
            None => return result,
        };
    }

    if cursor <= char::MAX {
        result.push(CharRange::new(cursor, char::MAX));
    }

    result
}

fn next_scalar(ch: char) -> Option<char> {
    let mut value = ch as u32 + 1;
    while value <= char::MAX as u32 {
        if let Some(next) = char::from_u32(value) {
            return Some(next);
        }
        value += 1;
    }
    None
}

fn prev_scalar(ch: char) -> Option<char> {
    let mut value = ch as u32;
    while value > 0 {
        value -= 1;
        if let Some(prev) = char::from_u32(value) {
            return Some(prev);
        }
    }
    None
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

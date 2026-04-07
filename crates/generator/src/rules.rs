use faputa_meta::mir::{MirProgram, MirRule};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::expr::generate_expr;
use crate::statement::generate_statements;

/// Generate a function for each non-inlined rule in the program.
pub(crate) fn generate_rules(ir: &MirProgram) -> TokenStream {
    let fns: Vec<_> = ir
        .rules
        .iter()
        .map(|rule| generate_rule(rule, ir))
        .collect();

    quote! { #(#fns)* }
}

fn generate_rule(rule: &MirRule, ir: &MirProgram) -> TokenStream {
    let fn_name = format_ident!("{}", rule.name);
    let label = rule.error_label.as_deref().unwrap_or(&rule.name);

    tracing::trace!(
        rule = %rule.name,
        entry_point = rule.is_entry_point,
        needs_context = rule.needs_context,
        needs_trace = rule.needs_trace,
        label = %label,
        inline = rule.inline,
        "generating rule"
    );

    let guard_code = generate_statements(&rule.guards, &rule.emits);
    let expr_code = generate_expr(&rule.expr, ir);

    let has_statements = !rule.guards.is_empty() || !rule.emits.is_empty();

    if rule.needs_trace {
        // Entry point: keep trace and surfaced rule label context.
        debug_assert!(rule.needs_context);
        if has_statements {
            quote! {
                fn #fn_name<'i>(input: &mut Input<'i, ParseState<'i>>) -> ModalResult<()> {
                    input.track_pos(input.current_token_start());
                    winnow::combinator::trace(#label, |input: &mut Input<'i, ParseState<'i>>| {
                        #guard_code
                        (#expr_code).void().parse_next(input)
                    })
                    .context(StrContext::Label(#label))
                    .parse_next(input)
                }
            }
        } else {
            quote! {
                fn #fn_name<'i>(input: &mut Input<'i, ParseState<'i>>) -> ModalResult<()> {
                    input.track_pos(input.current_token_start());
                    winnow::combinator::trace(#label, (#expr_code).void())
                        .context(StrContext::Label(#label))
                        .parse_next(input)
                }
            }
        }
    } else if rule.needs_context {
        // Explicitly labeled helper rule: preserve user-authored context.
        if has_statements {
            quote! {
                fn #fn_name<'i>(input: &mut Input<'i, ParseState<'i>>) -> ModalResult<()> {
                    input.track_pos(input.current_token_start());
                    (|input: &mut Input<'i, ParseState<'i>>| {
                        #guard_code
                        (#expr_code).void().parse_next(input)
                    })
                    .context(StrContext::Label(#label))
                    .parse_next(input)
                }
            }
        } else {
            quote! {
                fn #fn_name<'i>(input: &mut Input<'i, ParseState<'i>>) -> ModalResult<()> {
                    input.track_pos(input.current_token_start());
                    (#expr_code).void()
                        .context(StrContext::Label(#label))
                        .parse_next(input)
                }
            }
        }
    } else {
        // Internal helper rule: avoid per-call context wrappers on the hot path.
        if has_statements {
            quote! {
                fn #fn_name<'i>(input: &mut Input<'i, ParseState<'i>>) -> ModalResult<()> {
                    input.track_pos(input.current_token_start());
                    (|input: &mut Input<'i, ParseState<'i>>| {
                        #guard_code
                        (#expr_code).void().parse_next(input)
                    })
                    .parse_next(input)
                }
            }
        } else {
            quote! {
                fn #fn_name<'i>(input: &mut Input<'i, ParseState<'i>>) -> ModalResult<()> {
                    input.track_pos(input.current_token_start());
                    (#expr_code).void()
                        .parse_next(input)
                }
            }
        }
    }
}

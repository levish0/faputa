use nanachi_meta::ir::{IrProgram, IrRule};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::expr::generate_expr;
use crate::statement::generate_statements;

/// Generate a function for each non-inlined rule in the program.
pub(crate) fn generate_rules(ir: &IrProgram) -> TokenStream {
    let fns: Vec<_> = ir
        .rules
        .iter()
        .map(|rule| generate_rule(rule, ir))
        .collect();

    quote! { #(#fns)* }
}

fn generate_rule(rule: &IrRule, ir: &IrProgram) -> TokenStream {
    let fn_name = format_ident!("{}", rule.name);
    let rule_name = &rule.name;
    let is_entry_point = rule.ref_count == 0;

    let guard_code = generate_statements(&rule.guards, &rule.emits);
    let expr_code = generate_expr(&rule.expr, ir);

    let has_statements = !rule.guards.is_empty() || !rule.emits.is_empty();

    if is_entry_point {
        // Entry point: full trace + context + track_pos
        if has_statements {
            quote! {
                fn #fn_name<'i>(input: &mut Input<'i, ParseState>) -> ModalResult<&'i str> {
                    input.state.track_pos(input.current_token_start());
                    winnow::combinator::trace(#rule_name, |input: &mut Input<'i, ParseState>| {
                        #guard_code
                        (#expr_code).take().parse_next(input)
                    })
                    .context(StrContext::Label(#rule_name))
                    .parse_next(input)
                }
            }
        } else {
            quote! {
                fn #fn_name<'i>(input: &mut Input<'i, ParseState>) -> ModalResult<&'i str> {
                    input.state.track_pos(input.current_token_start());
                    winnow::combinator::trace(#rule_name, (#expr_code).take())
                        .context(StrContext::Label(#rule_name))
                        .parse_next(input)
                }
            }
        }
    } else {
        // Internal rule: no trace, no context, no track_pos
        if has_statements {
            quote! {
                fn #fn_name<'i>(input: &mut Input<'i, ParseState>) -> ModalResult<&'i str> {
                    (|input: &mut Input<'i, ParseState>| {
                        #guard_code
                        (#expr_code).take().parse_next(input)
                    }).parse_next(input)
                }
            }
        } else {
            quote! {
                fn #fn_name<'i>(input: &mut Input<'i, ParseState>) -> ModalResult<&'i str> {
                    (#expr_code).take().parse_next(input)
                }
            }
        }
    }
}

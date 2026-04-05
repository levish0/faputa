use nanachi_meta::ast::{Grammar, Item, RuleDef};
use proc_macro2::TokenStream;
use quote::quote;

use crate::expr::{generate_expr, rule_fn_ident};
use crate::statement::generate_statements;

/// Generate a function for each rule in the grammar.
pub(crate) fn generate_rules(grammar: &Grammar) -> TokenStream {
    let fns: Vec<_> = grammar
        .items
        .iter()
        .filter_map(|item| match item {
            Item::RuleDef(rule) => {
                let basic = generate_rule(rule, false);
                let detailed = generate_rule(rule, true);
                Some(quote! {
                    #basic
                    #detailed
                })
            }
            _ => None,
        })
        .collect();

    quote! { #(#fns)* }
}

fn generate_rule(rule: &RuleDef, detailed: bool) -> TokenStream {
    let fn_name = rule_fn_ident(&rule.name, detailed);
    let rule_name = &rule.name;

    let guard_code = generate_statements(&rule.body.statements);
    let expr_code = generate_expr(&rule.body.expr, detailed);

    let has_statements = !rule.body.statements.is_empty();

    // Each rule returns &'i str via .take() — the matched span of input.
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
}

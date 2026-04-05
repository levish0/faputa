mod expr;
mod rules;
mod state;
mod statement;

use nanachi_meta::ast::{Grammar, Item};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

fn generate_module_inner(grammar: &Grammar) -> TokenStream {
    let state_code = state::generate_state(grammar);
    let rules_code = rules::generate_rules(grammar);
    let entry_code = generate_entry(grammar);

    quote::quote! {
        use nanachi::winnow;
        use nanachi::winnow::prelude::*;
        use nanachi::winnow::combinator::*;
        use nanachi::winnow::token::*;
        use nanachi::winnow::stream::Location;
        use nanachi::{Input, State};

        #state_code
        #rules_code
        #entry_code
    }
}

/// Generate parser code as `pub mod __nanachi { ... }` (for build.rs).
pub fn generate(grammar: &Grammar) -> TokenStream {
    let inner = generate_module_inner(grammar);
    quote::quote! {
        #[allow(dead_code, unused_imports, unused_variables)]
        pub mod __nanachi {
            #inner
        }
    }
}

/// Generate parser code as `#[doc(hidden)] mod <name> { ... }` (for derive).
pub fn generate_with_mod(grammar: &Grammar, mod_name: &proc_macro2::Ident) -> TokenStream {
    let inner = generate_module_inner(grammar);
    quote::quote! {
        #[doc(hidden)]
        #[allow(dead_code, unused_imports, unused_variables)]
        mod #mod_name {
            #inner
        }
    }
}

/// Generate `pub fn parse_<rule>(source: &str) -> Result<&str, String>` for each rule.
fn generate_entry(grammar: &Grammar) -> TokenStream {
    let entries: Vec<_> = grammar
        .items
        .iter()
        .filter_map(|item| match item {
            Item::RuleDef(rule) => {
                let parse_fn = format_ident!("parse_{}", rule.name);
                let rule_fn = format_ident!("{}", rule.name);
                Some(quote! {
                    pub fn #parse_fn(source: &str) -> Result<&str, String> {
                        let state = ParseState::new(source);
                        let mut input = Input {
                            input: nanachi::LocatingSlice::new(source),
                            state,
                        };
                        let matched = #rule_fn.parse_next(&mut input)
                            .map_err(|e| format!("{e}"))?;
                        if !input.input.is_empty() {
                            return Err(format!(
                                "unexpected trailing input at position {}",
                                input.current_token_start()
                            ));
                        }
                        Ok(matched)
                    }
                })
            }
            _ => None,
        })
        .collect();

    quote! { #(#entries)* }
}

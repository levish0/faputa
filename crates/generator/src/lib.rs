mod expr;
mod rules;
mod state;
mod statement;

use nanachi_meta::ast::{Grammar, Item};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate Rust + winnow parser code from a validated nanachi grammar.
pub fn generate(grammar: &Grammar) -> TokenStream {
    let state_code = state::generate_state(grammar);
    let rules_code = rules::generate_rules(grammar);
    let entry_code = generate_entry(grammar);

    quote::quote! {
        mod __nanachi {
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
}

/// Generate `pub fn parse(input: &str) -> Result<(), String>` using the first rule as entry point.
fn generate_entry(grammar: &Grammar) -> TokenStream {
    let first_rule = grammar.items.iter().find_map(|item| match item {
        Item::RuleDef(rule) => Some(&rule.name),
        _ => None,
    });

    let Some(entry_name) = first_rule else {
        return quote! {};
    };

    let entry_fn = format_ident!("{}", entry_name);

    quote! {
        pub fn parse(source: &str) -> Result<(), String> {
            let state = ParseState::new(source);
            let mut input = Input {
                input: nanachi::LocatingSlice::new(source),
                state,
            };
            #entry_fn.parse_next(&mut input).map_err(|e| format!("{e}"))?;
            if !input.input.is_empty() {
                return Err(format!(
                    "unexpected trailing input at position {}",
                    input.input.location()
                ));
            }
            Ok(())
        }
    }
}

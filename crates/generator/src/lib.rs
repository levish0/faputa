mod expr;
mod rules;
mod state;
mod statement;

use nanachi_meta::ast::Grammar;
use nanachi_meta::ir::{self, IrProgram};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[tracing::instrument(skip_all)]
fn generate_module_inner(grammar: &Grammar) -> TokenStream {
    let ir = ir::lower(grammar);
    let ir = ir::optimize(ir);

    let state_code = state::generate_state(&ir);
    let rules_code = rules::generate_rules(&ir);
    let entry_code = generate_entry(&ir);
    tracing::debug!(rules = ir.rules.len(), "code generation complete");
    quote::quote! {
        use nanachi::winnow;
        use nanachi::winnow::prelude::*;
        use nanachi::winnow::combinator::*;
        use nanachi::winnow::token::*;
        use nanachi::winnow::stream::Location;
        use nanachi::winnow::error::{StrContext, StrContextValue};
        use nanachi::{Input, LineIndex, State};

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
fn generate_entry(ir: &IrProgram) -> TokenStream {
    let entries: Vec<_> = ir
        .rules
        .iter()
        .map(|rule| {
            let parse_fn = format_ident!("parse_{}", rule.name);
            let rule_fn = format_ident!("{}", rule.name);
            quote! {
                pub fn #parse_fn(source: &str) -> Result<&str, String> {
                    let state = ParseState::new(source);
                    let mut input = Input {
                        input: nanachi::LocatingSlice::new(source),
                        state,
                    };
                    let matched = #rule_fn.parse_next(&mut input)
                        .map_err(|e| {
                            let offset = input.state.furthest_pos();
                            let line_index = LineIndex::new(source);
                            let (line, col) = line_index.line_col(offset);
                            let inner = match e {
                                winnow::error::ErrMode::Backtrack(c)
                                | winnow::error::ErrMode::Cut(c) => format!("{c}"),
                                _ => format!("{e}"),
                            };
                            if inner.is_empty() {
                                format!("parse error at {line}:{col}")
                            } else {
                                format!("parse error at {line}:{col}: {inner}")
                            }
                        })?;
                    if !input.input.is_empty() {
                        let offset = input.current_token_start();
                        let line_index = LineIndex::new(source);
                        let (line, col) = line_index.line_col(offset);
                        return Err(format!(
                            "unexpected trailing input at {line}:{col}"
                        ));
                    }
                    Ok(matched)
                }
            }
        })
        .collect();

    quote! { #(#entries)* }
}

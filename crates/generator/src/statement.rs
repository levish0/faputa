use nanachi_meta::ast::{BuiltinPredicate, CompareOp, EmitStmt, GuardCondition, Statement};
use proc_macro2::TokenStream;
use quote::quote;

/// Generate code for a list of statements (guards, emits) that precede a rule's expression.
pub(crate) fn generate_statements(statements: &[Statement]) -> TokenStream {
    let stmts: Vec<_> = statements.iter().map(generate_statement).collect();
    quote! { #(#stmts)* }
}

fn generate_statement(stmt: &Statement) -> TokenStream {
    match stmt {
        Statement::Guard(guard) => generate_guard(&guard.condition),
        Statement::Emit(emit) => generate_emit(emit),
    }
}

fn generate_guard(condition: &GuardCondition) -> TokenStream {
    match condition {
        GuardCondition::NotFlag(name) => {
            quote! {
                if input.state.get_flag(#name) {
                    return Err(winnow::error::ErrMode::Backtrack(
                        winnow::error::ContextError::new(),
                    ));
                }
            }
        }
        GuardCondition::IsFlag(name) => {
            quote! {
                if !input.state.get_flag(#name) {
                    return Err(winnow::error::ErrMode::Backtrack(
                        winnow::error::ContextError::new(),
                    ));
                }
            }
        }
        GuardCondition::Builtin(builtin) => generate_builtin_guard(builtin),
        GuardCondition::Compare { name, op, value } => {
            let value = *value as usize;
            let comparison = match op {
                CompareOp::Eq => quote! { counter == #value },
                CompareOp::Ne => quote! { counter != #value },
                CompareOp::Lt => quote! { counter < #value },
                CompareOp::Le => quote! { counter <= #value },
                CompareOp::Gt => quote! { counter > #value },
                CompareOp::Ge => quote! { counter >= #value },
            };
            quote! {
                {
                    let counter = input.state.get_counter(#name);
                    if !(#comparison) {
                        return Err(winnow::error::ErrMode::Backtrack(
                            winnow::error::ContextError::new(),
                        ));
                    }
                }
            }
        }
    }
}

fn generate_builtin_guard(builtin: &BuiltinPredicate) -> TokenStream {
    match builtin {
        BuiltinPredicate::Soi => {
            quote! {
                if input.input.location() != 0 {
                    return Err(winnow::error::ErrMode::Backtrack(
                        winnow::error::ContextError::new(),
                    ));
                }
            }
        }
        BuiltinPredicate::LineStart => {
            quote! {
                {
                    let pos = input.input.location();
                    if !input.state.is_at_line_start(pos) {
                        return Err(winnow::error::ErrMode::Backtrack(
                            winnow::error::ContextError::new(),
                        ));
                    }
                }
            }
        }
        // EOI, ANY, LineEnd as guards don't make much sense but handle gracefully
        BuiltinPredicate::Eoi => {
            quote! {
                if !input.input.is_empty() {
                    return Err(winnow::error::ErrMode::Backtrack(
                        winnow::error::ContextError::new(),
                    ));
                }
            }
        }
        BuiltinPredicate::LineEnd => {
            quote! {
                {
                    let pos = input.input.location();
                    if !input.state.is_at_line_end(pos) {
                        return Err(winnow::error::ErrMode::Backtrack(
                            winnow::error::ContextError::new(),
                        ));
                    }
                }
            }
        }
        BuiltinPredicate::Any => {
            // guard ANY always passes (there's always "any" possible)
            quote! {}
        }
    }
}

fn generate_emit(emit: &EmitStmt) -> TokenStream {
    let name = &emit.counter;
    quote! {
        input.state.increment_counter(#name, 1);
    }
}

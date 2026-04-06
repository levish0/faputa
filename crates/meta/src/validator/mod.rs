mod errors;
mod rules;
mod state;

pub use errors::ValidationError;

use crate::ast::Grammar;

/// Validate a parsed grammar for semantic correctness.
#[tracing::instrument(skip_all)]
pub fn validate(grammar: &Grammar) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    let ctx = rules::collect_definitions(grammar, &mut errors);
    rules::check_references(grammar, &ctx, &mut errors);
    state::check_state_usage(grammar, &ctx, &mut errors);

    if errors.is_empty() {
        tracing::debug!("validation passed");
        Ok(())
    } else {
        tracing::warn!(error_count = errors.len(), "validation failed");
        Err(errors)
    }
}

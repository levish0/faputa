pub mod ast;
pub mod hir;
pub mod lexer;
pub mod mir;
pub mod parser;
pub mod validator;

/// Parse and validate a `.faputa` source string.
///
/// This is a convenience wrapper that runs [`parser::parse`] followed by
/// [`validator::validate`].  Callers that need finer control (e.g. showing
/// an AST even when validation fails) should call those functions directly.
#[tracing::instrument(skip_all, fields(source_len = source.len()))]
pub fn compile(source: &str) -> Result<ast::Grammar, CompileError> {
    let grammar = parser::parse(source).map_err(CompileError::Parse)?;
    tracing::debug!(
        rules = grammar
            .items
            .iter()
            .filter(|i| matches!(i, ast::Item::RuleDef(_)))
            .count(),
        "parsed grammar"
    );
    validator::validate(&grammar).map_err(CompileError::Validation)?;
    tracing::debug!("validation passed");
    Ok(grammar)
}

#[derive(Debug)]
pub enum CompileError {
    Parse(parser::ParseError),
    Validation(Vec<validator::ValidationError>),
}

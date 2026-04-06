pub mod ast;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod validator;

/// Parse and validate a `.nanachi` source string.
///
/// This is a convenience wrapper that runs [`parser::parse`] followed by
/// [`validator::validate`].  Callers that need finer control (e.g. showing
/// an AST even when validation fails) should call those functions directly.
pub fn compile(source: &str) -> Result<ast::Grammar, CompileError> {
    let grammar = parser::parse(source).map_err(CompileError::Parse)?;
    validator::validate(&grammar).map_err(CompileError::Validation)?;
    Ok(grammar)
}

#[derive(Debug)]
pub enum CompileError {
    Parse(parser::ParseError),
    Validation(Vec<validator::ValidationError>),
}

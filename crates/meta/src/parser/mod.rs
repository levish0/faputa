mod error;
mod expr;
mod grammar;
mod state;
mod statement;
mod tokens;

pub use error::ParseError;

use crate::ast::Grammar;

/// Parse a .nanachi source string into a Grammar AST.
#[tracing::instrument(skip_all)]
pub fn parse(source: &str) -> Result<Grammar, ParseError> {
    let mut parser = tokens::TokenStream::new(source)?;
    let grammar = grammar::parse_grammar(&mut parser)?;
    tracing::debug!(items = grammar.items.len(), "parse complete");
    Ok(grammar)
}

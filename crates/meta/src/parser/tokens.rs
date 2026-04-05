use crate::lexer::Token;
use logos::Logos;

use super::error::ParseError;

/// Wrapper around the logos token stream with position tracking.
pub(crate) struct TokenStream<'src> {
    tokens: Vec<(Token<'src>, std::ops::Range<usize>)>,
    pos: usize,
}

impl<'src> TokenStream<'src> {
    pub fn new(source: &'src str) -> Self {
        let tokens: Vec<_> = Token::lexer(source)
            .spanned()
            .filter_map(|(tok, span)| tok.ok().map(|t| (t, span)))
            .collect();
        TokenStream { tokens, pos: 0 }
    }

    pub fn peek(&self) -> Option<&Token<'src>> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    pub fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map(|(_, span)| span.start)
            .unwrap_or(0)
    }

    pub fn advance(&mut self) -> Option<Token<'src>> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].0.clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    pub fn skip_newlines(&mut self) {
        while self.peek() == Some(&Token::Newline) {
            self.advance();
        }
    }

    pub fn expect(&mut self, expected: &Token<'_>) -> Result<(), ParseError> {
        match self.peek() {
            Some(tok) if tok == expected => {
                self.advance();
                Ok(())
            }
            other => Err(self.error(format!("expected {expected:?}, got {other:?}"))),
        }
    }

    pub fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Some(Token::Ident(name)) => Ok(name.to_string()),
            other => Err(self.error(format!("expected identifier, got {other:?}"))),
        }
    }

    pub fn expect_number(&mut self) -> Result<u32, ParseError> {
        match self.advance() {
            Some(Token::Number(n)) => Ok(n),
            other => Err(self.error(format!("expected number, got {other:?}"))),
        }
    }

    pub fn error(&self, message: String) -> ParseError {
        ParseError {
            message,
            offset: self.offset(),
        }
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    pub fn save(&self) -> usize {
        self.pos
    }

    pub fn restore(&mut self, pos: usize) {
        self.pos = pos;
    }
}

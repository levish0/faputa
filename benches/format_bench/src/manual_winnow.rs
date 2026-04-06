use faputa::winnow;
use faputa::winnow::combinator::{eof, opt, repeat};
use faputa::winnow::error::{ContextError, ErrMode};
use faputa::winnow::prelude::*;
use faputa::winnow::token::{literal, one_of, take_till, take_while};

fn ws(input: &mut &str) -> ModalResult<()> {
    take_while(0.., (' ', '\t', '\n', '\r'))
        .void()
        .parse_next(input)
}

fn hex(input: &mut &str) -> ModalResult<char> {
    one_of(('0'..='9', 'a'..='f', 'A'..='F')).parse_next(input)
}

fn unicode_escape(input: &mut &str) -> ModalResult<()> {
    ('u', hex, hex, hex, hex).void().parse_next(input)
}

fn escape(input: &mut &str) -> ModalResult<()> {
    (
        '\\',
        winnow::combinator::alt((
            one_of(('"', '\\', '/', 'b', 'f', 'n', 'r', 't')).void(),
            unicode_escape,
        )),
    )
        .void()
        .parse_next(input)
}

fn string(input: &mut &str) -> ModalResult<()> {
    '"'.void().parse_next(input)?;

    loop {
        take_till(0.., ['"', '\\']).void().parse_next(input)?;

        match input.chars().next() {
            Some('"') => {
                '"'.void().parse_next(input)?;
                return Ok(());
            }
            Some('\\') => escape(input)?,
            _ => return Err(ErrMode::Backtrack(ContextError::new())),
        }
    }
}

fn integer(input: &mut &str) -> ModalResult<()> {
    opt('-').void().parse_next(input)?;

    match input.chars().next() {
        Some('0') => '0'.void().parse_next(input),
        Some('1'..='9') => (one_of('1'..='9'), take_while(0.., '0'..='9'))
            .void()
            .parse_next(input),
        _ => Err(ErrMode::Backtrack(ContextError::new())),
    }
}

fn fraction(input: &mut &str) -> ModalResult<()> {
    ('.', take_while(1.., '0'..='9')).void().parse_next(input)
}

fn exponent(input: &mut &str) -> ModalResult<()> {
    (
        one_of(('e', 'E')),
        opt(one_of(('+', '-'))),
        take_while(1.., '0'..='9'),
    )
        .void()
        .parse_next(input)
}

fn number(input: &mut &str) -> ModalResult<()> {
    (integer, opt(fraction), opt(exponent))
        .void()
        .parse_next(input)
}

fn array_items(input: &mut &str) -> ModalResult<()> {
    value.parse_next(input)?;
    repeat(0.., (ws, ',', ws, value))
        .fold(|| (), |(), _| ())
        .parse_next(input)
}

fn array(input: &mut &str) -> ModalResult<()> {
    ('[', ws, opt(array_items), ws, ']')
        .void()
        .parse_next(input)
}

fn pair(input: &mut &str) -> ModalResult<()> {
    (ws, string, ws, ':', ws, value).void().parse_next(input)
}

fn object_items(input: &mut &str) -> ModalResult<()> {
    pair.parse_next(input)?;
    repeat(0.., (ws, ',', ws, pair))
        .fold(|| (), |(), _| ())
        .parse_next(input)
}

fn object(input: &mut &str) -> ModalResult<()> {
    ('{', ws, opt(object_items), ws, '}')
        .void()
        .parse_next(input)
}

fn value(input: &mut &str) -> ModalResult<()> {
    ws.parse_next(input)?;

    match input.chars().next() {
        Some('{') => object.parse_next(input)?,
        Some('[') => array.parse_next(input)?,
        Some('"') => string.parse_next(input)?,
        Some('-' | '0'..='9') => number.parse_next(input)?,
        Some('t') => literal("true").void().parse_next(input)?,
        Some('f') => literal("false").void().parse_next(input)?,
        Some('n') => literal("null").void().parse_next(input)?,
        _ => return Err(ErrMode::Backtrack(ContextError::new())),
    }

    ws.parse_next(input)
}

fn json(input: &mut &str) -> ModalResult<()> {
    (ws, value, ws, eof).void().parse_next(input)
}

pub fn parse_json(source: &str) -> Result<(), String> {
    let mut input = source;
    json.parse_next(&mut input).map_err(|e| e.to_string())
}

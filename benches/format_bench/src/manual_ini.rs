use faputa::winnow::combinator::{alt, eof, repeat};
use faputa::winnow::error::{ContextError, ErrMode};
use faputa::winnow::prelude::*;
use faputa::winnow::token::take_while;

fn line_ending(input: &mut &str) -> ModalResult<()> {
    alt(("\r\n", "\n")).void().parse_next(input)
}

fn name(input: &mut &str) -> ModalResult<()> {
    take_while(1.., ('0'..='9', 'a'..='z', 'A'..='Z', '.', '_', '/'))
        .void()
        .parse_next(input)
}

fn value(input: &mut &str) -> ModalResult<()> {
    take_while(0.., ('0'..='9', 'a'..='z', 'A'..='Z', '.', '_', '/'))
        .void()
        .parse_next(input)
}

fn spaces0(input: &mut &str) -> ModalResult<()> {
    take_while(0.., ' ').void().parse_next(input)
}

fn section(input: &mut &str) -> ModalResult<()> {
    ('[', spaces0, name, spaces0, ']').void().parse_next(input)
}

fn property(input: &mut &str) -> ModalResult<()> {
    (name, spaces0, '=', spaces0, value)
        .void()
        .parse_next(input)
}

fn line(input: &mut &str) -> ModalResult<()> {
    spaces0.parse_next(input)?;

    match input.chars().next() {
        Some('\r' | '\n') => {}
        Some('[') => section.parse_next(input)?,
        Some('0'..='9' | 'a'..='z' | 'A'..='Z' | '.' | '_' | '/') => property.parse_next(input)?,
        _ => return Err(ErrMode::Backtrack(ContextError::new())),
    }

    spaces0.parse_next(input)?;
    line_ending.parse_next(input)
}

fn file(input: &mut &str) -> ModalResult<()> {
    repeat(0.., line)
        .fold(|| (), |(), _| ())
        .parse_next(input)?;
    eof.parse_next(input)
}

pub fn parse(source: &str) -> Result<(), String> {
    let mut input = source;
    file.parse_next(&mut input).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_sample() {
        let input = "[section_0]\nkey_0=path/value_0\n\n";
        parse(input).expect("ini parser should accept simple section/property input");
    }

    #[test]
    fn rejects_bad_header() {
        let err = parse("[section\n").unwrap_err();
        assert!(!err.is_empty());
    }
}

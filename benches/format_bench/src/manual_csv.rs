use faputa::winnow::combinator::{alt, eof, repeat};
use faputa::winnow::prelude::*;
use faputa::winnow::token::take_while;

fn line_ending(input: &mut &str) -> ModalResult<()> {
    alt(("\r\n", "\n")).void().parse_next(input)
}

fn field(input: &mut &str) -> ModalResult<()> {
    take_while(1.., ('0'..='9', '.', '-'))
        .void()
        .parse_next(input)
}

fn record(input: &mut &str) -> ModalResult<()> {
    field.parse_next(input)?;
    repeat(0.., (',', field))
        .fold(|| (), |(), _| ())
        .parse_next(input)
}

fn file(input: &mut &str) -> ModalResult<()> {
    repeat(0.., (record, line_ending))
        .fold(|| (), |(), _| ())
        .parse_next(input)?;
    eof.void().parse_next(input)
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
        let input = "1.000,2.500,-3.125\n4.000,5.000,6.000\n";
        parse(input).expect("csv parser should accept numeric records");
    }

    #[test]
    fn rejects_bad_cell() {
        let err = parse("1.0,abc\n").unwrap_err();
        assert!(!err.is_empty());
    }
}

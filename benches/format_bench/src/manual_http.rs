use faputa::winnow::combinator::{alt, eof, repeat};
use faputa::winnow::error::{ContextError, ErrMode};
use faputa::winnow::prelude::*;
use faputa::winnow::token::{literal, take_till, take_while};

fn line_ending(input: &mut &str) -> ModalResult<()> {
    alt(("\r\n", "\n")).void().parse_next(input)
}

fn method(input: &mut &str) -> ModalResult<()> {
    alt((
        literal("GET"),
        literal("DELETE"),
        literal("POST"),
        literal("PUT"),
    ))
    .void()
    .parse_next(input)
}

fn spaces1(input: &mut &str) -> ModalResult<()> {
    take_while(1.., ' ').void().parse_next(input)
}

fn hspace1(input: &mut &str) -> ModalResult<()> {
    take_while(1.., (' ', '\t')).void().parse_next(input)
}

fn uri(input: &mut &str) -> ModalResult<()> {
    take_till(1.., [' ', '\t', '\r', '\n'])
        .void()
        .parse_next(input)
}

fn version(input: &mut &str) -> ModalResult<()> {
    take_while(1.., ('0'..='9', '.')).void().parse_next(input)
}

fn header_name(input: &mut &str) -> ModalResult<()> {
    take_till(1.., [':', '\r', '\n']).void().parse_next(input)
}

fn header_value(input: &mut &str) -> ModalResult<()> {
    take_till(1.., ['\r', '\n']).void().parse_next(input)
}

fn request_line(input: &mut &str) -> ModalResult<()> {
    (
        method,
        spaces1,
        uri,
        spaces1,
        literal("HTTP/"),
        version,
        line_ending,
    )
        .void()
        .parse_next(input)
}

fn header(input: &mut &str) -> ModalResult<()> {
    (header_name, ':', hspace1, header_value, line_ending)
        .void()
        .parse_next(input)
}

fn request(input: &mut &str) -> ModalResult<()> {
    request_line.parse_next(input)?;

    loop {
        let remaining = *input;
        if remaining.starts_with("\r\n") || remaining.starts_with('\n') {
            break;
        }
        if remaining.is_empty() {
            return Err(ErrMode::Backtrack(ContextError::new()));
        }
        header.parse_next(input)?;
    }

    line_ending.parse_next(input)
}

fn delimiter(input: &mut &str) -> ModalResult<()> {
    repeat(1.., line_ending)
        .fold(|| (), |(), _| ())
        .parse_next(input)
}

fn http(input: &mut &str) -> ModalResult<()> {
    while !input.is_empty() {
        let remaining = *input;
        if remaining.starts_with("\r\n") || remaining.starts_with('\n') {
            delimiter.parse_next(input)?;
        } else {
            request.parse_next(input)?;
        }
    }

    eof.void().parse_next(input)
}

pub fn parse(source: &str) -> Result<(), String> {
    let mut input = source;
    http.parse_next(&mut input).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_sample() {
        let input = concat!(
            "GET /api/v1/items/1?page=1 HTTP/1.1\r\n",
            "Host: bench.example\r\n",
            "User-Agent: faputa-bench-1\r\n",
            "\r\n"
        );
        parse(input).expect("http parser should accept simple request blocks");
    }

    #[test]
    fn rejects_missing_blank_line() {
        let err = parse("GET /x HTTP/1.1\r\nHost: a\r\n").unwrap_err();
        assert!(!err.is_empty());
    }
}

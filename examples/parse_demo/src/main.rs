use nanachi_derive::Parser;
use std::env;
use std::fs;
use std::process;

#[derive(Parser)]
#[grammar("grammar.nanachi")]
struct Demo;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        format!("{manifest}/sample.txt")
    });

    let source = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {path}: {e}");
        process::exit(1);
    });

    for (i, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let lineno = i + 1;
        match Demo::parse_assign(line) {
            Ok(matched) => println!("  line {lineno}: OK  {matched}"),
            Err(e) => println!("  line {lineno}: ERR {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Demo;

    #[test]
    fn sample_lines_parse() {
        for line in include_str!("../sample.txt").lines() {
            assert_eq!(Demo::parse_assign(line).unwrap(), line);
        }
    }

    #[test]
    fn assign_rejects_missing_rhs() {
        Demo::parse_assign("x=").unwrap_err();
    }

    #[test]
    fn assign_rejects_digit_lhs() {
        Demo::parse_assign("1x=2").unwrap_err();
    }

    #[test]
    fn parse_error_reports_location() {
        let err = Demo::parse_assign("1x=2").unwrap_err();

        assert!(err.starts_with("parse error at 1:1:"));
        assert!(err.contains("invalid alpha"));
    }

    #[test]
    fn trailing_input_reports_location() {
        let err = Demo::parse_ident("x!").unwrap_err();

        assert_eq!(err, "unexpected trailing input at 1:2");
    }

    #[test]
    fn ident_and_number_rules_work() {
        assert_eq!(Demo::parse_ident("hello123").unwrap(), "hello123");
        assert_eq!(Demo::parse_number("42").unwrap(), "42");
    }
}

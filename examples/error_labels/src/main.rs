use nanachi_derive::Parser;
use std::env;
use std::fs;
use std::process;

#[derive(Parser)]
#[grammar("grammar.nanachi")]
struct ErrorLabels;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
        .with_writer(std::io::stderr)
        .init();

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
        match ErrorLabels::parse_program(line) {
            Ok(matched) => println!("  line {lineno}: OK  {matched}"),
            Err(e) => println!("  line {lineno}: ERR {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorLabels;

    #[test]
    fn valid_assign_number() {
        assert_eq!(ErrorLabels::parse_program("x=42").unwrap(), "x=42");
    }

    #[test]
    fn valid_assign_ident() {
        assert_eq!(ErrorLabels::parse_program("x=y").unwrap(), "x=y");
    }

    #[test]
    fn valid_assign_string() {
        assert_eq!(
            ErrorLabels::parse_program(r#"x="hello""#).unwrap(),
            r#"x="hello""#
        );
    }

    #[test]
    fn valid_assign_float() {
        assert_eq!(ErrorLabels::parse_program("x=3.14").unwrap(), "x=3.14");
    }

    #[test]
    fn error_missing_equals() {
        let err = ErrorLabels::parse_program("x+42").unwrap_err();
        assert!(err.contains("parse error at 1:"));
    }

    #[test]
    fn error_missing_value() {
        let err = ErrorLabels::parse_program("x=").unwrap_err();
        assert!(err.contains("parse error at 1:"));
    }

    #[test]
    fn error_bad_start() {
        let err = ErrorLabels::parse_program("123=x").unwrap_err();
        assert!(err.contains("parse error at 1:"));
    }
}

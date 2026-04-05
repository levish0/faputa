use nanachi_derive::Parser;
use std::env;
use std::fs;
use std::process;

#[derive(Parser)]
#[grammar("grammar.nanachi")]
struct Json;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        // Default to bundled sample
        let manifest = env!("CARGO_MANIFEST_DIR");
        format!("{manifest}/sample.json")
    });

    let source = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {path}: {e}");
        process::exit(1);
    });

    match Json::parse_json(&source) {
        Ok(matched) => {
            println!("Valid JSON ({} bytes)", matched.len());
            println!("{matched}");
        }
        Err(e) => {
            eprintln!("Invalid JSON: {e}");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Json;

    #[test]
    fn sample_json_parses() {
        let sample = include_str!("../sample.json");
        assert_eq!(Json::parse_json(sample).unwrap(), sample);
    }

    #[test]
    fn json_accepts_scalar_with_whitespace() {
        assert_eq!(Json::parse_json(" \n null \n ").unwrap(), " \n null \n ");
    }

    #[test]
    fn json_rejects_trailing_comma() {
        Json::parse_json(r#"{"a":1,}"#).unwrap_err();
    }

    #[test]
    fn json_rejects_unclosed_object() {
        Json::parse_json(r#"{"a":1"#).unwrap_err();
    }

    #[test]
    fn json_parse_error_reports_multiline_location() {
        let err = Json::parse_json("{\n  \"a\": 1,\n}").unwrap_err();

        assert!(err.starts_with("parse error at 3:1:"));
    }

    #[test]
    fn string_trailing_input_reports_location() {
        let err = Json::parse_string("\"x\"!").unwrap_err();

        assert_eq!(err, "unexpected trailing input at 1:4");
    }

    #[test]
    fn string_rule_accepts_escapes() {
        assert_eq!(
            Json::parse_string(r#""line\nbreak""#).unwrap(),
            r#""line\nbreak""#
        );
        assert_eq!(Json::parse_string(r#""\u0041""#).unwrap(), r#""\u0041""#);
    }
}

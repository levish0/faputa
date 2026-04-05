use nanachi_meta::validator::ValidationError;
use nanachi_meta::{CompileError, compile};

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to find workspace root")
}

fn valid_fixture_source(name: &str) -> String {
    let path = workspace_root().join(format!("fixtures/valid/{name}.nanachi"));
    let path = path.display().to_string();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn invalid_fixture_source(name: &str) -> String {
    let path = workspace_root().join(format!("fixtures/invalid/{name}.nanachi"));
    let path = path.display().to_string();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn syntax_invalid_fixture_source(name: &str) -> String {
    let path = workspace_root().join(format!("fixtures/syntax_invalid/{name}.nanachi"));
    let path = path.display().to_string();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn compile_valid_fixture() {
    let source = valid_fixture_source("nested_formatting");
    let grammar = compile(&source).unwrap();
    assert_eq!(grammar.items.len(), 11);
}

#[test]
fn compile_valid_dirty_fixture() {
    let source = valid_fixture_source("chaos_combo");
    let grammar = compile(&source).unwrap();
    assert_eq!(grammar.items.len(), 8);
}

#[test]
fn compile_reports_parse_errors_before_validation() {
    let err = compile(r#"entry = { "x" @ "y" }"#).unwrap_err();
    match err {
        CompileError::Parse(parse_err) => {
            assert_eq!(parse_err.offset, 14);
            assert!(parse_err.message.contains("unexpected character '@'"));
        }
        other => panic!("expected parse error, got {other:?}"),
    }
}

#[test]
fn compile_reports_validation_errors() {
    let err = compile(
        r#"
let flag inside
entry = {
    emit inside
    missing_rule
}
"#,
    )
    .unwrap_err();

    match err {
        CompileError::Validation(errors) => {
            assert!(errors.iter().any(|e| matches!(
                e,
                ValidationError::ExpectedCounter { name, used_in }
                    if name == "inside" && used_in == "entry"
            )));
            assert!(errors.iter().any(|e| matches!(
                e,
                ValidationError::UndefinedRule { name, used_in }
                    if name == "missing_rule" && used_in == "entry"
            )));
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn compile_accumulates_many_validation_errors() {
    let err = compile(
        r#"
let flag seen
let flag seen

entry = {
    emit seen
    with missing {
        when seen > 0 { missing_rule }
    }
}

entry = { with seen += 1 { other_missing } }
"#,
    )
    .unwrap_err();

    match err {
        CompileError::Validation(errors) => {
            assert!(errors.len() >= 6);
            assert!(
                errors.iter().any(
                    |e| matches!(e, ValidationError::DuplicateState { name } if name == "seen")
                )
            );
            assert!(
                errors.iter().any(
                    |e| matches!(e, ValidationError::DuplicateRule { name } if name == "entry")
                )
            );
            assert!(errors.iter().any(|e| matches!(
                e,
                ValidationError::UndefinedState { name, .. } if name == "missing"
            )));
            assert!(errors.iter().any(|e| matches!(
                e,
                ValidationError::UndefinedRule { name, .. } if name == "missing_rule"
            )));
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn compile_reports_validation_errors_from_fixture() {
    let source = invalid_fixture_source("many_errors");
    let err = compile(&source).unwrap_err();

    match err {
        CompileError::Validation(errors) => {
            assert!(errors.iter().any(|e| matches!(
                e,
                ValidationError::DuplicateState { name } if name == "seen"
            )));
            assert!(errors.iter().any(|e| matches!(
                e,
                ValidationError::UndefinedState { name, .. } if name == "missing"
            )));
            assert!(errors.iter().any(|e| matches!(
                e,
                ValidationError::UndefinedRule { name, .. } if name == "missing_rule"
            )));
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn compile_reports_parse_errors_from_syntax_fixture() {
    let source = syntax_invalid_fixture_source("unsupported_state_kind");
    let err = compile(&source).unwrap_err();

    match err {
        CompileError::Parse(parse_err) => {
            assert_eq!(parse_err.offset, 10);
            assert!(parse_err.message.contains("expected 'flag' or 'counter'"));
        }
        other => panic!("expected parse error, got {other:?}"),
    }
}

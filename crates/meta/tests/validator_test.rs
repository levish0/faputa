use faputa_meta::parser;
use faputa_meta::validator::{self, ValidationError};

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to find workspace root")
}

fn read_fixture_source(path: &std::path::Path) -> String {
    let path = path.display().to_string();
    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    source.replace("\r\n", "\n").replace('\r', "\n")
}

fn validate_valid(name: &str) {
    let path = workspace_root().join(format!("fixtures/valid/{name}.faputa"));
    let path_str = path.display().to_string();
    let source = read_fixture_source(&path);
    let grammar = parser::parse(&source).unwrap_or_else(|e| panic!("{path_str}: {e}"));
    if let Err(errors) = validator::validate(&grammar) {
        panic!("{path_str}: validation errors: {errors:?}");
    }
}

fn validate_invalid(name: &str) -> Vec<ValidationError> {
    let path = workspace_root().join(format!("fixtures/invalid/{name}.faputa"));
    let path_str = path.display().to_string();
    let source = read_fixture_source(&path);
    let grammar = parser::parse(&source).unwrap_or_else(|e| panic!("{path_str}: {e}"));
    validator::validate(&grammar).expect_err(&format!("{name} should have validation errors"))
}

fn validate_invalid_source(source: &str) -> Vec<ValidationError> {
    let grammar = parser::parse(source).unwrap();
    validator::validate(&grammar).expect_err("source should have validation errors")
}

// ── Valid grammars should pass ──

#[test]
fn valid_basic_rules() {
    validate_valid("basic_rules");
}

#[test]
fn valid_stateful_bold() {
    validate_valid("stateful_bold");
}

#[test]
fn valid_nested_formatting() {
    validate_valid("nested_formatting");
}

#[test]
fn valid_depth_and_braces() {
    validate_valid("depth_and_braces");
}

#[test]
fn valid_when_conditional() {
    validate_valid("when_conditional");
}

#[test]
fn valid_chaos_combo() {
    validate_valid("chaos_combo");
}

#[test]
fn valid_stateful_extensions() {
    validate_valid("stateful_extensions");
}

// ── Invalid grammars should produce errors ──

#[test]
fn invalid_undefined_rule() {
    let errors = validate_invalid("undefined_rule");
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::UndefinedRule { name, .. } if name == "b"
    )));
}

#[test]
fn invalid_undefined_state() {
    let errors = validate_invalid("undefined_state");
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::UndefinedState { name, .. } if name == "inside_bold"
    )));
}

#[test]
fn invalid_wrong_state_kind() {
    let errors = validate_invalid("wrong_state_kind");
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::ExpectedCounter { name, .. } if name == "inside_bold"
    )));
}

#[test]
fn invalid_duplicates() {
    let errors = validate_invalid("duplicates");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::DuplicateState { name } if name == "x"))
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::DuplicateRule { name } if name == "a"))
    );
}

#[test]
fn invalid_nested_state_misuse_fixture() {
    let errors = validate_invalid("nested_state_misuse");
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::ExpectedFlag { name, used_in } if name == "depth" && used_in == "entry"
    )));
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::ExpectedCounter { name, used_in } if name == "inside" && used_in == "entry"
    )));
}

#[test]
fn invalid_many_errors_fixture() {
    let errors = validate_invalid("many_errors");
    assert!(errors.len() >= 6);
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::DuplicateState { name } if name == "seen"))
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::DuplicateRule { name } if name == "entry"))
    );
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::UndefinedState { name, used_in } if name == "missing" && used_in == "entry"
    )));
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::UndefinedRule { name, used_in } if name == "missing_rule" && used_in == "entry"
    )));
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::UndefinedRule { name, used_in } if name == "other_missing" && used_in == "entry"
    )));
}

#[test]
fn invalid_nested_state_usage_inside_stateful_expressions() {
    let errors = validate_invalid_source(
        r#"
let flag inside
let counter depth
inner = { "x" }
entry = { with depth { when inside > 0 { inner } } }
"#,
    );

    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::ExpectedFlag { name, used_in } if name == "depth" && used_in == "entry"
    )));
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::ExpectedCounter { name, used_in } if name == "inside" && used_in == "entry"
    )));
}

#[test]
fn invalid_many_errors_are_accumulated() {
    let errors = validate_invalid_source(
        r#"
let flag seen
let flag seen

entry = {
    inc seen
    with missing {
        when seen > 0 { missing_rule }
    }
}

entry = { with seen += 1 { other_missing } }
"#,
    );

    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::DuplicateState { name } if name == "seen"))
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::DuplicateRule { name } if name == "entry"))
    );
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::ExpectedCounter { name, used_in } if name == "seen" && used_in == "entry"
    )));
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::UndefinedState { name, used_in } if name == "missing" && used_in == "entry"
    )));
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::UndefinedRule { name, used_in } if name == "missing_rule" && used_in == "entry"
    )));
    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::UndefinedRule { name, used_in } if name == "other_missing" && used_in == "entry"
    )));
}

#[test]
fn valid_nested_state_usage_passes() {
    let grammar = parser::parse(
        r#"
let flag inside
let counter depth
inner = { "x" }
entry = { depth_limit(3) { with inside { when depth > 0 { inner } } } }
"#,
    )
    .unwrap();

    validator::validate(&grammar).unwrap();
}

#[test]
fn invalid_numeric_expr_counter_refs_must_be_counters() {
    let errors = validate_invalid_source(
        r#"
let flag enabled
let counter depth
entry = {
    if depth >= enabled { "x" } else { "y" }
}
"#,
    );

    assert!(errors.iter().any(|e| matches!(
        e,
        ValidationError::ExpectedCounter { name, used_in } if name == "enabled" && used_in == "entry"
    )));
}

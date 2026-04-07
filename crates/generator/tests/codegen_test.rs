use faputa_meta::parser;
use faputa_meta::validator;

/// Helper: parse + validate + generate, return the generated code as a string.
fn generate_code(source: &str) -> String {
    let grammar = parser::parse(source).expect("parse failed");
    validator::validate(&grammar).expect("validation failed");
    let tokens = faputa_generator::generate(&grammar);
    tokens.to_string()
}

#[test]
fn generates_empty_state_for_no_declarations() {
    let code = generate_code("greeting = { \"hello\" \"world\" }");
    assert!(code.contains("struct ParseState"));
    assert!(code.contains("impl < 'i > State for ParseState < 'i >"));
    assert!(code.contains("fn greeting"));
}

#[test]
fn generates_flag_fields() {
    let code = generate_code(
        r#"
        let flag inside_bold
        let flag inside_italic
        bold = { "**" "text" "**" }
    "#,
    );
    assert!(code.contains("inside_bold : bool"));
    assert!(code.contains("inside_italic : bool"));
}

#[test]
fn generates_counter_fields() {
    let code = generate_code(
        r#"
        let counter depth
        let counter section_counter
        doc = { "text" }
    "#,
    );
    assert!(code.contains("depth : usize"));
    assert!(code.contains("section_counter : usize"));
}

#[test]
fn generates_string_literal() {
    let code = generate_code(r#"hello = { "hello" }"#);
    assert!(code.contains(r#"literal ("hello")"#));
}

#[test]
fn generates_error_context() {
    let code = generate_code(r#"hello = { "hello" }"#);
    assert!(code.contains("parse_hello"));
    assert!(code.contains("StrContext"));
    // Entry-point rule gets Label context (not per-terminal Expected)
    assert!(code.contains("Label (\"hello\")"));
}

#[test]
fn generates_rule_level_error_label() {
    let code = generate_code(r#"value = @ "JSON value" { "x" }"#);
    assert!(code.contains("Label (\"JSON value\")"));
    assert!(code.contains("trace (\"JSON value\""));
    assert!(!code.contains("Label (\"value\")"));
}

#[test]
fn generates_char_range() {
    let code = generate_code("alpha = { 'a'..'z' }");
    assert!(code.contains("one_of"));
}

#[test]
fn generates_sequence() {
    let code = generate_code(r#"pair = { "a" "b" }"#);
    // Adjacent single-char literals get fused into one literal "ab"
    assert!(code.contains(r#"literal ("ab")"#));
}

#[test]
fn generates_choice() {
    let code = generate_code(r#"ab = { "a" | "b" }"#);
    // Single-char choices are merged into a single CharSet → one_of
    assert!(code.contains("one_of"));
}

#[test]
fn generates_repetition_variants() {
    let code = generate_code(
        r#"
        r = { "a"+ "b"* "c"? "d"{3} "e"{1,5} }
    "#,
    );
    assert!(code.contains("repeat (1")); // 1.. for +
    assert!(code.contains("checkpoint")); // unbounded non-nullable repeat lowers to direct loop
    assert!(code.contains("opt")); // ?
}

#[test]
fn generates_lookahead() {
    let code = generate_code(r#"la = { &"a" !"b" }"#);
    assert!(code.contains("peek"));
    assert!(code.contains("not"));
}

#[test]
fn generates_guard_code() {
    let code = generate_code(
        r#"
        let flag inside_bold
        bold = {
            guard !inside_bold
            "**" "text" "**"
        }
    "#,
    );
    assert!(code.contains("get_flag"));
    assert!(code.contains("inside_bold"));
}

#[test]
fn generates_with_flag_code() {
    let code = generate_code(
        r#"
        let flag inside_bold
        bold = {
            with inside_bold {
                "**" "text" "**"
            }
        }
    "#,
    );
    assert!(code.contains("set_flag"));
    assert!(code.contains("get_flag"));
}

#[test]
fn generates_with_increment_code() {
    let code = generate_code(
        r#"
        let counter depth
        nested = {
            with depth += 1 {
                "(" ")"
            }
        }
    "#,
    );
    assert!(code.contains("increment_counter"));
    assert!(code.contains("decrement_counter"));
}

#[test]
fn generates_emit_code() {
    let code = generate_code(
        r##"
        let counter section_counter
        header = {
            emit section_counter
            "#" "text"
        }
    "##,
    );
    assert!(code.contains("increment_counter"));
    assert!(code.contains("section_counter"));
}

#[test]
fn generates_when_code() {
    let code = generate_code(
        r#"
        let counter depth
        conditional = {
            when depth > 0 {
                "nested"
            }
            "base"
        }
    "#,
    );
    assert!(code.contains("get_counter"));
}

#[test]
fn generates_if_code() {
    let code = generate_code(
        r#"
        let counter depth
        let counter limit
        branch = {
            if depth >= limit {
                "x"
            } else {
                "y"
            }
        }
    "#,
    );
    assert!(code.contains("if input . state . get_counter (\"depth\") >="));
    assert!(code.contains("else"));
}

#[test]
fn generates_measure_code() {
    let code = generate_code(
        r#"
        let counter width
        fence = {
            measure width { "`"{3,} }
        }
    "#,
    );
    assert!(code.contains("matched . chars () . count ()"));
    assert!(code.contains("set_counter (\"width\""));
}

#[test]
fn generates_dynamic_repeat_code() {
    let code = generate_code(
        r#"
        let counter width
        fence = {
            "x"{width,}
        }
    "#,
    );
    assert!(code.contains("let min = input . state . get_counter (\"width\")"));
    assert!(code.contains("let checkpoint = input . checkpoint ()"));
}

#[test]
fn generates_depth_limit_code() {
    let code = generate_code(
        r#"
        block = {
            depth_limit(64) {
                "(" ")"
            }
        }
    "#,
    );
    assert!(code.contains("__recursion_depth"));
    assert!(code.contains("64"));
}

#[test]
fn generates_rule_references() {
    let code = generate_code(
        r#"
        alpha = { 'a'..'z' }
        ident = { alpha alpha* }
    "#,
    );
    assert!(code.contains("fn alpha"));
    assert!(code.contains("fn ident"));
    assert!(code.contains("alpha"));
}

#[test]
fn skips_default_context_for_internal_helper_rules() {
    let code = generate_code(
        r#"
        word = { ('a'..'z' | 'A'..'Z')+ }
        pair = { word word }
    "#,
    );
    assert!(code.contains("Label (\"pair\")"));
    assert!(!code.contains("Label (\"word\")"));
}

#[test]
fn preserves_explicit_context_for_internal_helper_rules() {
    let code = generate_code(
        r#"
        word = @ "word" { ('a'..'z' | 'A'..'Z')+ }
        pair = { word word }
    "#,
    );
    assert!(code.contains("Label (\"pair\")"));
    assert!(code.contains("Label (\"word\")"));
}

#[test]
fn generates_builtin_exprs() {
    let code = generate_code(
        r#"
        start = { SOI "begin" }
        finish = { "end" EOI }
        anything = { ANY }
    "#,
    );
    // SOI is now a closure checking position == 0
    assert!(code.contains("current_token_start () == 0"));
    assert!(code.contains("eof"));
    assert!(code.contains("any"));
}

#[test]
fn generates_nested_alt_for_large_choice() {
    // 25 branches — should produce nested alt() calls
    let branches: Vec<_> = (0..25).map(|i| format!(r#""x{i}""#)).collect();
    let choice = branches.join(" | ");
    let source = format!("big = {{ {choice} }}");
    let code = generate_code(&source);

    // Should have multiple alt( calls due to chunking
    let alt_count = code.matches("alt (").count();
    assert!(
        alt_count >= 2,
        "expected nested alt calls for 25 branches, got {alt_count} alt() calls"
    );
}

// ── Simple grammar: identifiers ──

mod simple {
    include!(concat!(env!("OUT_DIR"), "/example_simple.rs"));
}

#[test]
fn simple_parses_single_letter() {
    simple::__nanachi::parse("a").unwrap();
}

#[test]
fn simple_parses_uppercase() {
    simple::__nanachi::parse("Z").unwrap();
}

#[test]
fn simple_rejects_digit() {
    simple::__nanachi::parse("1").unwrap_err();
}

#[test]
fn simple_rejects_empty() {
    simple::__nanachi::parse("").unwrap_err();
}

// ── Stateful: bold markers ──

mod bold {
    include!(concat!(env!("OUT_DIR"), "/example_markdown_bold.rs"));
}

#[test]
fn bold_parses_plain_text() {
    bold::__nanachi::parse("x").unwrap();
}

#[test]
fn bold_parses_bold_marker() {
    // Entry point is `inline` which matches one char or bold.
    // "**x**" is bold wrapping single char — inline matches it as one unit.
    bold::__nanachi::parse("x").unwrap();
}

// ��─ Fixture: basic_rules ──

mod basic_rules {
    include!(concat!(env!("OUT_DIR"), "/fixture_basic_rules.rs"));
}

#[test]
fn basic_rules_parses_alpha() {
    basic_rules::__nanachi::parse("z").unwrap();
}

#[test]
fn basic_rules_rejects_digit() {
    basic_rules::__nanachi::parse("5").unwrap_err();
}

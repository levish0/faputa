// ── Simple grammar: identifiers ──

mod simple {
    include!(concat!(env!("OUT_DIR"), "/example_simple.rs"));
}

#[test]
fn simple_alpha_single_letter() {
    assert_eq!(simple::__nanachi::parse_alpha("a").unwrap(), "a");
    assert_eq!(simple::__nanachi::parse_alpha("Z").unwrap(), "Z");
}

#[test]
fn simple_alpha_rejects_digit() {
    simple::__nanachi::parse_alpha("1").unwrap_err();
}

#[test]
fn simple_ident_multi_char() {
    assert_eq!(
        simple::__nanachi::parse_ident("hello123").unwrap(),
        "hello123"
    );
}

#[test]
fn simple_ident_single_letter() {
    assert_eq!(simple::__nanachi::parse_ident("x").unwrap(), "x");
}

#[test]
fn simple_ident_rejects_digit_start() {
    simple::__nanachi::parse_ident("123").unwrap_err();
}

#[test]
fn simple_ident_rejects_empty() {
    simple::__nanachi::parse_ident("").unwrap_err();
}

// ── Stateful: bold markers ──

mod bold {
    include!(concat!(env!("OUT_DIR"), "/example_markdown_bold.rs"));
}

#[test]
fn bold_text_matches_non_star() {
    assert_eq!(bold::__nanachi::parse_text("x").unwrap(), "x");
}

#[test]
fn bold_text_rejects_double_star() {
    bold::__nanachi::parse_text("**").unwrap_err();
}

#[test]
fn bold_parses_bold_single_char() {
    assert_eq!(bold::__nanachi::parse_bold("**x**").unwrap(), "**x**");
}

#[test]
fn bold_parses_bold_multi_char() {
    assert_eq!(
        bold::__nanachi::parse_bold("**hello**").unwrap(),
        "**hello**"
    );
}

#[test]
fn bold_rejects_empty() {
    bold::__nanachi::parse_bold("").unwrap_err();
}

// ── Fixture: basic_rules ──

mod basic_rules {
    include!(concat!(env!("OUT_DIR"), "/fixture_basic_rules.rs"));
}

#[test]
fn basic_rules_alpha() {
    assert_eq!(basic_rules::__nanachi::parse_alpha("z").unwrap(), "z");
}

#[test]
fn basic_rules_digit() {
    assert_eq!(basic_rules::__nanachi::parse_digit("5").unwrap(), "5");
}

#[test]
fn basic_rules_ident() {
    assert_eq!(
        basic_rules::__nanachi::parse_ident("foo42").unwrap(),
        "foo42"
    );
}

#[test]
fn basic_rules_digit_rejects_alpha() {
    basic_rules::__nanachi::parse_digit("a").unwrap_err();
}

// ── Example: nested_braces ──

mod nested_braces {
    include!(concat!(env!("OUT_DIR"), "/example_nested_braces.rs"));
}

#[test]
fn nested_braces_document_accepts_empty() {
    assert_eq!(nested_braces::__nanachi::parse_document("").unwrap(), "");
}

#[test]
fn nested_braces_block_parses_simple_block() {
    assert_eq!(
        nested_braces::__nanachi::parse_block("{{{x}}}").unwrap(),
        "{{{x}}}"
    );
}

#[test]
fn nested_braces_block_rejects_unclosed_block() {
    nested_braces::__nanachi::parse_block("{{{x}}").unwrap_err();
}

// ── Fixture: stateful_bold ──

mod stateful_bold {
    include!(concat!(env!("OUT_DIR"), "/fixture_stateful_bold.rs"));
}

#[test]
fn stateful_bold_parses_text() {
    assert_eq!(stateful_bold::__nanachi::parse_text("x").unwrap(), "x");
}

#[test]
fn stateful_bold_parses_bold() {
    assert_eq!(
        stateful_bold::__nanachi::parse_bold("**x**").unwrap(),
        "**x**"
    );
}

#[test]
fn stateful_bold_text_rejects_marker() {
    stateful_bold::__nanachi::parse_text("**").unwrap_err();
}

// ── Fixture: nested_formatting ──

mod nested_formatting {
    include!(concat!(env!("OUT_DIR"), "/fixture_nested_formatting.rs"));
}

#[test]
fn nested_formatting_parses_bold() {
    assert_eq!(
        nested_formatting::__nanachi::parse_bold("**x**").unwrap(),
        "**x**"
    );
}

#[test]
fn nested_formatting_parses_header() {
    assert_eq!(
        nested_formatting::__nanachi::parse_header("## x").unwrap(),
        "## x"
    );
}

#[test]
fn nested_formatting_text_rejects_format_marker() {
    nested_formatting::__nanachi::parse_text("*").unwrap_err();
}

// ── Fixture: depth_and_braces ──

mod depth_and_braces {
    include!(concat!(env!("OUT_DIR"), "/fixture_depth_and_braces.rs"));
}

#[test]
fn depth_and_braces_parses_raw_block() {
    assert_eq!(
        depth_and_braces::__nanachi::parse_raw_block("{{{x}}}").unwrap(),
        "{{{x}}}"
    );
}

#[test]
fn depth_and_braces_parses_paragraph() {
    assert_eq!(
        depth_and_braces::__nanachi::parse_paragraph("abc").unwrap(),
        "abc"
    );
}

#[test]
fn depth_and_braces_document_rejects_empty() {
    depth_and_braces::__nanachi::parse_document("").unwrap_err();
}

// ── Fixture: when_conditional ──

mod when_conditional {
    include!(concat!(env!("OUT_DIR"), "/fixture_when_conditional.rs"));
}

#[test]
fn when_conditional_parses_plain_newline() {
    assert_eq!(
        when_conditional::__nanachi::parse_newline("\n").unwrap(),
        "\n"
    );
}

#[test]
fn when_conditional_rejects_non_newline() {
    when_conditional::__nanachi::parse_newline("x").unwrap_err();
}

// ── Fixture: chaos_combo ──

mod chaos_combo {
    include!(concat!(env!("OUT_DIR"), "/fixture_chaos_combo.rs"));
}

#[test]
fn chaos_combo_parses_name() {
    assert_eq!(chaos_combo::__nanachi::parse_name("AbC").unwrap(), "AbC");
}

#[test]
fn chaos_combo_tag_rejects_plain_angle_tag() {
    chaos_combo::__nanachi::parse_tag("<Tag>").unwrap_err();
}

#[test]
fn chaos_combo_parses_escaped_pair() {
    assert_eq!(
        chaos_combo::__nanachi::parse_escaped("\\n\"").unwrap(),
        "\\n\""
    );
}

#[test]
fn chaos_combo_text_rejects_tag_start() {
    chaos_combo::__nanachi::parse_text("<").unwrap_err();
}

#[test]
fn chaos_combo_document_parses_escaped_chunk() {
    assert_eq!(
        chaos_combo::__nanachi::parse_document("\\n\"").unwrap(),
        "\\n\""
    );
}

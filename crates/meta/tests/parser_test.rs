use faputa_meta::ast::*;
use faputa_meta::parser;

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to find workspace root")
}

fn read_fixture_source(path: &std::path::Path) -> String {
    let path = path.display().to_string();
    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    // Keep parser offsets stable across Windows and Unix checkouts.
    source.replace("\r\n", "\n").replace('\r', "\n")
}

fn parse_fixture(name: &str) -> Grammar {
    let path = workspace_root().join(format!("fixtures/valid/{name}.faputa"));
    let path_str = path.display().to_string();
    let source = read_fixture_source(&path);
    parser::parse(&source).unwrap_or_else(|e| panic!("{path_str}: {e}"))
}

fn parse_invalid_fixture(name: &str) -> faputa_meta::parser::ParseError {
    let path = workspace_root().join(format!("fixtures/syntax_invalid/{name}.faputa"));
    let source = read_fixture_source(&path);
    parser::parse(&source).unwrap_err()
}

fn parse_inline_expr(source: &str) -> Expr {
    let grammar = parser::parse(source).unwrap();
    let Item::RuleDef(rule) = &grammar.items[0] else {
        panic!("expected RuleDef");
    };
    rule.body.expr.clone()
}

fn num(value: u32) -> NumericExpr {
    NumericExpr::Literal(value)
}

// ── Fixture-based tests ──

#[test]
fn parse_basic_rules() {
    let grammar = parse_fixture("basic_rules");

    // 3 rules: alpha, digit, ident
    assert_eq!(grammar.items.len(), 3);

    let Item::RuleDef(alpha) = &grammar.items[0] else {
        panic!("expected RuleDef");
    };
    assert_eq!(alpha.name, "alpha");
    assert_eq!(
        alpha.body.expr,
        Expr::Choice(vec![Expr::CharRange('a', 'z'), Expr::CharRange('A', 'Z'),])
    );

    let Item::RuleDef(ident) = &grammar.items[2] else {
        panic!("expected RuleDef");
    };
    assert_eq!(ident.name, "ident");
    // ident = { alpha (alpha | digit)* }
    assert_eq!(
        ident.body.expr,
        Expr::Seq(vec![
            Expr::Ident("alpha".to_string()),
            Expr::Repeat {
                expr: Box::new(Expr::Group(Box::new(Expr::Choice(vec![
                    Expr::Ident("alpha".to_string()),
                    Expr::Ident("digit".to_string()),
                ])))),
                kind: RepeatKind::ZeroOrMore,
            },
        ])
    );
}

#[test]
fn parse_stateful_bold() {
    let grammar = parse_fixture("stateful_bold");

    // 1 state decl + 3 rules
    assert_eq!(grammar.items.len(), 4);

    assert_eq!(
        grammar.items[0],
        Item::StateDecl(StateDecl {
            kind: StateKind::Flag,
            name: "inside_bold".to_string(),
        })
    );

    let Item::RuleDef(bold) = &grammar.items[2] else {
        panic!("expected RuleDef");
    };
    assert_eq!(bold.name, "bold");
    assert_eq!(bold.body.statements.len(), 1);
    assert_eq!(
        bold.body.statements[0],
        Statement::Guard(GuardStmt {
            condition: GuardCondition::NotFlag("inside_bold".to_string()),
        })
    );

    let Expr::With(with) = &bold.body.expr else {
        panic!("expected With expr");
    };
    assert_eq!(with.flag, "inside_bold");
}

#[test]
fn parse_nested_formatting() {
    let grammar = parse_fixture("nested_formatting");

    // 5 state decls + 6 rules = 11 items
    assert_eq!(grammar.items.len(), 11);

    // Check header has emit + guard LINE_START
    let Item::RuleDef(header) = &grammar.items[9] else {
        panic!("expected RuleDef for header");
    };
    assert_eq!(header.name, "header");
    assert_eq!(header.body.statements.len(), 3); // guard LINE_START, guard !inside_header, emit

    assert_eq!(
        header.body.statements[0],
        Statement::Guard(GuardStmt {
            condition: GuardCondition::Builtin(BuiltinPredicate::LineStart),
        })
    );
    assert_eq!(
        header.body.statements[2],
        Statement::Emit(EmitStmt {
            counter: "section_counter".to_string(),
        })
    );
}

#[test]
fn parse_depth_and_braces() {
    let grammar = parse_fixture("depth_and_braces");

    // 1 state decl + 5 rules (document, block, raw_block, raw_content, paragraph)
    assert_eq!(grammar.items.len(), 6);

    // document uses depth_limit
    let Item::RuleDef(doc) = &grammar.items[1] else {
        panic!("expected RuleDef");
    };
    assert_eq!(doc.name, "document");
    let Expr::DepthLimit(dl) = &doc.body.expr else {
        panic!("expected DepthLimit expr");
    };
    assert_eq!(dl.limit, num(64));

    // raw_block uses with increment
    let Item::RuleDef(raw) = &grammar.items[3] else {
        panic!("expected RuleDef");
    };
    assert_eq!(raw.name, "raw_block");
}

#[test]
fn parse_when_conditional() {
    let grammar = parse_fixture("when_conditional");

    // 2 state decls + 1 rule
    assert_eq!(grammar.items.len(), 3);

    let Item::RuleDef(newline) = &grammar.items[2] else {
        panic!("expected RuleDef");
    };
    assert_eq!(newline.name, "newline");

    // guard !inside_header
    assert_eq!(
        newline.body.statements[0],
        Statement::Guard(GuardStmt {
            condition: GuardCondition::NotFlag("inside_header".to_string()),
        })
    );

    // Body starts with when expression in a sequence
    let Expr::Seq(seq) = &newline.body.expr else {
        panic!("expected Seq expr, got {:?}", newline.body.expr);
    };

    let Expr::When(when) = &seq[0] else {
        panic!("expected When expr");
    };
    assert_eq!(
        when.condition,
        GuardCondition::Compare {
            name: "trim_brace_depth".to_string(),
            op: CompareOp::Gt,
            value: num(0),
        }
    );
}

#[test]
fn parse_chaos_combo_fixture() {
    let grammar = parse_fixture("chaos_combo");
    assert_eq!(grammar.items.len(), 8);

    let Item::RuleDef(document) = &grammar.items[2] else {
        panic!("expected RuleDef");
    };
    assert_eq!(document.name, "document");
    assert_eq!(
        document.body.statements,
        vec![Statement::Guard(GuardStmt {
            condition: GuardCondition::Builtin(BuiltinPredicate::Soi),
        })]
    );
    let Expr::DepthLimit(limit) = &document.body.expr else {
        panic!("expected DepthLimit");
    };
    assert_eq!(limit.limit, num(4));

    let Item::RuleDef(tag) = &grammar.items[4] else {
        panic!("expected RuleDef");
    };
    let Expr::With(with_expr) = &tag.body.expr else {
        panic!("expected With");
    };
    assert_eq!(with_expr.flag, "inside_tag");

    let Expr::Seq(seq) = with_expr.body.as_ref() else {
        panic!("expected sequence inside with");
    };
    assert!(matches!(
        &seq[2],
        Expr::WithIncrement(WithIncrementExpr { counter, amount, .. })
            if counter == "nesting" && *amount == num(1)
    ));

    let Item::RuleDef(escaped) = &grammar.items[6] else {
        panic!("expected RuleDef");
    };
    assert_eq!(
        escaped.body.expr,
        Expr::Seq(vec![
            Expr::StringLit("\\n".to_string()),
            Expr::StringLit("\"".to_string()),
        ])
    );
}

// ── Inline unit tests (kept for fine-grained edge cases) ──

#[test]
fn parse_simple_sequence() {
    let grammar = parser::parse(r#"greeting = { "hello" "world" }"#).unwrap();
    let Item::RuleDef(rule) = &grammar.items[0] else {
        panic!("expected RuleDef");
    };
    assert_eq!(
        rule.body.expr,
        Expr::Seq(vec![
            Expr::StringLit("hello".to_string()),
            Expr::StringLit("world".to_string()),
        ])
    );
}

#[test]
fn parse_choice() {
    let grammar = parser::parse(r#"ab = { "a" | "b" | "c" }"#).unwrap();
    let Item::RuleDef(rule) = &grammar.items[0] else {
        panic!("expected RuleDef");
    };
    assert_eq!(
        rule.body.expr,
        Expr::Choice(vec![
            Expr::StringLit("a".to_string()),
            Expr::StringLit("b".to_string()),
            Expr::StringLit("c".to_string()),
        ])
    );
}

#[test]
fn parse_repeat_range() {
    let grammar = parser::parse("hashes = { \"#\"{1,6} }").unwrap();
    let Item::RuleDef(rule) = &grammar.items[0] else {
        panic!("expected RuleDef");
    };
    assert_eq!(
        rule.body.expr,
        Expr::Repeat {
            expr: Box::new(Expr::StringLit("#".to_string())),
            kind: RepeatKind::Range(num(1), num(6)),
        }
    );
}

#[test]
fn parse_neg_lookahead() {
    let grammar = parser::parse(r#"not_end = { !("}}}" ) ANY }"#).unwrap();
    let Item::RuleDef(rule) = &grammar.items[0] else {
        panic!("expected RuleDef");
    };
    assert_eq!(
        rule.body.expr,
        Expr::Seq(vec![
            Expr::NegLookahead(Box::new(Expr::Group(Box::new(Expr::StringLit(
                "}}}".to_string()
            ))))),
            Expr::Builtin(BuiltinPredicate::Any),
        ])
    );
}

#[test]
fn parse_empty_grammar() {
    let grammar = parser::parse("").unwrap();
    assert!(grammar.items.is_empty());
}

#[test]
fn parse_comments_only() {
    let grammar = parser::parse("// just a comment\n// another one\n").unwrap();
    assert!(grammar.items.is_empty());
}

#[test]
fn parse_repeat_variants() {
    assert_eq!(
        parse_inline_expr(r#"exact = { "x"{2} }"#),
        Expr::Repeat {
            expr: Box::new(Expr::StringLit("x".to_string())),
            kind: RepeatKind::Exact(num(2)),
        }
    );
    assert_eq!(
        parse_inline_expr(r#"at_least = { "x"{2,} }"#),
        Expr::Repeat {
            expr: Box::new(Expr::StringLit("x".to_string())),
            kind: RepeatKind::AtLeast(num(2)),
        }
    );
    assert_eq!(
        parse_inline_expr(r#"at_most = { "x"{,3} }"#),
        Expr::Repeat {
            expr: Box::new(Expr::StringLit("x".to_string())),
            kind: RepeatKind::AtMost(num(3)),
        }
    );
}

#[test]
fn parse_dynamic_numeric_expressions() {
    assert_eq!(
        parse_inline_expr(r#"rule = { "x"{depth,limit} }"#),
        Expr::Repeat {
            expr: Box::new(Expr::StringLit("x".to_string())),
            kind: RepeatKind::Range(
                NumericExpr::Counter("depth".to_string()),
                NumericExpr::Counter("limit".to_string()),
            ),
        }
    );

    assert_eq!(
        parse_inline_expr(r#"rule = { with depth += width { "x" } }"#),
        Expr::WithIncrement(WithIncrementExpr {
            counter: "depth".to_string(),
            amount: NumericExpr::Counter("width".to_string()),
            body: Box::new(Expr::StringLit("x".to_string())),
        })
    );

    assert_eq!(
        parse_inline_expr(r#"rule = { depth_limit(limit) { "x" } }"#),
        Expr::DepthLimit(DepthLimitExpr {
            limit: NumericExpr::Counter("limit".to_string()),
            body: Box::new(Expr::StringLit("x".to_string())),
        })
    );
}

#[test]
fn parse_measure_and_if_exprs() {
    assert_eq!(
        parse_inline_expr(r#"rule = { measure width { "x"+ } }"#),
        Expr::Measure(MeasureExpr {
            counter: "width".to_string(),
            body: Box::new(Expr::Repeat {
                expr: Box::new(Expr::StringLit("x".to_string())),
                kind: RepeatKind::OneOrMore,
            }),
        })
    );

    assert_eq!(
        parse_inline_expr(r#"rule = { if depth >= limit { "x" } else { "y" } }"#),
        Expr::If(IfExpr {
            condition: GuardCondition::Compare {
                name: "depth".to_string(),
                op: CompareOp::Ge,
                value: NumericExpr::Counter("limit".to_string()),
            },
            then_body: Box::new(Expr::StringLit("x".to_string())),
            else_body: Box::new(Expr::StringLit("y".to_string())),
        })
    );
}

#[test]
fn parse_escaped_string_literal() {
    assert_eq!(
        parse_inline_expr("newline = { \"\\n\" }"),
        Expr::StringLit("\n".to_string())
    );
    assert_eq!(
        parse_inline_expr("tab = { \"\\t\" }"),
        Expr::StringLit("\t".to_string())
    );
    assert_eq!(
        parse_inline_expr("slash = { \"\\\\\" }"),
        Expr::StringLit("\\".to_string())
    );
    assert_eq!(
        parse_inline_expr("quote = { \"\\\"\" }"),
        Expr::StringLit("\"".to_string())
    );
}

#[test]
fn parse_escaped_char_range() {
    assert_eq!(
        parse_inline_expr(r"linebreak = { '\n'..'\r' }"),
        Expr::CharRange('\n', '\r')
    );
}

#[test]
fn parse_positive_lookahead_group_sequence() {
    assert_eq!(
        parse_inline_expr(r#"rule = { &("a" | "b") "c" }"#),
        Expr::Seq(vec![
            Expr::PosLookahead(Box::new(Expr::Group(Box::new(Expr::Choice(vec![
                Expr::StringLit("a".to_string()),
                Expr::StringLit("b".to_string()),
            ]))))),
            Expr::StringLit("c".to_string()),
        ])
    );
}

#[test]
fn parse_choice_has_lower_precedence_than_sequence() {
    assert_eq!(
        parse_inline_expr(r#"rule = { "a" | "b" "c" }"#),
        Expr::Choice(vec![
            Expr::StringLit("a".to_string()),
            Expr::Seq(vec![
                Expr::StringLit("b".to_string()),
                Expr::StringLit("c".to_string()),
            ]),
        ])
    );
}

#[test]
fn parse_nested_stateful_expression_combo() {
    let grammar = parser::parse(
        r#"
let flag inside
let counter depth
inner = { "x" }
rule = { depth_limit(3) { with inside { when depth > 0 { inner } } } }
"#,
    )
    .unwrap();

    let Item::RuleDef(rule) = &grammar.items[3] else {
        panic!("expected RuleDef");
    };
    let Expr::DepthLimit(depth_limit) = &rule.body.expr else {
        panic!("expected DepthLimit expr");
    };
    assert_eq!(depth_limit.limit, num(3));

    let Expr::With(with_expr) = depth_limit.body.as_ref() else {
        panic!("expected With expr");
    };
    assert_eq!(with_expr.flag, "inside");

    let Expr::When(when_expr) = with_expr.body.as_ref() else {
        panic!("expected When expr");
    };
    assert_eq!(
        when_expr.condition,
        GuardCondition::Compare {
            name: "depth".to_string(),
            op: CompareOp::Gt,
            value: num(0),
        }
    );
    assert_eq!(when_expr.body.as_ref(), &Expr::Ident("inner".to_string()));
}

#[test]
fn parse_rejects_unexpected_character() {
    let err = parser::parse(r#"rule = { "x" $ "y" }"#).unwrap_err();
    assert_eq!(err.offset, 13);
    assert!(err.message.contains("unexpected character '$'"));
}

#[test]
fn parse_rejects_single_char_literal_without_range() {
    let err = parser::parse(r"rule = { '\n' }").unwrap_err();
    assert_eq!(err.offset, 14);
    assert!(err.message.contains("expected DotDot"));
}

#[test]
fn parse_rejects_malformed_repeat_bounds() {
    let err = parser::parse(r#"rule = { "x"{1,foo,} }"#).unwrap_err();
    assert_eq!(err.offset, 18);
    assert!(err.message.contains("expected RBrace"));
}

#[test]
fn parse_rejects_builtin_as_rule_name() {
    let err = parser::parse(r#"SOI = { "x" }"#).unwrap_err();
    assert_eq!(err.offset, 0);
    assert!(err.message.contains("expected 'let' or rule name"));
}

#[test]
fn parse_invalid_syntax_fixtures() {
    let cases = [
        ("unexpected_character", 13, "unexpected character '$'"),
        ("unterminated_rule", 17, "expected RBrace"),
        ("malformed_repeat", 18, "expected RBrace"),
        ("bare_char_literal", 14, "expected DotDot"),
        ("builtin_rule_name", 0, "expected 'let' or rule name"),
        ("unsupported_state_kind", 10, "expected 'flag' or 'counter'"),
        ("empty_rule_body", 9, "expected expression"),
        ("dangling_choice", 15, "expected expression"),
    ];

    for (name, offset, message) in cases {
        let err = parse_invalid_fixture(name);
        assert_eq!(err.offset, offset, "fixture {name}");
        assert!(
            err.message.contains(message),
            "fixture {name}: {err:?} does not contain {message:?}"
        );
    }
}

// ── Error label (@) tests ──

#[test]
fn parse_rule_level_error_label() {
    let grammar = parser::parse(r#"value = @ "JSON value" { "x" | "y" }"#).unwrap();
    let Item::RuleDef(rule) = &grammar.items[0] else {
        panic!("expected RuleDef");
    };
    assert_eq!(rule.error_label.as_deref(), Some("JSON value"));
}

#[test]
fn parse_rule_without_label_has_none() {
    let grammar = parser::parse(r#"value = { "x" }"#).unwrap();
    let Item::RuleDef(rule) = &grammar.items[0] else {
        panic!("expected RuleDef");
    };
    assert_eq!(rule.error_label, None);
}

#[test]
fn parse_expr_level_label() {
    let expr = parse_inline_expr(r#"r = { "x" @ "the x" | "y" }"#);
    match &expr {
        Expr::Choice(items) => {
            assert!(matches!(&items[0], Expr::Labeled { label, .. } if label == "the x"));
            assert!(matches!(&items[1], Expr::StringLit(s) if s == "y"));
        }
        other => panic!("expected Choice, got {other:?}"),
    }
}

#[test]
fn parse_label_after_repeat() {
    let expr = parse_inline_expr(r#"r = { "x"+ @ "one or more x" }"#);
    assert!(matches!(&expr, Expr::Labeled { label, .. } if label == "one or more x"));
}

#[test]
fn parse_error_labels_fixture() {
    parse_fixture("error_labels");
}

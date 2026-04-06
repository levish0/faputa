use crate::compile;
use crate::hir::{Boundary, CharRange, HirExpr, HirProgram};

use super::lower;

fn lower_source(source: &str) -> HirProgram {
    let grammar = compile(source).expect("compile failed");
    lower(&grammar)
}

#[test]
fn lowers_simple_rule() {
    let ir = lower_source(r#"hello = { "hello" }"#);
    assert_eq!(ir.rules.len(), 1);
    assert_eq!(ir.rules[0].name, "hello");
    assert_eq!(ir.rules[0].expr, HirExpr::Literal("hello".into()));
}

#[test]
fn resolves_rule_references() {
    let ir = lower_source(
        r#"
        alpha = { 'a'..'z' }
        ident = { alpha alpha* }
    "#,
    );
    assert_eq!(ir.rules.len(), 2);
    match &ir.rules[1].expr {
        HirExpr::Seq(items) => match &items[0] {
            HirExpr::RuleRef(0) => {}
            other => panic!("expected RuleRef(0), got {other:?}"),
        },
        other => panic!("expected Seq, got {other:?}"),
    }
}

#[test]
fn char_range_becomes_charset() {
    let ir = lower_source("alpha = { 'a'..'z' }");
    assert_eq!(
        ir.rules[0].expr,
        HirExpr::CharSet(vec![CharRange::new('a', 'z')])
    );
}

#[test]
fn repeat_kinds_unified() {
    let ir = lower_source(r#"r = { "a"+ "b"* "c"? "d"{3} "e"{1,5} }"#);
    match &ir.rules[0].expr {
        HirExpr::Seq(items) => {
            assert!(matches!(
                &items[0],
                HirExpr::Repeat {
                    min: 1,
                    max: None,
                    ..
                }
            ));
            assert!(matches!(
                &items[1],
                HirExpr::Repeat {
                    min: 0,
                    max: None,
                    ..
                }
            ));
            assert!(matches!(
                &items[2],
                HirExpr::Repeat {
                    min: 0,
                    max: Some(1),
                    ..
                }
            ));
            assert!(matches!(
                &items[3],
                HirExpr::Repeat {
                    min: 3,
                    max: Some(3),
                    ..
                }
            ));
            assert!(matches!(
                &items[4],
                HirExpr::Repeat {
                    min: 1,
                    max: Some(5),
                    ..
                }
            ));
        }
        other => panic!("expected Seq, got {other:?}"),
    }
}

#[test]
fn guards_extracted_from_body() {
    let ir = lower_source(
        r#"
        let flag inside_bold
        bold = {
            guard !inside_bold
            "**" "text" "**"
        }
    "#,
    );
    assert_eq!(ir.rules[0].guards.len(), 1);
    assert!(matches!(
        &ir.rules[0].guards[0],
        crate::ast::GuardCondition::NotFlag(name) if name == "inside_bold"
    ));
}

#[test]
fn emits_extracted_from_body() {
    let ir = lower_source(
        r##"
        let counter section_counter
        header = {
            emit section_counter
            "#" "text"
        }
    "##,
    );
    assert_eq!(ir.rules[0].emits, vec!["section_counter"]);
}

#[test]
fn builtins_lowered() {
    let ir = lower_source(
        r#"
        start = { SOI "begin" }
        finish = { "end" EOI }
        anything = { ANY }
    "#,
    );
    match &ir.rules[0].expr {
        HirExpr::Seq(items) => assert_eq!(items[0], HirExpr::Boundary(Boundary::Soi)),
        other => panic!("expected Seq, got {other:?}"),
    }
    match &ir.rules[1].expr {
        HirExpr::Seq(items) => assert_eq!(items[1], HirExpr::Boundary(Boundary::Eoi)),
        other => panic!("expected Seq, got {other:?}"),
    }
    assert_eq!(ir.rules[2].expr, HirExpr::Any);
}

#[test]
fn group_unwrapped() {
    let ir = lower_source(r#"g = { ("a" | "b") }"#);
    assert!(matches!(&ir.rules[0].expr, HirExpr::Choice(_)));
}

#[test]
fn single_element_seq_unwrapped() {
    let ir = lower_source(r#"s = { "hello" }"#);
    assert_eq!(ir.rules[0].expr, HirExpr::Literal("hello".into()));
}

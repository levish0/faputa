use crate::ir::IrExpr;

use super::optimized;

#[test]
fn trivial_rule_inlined() {
    let ir = optimized(
        r#"
        digit = { '0'..'9' }
        number = { digit+ }
    "#,
    );
    let number = ir.rules.iter().find(|r| r.name == "number").unwrap();
    assert!(
        matches!(
            &number.expr,
            IrExpr::TakeWhile {
                min: 1,
                max: None,
                ..
            }
        ),
        "expected TakeWhile, got {:?}",
        &number.expr
    );
}

#[test]
fn inlined_rules_kept_with_inline_flag() {
    let ir = optimized(
        r#"
        digit = { '0'..'9' }
        number = { digit+ }
    "#,
    );
    assert_eq!(ir.rules.len(), 2);
    let digit = ir.rules.iter().find(|r| r.name == "digit").unwrap();
    assert!(digit.inline);
    let number = ir.rules.iter().find(|r| r.name == "number").unwrap();
    assert!(!number.inline);
}

#[test]
fn non_trivial_rule_not_inlined() {
    let ir = optimized(
        r#"
        alpha = { 'a'..'z' | 'A'..'Z' }
        digit = { '0'..'9' }
        ident = { alpha (alpha | digit)* }
    "#,
    );
    let ident = ir.rules.iter().find(|r| r.name == "ident").unwrap();
    match &ident.expr {
        IrExpr::Seq(items) => {
            assert!(matches!(&items[0], IrExpr::CharSet(_)));
            assert!(
                matches!(&items[1], IrExpr::TakeWhile { .. }),
                "expected TakeWhile, got {:?}",
                &items[1]
            );
        }
        other => panic!("expected Seq, got {other:?}"),
    }
}

#[test]
fn stateful_rule_not_inlined() {
    let ir = optimized(
        r#"
        let flag active
        special = {
            guard active
            "x"
        }
        main = { special }
    "#,
    );
    assert!(ir.rules.iter().any(|r| r.name == "special"));
    let main = ir.rules.iter().find(|r| r.name == "main").unwrap();
    assert!(matches!(&main.expr, IrExpr::RuleRef(_)));
}

#[test]
fn ref_count_entry_point() {
    let ir = optimized(
        r#"
        main = { "hello" }
    "#,
    );
    assert_eq!(ir.rules[0].ref_count, 0);
}

#[test]
fn ref_count_internal_rule() {
    let ir = optimized(
        r#"
        let flag active
        special = {
            guard active
            "x"
        }
        a = { special }
        b = { special }
    "#,
    );
    let special = ir.rules.iter().find(|r| r.name == "special").unwrap();
    assert_eq!(special.ref_count, 2);
}

#[test]
fn small_single_use_helper_rule_inlined() {
    let ir = optimized(
        r#"
        ws = { (" " | "\n")* }
        digit = { '0'..'9' }
        comma_value = { ws "," ws digit+ }
        items = { digit+ comma_value* }
    "#,
    );
    let helper = ir.rules.iter().find(|r| r.name == "comma_value").unwrap();
    assert!(helper.inline);
}

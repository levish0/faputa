use crate::mir::MirExpr;

use super::optimized;

#[test]
fn unbounded_repeat_of_non_nullable_rule_becomes_loop() {
    let ir = optimized(
        r#"
        digit = { '0'..'9' }
        line = { digit+ "\n" }
        file = { line* }
    "#,
    );
    let file = ir.rules.iter().find(|r| r.name == "file").unwrap();
    assert!(matches!(&file.expr, MirExpr::Loop { min: 0, .. }));
}

#[test]
fn nullable_repeat_stays_repeat() {
    let ir = optimized(
        r#"
        sp = { " "* }
        file = { sp* }
    "#,
    );
    let file = ir.rules.iter().find(|r| r.name == "file").unwrap();
    assert!(matches!(
        &file.expr,
        MirExpr::Repeat {
            min: 0,
            max: None,
            ..
        }
    ));
}

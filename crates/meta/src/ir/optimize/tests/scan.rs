use crate::ir::IrExpr;

use super::optimized;

#[test]
fn scan_recognized_for_chunkable_dispatch_repeat() {
    let ir = optimized(
        r#"
        body = { ("\\" ("n" | "\\") | (!("\"" | "\\") ANY))* }
    "#,
    );
    assert!(matches!(&ir.rules[0].expr, IrExpr::Scan { min: 0, .. }));
}

#[test]
fn scan_skips_plain_only_repeat() {
    let ir = optimized(r#"body = { ('a'..'z')* }"#);
    assert!(matches!(
        &ir.rules[0].expr,
        IrExpr::TakeWhile {
            min: 0,
            max: None,
            ..
        }
    ));
}

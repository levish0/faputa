use crate::ir::IrExpr;

use super::optimized;

#[test]
fn dispatch_recognized_for_disjoint_choice() {
    let ir = optimized(r#"value = { "true" | "false" | "null" }"#);
    assert!(matches!(&ir.rules[0].expr, IrExpr::Dispatch(_)));
}

#[test]
fn dispatch_skips_overlapping_choice() {
    let ir = optimized(r#"value = { "true" | "trick" }"#);
    assert!(matches!(&ir.rules[0].expr, IrExpr::Choice(_)));
}

#[test]
fn dispatch_works_through_rule_refs() {
    let ir = optimized(
        r#"
        number = { "-"? '0'..'9'+ }
        array = { "[" "]"? }
        value = { number | array }
    "#,
    );
    let value = ir.rules.iter().find(|r| r.name == "value").unwrap();
    assert!(matches!(&value.expr, IrExpr::Dispatch(_)));
}

#[test]
fn dispatch_recognizes_neg_lookahead_any_plain_branch() {
    let ir = optimized(
        r#"
        hex = { '0'..'9' | 'a'..'f' | 'A'..'F' }
        uni_escape = { "\\u" hex hex hex hex }
        escape = { "\\" ("\"" | "\\" | "/" | "b" | "f" | "n" | "r" | "t") }
        string_char = { uni_escape | escape | (!("\"" | "\\") ANY) }
    "#,
    );
    let string_char = ir.rules.iter().find(|r| r.name == "string_char").unwrap();
    assert!(matches!(&string_char.expr, IrExpr::Dispatch(_)));
}

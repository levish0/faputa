use crate::mir::MirExpr;

use super::optimized;

#[test]
fn recognizes_delimited_string_shape() {
    let ir = optimized(
        r#"
        hex         = { '0'..'9' | 'a'..'f' | 'A'..'F' }
        uni_escape  = { "\\u" hex hex hex hex }
        escape      = { "\\" ("\"" | "\\" | "/" | "b" | "f" | "n" | "r" | "t") }
        string_char = { uni_escape | escape | (!("\"" | "\\") ANY) }
        string      = { "\"" string_char* "\"" }
    "#,
    );
    let string = ir.rules.iter().find(|r| r.name == "string").unwrap();
    assert!(matches!(&string.expr, MirExpr::Delimited { .. }));
}

#[test]
fn ignores_plain_sequence_without_delimiters() {
    let ir = optimized(r#"triple = { "a" "b" "c" }"#);
    let triple = ir.rules.iter().find(|r| r.name == "triple").unwrap();
    assert!(matches!(&triple.expr, MirExpr::Seq(_)));
}

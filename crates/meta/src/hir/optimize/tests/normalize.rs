use crate::hir::{CharRange, HirExpr};

use super::optimized;

#[test]
fn charset_merge_in_choice() {
    let ir = optimized(r#"alpha = { 'a'..'z' | 'A'..'Z' | "_" }"#);
    match &ir.rules[0].expr {
        HirExpr::CharSet(ranges) => assert_eq!(ranges.len(), 3),
        other => panic!("expected CharSet, got {other:?}"),
    }
}

#[test]
fn charset_merge_all_ranges() {
    let ir = optimized("alpha = { 'a'..'z' | 'A'..'Z' }");
    match &ir.rules[0].expr {
        HirExpr::CharSet(ranges) => assert_eq!(ranges.len(), 2),
        other => panic!("expected CharSet, got {other:?}"),
    }
}

#[test]
fn adjacent_ranges_coalesced() {
    let ir = optimized("az = { 'a'..'m' | 'n'..'z' }");
    match &ir.rules[0].expr {
        HirExpr::CharSet(ranges) => {
            assert_eq!(ranges.len(), 1);
            assert_eq!(ranges[0], CharRange::new('a', 'z'));
        }
        other => panic!("expected CharSet, got {other:?}"),
    }
}

#[test]
fn literal_fusion() {
    let ir = optimized(r#"kw = { "h" "e" "l" "l" "o" }"#);
    assert_eq!(ir.rules[0].expr, HirExpr::Literal("hello".into()));
}

#[test]
fn flatten_nested_seq() {
    let ir = optimized(r#"r = { "a" ("b" "c") }"#);
    match &ir.rules[0].expr {
        HirExpr::Literal(s) => assert_eq!(s, "abc"),
        other => panic!("expected fused Literal, got {other:?}"),
    }
}

#[test]
fn single_char_literal_to_charset_in_choice() {
    let ir = optimized(r#"ws = { " " | "\t" | "\n" | "\r" }"#);
    match &ir.rules[0].expr {
        HirExpr::CharSet(ranges) => assert_eq!(ranges.len(), 3),
        other => panic!("expected CharSet, got {other:?}"),
    }
}

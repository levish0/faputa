//! AST → IR lowering pass.
//!
//! Resolves rule references to indices, removes syntactic wrappers,
//! unifies repeat variants, and converts statements into the IR structure.

use std::collections::HashMap;

use super::{Boundary, CharRange, IrExpr, IrProgram, IrRule};
use crate::ast::{self, BuiltinPredicate, Expr, Grammar, Item, RepeatKind, Statement};

/// Lower a validated AST Grammar to an IR Program.
///
/// Panics if the grammar contains unresolved rule references
/// (should not happen after validation).
pub fn lower(grammar: &Grammar) -> IrProgram {
    // Build name → index map.
    let rule_indices: HashMap<&str, usize> = grammar
        .items
        .iter()
        .filter_map(|item| match item {
            Item::RuleDef(r) => Some(r.name.as_str()),
            _ => None,
        })
        .enumerate()
        .map(|(i, name)| (name, i))
        .collect();

    let state_decls: Vec<_> = grammar
        .items
        .iter()
        .filter_map(|item| match item {
            Item::StateDecl(decl) => Some(decl.clone()),
            _ => None,
        })
        .collect();

    let rules: Vec<_> = grammar
        .items
        .iter()
        .filter_map(|item| match item {
            Item::RuleDef(rule) => Some(lower_rule(rule, &rule_indices)),
            _ => None,
        })
        .collect();

    IrProgram { state_decls, rules }
}

fn lower_rule(rule: &ast::RuleDef, indices: &HashMap<&str, usize>) -> IrRule {
    let mut guards = Vec::new();
    let mut emits = Vec::new();

    for stmt in &rule.body.statements {
        match stmt {
            Statement::Guard(g) => guards.push(g.condition.clone()),
            Statement::Emit(e) => emits.push(e.counter.clone()),
        }
    }

    let expr = lower_expr(&rule.body.expr, indices);

    IrRule {
        name: rule.name.clone(),
        inline: false,
        error_label: rule.error_label.clone(),
        guards,
        emits,
        expr,
        ref_count: 0,
    }
}

fn lower_expr(expr: &Expr, indices: &HashMap<&str, usize>) -> IrExpr {
    match expr {
        Expr::StringLit(s) => IrExpr::Literal(s.clone()),

        Expr::CharRange(start, end) => IrExpr::CharSet(vec![CharRange::new(*start, *end)]),

        Expr::Ident(name) => {
            let index = indices[name.as_str()];
            IrExpr::RuleRef(index)
        }

        Expr::Builtin(builtin) => lower_builtin(builtin),

        Expr::Seq(exprs) => {
            let items: Vec<_> = exprs.iter().map(|e| lower_expr(e, indices)).collect();
            if items.len() == 1 {
                items.into_iter().next().unwrap()
            } else {
                IrExpr::Seq(items)
            }
        }

        Expr::Choice(exprs) => {
            let items: Vec<_> = exprs.iter().map(|e| lower_expr(e, indices)).collect();
            if items.len() == 1 {
                items.into_iter().next().unwrap()
            } else {
                IrExpr::Choice(items)
            }
        }

        Expr::Repeat { expr, kind } => {
            let (min, max) = match kind {
                RepeatKind::ZeroOrMore => (0, None),
                RepeatKind::OneOrMore => (1, None),
                RepeatKind::Optional => (0, Some(1)),
                RepeatKind::Exact(n) => (*n, Some(*n)),
                RepeatKind::AtLeast(n) => (*n, None),
                RepeatKind::AtMost(m) => (0, Some(*m)),
                RepeatKind::Range(n, m) => (*n, Some(*m)),
            };
            IrExpr::Repeat {
                expr: Box::new(lower_expr(expr, indices)),
                min,
                max,
            }
        }

        Expr::PosLookahead(inner) => IrExpr::PosLookahead(Box::new(lower_expr(inner, indices))),

        Expr::NegLookahead(inner) => IrExpr::NegLookahead(Box::new(lower_expr(inner, indices))),

        // Group is purely syntactic — unwrap.
        Expr::Group(inner) => lower_expr(inner, indices),

        Expr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(lower_expr(expr, indices)),
            label: label.clone(),
        },

        Expr::With(w) => IrExpr::WithFlag {
            flag: w.flag.clone(),
            body: Box::new(lower_expr(&w.body, indices)),
        },

        Expr::WithIncrement(w) => IrExpr::WithCounter {
            counter: w.counter.clone(),
            amount: w.amount,
            body: Box::new(lower_expr(&w.body, indices)),
        },

        Expr::When(w) => IrExpr::When {
            condition: w.condition.clone(),
            body: Box::new(lower_expr(&w.body, indices)),
        },

        Expr::DepthLimit(d) => IrExpr::DepthLimit {
            limit: d.limit,
            body: Box::new(lower_expr(&d.body, indices)),
        },
    }
}

fn lower_builtin(builtin: &BuiltinPredicate) -> IrExpr {
    match builtin {
        BuiltinPredicate::Soi => IrExpr::Boundary(Boundary::Soi),
        BuiltinPredicate::Eoi => IrExpr::Boundary(Boundary::Eoi),
        BuiltinPredicate::Any => IrExpr::Any,
        BuiltinPredicate::LineStart => IrExpr::Boundary(Boundary::LineStart),
        BuiltinPredicate::LineEnd => IrExpr::Boundary(Boundary::LineEnd),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile;

    fn lower_source(source: &str) -> IrProgram {
        let grammar = compile(source).expect("compile failed");
        lower(&grammar)
    }

    #[test]
    fn lowers_simple_rule() {
        let ir = lower_source(r#"hello = { "hello" }"#);
        assert_eq!(ir.rules.len(), 1);
        assert_eq!(ir.rules[0].name, "hello");
        assert_eq!(ir.rules[0].expr, IrExpr::Literal("hello".into()));
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
        // ident's expr should reference alpha by index 0
        match &ir.rules[1].expr {
            IrExpr::Seq(items) => match &items[0] {
                IrExpr::RuleRef(0) => {}
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
            IrExpr::CharSet(vec![CharRange::new('a', 'z')])
        );
    }

    #[test]
    fn repeat_kinds_unified() {
        let ir = lower_source(r#"r = { "a"+ "b"* "c"? "d"{3} "e"{1,5} }"#);
        match &ir.rules[0].expr {
            IrExpr::Seq(items) => {
                assert!(matches!(
                    &items[0],
                    IrExpr::Repeat {
                        min: 1,
                        max: None,
                        ..
                    }
                ));
                assert!(matches!(
                    &items[1],
                    IrExpr::Repeat {
                        min: 0,
                        max: None,
                        ..
                    }
                ));
                assert!(matches!(
                    &items[2],
                    IrExpr::Repeat {
                        min: 0,
                        max: Some(1),
                        ..
                    }
                ));
                assert!(matches!(
                    &items[3],
                    IrExpr::Repeat {
                        min: 3,
                        max: Some(3),
                        ..
                    }
                ));
                assert!(matches!(
                    &items[4],
                    IrExpr::Repeat {
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
            ast::GuardCondition::NotFlag(name) if name == "inside_bold"
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
            IrExpr::Seq(items) => assert_eq!(items[0], IrExpr::Boundary(Boundary::Soi)),
            other => panic!("expected Seq, got {other:?}"),
        }
        match &ir.rules[1].expr {
            IrExpr::Seq(items) => assert_eq!(items[1], IrExpr::Boundary(Boundary::Eoi)),
            other => panic!("expected Seq, got {other:?}"),
        }
        assert_eq!(ir.rules[2].expr, IrExpr::Any);
    }

    #[test]
    fn group_unwrapped() {
        let ir = lower_source(r#"g = { ("a" | "b") }"#);
        assert!(matches!(&ir.rules[0].expr, IrExpr::Choice(_)));
    }

    #[test]
    fn single_element_seq_unwrapped() {
        let ir = lower_source(r#"s = { "hello" }"#);
        assert_eq!(ir.rules[0].expr, IrExpr::Literal("hello".into()));
    }
}

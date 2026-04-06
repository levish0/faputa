//! AST → HIR lowering pass.
//!
//! Resolves rule references to indices, removes syntactic wrappers,
//! unifies repeat variants, and converts statements into the HIR structure.

use std::collections::HashMap;

use super::{Boundary, CharRange, HirExpr, HirProgram, HirRule};
use crate::ast::{self, BuiltinPredicate, Expr, Grammar, Item, RepeatKind, Statement};

/// Lower a validated AST Grammar to a HIR program.
///
/// Panics if the grammar contains unresolved rule references
/// (should not happen after validation).
#[tracing::instrument(skip_all, fields(rules = grammar.items.iter().filter(|i| matches!(i, Item::RuleDef(_))).count()))]
pub fn lower(grammar: &Grammar) -> HirProgram {
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
            Item::RuleDef(rule) => {
                let ir_rule = lower_rule(rule, &rule_indices);
                tracing::trace!(
                    rule = %ir_rule.name,
                    guards = ir_rule.guards.len(),
                    emits = ir_rule.emits.len(),
                    has_error_label = ir_rule.error_label.is_some(),
                    "lowered rule"
                );
                Some(ir_rule)
            }
            _ => None,
        })
        .collect();

    HirProgram { state_decls, rules }
}

fn lower_rule(rule: &ast::RuleDef, indices: &HashMap<&str, usize>) -> HirRule {
    let mut guards = Vec::new();
    let mut emits = Vec::new();

    for stmt in &rule.body.statements {
        match stmt {
            Statement::Guard(g) => guards.push(g.condition.clone()),
            Statement::Emit(e) => emits.push(e.counter.clone()),
        }
    }

    let expr = lower_expr(&rule.body.expr, indices);

    HirRule {
        name: rule.name.clone(),
        inline: false,
        error_label: rule.error_label.clone(),
        guards,
        emits,
        expr,
        ref_count: 0,
    }
}

fn lower_expr(expr: &Expr, indices: &HashMap<&str, usize>) -> HirExpr {
    match expr {
        Expr::StringLit(s) => HirExpr::Literal(s.clone()),

        Expr::CharRange(start, end) => HirExpr::CharSet(vec![CharRange::new(*start, *end)]),

        Expr::Ident(name) => {
            let index = indices[name.as_str()];
            HirExpr::RuleRef(index)
        }

        Expr::Builtin(builtin) => lower_builtin(builtin),

        Expr::Seq(exprs) => {
            let items: Vec<_> = exprs.iter().map(|e| lower_expr(e, indices)).collect();
            if items.len() == 1 {
                items.into_iter().next().unwrap()
            } else {
                HirExpr::Seq(items)
            }
        }

        Expr::Choice(exprs) => {
            let items: Vec<_> = exprs.iter().map(|e| lower_expr(e, indices)).collect();
            if items.len() == 1 {
                items.into_iter().next().unwrap()
            } else {
                HirExpr::Choice(items)
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
            HirExpr::Repeat {
                expr: Box::new(lower_expr(expr, indices)),
                min,
                max,
            }
        }

        Expr::PosLookahead(inner) => HirExpr::PosLookahead(Box::new(lower_expr(inner, indices))),

        Expr::NegLookahead(inner) => HirExpr::NegLookahead(Box::new(lower_expr(inner, indices))),

        // Group is purely syntactic — unwrap.
        Expr::Group(inner) => lower_expr(inner, indices),

        Expr::Labeled { expr, label } => HirExpr::Labeled {
            expr: Box::new(lower_expr(expr, indices)),
            label: label.clone(),
        },

        Expr::With(w) => HirExpr::WithFlag {
            flag: w.flag.clone(),
            body: Box::new(lower_expr(&w.body, indices)),
        },

        Expr::WithIncrement(w) => HirExpr::WithCounter {
            counter: w.counter.clone(),
            amount: w.amount,
            body: Box::new(lower_expr(&w.body, indices)),
        },

        Expr::When(w) => HirExpr::When {
            condition: w.condition.clone(),
            body: Box::new(lower_expr(&w.body, indices)),
        },

        Expr::DepthLimit(d) => HirExpr::DepthLimit {
            limit: d.limit,
            body: Box::new(lower_expr(&d.body, indices)),
        },
    }
}

fn lower_builtin(builtin: &BuiltinPredicate) -> HirExpr {
    match builtin {
        BuiltinPredicate::Soi => HirExpr::Boundary(Boundary::Soi),
        BuiltinPredicate::Eoi => HirExpr::Boundary(Boundary::Eoi),
        BuiltinPredicate::Any => HirExpr::Any,
        BuiltinPredicate::LineStart => HirExpr::Boundary(Boundary::LineStart),
        BuiltinPredicate::LineEnd => HirExpr::Boundary(Boundary::LineEnd),
    }
}

#[cfg(test)]
#[path = "lower/tests.rs"]
mod tests;

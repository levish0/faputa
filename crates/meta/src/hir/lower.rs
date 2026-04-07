//! AST → HIR lowering pass.
//!
//! Resolves rule references to indices, removes syntactic wrappers,
//! unifies repeat variants, and converts statements into the HIR structure.

use std::collections::HashMap;

use super::{Boundary, CharRange, HirExpr, HirProgram, HirRule};
use crate::ast::NumericExpr;
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
                    increments = ir_rule.increments.len(),
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
    let mut increments = Vec::new();

    for stmt in &rule.body.statements {
        match stmt {
            Statement::Guard(g) => guards.push(g.condition.clone()),
            Statement::Inc(e) => increments.push(e.counter.clone()),
        }
    }

    let expr = lower_expr(&rule.body.expr, indices);

    HirRule {
        name: rule.name.clone(),
        inline: false,
        error_label: rule.error_label.clone(),
        guards,
        increments,
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

        Expr::Repeat { expr, kind } => lower_repeat(kind, expr, indices),

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
            amount: w.amount.clone(),
            body: Box::new(lower_expr(&w.body, indices)),
        },

        Expr::When(w) => HirExpr::When {
            condition: w.condition.clone(),
            body: Box::new(lower_expr(&w.body, indices)),
        },

        Expr::If(w) => HirExpr::If {
            condition: w.condition.clone(),
            then_body: Box::new(lower_expr(&w.then_body, indices)),
            else_body: Box::new(lower_expr(&w.else_body, indices)),
        },

        Expr::Measure(w) => HirExpr::Measure {
            counter: w.counter.clone(),
            body: Box::new(lower_expr(&w.body, indices)),
        },

        Expr::DepthLimit(d) => HirExpr::DepthLimit {
            limit: d.limit.clone(),
            body: Box::new(lower_expr(&d.body, indices)),
        },
    }
}

fn lower_repeat(kind: &RepeatKind, expr: &Expr, indices: &HashMap<&str, usize>) -> HirExpr {
    let lowered = Box::new(lower_expr(expr, indices));

    match kind {
        RepeatKind::ZeroOrMore => HirExpr::Repeat {
            expr: lowered,
            min: 0,
            max: None,
        },
        RepeatKind::OneOrMore => HirExpr::Repeat {
            expr: lowered,
            min: 1,
            max: None,
        },
        RepeatKind::Optional => HirExpr::Repeat {
            expr: lowered,
            min: 0,
            max: Some(1),
        },
        RepeatKind::Exact(value) => match value.as_literal() {
            Some(n) => HirExpr::Repeat {
                expr: lowered,
                min: n,
                max: Some(n),
            },
            None => HirExpr::RepeatDynamic {
                expr: lowered,
                min: value.clone(),
                max: Some(value.clone()),
            },
        },
        RepeatKind::AtLeast(value) => match value.as_literal() {
            Some(n) => HirExpr::Repeat {
                expr: lowered,
                min: n,
                max: None,
            },
            None => HirExpr::RepeatDynamic {
                expr: lowered,
                min: value.clone(),
                max: None,
            },
        },
        RepeatKind::AtMost(value) => match value.as_literal() {
            Some(m) => HirExpr::Repeat {
                expr: lowered,
                min: 0,
                max: Some(m),
            },
            None => HirExpr::RepeatDynamic {
                expr: lowered,
                min: NumericExpr::Literal(0),
                max: Some(value.clone()),
            },
        },
        RepeatKind::Range(min, max) => match (min.as_literal(), max.as_literal()) {
            (Some(min), Some(max)) => HirExpr::Repeat {
                expr: lowered,
                min,
                max: Some(max),
            },
            _ => HirExpr::RepeatDynamic {
                expr: lowered,
                min: min.clone(),
                max: Some(max.clone()),
            },
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

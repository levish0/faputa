use crate::ast::*;

use super::errors::ValidationError;
use super::rules::DefinitionContext;

/// Check that all state variable usages match their declared kind.
pub(crate) fn check_state_usage(
    grammar: &Grammar,
    ctx: &DefinitionContext,
    errors: &mut Vec<ValidationError>,
) {
    for item in &grammar.items {
        if let Item::RuleDef(rule) = item {
            for stmt in &rule.body.statements {
                check_statement(stmt, &rule.name, ctx, errors);
            }
            check_expr_state(&rule.body.expr, &rule.name, ctx, errors);
        }
    }
}

fn check_statement(
    stmt: &Statement,
    rule_name: &str,
    ctx: &DefinitionContext,
    errors: &mut Vec<ValidationError>,
) {
    match stmt {
        Statement::Guard(g) => check_guard_condition(&g.condition, rule_name, ctx, errors),
        Statement::Inc(e) => {
            check_state_is_counter(&e.counter, rule_name, ctx, errors);
        }
    }
}

fn check_guard_condition(
    cond: &GuardCondition,
    rule_name: &str,
    ctx: &DefinitionContext,
    errors: &mut Vec<ValidationError>,
) {
    match cond {
        GuardCondition::IsFlag(name) | GuardCondition::NotFlag(name) => {
            check_state_is_flag(name, rule_name, ctx, errors);
        }
        GuardCondition::Compare { name, value, .. } => {
            check_state_is_counter(name, rule_name, ctx, errors);
            check_numeric_expr(value, rule_name, ctx, errors);
        }
        GuardCondition::Builtin(_) => {}
    }
}

fn check_expr_state(
    expr: &Expr,
    rule_name: &str,
    ctx: &DefinitionContext,
    errors: &mut Vec<ValidationError>,
) {
    match expr {
        Expr::With(w) => {
            check_state_is_flag(&w.flag, rule_name, ctx, errors);
            check_expr_state(&w.body, rule_name, ctx, errors);
        }
        Expr::WithIncrement(w) => {
            check_state_is_counter(&w.counter, rule_name, ctx, errors);
            check_numeric_expr(&w.amount, rule_name, ctx, errors);
            check_expr_state(&w.body, rule_name, ctx, errors);
        }
        Expr::When(w) => {
            check_guard_condition(&w.condition, rule_name, ctx, errors);
            check_expr_state(&w.body, rule_name, ctx, errors);
        }
        Expr::If(w) => {
            check_guard_condition(&w.condition, rule_name, ctx, errors);
            check_expr_state(&w.then_body, rule_name, ctx, errors);
            check_expr_state(&w.else_body, rule_name, ctx, errors);
        }
        Expr::Measure(w) => {
            check_state_is_counter(&w.counter, rule_name, ctx, errors);
            check_expr_state(&w.body, rule_name, ctx, errors);
        }
        Expr::DepthLimit(d) => {
            check_numeric_expr(&d.limit, rule_name, ctx, errors);
            check_expr_state(&d.body, rule_name, ctx, errors);
        }
        Expr::Seq(exprs) | Expr::Choice(exprs) => {
            for e in exprs {
                check_expr_state(e, rule_name, ctx, errors);
            }
        }
        Expr::Repeat { expr, kind } => {
            check_repeat_kind(kind, rule_name, ctx, errors);
            check_expr_state(expr, rule_name, ctx, errors);
        }
        Expr::PosLookahead(expr) | Expr::NegLookahead(expr) | Expr::Group(expr) => {
            check_expr_state(expr, rule_name, ctx, errors);
        }
        Expr::Labeled { expr, .. } => {
            check_expr_state(expr, rule_name, ctx, errors);
        }
        Expr::StringLit(_) | Expr::CharRange(_, _) | Expr::Ident(_) | Expr::Builtin(_) => {}
    }
}

fn check_repeat_kind(
    kind: &RepeatKind,
    rule_name: &str,
    ctx: &DefinitionContext,
    errors: &mut Vec<ValidationError>,
) {
    match kind {
        RepeatKind::ZeroOrMore | RepeatKind::OneOrMore | RepeatKind::Optional => {}
        RepeatKind::Exact(value) | RepeatKind::AtLeast(value) | RepeatKind::AtMost(value) => {
            check_numeric_expr(value, rule_name, ctx, errors);
        }
        RepeatKind::Range(min, max) => {
            check_numeric_expr(min, rule_name, ctx, errors);
            check_numeric_expr(max, rule_name, ctx, errors);
        }
    }
}

fn check_numeric_expr(
    expr: &NumericExpr,
    rule_name: &str,
    ctx: &DefinitionContext,
    errors: &mut Vec<ValidationError>,
) {
    match expr {
        NumericExpr::Literal(_) => {}
        NumericExpr::Counter(name) => check_state_is_counter(name, rule_name, ctx, errors),
    }
}

fn check_state_is_flag(
    name: &str,
    rule_name: &str,
    ctx: &DefinitionContext,
    errors: &mut Vec<ValidationError>,
) {
    match ctx.states.get(name) {
        None => {
            errors.push(ValidationError::UndefinedState {
                name: name.to_string(),
                used_in: rule_name.to_string(),
            });
        }
        Some(StateKind::Counter) => {
            errors.push(ValidationError::ExpectedFlag {
                name: name.to_string(),
                used_in: rule_name.to_string(),
            });
        }
        Some(StateKind::Flag) => {}
    }
}

fn check_state_is_counter(
    name: &str,
    rule_name: &str,
    ctx: &DefinitionContext,
    errors: &mut Vec<ValidationError>,
) {
    match ctx.states.get(name) {
        None => {
            errors.push(ValidationError::UndefinedState {
                name: name.to_string(),
                used_in: rule_name.to_string(),
            });
        }
        Some(StateKind::Flag) => {
            errors.push(ValidationError::ExpectedCounter {
                name: name.to_string(),
                used_in: rule_name.to_string(),
            });
        }
        Some(StateKind::Counter) => {}
    }
}

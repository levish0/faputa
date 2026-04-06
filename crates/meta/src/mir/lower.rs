use crate::hir::{HirExpr, HirProgram};

use super::{MirExpr, MirProgram, MirRule};

pub fn lower(program: &HirProgram) -> MirProgram {
    MirProgram {
        state_decls: program.state_decls.clone(),
        rules: program
            .rules
            .iter()
            .map(|rule| MirRule {
                name: rule.name.clone(),
                inline: rule.inline,
                error_label: rule.error_label.clone(),
                guards: rule.guards.clone(),
                emits: rule.emits.clone(),
                expr: lower_expr(&rule.expr),
                ref_count: rule.ref_count,
            })
            .collect(),
    }
}

fn lower_expr(expr: &HirExpr) -> MirExpr {
    match expr {
        HirExpr::Literal(s) => MirExpr::Literal(s.clone()),
        HirExpr::CharSet(ranges) => MirExpr::CharSet(ranges.clone()),
        HirExpr::Any => MirExpr::Any,
        HirExpr::Boundary(boundary) => MirExpr::Boundary(*boundary),
        HirExpr::RuleRef(idx) => MirExpr::RuleRef(*idx),
        HirExpr::Seq(items) => MirExpr::Seq(items.iter().map(lower_expr).collect()),
        HirExpr::Choice(items) => MirExpr::Choice(items.iter().map(lower_expr).collect()),
        HirExpr::Repeat { expr, min, max } => MirExpr::Repeat {
            expr: Box::new(lower_expr(expr)),
            min: *min,
            max: *max,
        },
        HirExpr::PosLookahead(inner) => MirExpr::PosLookahead(Box::new(lower_expr(inner))),
        HirExpr::NegLookahead(inner) => MirExpr::NegLookahead(Box::new(lower_expr(inner))),
        HirExpr::WithFlag { flag, body } => MirExpr::WithFlag {
            flag: flag.clone(),
            body: Box::new(lower_expr(body)),
        },
        HirExpr::WithCounter {
            counter,
            amount,
            body,
        } => MirExpr::WithCounter {
            counter: counter.clone(),
            amount: *amount,
            body: Box::new(lower_expr(body)),
        },
        HirExpr::When { condition, body } => MirExpr::When {
            condition: condition.clone(),
            body: Box::new(lower_expr(body)),
        },
        HirExpr::DepthLimit { limit, body } => MirExpr::DepthLimit {
            limit: *limit,
            body: Box::new(lower_expr(body)),
        },
        HirExpr::Labeled { expr, label } => MirExpr::Labeled {
            expr: Box::new(lower_expr(expr)),
            label: label.clone(),
        },
    }
}

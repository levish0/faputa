use crate::ir::{DispatchArm as IrDispatchArm, IrExpr, IrProgram};

use super::{DispatchArm, MirExpr, MirProgram, MirRule};

pub fn lower(program: &IrProgram) -> MirProgram {
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

fn lower_expr(expr: &IrExpr) -> MirExpr {
    match expr {
        IrExpr::Literal(s) => MirExpr::Literal(s.clone()),
        IrExpr::CharSet(ranges) => MirExpr::CharSet(ranges.clone()),
        IrExpr::Any => MirExpr::Any,
        IrExpr::Boundary(boundary) => MirExpr::Boundary(*boundary),
        IrExpr::RuleRef(idx) => MirExpr::RuleRef(*idx),
        IrExpr::Seq(items) => MirExpr::Seq(items.iter().map(lower_expr).collect()),
        IrExpr::Choice(items) => MirExpr::Choice(items.iter().map(lower_expr).collect()),
        IrExpr::Dispatch(arms) => MirExpr::Dispatch(arms.iter().map(lower_arm).collect()),
        IrExpr::Repeat { expr, min, max } => MirExpr::Repeat {
            expr: Box::new(lower_expr(expr)),
            min: *min,
            max: *max,
        },
        IrExpr::PosLookahead(inner) => MirExpr::PosLookahead(Box::new(lower_expr(inner))),
        IrExpr::NegLookahead(inner) => MirExpr::NegLookahead(Box::new(lower_expr(inner))),
        IrExpr::WithFlag { flag, body } => MirExpr::WithFlag {
            flag: flag.clone(),
            body: Box::new(lower_expr(body)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => MirExpr::WithCounter {
            counter: counter.clone(),
            amount: *amount,
            body: Box::new(lower_expr(body)),
        },
        IrExpr::When { condition, body } => MirExpr::When {
            condition: condition.clone(),
            body: Box::new(lower_expr(body)),
        },
        IrExpr::DepthLimit { limit, body } => MirExpr::DepthLimit {
            limit: *limit,
            body: Box::new(lower_expr(body)),
        },
        IrExpr::TakeWhile { ranges, min, max } => MirExpr::TakeWhile {
            ranges: ranges.clone(),
            min: *min,
            max: *max,
        },
        IrExpr::Scan {
            plain_ranges,
            specials,
            min,
        } => MirExpr::Scan {
            plain_ranges: plain_ranges.clone(),
            specials: specials.iter().map(lower_arm).collect(),
            min: *min,
        },
        IrExpr::Labeled { expr, label } => MirExpr::Labeled {
            expr: Box::new(lower_expr(expr)),
            label: label.clone(),
        },
    }
}

fn lower_arm(arm: &IrDispatchArm) -> DispatchArm {
    DispatchArm {
        ranges: arm.ranges.clone(),
        expr: Box::new(lower_expr(&arm.expr)),
    }
}

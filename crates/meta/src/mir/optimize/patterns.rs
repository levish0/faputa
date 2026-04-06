use crate::mir::{DispatchArm, MirExpr, MirProgram};

pub(super) fn recognize_take_while(mut program: MirProgram) -> MirProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = recognize_take_while_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "recognize_take_while: transformed");
        }
    }
    program
}

fn recognize_take_while_expr(expr: MirExpr) -> MirExpr {
    match expr {
        MirExpr::Repeat {
            expr: inner,
            min,
            max,
        } => {
            let inner = recognize_take_while_expr(*inner);
            if let MirExpr::CharSet(ranges) = inner {
                MirExpr::TakeWhile { ranges, min, max }
            } else {
                MirExpr::Repeat {
                    expr: Box::new(inner),
                    min,
                    max,
                }
            }
        }
        MirExpr::Loop { body, min } => MirExpr::Loop {
            body: Box::new(recognize_take_while_expr(*body)),
            min,
        },
        MirExpr::Seq(items) => {
            MirExpr::Seq(items.into_iter().map(recognize_take_while_expr).collect())
        }
        MirExpr::Choice(items) => {
            MirExpr::Choice(items.into_iter().map(recognize_take_while_expr).collect())
        }
        MirExpr::Dispatch(arms) => MirExpr::Dispatch(
            arms.into_iter()
                .map(|arm| DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_take_while_expr(*arm.expr)),
                })
                .collect(),
        ),
        MirExpr::PosLookahead(inner) => {
            MirExpr::PosLookahead(Box::new(recognize_take_while_expr(*inner)))
        }
        MirExpr::NegLookahead(inner) => {
            MirExpr::NegLookahead(Box::new(recognize_take_while_expr(*inner)))
        }
        MirExpr::WithFlag { flag, body } => MirExpr::WithFlag {
            flag,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        MirExpr::WithCounter {
            counter,
            amount,
            body,
        } => MirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        MirExpr::When { condition, body } => MirExpr::When {
            condition,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        MirExpr::DepthLimit { limit, body } => MirExpr::DepthLimit {
            limit,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        MirExpr::Scan {
            plain_ranges,
            specials,
            min,
        } => MirExpr::Scan {
            plain_ranges,
            specials: specials
                .into_iter()
                .map(|arm| DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_take_while_expr(*arm.expr)),
                })
                .collect(),
            min,
        },
        MirExpr::SeparatedList { first, rest } => MirExpr::SeparatedList {
            first: Box::new(recognize_take_while_expr(*first)),
            rest: Box::new(recognize_take_while_expr(*rest)),
        },
        MirExpr::Labeled { expr, label } => MirExpr::Labeled {
            expr: Box::new(recognize_take_while_expr(*expr)),
            label,
        },
        other => other,
    }
}

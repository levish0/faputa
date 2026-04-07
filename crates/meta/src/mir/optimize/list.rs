use crate::mir::{MirExpr, MirProgram};

pub(super) fn recognize_separated_list(mut program: MirProgram) -> MirProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = recognize_list_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "recognize_separated_list: transformed");
        }
    }
    program
}

fn recognize_list_expr(expr: MirExpr) -> MirExpr {
    match expr {
        MirExpr::Seq(items) => {
            let items: Vec<_> = items.into_iter().map(recognize_list_expr).collect();
            if let [
                first,
                MirExpr::Repeat {
                    expr: rest,
                    min: 0,
                    max: None,
                },
            ] = items.as_slice()
            {
                if matches!(&**rest, MirExpr::Seq(parts) if parts.len() >= 2) {
                    return MirExpr::SeparatedList {
                        first: Box::new(first.clone()),
                        rest: Box::new((**rest).clone()),
                    };
                }
            }
            MirExpr::Seq(items)
        }
        MirExpr::Choice(items) => {
            MirExpr::Choice(items.into_iter().map(recognize_list_expr).collect())
        }
        MirExpr::Dispatch(arms) => MirExpr::Dispatch(
            arms.into_iter()
                .map(|arm| crate::mir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_list_expr(*arm.expr)),
                })
                .collect(),
        ),
        MirExpr::Repeat { expr, min, max } => MirExpr::Repeat {
            expr: Box::new(recognize_list_expr(*expr)),
            min,
            max,
        },
        MirExpr::RepeatDynamic { expr, min, max } => MirExpr::RepeatDynamic {
            expr: Box::new(recognize_list_expr(*expr)),
            min,
            max,
        },
        MirExpr::Loop { body, min } => MirExpr::Loop {
            body: Box::new(recognize_list_expr(*body)),
            min,
        },
        MirExpr::PosLookahead(inner) => {
            MirExpr::PosLookahead(Box::new(recognize_list_expr(*inner)))
        }
        MirExpr::NegLookahead(inner) => {
            MirExpr::NegLookahead(Box::new(recognize_list_expr(*inner)))
        }
        MirExpr::WithFlag { flag, body } => MirExpr::WithFlag {
            flag,
            body: Box::new(recognize_list_expr(*body)),
        },
        MirExpr::WithCounter {
            counter,
            amount,
            body,
        } => MirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(recognize_list_expr(*body)),
        },
        MirExpr::When { condition, body } => MirExpr::When {
            condition,
            body: Box::new(recognize_list_expr(*body)),
        },
        MirExpr::If {
            condition,
            then_body,
            else_body,
        } => MirExpr::If {
            condition,
            then_body: Box::new(recognize_list_expr(*then_body)),
            else_body: Box::new(recognize_list_expr(*else_body)),
        },
        MirExpr::Measure { counter, body } => MirExpr::Measure {
            counter,
            body: Box::new(recognize_list_expr(*body)),
        },
        MirExpr::DepthLimit { limit, body } => MirExpr::DepthLimit {
            limit,
            body: Box::new(recognize_list_expr(*body)),
        },
        MirExpr::Scan {
            plain_ranges,
            specials,
            min,
        } => MirExpr::Scan {
            plain_ranges,
            specials: specials
                .into_iter()
                .map(|arm| crate::mir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_list_expr(*arm.expr)),
                })
                .collect(),
            min,
        },
        MirExpr::SeparatedList { first, rest } => MirExpr::SeparatedList {
            first: Box::new(recognize_list_expr(*first)),
            rest: Box::new(recognize_list_expr(*rest)),
        },
        MirExpr::Labeled { expr, label } => MirExpr::Labeled {
            expr: Box::new(recognize_list_expr(*expr)),
            label,
        },
        other => other,
    }
}

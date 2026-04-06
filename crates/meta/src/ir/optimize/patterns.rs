use crate::ir::{IrExpr, IrProgram};

pub(super) fn recognize_take_while(mut program: IrProgram) -> IrProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = recognize_take_while_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "recognize_take_while: fused pattern");
        }
    }
    program
}

fn recognize_take_while_expr(expr: IrExpr) -> IrExpr {
    match expr {
        IrExpr::Repeat {
            expr: inner,
            min,
            max,
        } => {
            let inner = recognize_take_while_expr(*inner);
            if let IrExpr::CharSet(ranges) = inner {
                IrExpr::TakeWhile { ranges, min, max }
            } else {
                IrExpr::Repeat {
                    expr: Box::new(inner),
                    min,
                    max,
                }
            }
        }
        IrExpr::Seq(items) => {
            IrExpr::Seq(items.into_iter().map(recognize_take_while_expr).collect())
        }
        IrExpr::Choice(items) => {
            IrExpr::Choice(items.into_iter().map(recognize_take_while_expr).collect())
        }
        IrExpr::Dispatch(arms) => IrExpr::Dispatch(
            arms.into_iter()
                .map(|arm| crate::ir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_take_while_expr(*arm.expr)),
                })
                .collect(),
        ),
        IrExpr::PosLookahead(inner) => {
            IrExpr::PosLookahead(Box::new(recognize_take_while_expr(*inner)))
        }
        IrExpr::NegLookahead(inner) => {
            IrExpr::NegLookahead(Box::new(recognize_take_while_expr(*inner)))
        }
        IrExpr::WithFlag { flag, body } => IrExpr::WithFlag {
            flag,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => IrExpr::WithCounter {
            counter,
            amount,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        IrExpr::When { condition, body } => IrExpr::When {
            condition,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        IrExpr::DepthLimit { limit, body } => IrExpr::DepthLimit {
            limit,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        IrExpr::Scan {
            plain_ranges,
            specials,
            min,
        } => IrExpr::Scan {
            plain_ranges,
            specials: specials
                .into_iter()
                .map(|arm| crate::ir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_take_while_expr(*arm.expr)),
                })
                .collect(),
            min,
        },
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(recognize_take_while_expr(*expr)),
            label,
        },
        other => other,
    }
}

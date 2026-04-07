use crate::mir::{MirExpr, MirProgram, MirRule};

pub(super) fn recognize_unbounded_loop(mut program: MirProgram) -> MirProgram {
    let snapshot = program.rules.clone();
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = recognize_loop_expr(rule.expr.clone(), &snapshot);
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "recognize_unbounded_loop: transformed");
        }
    }
    program
}

fn recognize_loop_expr(expr: MirExpr, rules: &[MirRule]) -> MirExpr {
    match expr {
        MirExpr::Seq(items) => MirExpr::Seq(
            items
                .into_iter()
                .map(|item| recognize_loop_expr(item, rules))
                .collect(),
        ),
        MirExpr::Choice(items) => MirExpr::Choice(
            items
                .into_iter()
                .map(|item| recognize_loop_expr(item, rules))
                .collect(),
        ),
        MirExpr::Dispatch(arms) => MirExpr::Dispatch(
            arms.into_iter()
                .map(|arm| crate::mir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_loop_expr(*arm.expr, rules)),
                })
                .collect(),
        ),
        MirExpr::Repeat { expr, min, max } => {
            let expr = recognize_loop_expr(*expr, rules);
            if max.is_none()
                && min_consumption(&expr, rules, &mut Vec::new()).is_some_and(|n| n > 0)
            {
                MirExpr::Loop {
                    body: Box::new(expr),
                    min,
                }
            } else {
                MirExpr::Repeat {
                    expr: Box::new(expr),
                    min,
                    max,
                }
            }
        }
        MirExpr::RepeatDynamic { expr, min, max } => MirExpr::RepeatDynamic {
            expr: Box::new(recognize_loop_expr(*expr, rules)),
            min,
            max,
        },
        MirExpr::Loop { body, min } => MirExpr::Loop {
            body: Box::new(recognize_loop_expr(*body, rules)),
            min,
        },
        MirExpr::PosLookahead(inner) => {
            MirExpr::PosLookahead(Box::new(recognize_loop_expr(*inner, rules)))
        }
        MirExpr::NegLookahead(inner) => {
            MirExpr::NegLookahead(Box::new(recognize_loop_expr(*inner, rules)))
        }
        MirExpr::WithFlag { flag, body } => MirExpr::WithFlag {
            flag,
            body: Box::new(recognize_loop_expr(*body, rules)),
        },
        MirExpr::WithCounter {
            counter,
            amount,
            body,
        } => MirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(recognize_loop_expr(*body, rules)),
        },
        MirExpr::When { condition, body } => MirExpr::When {
            condition,
            body: Box::new(recognize_loop_expr(*body, rules)),
        },
        MirExpr::If {
            condition,
            then_body,
            else_body,
        } => MirExpr::If {
            condition,
            then_body: Box::new(recognize_loop_expr(*then_body, rules)),
            else_body: Box::new(recognize_loop_expr(*else_body, rules)),
        },
        MirExpr::Measure { counter, body } => MirExpr::Measure {
            counter,
            body: Box::new(recognize_loop_expr(*body, rules)),
        },
        MirExpr::DepthLimit { limit, body } => MirExpr::DepthLimit {
            limit,
            body: Box::new(recognize_loop_expr(*body, rules)),
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
                    expr: Box::new(recognize_loop_expr(*arm.expr, rules)),
                })
                .collect(),
            min,
        },
        MirExpr::SeparatedList { first, rest } => MirExpr::SeparatedList {
            first: Box::new(recognize_loop_expr(*first, rules)),
            rest: Box::new(recognize_loop_expr(*rest, rules)),
        },
        MirExpr::Labeled { expr, label } => MirExpr::Labeled {
            expr: Box::new(recognize_loop_expr(*expr, rules)),
            label,
        },
        other => other,
    }
}

fn min_consumption(expr: &MirExpr, rules: &[MirRule], visiting: &mut Vec<usize>) -> Option<u32> {
    match expr {
        MirExpr::Literal(s) => Some(s.chars().count() as u32),
        MirExpr::CharSet(_) | MirExpr::Any => Some(1),
        MirExpr::Boundary(_) => Some(0),
        MirExpr::RuleRef(idx) => {
            if visiting.contains(idx) {
                return None;
            }
            visiting.push(*idx);
            let result = min_consumption(&rules[*idx].expr, rules, visiting);
            visiting.pop();
            result
        }
        MirExpr::Seq(items) => {
            let mut total = 0u32;
            for item in items {
                total = total.saturating_add(min_consumption(item, rules, visiting)?);
            }
            Some(total)
        }
        MirExpr::Choice(items) => {
            let mut min_value: Option<u32> = None;
            for item in items {
                let value = min_consumption(item, rules, visiting)?;
                min_value = Some(match min_value {
                    Some(current) => current.min(value),
                    None => value,
                });
            }
            min_value
        }
        MirExpr::Dispatch(arms) => {
            let mut min_value: Option<u32> = None;
            for arm in arms {
                let value = min_consumption(&arm.expr, rules, visiting)?;
                min_value = Some(match min_value {
                    Some(current) => current.min(value),
                    None => value,
                });
            }
            min_value
        }
        MirExpr::Repeat { expr, min, .. } => {
            let inner = min_consumption(expr, rules, visiting)?;
            Some(inner.saturating_mul(*min))
        }
        MirExpr::RepeatDynamic { .. } => Some(0),
        MirExpr::Loop { body, min } => {
            let inner = min_consumption(body, rules, visiting)?;
            Some(inner.saturating_mul(*min))
        }
        MirExpr::PosLookahead(_) | MirExpr::NegLookahead(_) => Some(0),
        MirExpr::WithFlag { body, .. }
        | MirExpr::WithCounter { body, .. }
        | MirExpr::Measure { body, .. }
        | MirExpr::DepthLimit { body, .. }
        | MirExpr::Labeled { expr: body, .. } => min_consumption(body, rules, visiting),
        MirExpr::When { .. } => Some(0),
        MirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            let then_min = min_consumption(then_body, rules, visiting)?;
            let else_min = min_consumption(else_body, rules, visiting)?;
            Some(then_min.min(else_min))
        }
        MirExpr::TakeWhile { min, .. } | MirExpr::Scan { min, .. } => Some(*min),
        MirExpr::SeparatedList { first, .. } => min_consumption(first, rules, visiting),
    }
}

use crate::mir::{MirExpr, MirProgram, MirRule};

pub(super) fn recognize_delimited(mut program: MirProgram) -> MirProgram {
    let snapshot = program.rules.clone();
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = recognize_delimited_expr(rule.expr.clone(), &snapshot);
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "recognize_delimited: transformed");
        }
    }
    program
}

fn recognize_delimited_expr(expr: MirExpr, rules: &[MirRule]) -> MirExpr {
    match expr {
        MirExpr::Seq(items) => {
            let items: Vec<_> = items
                .into_iter()
                .map(|item| recognize_delimited_expr(item, rules))
                .collect();
            if let Some(delimited) = build_delimited(&items, rules) {
                delimited
            } else {
                MirExpr::Seq(items)
            }
        }
        MirExpr::Choice(items) => MirExpr::Choice(
            items
                .into_iter()
                .map(|item| recognize_delimited_expr(item, rules))
                .collect(),
        ),
        MirExpr::Dispatch(arms) => MirExpr::Dispatch(
            arms.into_iter()
                .map(|arm| crate::mir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_delimited_expr(*arm.expr, rules)),
                })
                .collect(),
        ),
        MirExpr::Repeat { expr, min, max } => MirExpr::Repeat {
            expr: Box::new(recognize_delimited_expr(*expr, rules)),
            min,
            max,
        },
        MirExpr::Loop { body, min } => MirExpr::Loop {
            body: Box::new(recognize_delimited_expr(*body, rules)),
            min,
        },
        MirExpr::Delimited { open, body, close } => MirExpr::Delimited {
            open: Box::new(recognize_delimited_expr(*open, rules)),
            body: Box::new(recognize_delimited_expr(*body, rules)),
            close: Box::new(recognize_delimited_expr(*close, rules)),
        },
        MirExpr::PosLookahead(inner) => {
            MirExpr::PosLookahead(Box::new(recognize_delimited_expr(*inner, rules)))
        }
        MirExpr::NegLookahead(inner) => {
            MirExpr::NegLookahead(Box::new(recognize_delimited_expr(*inner, rules)))
        }
        MirExpr::WithFlag { flag, body } => MirExpr::WithFlag {
            flag,
            body: Box::new(recognize_delimited_expr(*body, rules)),
        },
        MirExpr::WithCounter {
            counter,
            amount,
            body,
        } => MirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(recognize_delimited_expr(*body, rules)),
        },
        MirExpr::When { condition, body } => MirExpr::When {
            condition,
            body: Box::new(recognize_delimited_expr(*body, rules)),
        },
        MirExpr::DepthLimit { limit, body } => MirExpr::DepthLimit {
            limit,
            body: Box::new(recognize_delimited_expr(*body, rules)),
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
                    expr: Box::new(recognize_delimited_expr(*arm.expr, rules)),
                })
                .collect(),
            min,
        },
        MirExpr::SeparatedList { first, rest } => MirExpr::SeparatedList {
            first: Box::new(recognize_delimited_expr(*first, rules)),
            rest: Box::new(recognize_delimited_expr(*rest, rules)),
        },
        MirExpr::Labeled { expr, label } => MirExpr::Labeled {
            expr: Box::new(recognize_delimited_expr(*expr, rules)),
            label,
        },
        other => other,
    }
}

fn build_delimited(items: &[MirExpr], rules: &[MirRule]) -> Option<MirExpr> {
    if items.len() < 3 {
        return None;
    }

    let open = &items[0];
    let close = items.last().unwrap();
    if !is_delimiter_like(open, rules, &mut Vec::new())
        || !is_delimiter_like(close, rules, &mut Vec::new())
    {
        return None;
    }

    let body = if items.len() == 3 {
        items[1].clone()
    } else {
        MirExpr::Seq(items[1..items.len() - 1].to_vec())
    };
    if is_delimiter_like(&body, rules, &mut Vec::new()) {
        return None;
    }

    Some(MirExpr::Delimited {
        open: Box::new(open.clone()),
        body: Box::new(body),
        close: Box::new(close.clone()),
    })
}

fn is_delimiter_like(expr: &MirExpr, rules: &[MirRule], visiting: &mut Vec<usize>) -> bool {
    match expr {
        MirExpr::Literal(s) => !s.is_empty(),
        MirExpr::CharSet(_) => true,
        MirExpr::Choice(items) => {
            !items.is_empty()
                && items
                    .iter()
                    .all(|item| is_delimiter_like(item, rules, visiting))
        }
        MirExpr::Dispatch(arms) => {
            !arms.is_empty()
                && arms
                    .iter()
                    .all(|arm| is_delimiter_like(&arm.expr, rules, visiting))
        }
        MirExpr::Labeled { expr, .. } => is_delimiter_like(expr, rules, visiting),
        MirExpr::RuleRef(idx) => {
            if visiting.contains(idx) {
                return false;
            }
            visiting.push(*idx);
            let result = is_delimiter_like(&rules[*idx].expr, rules, visiting);
            visiting.pop();
            result
        }
        _ => false,
    }
}

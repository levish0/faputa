use std::collections::HashSet;

use crate::ir::{IrExpr, IrProgram, IrRule};

pub(super) fn inline_trivial_rules(mut program: IrProgram) -> IrProgram {
    let mut referenced_by_others: HashSet<usize> = HashSet::new();
    for rule in &program.rules {
        collect_refs(&rule.expr, &mut referenced_by_others);
    }

    let inline_set: HashSet<usize> = program
        .rules
        .iter()
        .enumerate()
        .filter(|(i, rule)| is_trivial(rule) && referenced_by_others.contains(i))
        .map(|(i, _)| i)
        .collect();

    for (i, rule) in program.rules.iter_mut().enumerate() {
        if inline_set.contains(&i) {
            rule.inline = true;
            tracing::trace!(rule = %rule.name, "inline_trivial_rules: marked for inlining");
        }
    }

    let inline_exprs: Vec<Option<IrExpr>> = program
        .rules
        .iter()
        .map(|r| if r.inline { Some(r.expr.clone()) } else { None })
        .collect();

    for rule in &mut program.rules {
        rule.expr = inline_refs(rule.expr.clone(), &inline_exprs);
    }

    program
}

pub(super) fn inline_small_single_use_rules(mut program: IrProgram) -> IrProgram {
    let mut ref_counts = vec![0usize; program.rules.len()];
    for rule in &program.rules {
        count_raw_refs(&rule.expr, &mut ref_counts);
    }

    let inline_set: HashSet<usize> = program
        .rules
        .iter()
        .enumerate()
        .filter(|(i, rule)| {
            ref_counts[*i] == 1
                && is_small_inline_candidate(rule)
                && !contains_rule_ref(&rule.expr, *i)
        })
        .map(|(i, _)| i)
        .collect();

    if inline_set.is_empty() {
        return program;
    }

    for (i, rule) in program.rules.iter_mut().enumerate() {
        if inline_set.contains(&i) {
            rule.inline = true;
            tracing::trace!(rule = %rule.name, "inline_small_single_use_rules: marked for inlining");
        }
    }

    let inline_exprs: Vec<Option<IrExpr>> = program
        .rules
        .iter()
        .map(|r| if r.inline { Some(r.expr.clone()) } else { None })
        .collect();

    for rule in &mut program.rules {
        rule.expr = inline_refs(rule.expr.clone(), &inline_exprs);
    }

    program
}

fn is_trivial(rule: &IrRule) -> bool {
    if !rule.guards.is_empty() || !rule.emits.is_empty() {
        return false;
    }
    matches!(
        &rule.expr,
        IrExpr::Literal(_)
            | IrExpr::CharSet(_)
            | IrExpr::Any
            | IrExpr::Boundary(_)
            | IrExpr::TakeWhile { .. }
            | IrExpr::Scan { .. }
    )
}

fn is_small_inline_candidate(rule: &IrRule) -> bool {
    if !rule.guards.is_empty() || !rule.emits.is_empty() || rule.error_label.is_some() {
        return false;
    }

    estimate_cost(&rule.expr) <= 14
}

fn estimate_cost(expr: &IrExpr) -> usize {
    match expr {
        IrExpr::Literal(_) | IrExpr::CharSet(_) | IrExpr::Any | IrExpr::Boundary(_) => 1,
        IrExpr::RuleRef(_) => 1,
        IrExpr::Seq(items) | IrExpr::Choice(items) => {
            1 + items.iter().map(estimate_cost).sum::<usize>()
        }
        IrExpr::Dispatch(arms) => {
            1 + arms.iter().map(|arm| estimate_cost(&arm.expr)).sum::<usize>()
        }
        IrExpr::Repeat { expr, .. }
        | IrExpr::PosLookahead(expr)
        | IrExpr::NegLookahead(expr)
        | IrExpr::Labeled { expr, .. } => 1 + estimate_cost(expr),
        IrExpr::WithFlag { body: _, .. }
        | IrExpr::WithCounter { body: _, .. }
        | IrExpr::When { body: _, .. }
        | IrExpr::DepthLimit { body: _, .. } => usize::MAX / 4,
        IrExpr::TakeWhile { .. } => 1,
        IrExpr::Scan { specials, .. } => {
            1 + specials.iter().map(|arm| estimate_cost(&arm.expr)).sum::<usize>()
        }
    }
}

fn contains_rule_ref(expr: &IrExpr, needle: usize) -> bool {
    match expr {
        IrExpr::RuleRef(idx) => *idx == needle,
        IrExpr::Seq(items) | IrExpr::Choice(items) => {
            items.iter().any(|item| contains_rule_ref(item, needle))
        }
        IrExpr::Dispatch(arms) => arms.iter().any(|arm| contains_rule_ref(&arm.expr, needle)),
        IrExpr::Repeat { expr, .. }
        | IrExpr::PosLookahead(expr)
        | IrExpr::NegLookahead(expr)
        | IrExpr::WithFlag { body: expr, .. }
        | IrExpr::WithCounter { body: expr, .. }
        | IrExpr::When { body: expr, .. }
        | IrExpr::DepthLimit { body: expr, .. }
        | IrExpr::Labeled { expr, .. } => contains_rule_ref(expr, needle),
        IrExpr::Scan { specials, .. } => specials
            .iter()
            .any(|arm| contains_rule_ref(&arm.expr, needle)),
        _ => false,
    }
}

fn count_raw_refs(expr: &IrExpr, counts: &mut [usize]) {
    match expr {
        IrExpr::RuleRef(idx) => {
            counts[*idx] += 1;
        }
        IrExpr::Seq(items) | IrExpr::Choice(items) => {
            for item in items {
                count_raw_refs(item, counts);
            }
        }
        IrExpr::Dispatch(arms) => {
            for arm in arms {
                count_raw_refs(&arm.expr, counts);
            }
        }
        IrExpr::Repeat { expr, .. }
        | IrExpr::PosLookahead(expr)
        | IrExpr::NegLookahead(expr)
        | IrExpr::WithFlag { body: expr, .. }
        | IrExpr::WithCounter { body: expr, .. }
        | IrExpr::When { body: expr, .. }
        | IrExpr::DepthLimit { body: expr, .. }
        | IrExpr::Labeled { expr, .. } => count_raw_refs(expr, counts),
        IrExpr::Scan { specials, .. } => {
            for arm in specials {
                count_raw_refs(&arm.expr, counts);
            }
        }
        _ => {}
    }
}

fn inline_refs(expr: IrExpr, inline_exprs: &[Option<IrExpr>]) -> IrExpr {
    match expr {
        IrExpr::RuleRef(idx) => {
            if let Some(Some(inlined)) = inline_exprs.get(idx) {
                inlined.clone()
            } else {
                IrExpr::RuleRef(idx)
            }
        }
        IrExpr::Seq(items) => IrExpr::Seq(
            items
                .into_iter()
                .map(|e| inline_refs(e, inline_exprs))
                .collect(),
        ),
        IrExpr::Choice(items) => IrExpr::Choice(
            items
                .into_iter()
                .map(|e| inline_refs(e, inline_exprs))
                .collect(),
        ),
        IrExpr::Dispatch(arms) => IrExpr::Dispatch(
            arms.into_iter()
                .map(|arm| crate::ir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(inline_refs(*arm.expr, inline_exprs)),
                })
                .collect(),
        ),
        IrExpr::Repeat { expr, min, max } => IrExpr::Repeat {
            expr: Box::new(inline_refs(*expr, inline_exprs)),
            min,
            max,
        },
        IrExpr::PosLookahead(inner) => {
            IrExpr::PosLookahead(Box::new(inline_refs(*inner, inline_exprs)))
        }
        IrExpr::NegLookahead(inner) => {
            IrExpr::NegLookahead(Box::new(inline_refs(*inner, inline_exprs)))
        }
        IrExpr::WithFlag { flag, body } => IrExpr::WithFlag {
            flag,
            body: Box::new(inline_refs(*body, inline_exprs)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => IrExpr::WithCounter {
            counter,
            amount,
            body: Box::new(inline_refs(*body, inline_exprs)),
        },
        IrExpr::When { condition, body } => IrExpr::When {
            condition,
            body: Box::new(inline_refs(*body, inline_exprs)),
        },
        IrExpr::DepthLimit { limit, body } => IrExpr::DepthLimit {
            limit,
            body: Box::new(inline_refs(*body, inline_exprs)),
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
                    expr: Box::new(inline_refs(*arm.expr, inline_exprs)),
                })
                .collect(),
            min,
        },
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(inline_refs(*expr, inline_exprs)),
            label,
        },
        other => other,
    }
}

pub(super) fn eliminate_dead_rules(program: IrProgram) -> IrProgram {
    program
}

fn collect_refs(expr: &IrExpr, refs: &mut HashSet<usize>) {
    match expr {
        IrExpr::RuleRef(idx) => {
            refs.insert(*idx);
        }
        IrExpr::Seq(items) | IrExpr::Choice(items) => {
            for item in items {
                collect_refs(item, refs);
            }
        }
        IrExpr::Dispatch(arms) => {
            for arm in arms {
                collect_refs(&arm.expr, refs);
            }
        }
        IrExpr::Repeat { expr, .. }
        | IrExpr::PosLookahead(expr)
        | IrExpr::NegLookahead(expr)
        | IrExpr::WithFlag { body: expr, .. }
        | IrExpr::WithCounter { body: expr, .. }
        | IrExpr::When { body: expr, .. }
        | IrExpr::DepthLimit { body: expr, .. }
        | IrExpr::Labeled { expr, .. } => {
            collect_refs(expr, refs);
        }
        IrExpr::Scan { specials, .. } => {
            for arm in specials {
                collect_refs(&arm.expr, refs);
            }
        }
        _ => {}
    }
}

pub(super) fn compute_ref_counts(mut program: IrProgram) -> IrProgram {
    let mut counts = vec![0usize; program.rules.len()];
    for rule in &program.rules {
        if !rule.inline {
            count_refs(&rule.expr, &mut counts);
        }
    }
    for (i, rule) in program.rules.iter_mut().enumerate() {
        rule.ref_count = counts[i];
    }
    program
}

fn count_refs(expr: &IrExpr, counts: &mut [usize]) {
    match expr {
        IrExpr::RuleRef(idx) => {
            counts[*idx] += 1;
        }
        IrExpr::Seq(items) | IrExpr::Choice(items) => {
            for item in items {
                count_refs(item, counts);
            }
        }
        IrExpr::Dispatch(arms) => {
            for arm in arms {
                count_refs(&arm.expr, counts);
            }
        }
        IrExpr::Repeat { expr, .. }
        | IrExpr::PosLookahead(expr)
        | IrExpr::NegLookahead(expr)
        | IrExpr::WithFlag { body: expr, .. }
        | IrExpr::WithCounter { body: expr, .. }
        | IrExpr::When { body: expr, .. }
        | IrExpr::DepthLimit { body: expr, .. }
        | IrExpr::Labeled { expr, .. } => {
            count_refs(expr, counts);
        }
        IrExpr::Scan { specials, .. } => {
            for arm in specials {
                count_refs(&arm.expr, counts);
            }
        }
        _ => {}
    }
}

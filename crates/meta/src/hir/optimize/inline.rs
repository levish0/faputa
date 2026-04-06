use std::collections::HashSet;

use crate::hir::{HirExpr, HirProgram, HirRule};

pub(super) fn inline_trivial_rules(mut program: HirProgram) -> HirProgram {
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

    let inline_exprs: Vec<Option<HirExpr>> = program
        .rules
        .iter()
        .map(|r| if r.inline { Some(r.expr.clone()) } else { None })
        .collect();

    for rule in &mut program.rules {
        rule.expr = inline_refs(rule.expr.clone(), &inline_exprs);
    }

    program
}

pub(super) fn inline_small_single_use_rules(mut program: HirProgram) -> HirProgram {
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

    let inline_exprs: Vec<Option<HirExpr>> = program
        .rules
        .iter()
        .map(|r| if r.inline { Some(r.expr.clone()) } else { None })
        .collect();

    for rule in &mut program.rules {
        rule.expr = inline_refs(rule.expr.clone(), &inline_exprs);
    }

    program
}

fn is_trivial(rule: &HirRule) -> bool {
    if !rule.guards.is_empty() || !rule.emits.is_empty() {
        return false;
    }
    matches!(
        &rule.expr,
        HirExpr::Literal(_) | HirExpr::CharSet(_) | HirExpr::Any | HirExpr::Boundary(_)
    )
}

fn is_small_inline_candidate(rule: &HirRule) -> bool {
    if !rule.guards.is_empty() || !rule.emits.is_empty() || rule.error_label.is_some() {
        return false;
    }

    estimate_cost(&rule.expr) <= 14
}

fn estimate_cost(expr: &HirExpr) -> usize {
    match expr {
        HirExpr::Literal(_) | HirExpr::CharSet(_) | HirExpr::Any | HirExpr::Boundary(_) => 1,
        HirExpr::RuleRef(_) => 1,
        HirExpr::Seq(items) | HirExpr::Choice(items) => {
            1 + items.iter().map(estimate_cost).sum::<usize>()
        }
        HirExpr::Repeat { expr, .. }
        | HirExpr::PosLookahead(expr)
        | HirExpr::NegLookahead(expr)
        | HirExpr::Labeled { expr, .. } => 1 + estimate_cost(expr),
        HirExpr::WithFlag { body: _, .. }
        | HirExpr::WithCounter { body: _, .. }
        | HirExpr::When { body: _, .. }
        | HirExpr::DepthLimit { body: _, .. } => usize::MAX / 4,
    }
}

fn contains_rule_ref(expr: &HirExpr, needle: usize) -> bool {
    match expr {
        HirExpr::RuleRef(idx) => *idx == needle,
        HirExpr::Seq(items) | HirExpr::Choice(items) => {
            items.iter().any(|item| contains_rule_ref(item, needle))
        }
        HirExpr::Repeat { expr, .. }
        | HirExpr::PosLookahead(expr)
        | HirExpr::NegLookahead(expr)
        | HirExpr::WithFlag { body: expr, .. }
        | HirExpr::WithCounter { body: expr, .. }
        | HirExpr::When { body: expr, .. }
        | HirExpr::DepthLimit { body: expr, .. }
        | HirExpr::Labeled { expr, .. } => contains_rule_ref(expr, needle),
        _ => false,
    }
}

fn count_raw_refs(expr: &HirExpr, counts: &mut [usize]) {
    match expr {
        HirExpr::RuleRef(idx) => {
            counts[*idx] += 1;
        }
        HirExpr::Seq(items) | HirExpr::Choice(items) => {
            for item in items {
                count_raw_refs(item, counts);
            }
        }
        HirExpr::Repeat { expr, .. }
        | HirExpr::PosLookahead(expr)
        | HirExpr::NegLookahead(expr)
        | HirExpr::WithFlag { body: expr, .. }
        | HirExpr::WithCounter { body: expr, .. }
        | HirExpr::When { body: expr, .. }
        | HirExpr::DepthLimit { body: expr, .. }
        | HirExpr::Labeled { expr, .. } => count_raw_refs(expr, counts),
        _ => {}
    }
}

fn inline_refs(expr: HirExpr, inline_exprs: &[Option<HirExpr>]) -> HirExpr {
    match expr {
        HirExpr::RuleRef(idx) => {
            if let Some(Some(inlined)) = inline_exprs.get(idx) {
                inlined.clone()
            } else {
                HirExpr::RuleRef(idx)
            }
        }
        HirExpr::Seq(items) => HirExpr::Seq(
            items
                .into_iter()
                .map(|e| inline_refs(e, inline_exprs))
                .collect(),
        ),
        HirExpr::Choice(items) => HirExpr::Choice(
            items
                .into_iter()
                .map(|e| inline_refs(e, inline_exprs))
                .collect(),
        ),
        HirExpr::Repeat { expr, min, max } => HirExpr::Repeat {
            expr: Box::new(inline_refs(*expr, inline_exprs)),
            min,
            max,
        },
        HirExpr::PosLookahead(inner) => {
            HirExpr::PosLookahead(Box::new(inline_refs(*inner, inline_exprs)))
        }
        HirExpr::NegLookahead(inner) => {
            HirExpr::NegLookahead(Box::new(inline_refs(*inner, inline_exprs)))
        }
        HirExpr::WithFlag { flag, body } => HirExpr::WithFlag {
            flag,
            body: Box::new(inline_refs(*body, inline_exprs)),
        },
        HirExpr::WithCounter {
            counter,
            amount,
            body,
        } => HirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(inline_refs(*body, inline_exprs)),
        },
        HirExpr::When { condition, body } => HirExpr::When {
            condition,
            body: Box::new(inline_refs(*body, inline_exprs)),
        },
        HirExpr::DepthLimit { limit, body } => HirExpr::DepthLimit {
            limit,
            body: Box::new(inline_refs(*body, inline_exprs)),
        },
        HirExpr::Labeled { expr, label } => HirExpr::Labeled {
            expr: Box::new(inline_refs(*expr, inline_exprs)),
            label,
        },
        other => other,
    }
}

pub(super) fn eliminate_dead_rules(program: HirProgram) -> HirProgram {
    program
}

fn collect_refs(expr: &HirExpr, refs: &mut HashSet<usize>) {
    match expr {
        HirExpr::RuleRef(idx) => {
            refs.insert(*idx);
        }
        HirExpr::Seq(items) | HirExpr::Choice(items) => {
            for item in items {
                collect_refs(item, refs);
            }
        }
        HirExpr::Repeat { expr, .. }
        | HirExpr::PosLookahead(expr)
        | HirExpr::NegLookahead(expr)
        | HirExpr::WithFlag { body: expr, .. }
        | HirExpr::WithCounter { body: expr, .. }
        | HirExpr::When { body: expr, .. }
        | HirExpr::DepthLimit { body: expr, .. }
        | HirExpr::Labeled { expr, .. } => {
            collect_refs(expr, refs);
        }
        _ => {}
    }
}

pub(super) fn compute_ref_counts(mut program: HirProgram) -> HirProgram {
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

fn count_refs(expr: &HirExpr, counts: &mut [usize]) {
    match expr {
        HirExpr::RuleRef(idx) => {
            counts[*idx] += 1;
        }
        HirExpr::Seq(items) | HirExpr::Choice(items) => {
            for item in items {
                count_refs(item, counts);
            }
        }
        HirExpr::Repeat { expr, .. }
        | HirExpr::PosLookahead(expr)
        | HirExpr::NegLookahead(expr)
        | HirExpr::WithFlag { body: expr, .. }
        | HirExpr::WithCounter { body: expr, .. }
        | HirExpr::When { body: expr, .. }
        | HirExpr::DepthLimit { body: expr, .. }
        | HirExpr::Labeled { expr, .. } => {
            count_refs(expr, counts);
        }
        _ => {}
    }
}

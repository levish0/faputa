use crate::hir::CharRange;
use crate::mir::{DispatchArm, MirExpr, MirProgram, MirRule};

pub(super) fn recognize_scan_repeat(mut program: MirProgram) -> MirProgram {
    let snapshot = program.rules.clone();
    for rule in &mut program.rules {
        rule.expr = recognize_scan_expr(rule.expr.clone(), &snapshot);
    }
    program
}

fn recognize_scan_expr(expr: MirExpr, rules: &[MirRule]) -> MirExpr {
    match expr {
        MirExpr::Seq(items) => MirExpr::Seq(
            items
                .into_iter()
                .map(|item| recognize_scan_expr(item, rules))
                .collect(),
        ),
        MirExpr::Choice(items) => MirExpr::Choice(
            items
                .into_iter()
                .map(|item| recognize_scan_expr(item, rules))
                .collect(),
        ),
        MirExpr::Dispatch(arms) => MirExpr::Dispatch(
            arms.into_iter()
                .map(|arm| DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_scan_expr(*arm.expr, rules)),
                })
                .collect(),
        ),
        MirExpr::Repeat { expr, min, max } => {
            let inner = recognize_scan_expr(*expr, rules);
            if max.is_none() {
                if let Some(scan) = build_scan_expr(&inner, min, rules) {
                    return scan;
                }
            }
            MirExpr::Repeat {
                expr: Box::new(inner),
                min,
                max,
            }
        }
        MirExpr::PosLookahead(inner) => {
            MirExpr::PosLookahead(Box::new(recognize_scan_expr(*inner, rules)))
        }
        MirExpr::NegLookahead(inner) => {
            MirExpr::NegLookahead(Box::new(recognize_scan_expr(*inner, rules)))
        }
        MirExpr::WithFlag { flag, body } => MirExpr::WithFlag {
            flag,
            body: Box::new(recognize_scan_expr(*body, rules)),
        },
        MirExpr::WithCounter {
            counter,
            amount,
            body,
        } => MirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(recognize_scan_expr(*body, rules)),
        },
        MirExpr::When { condition, body } => MirExpr::When {
            condition,
            body: Box::new(recognize_scan_expr(*body, rules)),
        },
        MirExpr::DepthLimit { limit, body } => MirExpr::DepthLimit {
            limit,
            body: Box::new(recognize_scan_expr(*body, rules)),
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
                    expr: Box::new(recognize_scan_expr(*arm.expr, rules)),
                })
                .collect(),
            min,
        },
        MirExpr::SeparatedList { first, rest } => MirExpr::SeparatedList {
            first: Box::new(recognize_scan_expr(*first, rules)),
            rest: Box::new(recognize_scan_expr(*rest, rules)),
        },
        MirExpr::Labeled { expr, label } => MirExpr::Labeled {
            expr: Box::new(recognize_scan_expr(*expr, rules)),
            label,
        },
        other => other,
    }
}

fn build_scan_expr(expr: &MirExpr, min: u32, rules: &[MirRule]) -> Option<MirExpr> {
    let MirExpr::Dispatch(arms) = expr else {
        return None;
    };

    let mut plain_ranges = Vec::new();
    let mut specials = Vec::new();

    for arm in arms {
        let mut visiting = Vec::new();
        let is_plain = single_char_consumption_ranges(&arm.expr, rules, &mut visiting)
            .map(super::coalesce_ranges)
            .is_some_and(|ranges| ranges == arm.ranges);

        if is_plain {
            plain_ranges.extend(arm.ranges.iter().copied());
        } else {
            specials.push(arm.clone());
        }
    }

    let plain_ranges = super::coalesce_ranges(plain_ranges);
    if plain_ranges.is_empty() || specials.is_empty() {
        return None;
    }

    Some(MirExpr::Scan {
        plain_ranges,
        specials,
        min,
    })
}

fn single_char_consumption_ranges(
    expr: &MirExpr,
    rules: &[MirRule],
    visiting: &mut Vec<usize>,
) -> Option<Vec<CharRange>> {
    match expr {
        MirExpr::Literal(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => Some(vec![CharRange::single(ch)]),
                _ => None,
            }
        }
        MirExpr::CharSet(ranges) => Some(ranges.clone()),
        MirExpr::Any => Some(vec![CharRange::new(char::MIN, char::MAX)]),
        MirExpr::RuleRef(idx) => {
            if visiting.contains(idx) {
                return None;
            }
            visiting.push(*idx);
            let result = single_char_consumption_ranges(&rules[*idx].expr, rules, visiting);
            visiting.pop();
            result
        }
        MirExpr::Seq(items) => match items.as_slice() {
            [single] => single_char_consumption_ranges(single, rules, visiting),
            [MirExpr::NegLookahead(inner), MirExpr::Any] => {
                let disallowed = single_char_set(inner, rules, visiting)?;
                Some(invert_ranges(&super::coalesce_ranges(disallowed)))
            }
            _ => None,
        },
        MirExpr::TakeWhile { ranges, min, max } if *min == 1 && *max == Some(1) => {
            Some(ranges.clone())
        }
        MirExpr::Labeled { expr, .. } => single_char_consumption_ranges(expr, rules, visiting),
        _ => None,
    }
}

fn single_char_set(
    expr: &MirExpr,
    rules: &[MirRule],
    visiting: &mut Vec<usize>,
) -> Option<Vec<CharRange>> {
    match expr {
        MirExpr::Literal(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => Some(vec![CharRange::single(ch)]),
                _ => None,
            }
        }
        MirExpr::CharSet(ranges) => Some(ranges.clone()),
        MirExpr::Any => Some(vec![CharRange::new(char::MIN, char::MAX)]),
        MirExpr::RuleRef(idx) => {
            if visiting.contains(idx) {
                return None;
            }
            visiting.push(*idx);
            let result = single_char_set(&rules[*idx].expr, rules, visiting);
            visiting.pop();
            result
        }
        MirExpr::Choice(items) => {
            let mut ranges = Vec::new();
            for item in items {
                ranges.extend(single_char_set(item, rules, visiting)?);
            }
            Some(super::coalesce_ranges(ranges))
        }
        MirExpr::Labeled { expr, .. } => single_char_set(expr, rules, visiting),
        _ => None,
    }
}

fn invert_ranges(ranges: &[CharRange]) -> Vec<CharRange> {
    if ranges.is_empty() {
        return vec![CharRange::new(char::MIN, char::MAX)];
    }
    let mut result = Vec::new();
    let mut cursor = char::MIN;
    for range in ranges {
        if cursor < range.start {
            if let Some(end) = prev_scalar(range.start) {
                if cursor <= end {
                    result.push(CharRange::new(cursor, end));
                }
            }
        }
        cursor = match next_scalar(range.end) {
            Some(next) => next,
            None => return result,
        };
    }
    if cursor <= char::MAX {
        result.push(CharRange::new(cursor, char::MAX));
    }
    result
}

fn next_scalar(ch: char) -> Option<char> {
    let mut value = ch as u32 + 1;
    while value <= char::MAX as u32 {
        if let Some(next) = char::from_u32(value) {
            return Some(next);
        }
        value += 1;
    }
    None
}

fn prev_scalar(ch: char) -> Option<char> {
    let mut value = ch as u32;
    while value > 0 {
        value -= 1;
        if let Some(prev) = char::from_u32(value) {
            return Some(prev);
        }
    }
    None
}

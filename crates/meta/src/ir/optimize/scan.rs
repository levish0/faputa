use crate::ir::{CharRange, DispatchArm, IrExpr, IrProgram, IrRule};

pub(super) fn recognize_scan_repeat(mut program: IrProgram) -> IrProgram {
    let snapshot = program.rules.clone();
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = recognize_scan_expr(rule.expr.clone(), &snapshot);
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "recognize_scan_repeat: lowered dispatch repeat");
        }
    }
    program
}

fn recognize_scan_expr(expr: IrExpr, rules: &[IrRule]) -> IrExpr {
    match expr {
        IrExpr::Seq(items) => IrExpr::Seq(
            items
                .into_iter()
                .map(|item| recognize_scan_expr(item, rules))
                .collect(),
        ),
        IrExpr::Choice(items) => IrExpr::Choice(
            items
                .into_iter()
                .map(|item| recognize_scan_expr(item, rules))
                .collect(),
        ),
        IrExpr::Dispatch(arms) => IrExpr::Dispatch(
            arms.into_iter()
                .map(|arm| DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_scan_expr(*arm.expr, rules)),
                })
                .collect(),
        ),
        IrExpr::Repeat { expr, min, max } => {
            let inner = recognize_scan_expr(*expr, rules);
            if max.is_none() {
                if let Some(scan) = build_scan_expr(&inner, min, rules) {
                    return scan;
                }
            }
            IrExpr::Repeat {
                expr: Box::new(inner),
                min,
                max,
            }
        }
        IrExpr::PosLookahead(inner) => {
            IrExpr::PosLookahead(Box::new(recognize_scan_expr(*inner, rules)))
        }
        IrExpr::NegLookahead(inner) => {
            IrExpr::NegLookahead(Box::new(recognize_scan_expr(*inner, rules)))
        }
        IrExpr::WithFlag { flag, body } => IrExpr::WithFlag {
            flag,
            body: Box::new(recognize_scan_expr(*body, rules)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => IrExpr::WithCounter {
            counter,
            amount,
            body: Box::new(recognize_scan_expr(*body, rules)),
        },
        IrExpr::When { condition, body } => IrExpr::When {
            condition,
            body: Box::new(recognize_scan_expr(*body, rules)),
        },
        IrExpr::DepthLimit { limit, body } => IrExpr::DepthLimit {
            limit,
            body: Box::new(recognize_scan_expr(*body, rules)),
        },
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(recognize_scan_expr(*expr, rules)),
            label,
        },
        IrExpr::Scan {
            plain_ranges,
            specials,
            min,
        } => IrExpr::Scan {
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
        other => other,
    }
}

fn build_scan_expr(expr: &IrExpr, min: u32, rules: &[IrRule]) -> Option<IrExpr> {
    let IrExpr::Dispatch(arms) = expr else {
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

    Some(IrExpr::Scan {
        plain_ranges,
        specials,
        min,
    })
}

fn single_char_consumption_ranges(
    expr: &IrExpr,
    rules: &[IrRule],
    visiting: &mut Vec<usize>,
) -> Option<Vec<CharRange>> {
    match expr {
        IrExpr::Literal(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => Some(vec![CharRange::single(ch)]),
                _ => None,
            }
        }
        IrExpr::CharSet(ranges) => Some(ranges.clone()),
        IrExpr::Any => Some(vec![CharRange::new(char::MIN, char::MAX)]),
        IrExpr::RuleRef(idx) => {
            if visiting.contains(idx) {
                return None;
            }
            visiting.push(*idx);
            let result = single_char_consumption_ranges(&rules[*idx].expr, rules, visiting);
            visiting.pop();
            result
        }
        IrExpr::Seq(items) => match items.as_slice() {
            [single] => single_char_consumption_ranges(single, rules, visiting),
            [IrExpr::NegLookahead(inner), IrExpr::Any] => {
                let disallowed = single_char_set(inner, rules, visiting)?;
                Some(invert_ranges(&super::coalesce_ranges(disallowed)))
            }
            _ => None,
        },
        IrExpr::TakeWhile { ranges, min, max } if *min == 1 && *max == Some(1) => {
            Some(ranges.clone())
        }
        IrExpr::Labeled { expr, .. } => single_char_consumption_ranges(expr, rules, visiting),
        _ => None,
    }
}

fn single_char_set(
    expr: &IrExpr,
    rules: &[IrRule],
    visiting: &mut Vec<usize>,
) -> Option<Vec<CharRange>> {
    match expr {
        IrExpr::Literal(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => Some(vec![CharRange::single(ch)]),
                _ => None,
            }
        }
        IrExpr::CharSet(ranges) => Some(ranges.clone()),
        IrExpr::Any => Some(vec![CharRange::new(char::MIN, char::MAX)]),
        IrExpr::RuleRef(idx) => {
            if visiting.contains(idx) {
                return None;
            }
            visiting.push(*idx);
            let result = single_char_set(&rules[*idx].expr, rules, visiting);
            visiting.pop();
            result
        }
        IrExpr::Choice(items) => {
            let mut ranges = Vec::new();
            for item in items {
                ranges.extend(single_char_set(item, rules, visiting)?);
            }
            Some(super::coalesce_ranges(ranges))
        }
        IrExpr::Labeled { expr, .. } => single_char_set(expr, rules, visiting),
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

use crate::ir::CharRange;
use crate::mir::{DispatchArm, MirExpr, MirProgram, MirRule};

pub(super) fn recognize_dispatch(mut program: MirProgram) -> MirProgram {
    let snapshot = program.rules.clone();
    for rule in &mut program.rules {
        rule.expr = recognize_dispatch_expr(rule.expr.clone(), &snapshot);
    }
    program
}

fn recognize_dispatch_expr(expr: MirExpr, rules: &[MirRule]) -> MirExpr {
    match expr {
        MirExpr::Seq(items) => MirExpr::Seq(
            items
                .into_iter()
                .map(|item| recognize_dispatch_expr(item, rules))
                .collect(),
        ),
        MirExpr::Choice(items) => {
            let items: Vec<_> = items
                .into_iter()
                .map(|item| recognize_dispatch_expr(item, rules))
                .collect();
            if let Some(arms) = build_dispatch_arms(&items, rules) {
                MirExpr::Dispatch(arms)
            } else {
                MirExpr::Choice(items)
            }
        }
        MirExpr::Dispatch(arms) => MirExpr::Dispatch(
            arms.into_iter()
                .map(|arm| DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_dispatch_expr(*arm.expr, rules)),
                })
                .collect(),
        ),
        MirExpr::Repeat { expr, min, max } => MirExpr::Repeat {
            expr: Box::new(recognize_dispatch_expr(*expr, rules)),
            min,
            max,
        },
        MirExpr::PosLookahead(inner) => {
            MirExpr::PosLookahead(Box::new(recognize_dispatch_expr(*inner, rules)))
        }
        MirExpr::NegLookahead(inner) => {
            MirExpr::NegLookahead(Box::new(recognize_dispatch_expr(*inner, rules)))
        }
        MirExpr::WithFlag { flag, body } => MirExpr::WithFlag {
            flag,
            body: Box::new(recognize_dispatch_expr(*body, rules)),
        },
        MirExpr::WithCounter {
            counter,
            amount,
            body,
        } => MirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(recognize_dispatch_expr(*body, rules)),
        },
        MirExpr::When { condition, body } => MirExpr::When {
            condition,
            body: Box::new(recognize_dispatch_expr(*body, rules)),
        },
        MirExpr::DepthLimit { limit, body } => MirExpr::DepthLimit {
            limit,
            body: Box::new(recognize_dispatch_expr(*body, rules)),
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
                    expr: Box::new(recognize_dispatch_expr(*arm.expr, rules)),
                })
                .collect(),
            min,
        },
        MirExpr::SeparatedList { first, rest } => MirExpr::SeparatedList {
            first: Box::new(recognize_dispatch_expr(*first, rules)),
            rest: Box::new(recognize_dispatch_expr(*rest, rules)),
        },
        MirExpr::Labeled { expr, label } => MirExpr::Labeled {
            expr: Box::new(recognize_dispatch_expr(*expr, rules)),
            label,
        },
        other => other,
    }
}

#[derive(Debug, Clone)]
struct FirstChars {
    ranges: Vec<CharRange>,
    nullable: bool,
}

fn build_dispatch_arms(items: &[MirExpr], rules: &[MirRule]) -> Option<Vec<DispatchArm>> {
    let mut groups: Vec<(Vec<CharRange>, Vec<MirExpr>)> = Vec::new();
    let mut seen: Vec<Vec<CharRange>> = Vec::new();

    for item in items {
        let first = first_chars(item, rules, &mut Vec::new())?;
        if first.nullable || first.ranges.is_empty() {
            return None;
        }

        let ranges = super::coalesce_ranges(first.ranges);
        if let Some((_, exprs)) = groups.iter_mut().find(|(existing, _)| *existing == ranges) {
            exprs.push(item.clone());
            continue;
        }
        if seen.iter().any(|existing| ranges_overlap_any(&ranges, existing)) {
            return None;
        }
        seen.push(ranges.clone());
        groups.push((ranges, vec![item.clone()]));
    }

    if groups.len() <= 1 {
        return None;
    }

    Some(
        groups
            .into_iter()
            .map(|(ranges, exprs)| DispatchArm {
                ranges,
                expr: Box::new(if exprs.len() == 1 {
                    exprs.into_iter().next().unwrap()
                } else {
                    MirExpr::Choice(exprs)
                }),
            })
            .collect(),
    )
}

fn first_chars(expr: &MirExpr, rules: &[MirRule], visiting: &mut Vec<usize>) -> Option<FirstChars> {
    match expr {
        MirExpr::Literal(s) => {
            let mut chars = s.chars();
            if let Some(ch) = chars.next() {
                Some(FirstChars {
                    ranges: vec![CharRange::single(ch)],
                    nullable: false,
                })
            } else {
                Some(FirstChars {
                    ranges: Vec::new(),
                    nullable: true,
                })
            }
        }
        MirExpr::CharSet(ranges) => Some(FirstChars {
            ranges: super::coalesce_ranges(ranges.clone()),
            nullable: false,
        }),
        MirExpr::Any => Some(FirstChars {
            ranges: vec![CharRange::new(char::MIN, char::MAX)],
            nullable: false,
        }),
        MirExpr::Boundary(_) => Some(FirstChars {
            ranges: Vec::new(),
            nullable: true,
        }),
        MirExpr::RuleRef(idx) => {
            if visiting.contains(idx) {
                return None;
            }
            visiting.push(*idx);
            let result = first_chars(&rules[*idx].expr, rules, visiting);
            visiting.pop();
            result
        }
        MirExpr::Seq(items) => {
            if let [MirExpr::NegLookahead(inner), MirExpr::Any, ..] = items.as_slice() {
                if let Some(disallowed) = single_char_set(inner, rules, visiting) {
                    return Some(FirstChars {
                        ranges: invert_ranges(&super::coalesce_ranges(disallowed)),
                        nullable: false,
                    });
                }
            }

            let mut ranges = Vec::new();
            let mut nullable = true;
            for item in items {
                let first = first_chars(item, rules, visiting)?;
                ranges.extend(first.ranges);
                if !first.nullable {
                    nullable = false;
                    break;
                }
            }
            Some(FirstChars {
                ranges: super::coalesce_ranges(ranges),
                nullable,
            })
        }
        MirExpr::Choice(items) => {
            let mut ranges = Vec::new();
            let mut nullable = false;
            for item in items {
                let first = first_chars(item, rules, visiting)?;
                ranges.extend(first.ranges);
                nullable |= first.nullable;
            }
            Some(FirstChars {
                ranges: super::coalesce_ranges(ranges),
                nullable,
            })
        }
        MirExpr::Dispatch(arms) => {
            let mut ranges = Vec::new();
            for arm in arms {
                ranges.extend(arm.ranges.iter().copied());
            }
            Some(FirstChars {
                ranges: super::coalesce_ranges(ranges),
                nullable: false,
            })
        }
        MirExpr::Repeat { expr, min, .. } => {
            let first = first_chars(expr, rules, visiting)?;
            Some(FirstChars {
                ranges: first.ranges,
                nullable: *min == 0 || first.nullable,
            })
        }
        MirExpr::PosLookahead(inner) => {
            let first = first_chars(inner, rules, visiting)?;
            Some(FirstChars {
                ranges: first.ranges,
                nullable: true,
            })
        }
        MirExpr::NegLookahead(_) => None,
        MirExpr::WithFlag { body, .. }
        | MirExpr::WithCounter { body, .. }
        | MirExpr::DepthLimit { body, .. }
        | MirExpr::Labeled { expr: body, .. } => first_chars(body, rules, visiting),
        MirExpr::When { .. } => None,
        MirExpr::TakeWhile { ranges, min, .. } => Some(FirstChars {
            ranges: super::coalesce_ranges(ranges.clone()),
            nullable: *min == 0,
        }),
        MirExpr::Scan {
            plain_ranges,
            specials,
            min,
        } => {
            let mut ranges = plain_ranges.clone();
            for arm in specials {
                ranges.extend(arm.ranges.iter().copied());
            }
            Some(FirstChars {
                ranges: super::coalesce_ranges(ranges),
                nullable: *min == 0,
            })
        }
        MirExpr::SeparatedList { first, .. } => first_chars(first, rules, visiting),
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

fn ranges_overlap_any(left: &[CharRange], right: &[CharRange]) -> bool {
    left.iter()
        .any(|l| right.iter().any(|r| ranges_overlap(*l, *r)))
}

fn ranges_overlap(left: CharRange, right: CharRange) -> bool {
    left.start <= right.end && right.start <= left.end
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

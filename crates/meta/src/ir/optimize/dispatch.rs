use crate::ir::{CharRange, DispatchArm, IrExpr, IrProgram, IrRule};

pub(super) fn recognize_dispatch(mut program: IrProgram) -> IrProgram {
    let snapshot = program.rules.clone();
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = recognize_dispatch_expr(rule.expr.clone(), &snapshot);
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "recognize_dispatch: lowered choice");
        }
    }
    program
}

fn recognize_dispatch_expr(expr: IrExpr, rules: &[IrRule]) -> IrExpr {
    match expr {
        IrExpr::Seq(items) => IrExpr::Seq(
            items
                .into_iter()
                .map(|item| recognize_dispatch_expr(item, rules))
                .collect(),
        ),
        IrExpr::Choice(items) => {
            let items: Vec<_> = items
                .into_iter()
                .map(|item| recognize_dispatch_expr(item, rules))
                .collect();
            if let Some(arms) = build_dispatch_arms(&items, rules) {
                IrExpr::Dispatch(arms)
            } else {
                IrExpr::Choice(items)
            }
        }
        IrExpr::Dispatch(arms) => IrExpr::Dispatch(
            arms.into_iter()
                .map(|arm| DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(recognize_dispatch_expr(*arm.expr, rules)),
                })
                .collect(),
        ),
        IrExpr::Repeat { expr, min, max } => IrExpr::Repeat {
            expr: Box::new(recognize_dispatch_expr(*expr, rules)),
            min,
            max,
        },
        IrExpr::PosLookahead(inner) => {
            IrExpr::PosLookahead(Box::new(recognize_dispatch_expr(*inner, rules)))
        }
        IrExpr::NegLookahead(inner) => {
            IrExpr::NegLookahead(Box::new(recognize_dispatch_expr(*inner, rules)))
        }
        IrExpr::WithFlag { flag, body } => IrExpr::WithFlag {
            flag,
            body: Box::new(recognize_dispatch_expr(*body, rules)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => IrExpr::WithCounter {
            counter,
            amount,
            body: Box::new(recognize_dispatch_expr(*body, rules)),
        },
        IrExpr::When { condition, body } => IrExpr::When {
            condition,
            body: Box::new(recognize_dispatch_expr(*body, rules)),
        },
        IrExpr::DepthLimit { limit, body } => IrExpr::DepthLimit {
            limit,
            body: Box::new(recognize_dispatch_expr(*body, rules)),
        },
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(recognize_dispatch_expr(*expr, rules)),
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
                    expr: Box::new(recognize_dispatch_expr(*arm.expr, rules)),
                })
                .collect(),
            min,
        },
        other => other,
    }
}

#[derive(Debug, Clone)]
struct FirstChars {
    ranges: Vec<CharRange>,
    nullable: bool,
}

fn build_dispatch_arms(items: &[IrExpr], rules: &[IrRule]) -> Option<Vec<DispatchArm>> {
    let mut groups: Vec<(Vec<CharRange>, Vec<IrExpr>)> = Vec::new();
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
        if seen
            .iter()
            .any(|existing| ranges_overlap_any(&ranges, existing))
        {
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
                    IrExpr::Choice(exprs)
                }),
            })
            .collect(),
    )
}

fn first_chars(expr: &IrExpr, rules: &[IrRule], visiting: &mut Vec<usize>) -> Option<FirstChars> {
    match expr {
        IrExpr::Literal(s) => {
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
        IrExpr::CharSet(ranges) => Some(FirstChars {
            ranges: super::coalesce_ranges(ranges.clone()),
            nullable: false,
        }),
        IrExpr::Any => Some(FirstChars {
            ranges: vec![CharRange::new(char::MIN, char::MAX)],
            nullable: false,
        }),
        IrExpr::Boundary(_) => Some(FirstChars {
            ranges: Vec::new(),
            nullable: true,
        }),
        IrExpr::RuleRef(idx) => {
            if visiting.contains(idx) {
                return None;
            }
            visiting.push(*idx);
            let result = first_chars(&rules[*idx].expr, rules, visiting);
            visiting.pop();
            result
        }
        IrExpr::Seq(items) => {
            if let [IrExpr::NegLookahead(inner), IrExpr::Any, ..] = items.as_slice() {
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
        IrExpr::Choice(items) => {
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
        IrExpr::Dispatch(arms) => {
            let mut ranges = Vec::new();
            for arm in arms {
                ranges.extend(arm.ranges.iter().copied());
            }
            Some(FirstChars {
                ranges: super::coalesce_ranges(ranges),
                nullable: false,
            })
        }
        IrExpr::Scan {
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
        IrExpr::Repeat { expr, min, .. } => {
            let first = first_chars(expr, rules, visiting)?;
            Some(FirstChars {
                ranges: first.ranges,
                nullable: *min == 0 || first.nullable,
            })
        }
        IrExpr::PosLookahead(inner) => {
            let first = first_chars(inner, rules, visiting)?;
            Some(FirstChars {
                ranges: first.ranges,
                nullable: true,
            })
        }
        IrExpr::NegLookahead(_) => None,
        IrExpr::WithFlag { body, .. }
        | IrExpr::WithCounter { body, .. }
        | IrExpr::DepthLimit { body, .. }
        | IrExpr::Labeled { expr: body, .. } => first_chars(body, rules, visiting),
        IrExpr::When { .. } => None,
        IrExpr::TakeWhile { ranges, min, .. } => Some(FirstChars {
            ranges: super::coalesce_ranges(ranges.clone()),
            nullable: *min == 0,
        }),
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

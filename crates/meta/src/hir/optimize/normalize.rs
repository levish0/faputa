use crate::hir::{CharRange, HirExpr, HirProgram};

pub(super) fn single_char_to_charset(mut program: HirProgram) -> HirProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = single_char_to_charset_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "single_char_to_charset: transformed");
        }
    }
    program
}

fn single_char_to_charset_expr(expr: HirExpr) -> HirExpr {
    match expr {
        HirExpr::Choice(items) => {
            let items: Vec<_> = items
                .into_iter()
                .map(|item| {
                    let item = single_char_to_charset_expr(item);
                    if let HirExpr::Literal(ref s) = item {
                        let mut chars = s.chars();
                        if let (Some(ch), None) = (chars.next(), chars.next()) {
                            return HirExpr::CharSet(vec![CharRange::single(ch)]);
                        }
                    }
                    item
                })
                .collect();
            HirExpr::Choice(items)
        }
        HirExpr::Seq(items) => {
            HirExpr::Seq(items.into_iter().map(single_char_to_charset_expr).collect())
        }
        HirExpr::Repeat { expr, min, max } => HirExpr::Repeat {
            expr: Box::new(single_char_to_charset_expr(*expr)),
            min,
            max,
        },
        HirExpr::PosLookahead(inner) => {
            HirExpr::PosLookahead(Box::new(single_char_to_charset_expr(*inner)))
        }
        HirExpr::NegLookahead(inner) => {
            HirExpr::NegLookahead(Box::new(single_char_to_charset_expr(*inner)))
        }
        HirExpr::WithFlag { flag, body } => HirExpr::WithFlag {
            flag,
            body: Box::new(single_char_to_charset_expr(*body)),
        },
        HirExpr::WithCounter {
            counter,
            amount,
            body,
        } => HirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(single_char_to_charset_expr(*body)),
        },
        HirExpr::When { condition, body } => HirExpr::When {
            condition,
            body: Box::new(single_char_to_charset_expr(*body)),
        },
        HirExpr::DepthLimit { limit, body } => HirExpr::DepthLimit {
            limit,
            body: Box::new(single_char_to_charset_expr(*body)),
        },
        HirExpr::Labeled { expr, label } => HirExpr::Labeled {
            expr: Box::new(single_char_to_charset_expr(*expr)),
            label,
        },
        other => other,
    }
}

pub(super) fn flatten(mut program: HirProgram) -> HirProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = flatten_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "flatten: transformed");
        }
    }
    program
}

fn flatten_expr(expr: HirExpr) -> HirExpr {
    match expr {
        HirExpr::Seq(items) => {
            let mut flat = Vec::new();
            for item in items {
                let item = flatten_expr(item);
                match item {
                    HirExpr::Seq(inner) => flat.extend(inner),
                    other => flat.push(other),
                }
            }
            if flat.len() == 1 {
                flat.into_iter().next().unwrap()
            } else {
                HirExpr::Seq(flat)
            }
        }
        HirExpr::Choice(items) => {
            let mut flat = Vec::new();
            for item in items {
                let item = flatten_expr(item);
                match item {
                    HirExpr::Choice(inner) => flat.extend(inner),
                    other => flat.push(other),
                }
            }
            if flat.len() == 1 {
                flat.into_iter().next().unwrap()
            } else {
                HirExpr::Choice(flat)
            }
        }
        HirExpr::Repeat { expr, min, max } => HirExpr::Repeat {
            expr: Box::new(flatten_expr(*expr)),
            min,
            max,
        },
        HirExpr::PosLookahead(inner) => HirExpr::PosLookahead(Box::new(flatten_expr(*inner))),
        HirExpr::NegLookahead(inner) => HirExpr::NegLookahead(Box::new(flatten_expr(*inner))),
        HirExpr::WithFlag { flag, body } => HirExpr::WithFlag {
            flag,
            body: Box::new(flatten_expr(*body)),
        },
        HirExpr::WithCounter {
            counter,
            amount,
            body,
        } => HirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(flatten_expr(*body)),
        },
        HirExpr::When { condition, body } => HirExpr::When {
            condition,
            body: Box::new(flatten_expr(*body)),
        },
        HirExpr::DepthLimit { limit, body } => HirExpr::DepthLimit {
            limit,
            body: Box::new(flatten_expr(*body)),
        },
        HirExpr::Labeled { expr, label } => HirExpr::Labeled {
            expr: Box::new(flatten_expr(*expr)),
            label,
        },
        other => other,
    }
}

pub(super) fn merge_charsets(mut program: HirProgram) -> HirProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = merge_charsets_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "merge_charsets: transformed");
        }
    }
    program
}

fn merge_charsets_expr(expr: HirExpr) -> HirExpr {
    match expr {
        HirExpr::Choice(items) => {
            let items: Vec<_> = items.into_iter().map(merge_charsets_expr).collect();
            let mut merged_ranges: Vec<CharRange> = Vec::new();
            let mut other: Vec<HirExpr> = Vec::new();

            for item in items {
                match item {
                    HirExpr::CharSet(ranges) => merged_ranges.extend(ranges),
                    HirExpr::Any => other.push(HirExpr::Any),
                    _ => other.push(item),
                }
            }

            if !merged_ranges.is_empty() {
                merged_ranges.sort();
                merged_ranges = super::coalesce_ranges(merged_ranges);
                let mut result = vec![HirExpr::CharSet(merged_ranges)];
                result.extend(other);
                if result.len() == 1 {
                    result.into_iter().next().unwrap()
                } else {
                    HirExpr::Choice(result)
                }
            } else if other.len() == 1 {
                other.into_iter().next().unwrap()
            } else {
                HirExpr::Choice(other)
            }
        }
        HirExpr::Seq(items) => HirExpr::Seq(items.into_iter().map(merge_charsets_expr).collect()),
        HirExpr::Repeat { expr, min, max } => HirExpr::Repeat {
            expr: Box::new(merge_charsets_expr(*expr)),
            min,
            max,
        },
        HirExpr::PosLookahead(inner) => {
            HirExpr::PosLookahead(Box::new(merge_charsets_expr(*inner)))
        }
        HirExpr::NegLookahead(inner) => {
            HirExpr::NegLookahead(Box::new(merge_charsets_expr(*inner)))
        }
        HirExpr::WithFlag { flag, body } => HirExpr::WithFlag {
            flag,
            body: Box::new(merge_charsets_expr(*body)),
        },
        HirExpr::WithCounter {
            counter,
            amount,
            body,
        } => HirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(merge_charsets_expr(*body)),
        },
        HirExpr::When { condition, body } => HirExpr::When {
            condition,
            body: Box::new(merge_charsets_expr(*body)),
        },
        HirExpr::DepthLimit { limit, body } => HirExpr::DepthLimit {
            limit,
            body: Box::new(merge_charsets_expr(*body)),
        },
        HirExpr::Labeled { expr, label } => HirExpr::Labeled {
            expr: Box::new(merge_charsets_expr(*expr)),
            label,
        },
        other => other,
    }
}

pub(super) fn fuse_literals(mut program: HirProgram) -> HirProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = fuse_literals_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "fuse_literals: transformed");
        }
    }
    program
}

fn fuse_literals_expr(expr: HirExpr) -> HirExpr {
    match expr {
        HirExpr::Seq(items) => {
            let items: Vec<_> = items.into_iter().map(fuse_literals_expr).collect();
            let mut fused: Vec<HirExpr> = Vec::new();

            for item in items {
                match (&mut fused.last_mut(), &item) {
                    (Some(HirExpr::Literal(prev)), HirExpr::Literal(next)) => {
                        prev.push_str(next);
                    }
                    _ => fused.push(item),
                }
            }

            if fused.len() == 1 {
                fused.into_iter().next().unwrap()
            } else {
                HirExpr::Seq(fused)
            }
        }
        HirExpr::Choice(items) => {
            HirExpr::Choice(items.into_iter().map(fuse_literals_expr).collect())
        }
        HirExpr::Repeat { expr, min, max } => HirExpr::Repeat {
            expr: Box::new(fuse_literals_expr(*expr)),
            min,
            max,
        },
        HirExpr::PosLookahead(inner) => {
            HirExpr::PosLookahead(Box::new(fuse_literals_expr(*inner)))
        }
        HirExpr::NegLookahead(inner) => {
            HirExpr::NegLookahead(Box::new(fuse_literals_expr(*inner)))
        }
        HirExpr::WithFlag { flag, body } => HirExpr::WithFlag {
            flag,
            body: Box::new(fuse_literals_expr(*body)),
        },
        HirExpr::WithCounter {
            counter,
            amount,
            body,
        } => HirExpr::WithCounter {
            counter,
            amount,
            body: Box::new(fuse_literals_expr(*body)),
        },
        HirExpr::When { condition, body } => HirExpr::When {
            condition,
            body: Box::new(fuse_literals_expr(*body)),
        },
        HirExpr::DepthLimit { limit, body } => HirExpr::DepthLimit {
            limit,
            body: Box::new(fuse_literals_expr(*body)),
        },
        HirExpr::Labeled { expr, label } => HirExpr::Labeled {
            expr: Box::new(fuse_literals_expr(*expr)),
            label,
        },
        other => other,
    }
}

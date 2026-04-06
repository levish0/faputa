use crate::ir::{CharRange, IrExpr, IrProgram};

pub(super) fn single_char_to_charset(mut program: IrProgram) -> IrProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = single_char_to_charset_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "single_char_to_charset: transformed");
        }
    }
    program
}

fn single_char_to_charset_expr(expr: IrExpr) -> IrExpr {
    match expr {
        IrExpr::Choice(items) => {
            let items: Vec<_> = items
                .into_iter()
                .map(|item| {
                    let item = single_char_to_charset_expr(item);
                    if let IrExpr::Literal(ref s) = item {
                        let mut chars = s.chars();
                        if let (Some(ch), None) = (chars.next(), chars.next()) {
                            return IrExpr::CharSet(vec![CharRange::single(ch)]);
                        }
                    }
                    item
                })
                .collect();
            IrExpr::Choice(items)
        }
        IrExpr::Seq(items) => {
            IrExpr::Seq(items.into_iter().map(single_char_to_charset_expr).collect())
        }
        IrExpr::Repeat { expr, min, max } => IrExpr::Repeat {
            expr: Box::new(single_char_to_charset_expr(*expr)),
            min,
            max,
        },
        IrExpr::PosLookahead(inner) => {
            IrExpr::PosLookahead(Box::new(single_char_to_charset_expr(*inner)))
        }
        IrExpr::NegLookahead(inner) => {
            IrExpr::NegLookahead(Box::new(single_char_to_charset_expr(*inner)))
        }
        IrExpr::WithFlag { flag, body } => IrExpr::WithFlag {
            flag,
            body: Box::new(single_char_to_charset_expr(*body)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => IrExpr::WithCounter {
            counter,
            amount,
            body: Box::new(single_char_to_charset_expr(*body)),
        },
        IrExpr::When { condition, body } => IrExpr::When {
            condition,
            body: Box::new(single_char_to_charset_expr(*body)),
        },
        IrExpr::DepthLimit { limit, body } => IrExpr::DepthLimit {
            limit,
            body: Box::new(single_char_to_charset_expr(*body)),
        },
        IrExpr::Dispatch(arms) => IrExpr::Dispatch(
            arms.into_iter()
                .map(|arm| crate::ir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(single_char_to_charset_expr(*arm.expr)),
                })
                .collect(),
        ),
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
                    expr: Box::new(single_char_to_charset_expr(*arm.expr)),
                })
                .collect(),
            min,
        },
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(single_char_to_charset_expr(*expr)),
            label,
        },
        other => other,
    }
}

pub(super) fn flatten(mut program: IrProgram) -> IrProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = flatten_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "flatten: transformed");
        }
    }
    program
}

fn flatten_expr(expr: IrExpr) -> IrExpr {
    match expr {
        IrExpr::Seq(items) => {
            let mut flat = Vec::new();
            for item in items {
                let item = flatten_expr(item);
                match item {
                    IrExpr::Seq(inner) => flat.extend(inner),
                    other => flat.push(other),
                }
            }
            if flat.len() == 1 {
                flat.into_iter().next().unwrap()
            } else {
                IrExpr::Seq(flat)
            }
        }
        IrExpr::Choice(items) => {
            let mut flat = Vec::new();
            for item in items {
                let item = flatten_expr(item);
                match item {
                    IrExpr::Choice(inner) => flat.extend(inner),
                    other => flat.push(other),
                }
            }
            if flat.len() == 1 {
                flat.into_iter().next().unwrap()
            } else {
                IrExpr::Choice(flat)
            }
        }
        IrExpr::Repeat { expr, min, max } => IrExpr::Repeat {
            expr: Box::new(flatten_expr(*expr)),
            min,
            max,
        },
        IrExpr::PosLookahead(inner) => IrExpr::PosLookahead(Box::new(flatten_expr(*inner))),
        IrExpr::NegLookahead(inner) => IrExpr::NegLookahead(Box::new(flatten_expr(*inner))),
        IrExpr::WithFlag { flag, body } => IrExpr::WithFlag {
            flag,
            body: Box::new(flatten_expr(*body)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => IrExpr::WithCounter {
            counter,
            amount,
            body: Box::new(flatten_expr(*body)),
        },
        IrExpr::When { condition, body } => IrExpr::When {
            condition,
            body: Box::new(flatten_expr(*body)),
        },
        IrExpr::DepthLimit { limit, body } => IrExpr::DepthLimit {
            limit,
            body: Box::new(flatten_expr(*body)),
        },
        IrExpr::Dispatch(arms) => IrExpr::Dispatch(
            arms.into_iter()
                .map(|arm| crate::ir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(flatten_expr(*arm.expr)),
                })
                .collect(),
        ),
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
                    expr: Box::new(flatten_expr(*arm.expr)),
                })
                .collect(),
            min,
        },
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(flatten_expr(*expr)),
            label,
        },
        other => other,
    }
}

pub(super) fn merge_charsets(mut program: IrProgram) -> IrProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = merge_charsets_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "merge_charsets: transformed");
        }
    }
    program
}

fn merge_charsets_expr(expr: IrExpr) -> IrExpr {
    match expr {
        IrExpr::Choice(items) => {
            let items: Vec<_> = items.into_iter().map(merge_charsets_expr).collect();
            let mut merged_ranges: Vec<CharRange> = Vec::new();
            let mut other: Vec<IrExpr> = Vec::new();

            for item in items {
                match item {
                    IrExpr::CharSet(ranges) => merged_ranges.extend(ranges),
                    IrExpr::Any => other.push(IrExpr::Any),
                    _ => other.push(item),
                }
            }

            if !merged_ranges.is_empty() {
                merged_ranges.sort();
                merged_ranges = super::coalesce_ranges(merged_ranges);
                let mut result = vec![IrExpr::CharSet(merged_ranges)];
                result.extend(other);
                if result.len() == 1 {
                    result.into_iter().next().unwrap()
                } else {
                    IrExpr::Choice(result)
                }
            } else if other.len() == 1 {
                other.into_iter().next().unwrap()
            } else {
                IrExpr::Choice(other)
            }
        }
        IrExpr::Seq(items) => IrExpr::Seq(items.into_iter().map(merge_charsets_expr).collect()),
        IrExpr::Repeat { expr, min, max } => IrExpr::Repeat {
            expr: Box::new(merge_charsets_expr(*expr)),
            min,
            max,
        },
        IrExpr::PosLookahead(inner) => IrExpr::PosLookahead(Box::new(merge_charsets_expr(*inner))),
        IrExpr::NegLookahead(inner) => IrExpr::NegLookahead(Box::new(merge_charsets_expr(*inner))),
        IrExpr::WithFlag { flag, body } => IrExpr::WithFlag {
            flag,
            body: Box::new(merge_charsets_expr(*body)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => IrExpr::WithCounter {
            counter,
            amount,
            body: Box::new(merge_charsets_expr(*body)),
        },
        IrExpr::When { condition, body } => IrExpr::When {
            condition,
            body: Box::new(merge_charsets_expr(*body)),
        },
        IrExpr::DepthLimit { limit, body } => IrExpr::DepthLimit {
            limit,
            body: Box::new(merge_charsets_expr(*body)),
        },
        IrExpr::Dispatch(arms) => IrExpr::Dispatch(
            arms.into_iter()
                .map(|arm| crate::ir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(merge_charsets_expr(*arm.expr)),
                })
                .collect(),
        ),
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
                    expr: Box::new(merge_charsets_expr(*arm.expr)),
                })
                .collect(),
            min,
        },
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(merge_charsets_expr(*expr)),
            label,
        },
        other => other,
    }
}

pub(super) fn fuse_literals(mut program: IrProgram) -> IrProgram {
    for rule in &mut program.rules {
        let before = rule.expr.clone();
        rule.expr = fuse_literals_expr(rule.expr.clone());
        if rule.expr != before {
            tracing::trace!(rule = %rule.name, "fuse_literals: transformed");
        }
    }
    program
}

fn fuse_literals_expr(expr: IrExpr) -> IrExpr {
    match expr {
        IrExpr::Seq(items) => {
            let items: Vec<_> = items.into_iter().map(fuse_literals_expr).collect();
            let mut fused: Vec<IrExpr> = Vec::new();

            for item in items {
                match (&mut fused.last_mut(), &item) {
                    (Some(IrExpr::Literal(prev)), IrExpr::Literal(next)) => {
                        prev.push_str(next);
                    }
                    _ => fused.push(item),
                }
            }

            if fused.len() == 1 {
                fused.into_iter().next().unwrap()
            } else {
                IrExpr::Seq(fused)
            }
        }
        IrExpr::Choice(items) => {
            IrExpr::Choice(items.into_iter().map(fuse_literals_expr).collect())
        }
        IrExpr::Repeat { expr, min, max } => IrExpr::Repeat {
            expr: Box::new(fuse_literals_expr(*expr)),
            min,
            max,
        },
        IrExpr::PosLookahead(inner) => IrExpr::PosLookahead(Box::new(fuse_literals_expr(*inner))),
        IrExpr::NegLookahead(inner) => IrExpr::NegLookahead(Box::new(fuse_literals_expr(*inner))),
        IrExpr::WithFlag { flag, body } => IrExpr::WithFlag {
            flag,
            body: Box::new(fuse_literals_expr(*body)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => IrExpr::WithCounter {
            counter,
            amount,
            body: Box::new(fuse_literals_expr(*body)),
        },
        IrExpr::When { condition, body } => IrExpr::When {
            condition,
            body: Box::new(fuse_literals_expr(*body)),
        },
        IrExpr::DepthLimit { limit, body } => IrExpr::DepthLimit {
            limit,
            body: Box::new(fuse_literals_expr(*body)),
        },
        IrExpr::Dispatch(arms) => IrExpr::Dispatch(
            arms.into_iter()
                .map(|arm| crate::ir::DispatchArm {
                    ranges: arm.ranges,
                    expr: Box::new(fuse_literals_expr(*arm.expr)),
                })
                .collect(),
        ),
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
                    expr: Box::new(fuse_literals_expr(*arm.expr)),
                })
                .collect(),
            min,
        },
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(fuse_literals_expr(*expr)),
            label,
        },
        other => other,
    }
}

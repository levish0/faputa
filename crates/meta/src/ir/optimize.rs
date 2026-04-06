//! IR → IR optimization passes.
//!
//! Each pass is a function `IrProgram → IrProgram`.
//! `optimize()` chains them in order.

use std::collections::HashSet;

use super::{CharRange, IrExpr, IrProgram, IrRule};

/// Run all optimization passes on the program.
pub fn optimize(program: IrProgram) -> IrProgram {
    let program = flatten(program);
    let program = merge_charsets(program);
    let program = fuse_literals(program);
    let program = inline_trivial_rules(program);
    let program = eliminate_dead_rules(program);
    program
}

// ── Pass 1: Flatten nested Seq/Choice ──

fn flatten(mut program: IrProgram) -> IrProgram {
    for rule in &mut program.rules {
        rule.expr = flatten_expr(rule.expr.clone());
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
        other => other,
    }
}

// ── Pass 2: Merge CharSets in Choice branches ──

fn merge_charsets(mut program: IrProgram) -> IrProgram {
    for rule in &mut program.rules {
        rule.expr = merge_charsets_expr(rule.expr.clone());
    }
    program
}

fn merge_charsets_expr(expr: IrExpr) -> IrExpr {
    match expr {
        IrExpr::Choice(items) => {
            let items: Vec<_> = items.into_iter().map(merge_charsets_expr).collect();

            // Partition into charset branches and non-charset branches.
            let mut merged_ranges: Vec<CharRange> = Vec::new();
            let mut other: Vec<IrExpr> = Vec::new();

            for item in items {
                match item {
                    IrExpr::CharSet(ranges) => merged_ranges.extend(ranges),
                    IrExpr::Any => {
                        // ANY absorbs all charsets — just return Any in choice
                        other.push(IrExpr::Any);
                    }
                    _ => other.push(item),
                }
            }

            if !merged_ranges.is_empty() {
                merged_ranges.sort();
                merged_ranges = coalesce_ranges(merged_ranges);
                // Put the merged charset at the front of the choice.
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
        IrExpr::Seq(items) => {
            IrExpr::Seq(items.into_iter().map(merge_charsets_expr).collect())
        }
        IrExpr::Repeat { expr, min, max } => IrExpr::Repeat {
            expr: Box::new(merge_charsets_expr(*expr)),
            min,
            max,
        },
        IrExpr::PosLookahead(inner) => {
            IrExpr::PosLookahead(Box::new(merge_charsets_expr(*inner)))
        }
        IrExpr::NegLookahead(inner) => {
            IrExpr::NegLookahead(Box::new(merge_charsets_expr(*inner)))
        }
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
        other => other,
    }
}

/// Merge overlapping/adjacent sorted char ranges.
fn coalesce_ranges(mut ranges: Vec<CharRange>) -> Vec<CharRange> {
    if ranges.is_empty() {
        return ranges;
    }
    ranges.sort_by_key(|r| (r.start, r.end));
    let mut result = vec![ranges[0]];
    for r in &ranges[1..] {
        let last = result.last_mut().unwrap();
        // Check if ranges overlap or are adjacent (e.g., 'a'..'z' and '{' next)
        let last_end_next = char::from_u32(last.end as u32 + 1);
        if r.start <= last.end || last_end_next == Some(r.start) {
            last.end = last.end.max(r.end);
        } else {
            result.push(*r);
        }
    }
    result
}

// ── Pass 3: Fuse adjacent Literals in Seq ──

fn fuse_literals(mut program: IrProgram) -> IrProgram {
    for rule in &mut program.rules {
        rule.expr = fuse_literals_expr(rule.expr.clone());
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
        IrExpr::PosLookahead(inner) => {
            IrExpr::PosLookahead(Box::new(fuse_literals_expr(*inner)))
        }
        IrExpr::NegLookahead(inner) => {
            IrExpr::NegLookahead(Box::new(fuse_literals_expr(*inner)))
        }
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
        other => other,
    }
}

// ── Pass 4: Inline trivial rules ──
//
// A rule is trivial if it has no guards, no emits, and its expression is
// a terminal or a simple combinator (CharSet, Literal, Any, Boundary).

fn inline_trivial_rules(mut program: IrProgram) -> IrProgram {
    // Collect which rules are referenced by other rules.
    let mut referenced_by_others: HashSet<usize> = HashSet::new();
    for rule in &program.rules {
        collect_refs(&rule.expr, &mut referenced_by_others);
    }

    // Only inline trivial rules that are referenced by at least one other rule.
    // Rules that are never referenced are entry points — keep them as-is.
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
        }
    }

    // Substitute inlined rules at call sites.
    let inline_exprs: Vec<Option<IrExpr>> = program
        .rules
        .iter()
        .map(|r| {
            if r.inline {
                Some(r.expr.clone())
            } else {
                None
            }
        })
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
    )
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
        IrExpr::Seq(items) => {
            IrExpr::Seq(items.into_iter().map(|e| inline_refs(e, inline_exprs)).collect())
        }
        IrExpr::Choice(items) => {
            IrExpr::Choice(items.into_iter().map(|e| inline_refs(e, inline_exprs)).collect())
        }
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
        other => other,
    }
}

// ── Pass 5: Eliminate dead rules ──
//
// Rules that are inlined and never referenced externally can be removed.
// We keep all non-inlined rules and any rule that is still referenced.

fn eliminate_dead_rules(mut program: IrProgram) -> IrProgram {
    // Collect all referenced rule indices from non-inlined rules.
    let mut referenced = HashSet::new();
    for rule in &program.rules {
        if !rule.inline {
            collect_refs(&rule.expr, &mut referenced);
        }
    }

    // Also keep all non-inlined rules (they're entry points).
    let keep: Vec<bool> = program
        .rules
        .iter()
        .enumerate()
        .map(|(i, rule)| !rule.inline || referenced.contains(&i))
        .collect();

    // Build old→new index mapping.
    let mut index_map: Vec<Option<usize>> = vec![None; program.rules.len()];
    let mut new_idx = 0;
    for (old_idx, &kept) in keep.iter().enumerate() {
        if kept {
            index_map[old_idx] = Some(new_idx);
            new_idx += 1;
        }
    }

    // Filter and reindex.
    let new_rules: Vec<IrRule> = program
        .rules
        .into_iter()
        .zip(keep.iter())
        .filter(|(_, kept)| **kept)
        .map(|(mut rule, _)| {
            rule.expr = reindex_refs(rule.expr, &index_map);
            rule
        })
        .collect();

    program.rules = new_rules;
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
        IrExpr::Repeat { expr, .. }
        | IrExpr::PosLookahead(expr)
        | IrExpr::NegLookahead(expr)
        | IrExpr::WithFlag { body: expr, .. }
        | IrExpr::WithCounter { body: expr, .. }
        | IrExpr::When { body: expr, .. }
        | IrExpr::DepthLimit { body: expr, .. } => {
            collect_refs(expr, refs);
        }
        _ => {}
    }
}

fn reindex_refs(expr: IrExpr, index_map: &[Option<usize>]) -> IrExpr {
    match expr {
        IrExpr::RuleRef(idx) => IrExpr::RuleRef(index_map[idx].expect("dangling rule ref")),
        IrExpr::Seq(items) => {
            IrExpr::Seq(items.into_iter().map(|e| reindex_refs(e, index_map)).collect())
        }
        IrExpr::Choice(items) => {
            IrExpr::Choice(items.into_iter().map(|e| reindex_refs(e, index_map)).collect())
        }
        IrExpr::Repeat { expr, min, max } => IrExpr::Repeat {
            expr: Box::new(reindex_refs(*expr, index_map)),
            min,
            max,
        },
        IrExpr::PosLookahead(inner) => {
            IrExpr::PosLookahead(Box::new(reindex_refs(*inner, index_map)))
        }
        IrExpr::NegLookahead(inner) => {
            IrExpr::NegLookahead(Box::new(reindex_refs(*inner, index_map)))
        }
        IrExpr::WithFlag { flag, body } => IrExpr::WithFlag {
            flag,
            body: Box::new(reindex_refs(*body, index_map)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => IrExpr::WithCounter {
            counter,
            amount,
            body: Box::new(reindex_refs(*body, index_map)),
        },
        IrExpr::When { condition, body } => IrExpr::When {
            condition,
            body: Box::new(reindex_refs(*body, index_map)),
        },
        IrExpr::DepthLimit { limit, body } => IrExpr::DepthLimit {
            limit,
            body: Box::new(reindex_refs(*body, index_map)),
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::lower;

    fn optimized(source: &str) -> IrProgram {
        let grammar = crate::compile(source).expect("compile failed");
        let ir = lower(&grammar);
        optimize(ir)
    }

    #[test]
    fn charset_merge_in_choice() {
        // alpha = { 'a'..'z' | 'A'..'Z' | "_" }
        // The two CharSets should merge; "_" as Literal stays separate.
        let ir = optimized(r#"alpha = { 'a'..'z' | 'A'..'Z' | "_" }"#);
        match &ir.rules[0].expr {
            IrExpr::Choice(items) => {
                // First item should be merged CharSet
                match &items[0] {
                    IrExpr::CharSet(ranges) => {
                        assert_eq!(ranges.len(), 2); // A..Z and a..z
                    }
                    other => panic!("expected CharSet, got {other:?}"),
                }
            }
            other => panic!("expected Choice, got {other:?}"),
        }
    }

    #[test]
    fn charset_merge_all_ranges() {
        // All branches are char ranges → single CharSet, no Choice.
        let ir = optimized("alpha = { 'a'..'z' | 'A'..'Z' }");
        match &ir.rules[0].expr {
            IrExpr::CharSet(ranges) => {
                assert_eq!(ranges.len(), 2);
            }
            other => panic!("expected CharSet, got {other:?}"),
        }
    }

    #[test]
    fn adjacent_ranges_coalesced() {
        // 'a'..'m' | 'n'..'z' → 'a'..'z'
        let ir = optimized("az = { 'a'..'m' | 'n'..'z' }");
        match &ir.rules[0].expr {
            IrExpr::CharSet(ranges) => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0], CharRange::new('a', 'z'));
            }
            other => panic!("expected CharSet, got {other:?}"),
        }
    }

    #[test]
    fn literal_fusion() {
        let ir = optimized(r#"kw = { "h" "e" "l" "l" "o" }"#);
        assert_eq!(ir.rules[0].expr, IrExpr::Literal("hello".into()));
    }

    #[test]
    fn trivial_rule_inlined() {
        let ir = optimized(
            r#"
            digit = { '0'..'9' }
            number = { digit+ }
        "#,
        );
        // digit should be inlined into number.
        // number's repeat body should be CharSet, not RuleRef.
        let number = ir.rules.iter().find(|r| r.name == "number").unwrap();
        match &number.expr {
            IrExpr::Repeat { expr, .. } => {
                assert!(matches!(**expr, IrExpr::CharSet(_)));
            }
            other => panic!("expected Repeat, got {other:?}"),
        }
    }

    #[test]
    fn dead_inlined_rule_eliminated() {
        let ir = optimized(
            r#"
            digit = { '0'..'9' }
            number = { digit+ }
        "#,
        );
        // digit is trivial, inlined, and only referenced by number.
        // After inlining, digit should be removed.
        assert_eq!(ir.rules.len(), 1);
        assert_eq!(ir.rules[0].name, "number");
    }

    #[test]
    fn non_trivial_rule_not_inlined() {
        let ir = optimized(
            r#"
            alpha = { 'a'..'z' | 'A'..'Z' }
            digit = { '0'..'9' }
            ident = { alpha (alpha | digit)* }
        "#,
        );
        // alpha and digit are trivial → inlined and eliminated.
        // ident should remain with CharSet in its body.
        let ident = ir.rules.iter().find(|r| r.name == "ident").unwrap();
        match &ident.expr {
            IrExpr::Seq(items) => {
                assert!(matches!(&items[0], IrExpr::CharSet(_)));
            }
            other => panic!("expected Seq, got {other:?}"),
        }
    }

    #[test]
    fn flatten_nested_seq() {
        // Seq(a, Seq(b, c)) → Seq(a, b, c)
        let ir = optimized(r#"r = { "a" ("b" "c") }"#);
        match &ir.rules[0].expr {
            IrExpr::Literal(s) => assert_eq!(s, "abc"), // all fused
            other => panic!("expected fused Literal, got {other:?}"),
        }
    }

    #[test]
    fn stateful_rule_not_inlined() {
        let ir = optimized(
            r#"
            let flag active
            special = {
                guard active
                "x"
            }
            main = { special }
        "#,
        );
        // special has a guard → not trivial → not inlined.
        assert!(ir.rules.iter().any(|r| r.name == "special"));
        let main = ir.rules.iter().find(|r| r.name == "main").unwrap();
        assert!(matches!(&main.expr, IrExpr::RuleRef(_)));
    }
}

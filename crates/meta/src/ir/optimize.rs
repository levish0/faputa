//! IR → IR optimization passes.
//!
//! Each pass is a function `IrProgram → IrProgram`.
//! `optimize()` chains them in order.

use std::collections::HashSet;

use super::{CharRange, IrExpr, IrProgram, IrRule};

/// Run all optimization passes on the program.
pub fn optimize(program: IrProgram) -> IrProgram {
    // Phase 1: Normalize
    let program = single_char_to_charset(program);
    let program = flatten(program);
    let program = merge_charsets(program);
    let program = fuse_literals(program);
    // Phase 2: Inline trivial rules (may expose new optimization opportunities)
    let program = inline_trivial_rules(program);
    // Phase 3: Re-normalize after inlining
    let program = flatten(program);
    let program = merge_charsets(program);
    let program = fuse_literals(program);
    // Phase 4: Recognize fused patterns
    let program = recognize_take_while(program);
    // Phase 5: Cleanup
    let program = eliminate_dead_rules(program);
    let program = compute_ref_counts(program);
    program
}

// ── Pass 0: Convert single-char Literals to CharSet in Choice ──
//
// Inside `Choice` branches, `Literal("x")` where x is a single character
// → `CharSet([CharRange::single('x')])`.
// This enables downstream CharSet merging. Only applied in Choice context
// to avoid interfering with literal fusion in Seq.

fn single_char_to_charset(mut program: IrProgram) -> IrProgram {
    for rule in &mut program.rules {
        rule.expr = single_char_to_charset_expr(rule.expr.clone());
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
                    // Convert single-char Literal to CharSet only in Choice context
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
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(single_char_to_charset_expr(*expr)),
            label,
        },
        other => other,
    }
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
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(flatten_expr(*expr)),
            label,
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
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(merge_charsets_expr(*expr)),
            label,
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
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(fuse_literals_expr(*expr)),
            label,
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
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(inline_refs(*expr, inline_exprs)),
            label,
        },
        other => other,
    }
}

// ── Pass 5: Eliminate dead rules ──
//
// Rules that are inlined and never referenced externally can be removed.
// We keep all non-inlined rules and any rule that is still referenced.

fn eliminate_dead_rules(program: IrProgram) -> IrProgram {
    // All user-defined rules are kept because each gets a `parse_<name>` entry point.
    // The `inline` flag only means the body was substituted into callers — the rule
    // itself must remain for external API access.
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
        | IrExpr::DepthLimit { body: expr, .. }
        | IrExpr::Labeled { expr, .. } => {
            collect_refs(expr, refs);
        }
        _ => {}
    }
}

// ── Pass 5.5: Recognize TakeWhile patterns ──
//
// `Repeat { expr: CharSet(ranges), min, max }` → `TakeWhile { ranges, min, max }`
// This enables efficient codegen (e.g. winnow's `take_while`).

fn recognize_take_while(mut program: IrProgram) -> IrProgram {
    for rule in &mut program.rules {
        rule.expr = recognize_take_while_expr(rule.expr.clone());
    }
    program
}

fn recognize_take_while_expr(expr: IrExpr) -> IrExpr {
    match expr {
        IrExpr::Repeat {
            expr: inner,
            min,
            max,
        } => {
            let inner = recognize_take_while_expr(*inner);
            if let IrExpr::CharSet(ranges) = inner {
                IrExpr::TakeWhile { ranges, min, max }
            } else {
                IrExpr::Repeat {
                    expr: Box::new(inner),
                    min,
                    max,
                }
            }
        }
        IrExpr::Seq(items) => {
            IrExpr::Seq(items.into_iter().map(recognize_take_while_expr).collect())
        }
        IrExpr::Choice(items) => {
            IrExpr::Choice(items.into_iter().map(recognize_take_while_expr).collect())
        }
        IrExpr::PosLookahead(inner) => {
            IrExpr::PosLookahead(Box::new(recognize_take_while_expr(*inner)))
        }
        IrExpr::NegLookahead(inner) => {
            IrExpr::NegLookahead(Box::new(recognize_take_while_expr(*inner)))
        }
        IrExpr::WithFlag { flag, body } => IrExpr::WithFlag {
            flag,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        IrExpr::WithCounter {
            counter,
            amount,
            body,
        } => IrExpr::WithCounter {
            counter,
            amount,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        IrExpr::When { condition, body } => IrExpr::When {
            condition,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        IrExpr::DepthLimit { limit, body } => IrExpr::DepthLimit {
            limit,
            body: Box::new(recognize_take_while_expr(*body)),
        },
        IrExpr::Labeled { expr, label } => IrExpr::Labeled {
            expr: Box::new(recognize_take_while_expr(*expr)),
            label,
        },
        other => other,
    }
}

// ── Pass 7: Compute ref_counts ──
//
// Count how many times each rule is referenced from non-inlined rules.
// ref_count == 0 means the rule is an entry point.

fn compute_ref_counts(mut program: IrProgram) -> IrProgram {
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
        _ => {}
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
        // "_" is a single-char Literal → converted to CharSet → all merge.
        let ir = optimized(r#"alpha = { 'a'..'z' | 'A'..'Z' | "_" }"#);
        match &ir.rules[0].expr {
            IrExpr::CharSet(ranges) => {
                // A..Z, _, a..z — but _ is adjacent to nothing so 3 ranges
                assert_eq!(ranges.len(), 3);
            }
            other => panic!("expected CharSet, got {other:?}"),
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
        // The Repeat { CharSet } pattern becomes TakeWhile.
        let number = ir.rules.iter().find(|r| r.name == "number").unwrap();
        match &number.expr {
            IrExpr::TakeWhile { ranges, min, max } => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0], CharRange::new('0', '9'));
                assert_eq!(*min, 1);
                assert_eq!(*max, None);
            }
            other => panic!("expected TakeWhile, got {other:?}"),
        }
    }

    #[test]
    fn inlined_rules_kept_with_inline_flag() {
        let ir = optimized(
            r#"
            digit = { '0'..'9' }
            number = { digit+ }
        "#,
        );
        // digit is trivial and inlined into number, but kept for entry-point access.
        assert_eq!(ir.rules.len(), 2);
        let digit = ir.rules.iter().find(|r| r.name == "digit").unwrap();
        assert!(digit.inline);
        let number = ir.rules.iter().find(|r| r.name == "number").unwrap();
        assert!(!number.inline);
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
        // alpha and digit are trivial → inlined but kept.
        // ident should have CharSet + TakeWhile (from the merged repeat).
        let ident = ir.rules.iter().find(|r| r.name == "ident").unwrap();
        match &ident.expr {
            IrExpr::Seq(items) => {
                assert!(matches!(&items[0], IrExpr::CharSet(_)));
                // (alpha | digit)* → merged CharSet repeat → TakeWhile
                assert!(matches!(&items[1], IrExpr::TakeWhile { .. }));
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

    // ── New pass tests ──

    #[test]
    fn single_char_literal_to_charset_in_choice() {
        // Single-char Literals in Choice should merge with CharSets.
        // " " | "\t" | "\n" | "\r" → chars \t(9), \n(10), \r(13), ' '(32)
        // \t and \n are adjacent → coalesced: [9..10, 13..13, 32..32] = 3 ranges
        let ir = optimized(r#"ws = { " " | "\t" | "\n" | "\r" }"#);
        match &ir.rules[0].expr {
            IrExpr::CharSet(ranges) => {
                assert_eq!(ranges.len(), 3);
            }
            other => panic!("expected CharSet, got {other:?}"),
        }
    }

    #[test]
    fn take_while_recognized() {
        // digit* → TakeWhile { ranges: [('0','9')], min: 0, max: None }
        let ir = optimized("d = { '0'..'9'* }");
        match &ir.rules[0].expr {
            IrExpr::TakeWhile { ranges, min, max } => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0], CharRange::new('0', '9'));
                assert_eq!(*min, 0);
                assert_eq!(*max, None);
            }
            other => panic!("expected TakeWhile, got {other:?}"),
        }
    }

    #[test]
    fn take_while_from_choice_repeat() {
        // (" " | "\t" | "\n" | "\r")* → single CharSet from merge → TakeWhile
        // \t and \n are adjacent → coalesced: 3 ranges
        let ir = optimized(r#"ws = { (" " | "\t" | "\n" | "\r")* }"#);
        match &ir.rules[0].expr {
            IrExpr::TakeWhile { ranges, min, max } => {
                assert_eq!(ranges.len(), 3);
                assert_eq!(*min, 0);
                assert_eq!(*max, None);
            }
            other => panic!("expected TakeWhile, got {other:?}"),
        }
    }

    #[test]
    fn take_while_bounded() {
        // digit{3} → TakeWhile with min=3, max=Some(3)
        let ir = optimized("d = { '0'..'9'{3} }");
        match &ir.rules[0].expr {
            IrExpr::TakeWhile { min, max, .. } => {
                assert_eq!(*min, 3);
                assert_eq!(*max, Some(3));
            }
            other => panic!("expected TakeWhile, got {other:?}"),
        }
    }

    #[test]
    fn ref_count_entry_point() {
        let ir = optimized(
            r#"
            main = { "hello" }
        "#,
        );
        assert_eq!(ir.rules[0].ref_count, 0); // not referenced → entry point
    }

    #[test]
    fn ref_count_internal_rule() {
        let ir = optimized(
            r#"
            let flag active
            special = {
                guard active
                "x"
            }
            a = { special }
            b = { special }
        "#,
        );
        let special = ir.rules.iter().find(|r| r.name == "special").unwrap();
        assert_eq!(special.ref_count, 2); // referenced by a and b
    }
}

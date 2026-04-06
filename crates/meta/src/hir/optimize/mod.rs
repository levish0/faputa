mod inline;
mod normalize;

#[cfg(test)]
mod tests;

use super::{CharRange, HirProgram};

/// Run all optimization passes on the program.
#[tracing::instrument(skip_all, fields(rules = program.rules.len()))]
pub fn optimize(program: HirProgram) -> HirProgram {
    // Phase 1: Normalize
    let program = normalize::single_char_to_charset(program);
    let program = normalize::flatten(program);
    let program = normalize::merge_charsets(program);
    let program = normalize::fuse_literals(program);
    tracing::debug!("phase 1 (normalize) complete");

    // Phase 2: Inline trivial rules (may expose new optimization opportunities)
    let program = inline::inline_trivial_rules(program);
    let inlined = program.rules.iter().filter(|r| r.inline).count();
    tracing::debug!(inlined, "phase 2 (inline) complete");

    // Phase 3: Re-normalize after inlining
    let program = normalize::flatten(program);
    let program = normalize::merge_charsets(program);
    let program = normalize::fuse_literals(program);
    tracing::debug!("phase 3 (re-normalize) complete");

    // Phase 3b: Inline small single-use helper rules and normalize again
    let program = inline::inline_small_single_use_rules(program);
    let program = normalize::flatten(program);
    let program = normalize::merge_charsets(program);
    let program = normalize::fuse_literals(program);
    tracing::debug!("phase 3b (small-rule inline) complete");

    // Phase 4: Cleanup
    let program = inline::eliminate_dead_rules(program);
    let program = inline::compute_ref_counts(program);
    let entry_points = program.rules.iter().filter(|r| r.ref_count == 0).count();
    tracing::debug!(entry_points, "phase 4 (cleanup) complete");
    program
}

pub(super) fn coalesce_ranges(mut ranges: Vec<CharRange>) -> Vec<CharRange> {
    if ranges.is_empty() {
        return ranges;
    }
    ranges.sort_by_key(|r| (r.start, r.end));
    let mut result = vec![ranges[0]];
    for r in &ranges[1..] {
        let last = result.last_mut().unwrap();
        let last_end_next = char::from_u32(last.end as u32 + 1);
        if r.start <= last.end || last_end_next == Some(r.start) {
            last.end = last.end.max(r.end);
        } else {
            result.push(*r);
        }
    }
    result
}

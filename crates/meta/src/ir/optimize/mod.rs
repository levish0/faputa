mod dispatch;
mod inline;
mod normalize;
mod patterns;
mod scan;

#[cfg(test)]
mod tests;

use super::{CharRange, IrProgram};

/// Run all optimization passes on the program.
#[tracing::instrument(skip_all, fields(rules = program.rules.len()))]
pub fn optimize(program: IrProgram) -> IrProgram {
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

    // Phase 4: Recognize fused patterns
    let program = patterns::recognize_take_while(program);
    tracing::debug!("phase 4 (pattern recognition) complete");

    // Phase 5: Cleanup
    let program = inline::eliminate_dead_rules(program);
    let program = inline::compute_ref_counts(program);
    let program = dispatch::recognize_dispatch(program);
    let program = scan::recognize_scan_repeat(program);
    let entry_points = program.rules.iter().filter(|r| r.ref_count == 0).count();
    tracing::debug!(entry_points, "phase 5 (cleanup) complete");
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

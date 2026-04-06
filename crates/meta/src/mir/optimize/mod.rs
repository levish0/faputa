mod delimited;
mod dispatch;
mod list;
mod patterns;
mod repeat;
mod scan;

#[cfg(test)]
mod tests;

use crate::hir::CharRange;

use super::MirProgram;

#[tracing::instrument(skip_all, fields(rules = program.rules.len()))]
pub fn optimize(program: MirProgram) -> MirProgram {
    let before = program.clone();
    let program = patterns::recognize_take_while(program);
    tracing::debug!(
        transformed = changed_rule_count(&before, &program),
        "phase 1 (take_while) complete"
    );
    let before = program.clone();
    let program = dispatch::recognize_dispatch(program);
    tracing::debug!(
        transformed = changed_rule_count(&before, &program),
        "phase 2 (dispatch) complete"
    );
    let before = program.clone();
    let program = scan::recognize_scan_repeat(program);
    tracing::debug!(
        transformed = changed_rule_count(&before, &program),
        "phase 3 (scan) complete"
    );
    let before = program.clone();
    let program = list::recognize_separated_list(program);
    tracing::debug!(
        transformed = changed_rule_count(&before, &program),
        "phase 4 (separated_list) complete"
    );
    let before = program.clone();
    let program = repeat::recognize_unbounded_loop(program);
    tracing::debug!(
        transformed = changed_rule_count(&before, &program),
        "phase 5 (loop) complete"
    );
    let before = program.clone();
    let program = delimited::recognize_delimited(program);
    tracing::debug!(
        transformed = changed_rule_count(&before, &program),
        "phase 6 (delimited) complete"
    );
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

fn changed_rule_count(before: &MirProgram, after: &MirProgram) -> usize {
    before
        .rules
        .iter()
        .zip(&after.rules)
        .filter(|(left, right)| left.expr != right.expr)
        .count()
}

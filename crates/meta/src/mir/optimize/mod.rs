mod dispatch;
mod list;
mod patterns;
mod scan;

#[cfg(test)]
mod tests;

use crate::hir::CharRange;

use super::MirProgram;

pub fn optimize(program: MirProgram) -> MirProgram {
    let program = patterns::recognize_take_while(program);
    let program = dispatch::recognize_dispatch(program);
    let program = scan::recognize_scan_repeat(program);
    list::recognize_separated_list(program)
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

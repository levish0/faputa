use crate::hir;
use crate::mir::{MirProgram, lower};

use super::optimize;

mod delimited;
mod dispatch;
mod list;
mod patterns;
mod repeat;
mod scan;

pub(super) fn optimized(source: &str) -> MirProgram {
    let grammar = crate::compile(source).expect("compile failed");
    let hir = hir::lower(&grammar);
    let hir = hir::optimize(hir);
    let mir = lower(&hir);
    optimize(mir)
}

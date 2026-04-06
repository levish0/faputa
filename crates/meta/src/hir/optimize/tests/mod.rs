use crate::hir::{HirProgram, lower};

use super::optimize;

mod inline;
mod normalize;

pub(super) fn optimized(source: &str) -> HirProgram {
    let grammar = crate::compile(source).expect("compile failed");
    let ir = lower(&grammar);
    optimize(ir)
}

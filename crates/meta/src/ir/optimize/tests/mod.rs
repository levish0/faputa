use crate::ir::{IrProgram, lower};

use super::optimize;

mod dispatch;
mod inline;
mod normalize;
mod patterns;
mod scan;

pub(super) fn optimized(source: &str) -> IrProgram {
    let grammar = crate::compile(source).expect("compile failed");
    let ir = lower(&grammar);
    optimize(ir)
}

use super::IrExpr;
use crate::ast::{GuardCondition, StateDecl};

/// A complete IR program ready for optimization and codegen.
#[derive(Debug, Clone, PartialEq)]
pub struct IrProgram {
    pub state_decls: Vec<StateDecl>,
    pub rules: Vec<IrRule>,
}

impl IrProgram {
    /// Look up a rule index by name.
    pub fn rule_index(&self, name: &str) -> Option<usize> {
        self.rules.iter().position(|r| r.name == name)
    }

    /// Look up a rule by index.
    pub fn rule(&self, index: usize) -> Option<&IrRule> {
        self.rules.get(index)
    }
}

/// A single named rule in the IR.
#[derive(Debug, Clone, PartialEq)]
pub struct IrRule {
    pub name: String,
    /// Whether the optimizer has decided to inline this rule at call sites.
    pub inline: bool,
    /// Pre-expression guards (fail-fast before attempting the match).
    pub guards: Vec<GuardCondition>,
    /// Pre-expression side effects (e.g., emit counter).
    pub emits: Vec<String>,
    /// The matching expression.
    pub expr: IrExpr,
}

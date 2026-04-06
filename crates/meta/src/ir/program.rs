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
    /// Optional custom error label from `rule = @ "label" { ... }`.
    /// Overrides the default `StrContext::Label(rule_name)` in codegen.
    pub error_label: Option<String>,
    /// Pre-expression guards (fail-fast before attempting the match).
    pub guards: Vec<GuardCondition>,
    /// Pre-expression side effects (e.g., emit counter).
    pub emits: Vec<String>,
    /// The matching expression.
    pub expr: IrExpr,
    /// Number of call sites referencing this rule (set by call-graph analysis).
    /// 0 means the rule is an entry point (not referenced by any other rule).
    pub ref_count: usize,
}

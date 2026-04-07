use crate::ast::{GuardCondition, StateDecl};

use super::MirExpr;

#[derive(Debug, Clone, PartialEq)]
pub struct MirProgram {
    pub state_decls: Vec<StateDecl>,
    pub rules: Vec<MirRule>,
}

impl MirProgram {
    pub fn rule_index(&self, name: &str) -> Option<usize> {
        self.rules.iter().position(|r| r.name == name)
    }

    pub fn rule(&self, index: usize) -> Option<&MirRule> {
        self.rules.get(index)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MirRule {
    pub name: String,
    pub inline: bool,
    pub error_label: Option<String>,
    pub is_entry_point: bool,
    pub needs_context: bool,
    pub needs_trace: bool,
    pub guards: Vec<GuardCondition>,
    pub increments: Vec<String>,
    pub expr: MirExpr,
}

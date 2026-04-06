use super::{Expr, StateDecl, Statement};

/// A complete .nanachi grammar file.
#[derive(Debug, Clone, PartialEq)]
pub struct Grammar {
    pub items: Vec<Item>,
}

/// Top-level item: state declaration or rule definition.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    StateDecl(StateDecl),
    RuleDef(RuleDef),
}

/// A named rule: `rule_name = { ... }` or `rule_name = @ "label" { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleDef {
    pub name: String,
    /// Optional custom error label: `rule = @ "custom label" { ... }`.
    pub error_label: Option<String>,
    pub body: RuleBody,
}

/// The contents inside `rule_name = { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleBody {
    pub statements: Vec<Statement>,
    pub expr: Expr,
}

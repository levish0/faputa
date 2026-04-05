/// Stateful statements that precede the main expression in a rule body.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Guard(GuardStmt),
    Emit(EmitStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuardStmt {
    pub condition: GuardCondition,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GuardCondition {
    /// `guard !flag_name`
    NotFlag(String),
    /// `guard flag_name`
    IsFlag(String),
    /// `guard LINE_START`, `guard SOI`, etc.
    Builtin(BuiltinPredicate),
    /// `guard counter_name > 0`, etc.
    Compare {
        name: String,
        op: CompareOp,
        value: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinPredicate {
    Soi,
    Eoi,
    Any,
    LineStart,
    LineEnd,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmitStmt {
    pub counter: String,
}

/// `let flag name` or `let counter name`.
#[derive(Debug, Clone, PartialEq)]
pub struct StateDecl {
    pub kind: StateKind,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Flag,
    Counter,
}
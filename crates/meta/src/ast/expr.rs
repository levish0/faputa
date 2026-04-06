use super::{BuiltinPredicate, GuardCondition};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// `"literal"`
    StringLit(String),

    /// `'a'..'z'`
    CharRange(char, char),

    /// Reference to another rule: `inline`, `bold`
    Ident(String),

    /// Built-in predicate: `SOI`, `EOI`, `ANY`, `LINE_START`, `LINE_END`
    Builtin(BuiltinPredicate),

    /// Sequence: `a b c`
    Seq(Vec<Expr>),

    /// Choice: `a | b | c`
    Choice(Vec<Expr>),

    /// Repetition: `p+`, `p*`, `p?`, `p{n,m}`
    Repeat { expr: Box<Expr>, kind: RepeatKind },

    /// Positive lookahead: `&p`
    PosLookahead(Box<Expr>),

    /// Negative lookahead: `!p`
    NegLookahead(Box<Expr>),

    /// Parenthesized group: `(a | b)`
    Group(Box<Expr>),

    /// `with flag_name { expr }`
    With(WithExpr),

    /// `with counter_name += n { expr }`
    WithIncrement(WithIncrementExpr),

    /// `when condition { expr }`
    When(WhenExpr),

    /// `depth_limit(n) { expr }`
    DepthLimit(DepthLimitExpr),

    /// `expr @ "custom error label"`
    Labeled { expr: Box<Expr>, label: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepeatKind {
    /// `p*`
    ZeroOrMore,
    /// `p+`
    OneOrMore,
    /// `p?`
    Optional,
    /// `p{n}`
    Exact(u32),
    /// `p{n,}`
    AtLeast(u32),
    /// `p{,m}`
    AtMost(u32),
    /// `p{n,m}`
    Range(u32, u32),
}

// ── Stateful expressions ──

#[derive(Debug, Clone, PartialEq)]
pub struct WithExpr {
    pub flag: String,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WithIncrementExpr {
    pub counter: String,
    pub amount: u32,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhenExpr {
    pub condition: GuardCondition,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DepthLimitExpr {
    pub limit: u32,
    pub body: Box<Expr>,
}

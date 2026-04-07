use super::{BuiltinPredicate, GuardCondition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericExpr {
    Literal(u32),
    Counter(String),
}

impl NumericExpr {
    pub fn as_literal(&self) -> Option<u32> {
        match self {
            Self::Literal(value) => Some(*value),
            Self::Counter(_) => None,
        }
    }
}

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

    /// `if condition { then } else { else }`
    If(IfExpr),

    /// `measure counter_name { expr }`
    Measure(MeasureExpr),

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
    Exact(NumericExpr),
    /// `p{n,}`
    AtLeast(NumericExpr),
    /// `p{,m}`
    AtMost(NumericExpr),
    /// `p{n,m}`
    Range(NumericExpr, NumericExpr),
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
    pub amount: NumericExpr,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhenExpr {
    pub condition: GuardCondition,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExpr {
    pub condition: GuardCondition,
    pub then_body: Box<Expr>,
    pub else_body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasureExpr {
    pub counter: String,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DepthLimitExpr {
    pub limit: NumericExpr,
    pub body: Box<Expr>,
}

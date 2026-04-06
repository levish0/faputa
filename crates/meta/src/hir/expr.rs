use crate::ast::GuardCondition;

/// HIR expression — grammar semantics after lowering and semantic cleanup.
///
/// Unlike the AST, this is optimized for semantic transformation:
/// - No `Group` (purely syntactic in AST)
/// - `CharSet` replaces individual `CharRange` (mergeable)
/// - `RuleRef(usize)` replaces `Ident(String)` (resolved)
/// - Unified `Repeat { min, max }` replaces all repeat variants
/// - Stateful guards/emits are separate from the expression tree
#[derive(Debug, Clone, PartialEq)]
pub enum HirExpr {
    // ── Terminals ──
    /// Match a literal string. Adjacent literals can be fused.
    Literal(String),

    /// Match one character from a set of inclusive ranges.
    /// `'a'..'z' | 'A'..'Z' | '_'` → `[('a','z'), ('A','Z'), ('_','_')]`
    CharSet(Vec<CharRange>),

    /// Match any single character.
    Any,

    /// Match a position boundary (zero-width).
    Boundary(Boundary),

    // ── Combinators ──
    /// Reference to another rule by index.
    RuleRef(usize),

    /// Match a sequence of expressions in order.
    Seq(Vec<HirExpr>),

    /// Ordered choice: try each in order, backtrack on failure.
    Choice(Vec<HirExpr>),

    /// Repetition with bounds.
    /// `*` = (0, None), `+` = (1, None), `?` = (0, Some(1)),
    /// `{n}` = (n, Some(n)), `{n,m}` = (n, Some(m))
    Repeat {
        expr: Box<HirExpr>,
        min: u32,
        max: Option<u32>,
    },

    /// Positive lookahead (zero-width).
    PosLookahead(Box<HirExpr>),

    /// Negative lookahead (zero-width).
    NegLookahead(Box<HirExpr>),

    // ── Stateful ──
    /// Set flag, run body, restore previous value.
    WithFlag { flag: String, body: Box<HirExpr> },

    /// Increment counter, run body, decrement on exit.
    WithCounter {
        counter: String,
        amount: u32,
        body: Box<HirExpr>,
    },

    /// Run body only if condition holds; otherwise succeed with no consumption.
    When {
        condition: GuardCondition,
        body: Box<HirExpr>,
    },

    /// Fail if recursion depth exceeds limit.
    DepthLimit { limit: u32, body: Box<HirExpr> },

    /// User-defined error label: `expr @ "custom message"`.
    ///
    /// Prevents optimization passes from merging through this boundary,
    /// preserving the user's intended error reporting structure.
    Labeled { expr: Box<HirExpr>, label: String },
}

/// An inclusive character range `(start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CharRange {
    pub start: char,
    pub end: char,
}

impl CharRange {
    pub fn new(start: char, end: char) -> Self {
        Self { start, end }
    }

    pub fn single(ch: char) -> Self {
        Self { start: ch, end: ch }
    }
}

/// Zero-width position assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boundary {
    /// Start of input (position 0).
    Soi,
    /// End of input (no remaining bytes).
    Eoi,
    /// Start of a line (position 0 or after `\n`).
    LineStart,
    /// End of a line (at `\n` or end of input).
    LineEnd,
}

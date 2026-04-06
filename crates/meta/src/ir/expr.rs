use crate::ast::GuardCondition;

/// IR expression — the core matching primitive.
///
/// Unlike the AST, this is optimized for transformation:
/// - No `Group` (purely syntactic in AST)
/// - `CharSet` replaces individual `CharRange` (mergeable)
/// - `RuleRef(usize)` replaces `Ident(String)` (resolved)
/// - Unified `Repeat { min, max }` replaces all repeat variants
/// - Stateful guards/emits are separate from the expression tree
#[derive(Debug, Clone, PartialEq)]
pub enum IrExpr {
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
    Seq(Vec<IrExpr>),

    /// Ordered choice: try each in order, backtrack on failure.
    Choice(Vec<IrExpr>),

    /// Repetition with bounds.
    /// `*` = (0, None), `+` = (1, None), `?` = (0, Some(1)),
    /// `{n}` = (n, Some(n)), `{n,m}` = (n, Some(m))
    Repeat {
        expr: Box<IrExpr>,
        min: u32,
        max: Option<u32>,
    },

    /// Positive lookahead (zero-width).
    PosLookahead(Box<IrExpr>),

    /// Negative lookahead (zero-width).
    NegLookahead(Box<IrExpr>),

    // ── Stateful ──
    /// Set flag, run body, restore previous value.
    WithFlag { flag: String, body: Box<IrExpr> },

    /// Increment counter, run body, decrement on exit.
    WithCounter {
        counter: String,
        amount: u32,
        body: Box<IrExpr>,
    },

    /// Run body only if condition holds; otherwise succeed with no consumption.
    When {
        condition: GuardCondition,
        body: Box<IrExpr>,
    },

    /// Fail if recursion depth exceeds limit.
    DepthLimit { limit: u32, body: Box<IrExpr> },

    /// Fused char-class repeat for efficient codegen (e.g. winnow `take_while`).
    ///
    /// Recognized from `Repeat { expr: CharSet(ranges), min, max }` patterns.
    TakeWhile {
        ranges: Vec<CharRange>,
        min: u32,
        max: Option<u32>,
    },

    /// A well-known ASCII character class from `winnow::ascii`.
    ///
    /// Recognized from `TakeWhile` patterns whose ranges match standard ASCII
    /// classes (e.g. digit0/1, alpha0/1, hex_digit0/1, multispace0/1, etc.).
    AsciiBuiltin(AsciiClass),

    /// User-defined error label: `expr @ "custom message"`.
    ///
    /// Prevents optimization passes from merging through this boundary,
    /// preserving the user's intended error reporting structure.
    Labeled {
        expr: Box<IrExpr>,
        label: String,
    },
}

/// Well-known ASCII character classes from `winnow::ascii`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsciiClass {
    /// `'0'..='9'` zero or more → `digit0`
    Digit0,
    /// `'0'..='9'` one or more → `digit1`
    Digit1,
    /// `'a'..='z' | 'A'..='Z'` zero or more → `alpha0`
    Alpha0,
    /// `'a'..='z' | 'A'..='Z'` one or more → `alpha1`
    Alpha1,
    /// `'a'..='z' | 'A'..='Z' | '0'..='9'` zero or more → `alphanumeric0`
    Alphanumeric0,
    /// `'a'..='z' | 'A'..='Z' | '0'..='9'` one or more → `alphanumeric1`
    Alphanumeric1,
    /// `'0'..='9' | 'a'..='f' | 'A'..='F'` zero or more → `hex_digit0`
    HexDigit0,
    /// `'0'..='9' | 'a'..='f' | 'A'..='F'` one or more → `hex_digit1`
    HexDigit1,
    /// `'0'..='7'` zero or more → `oct_digit0`
    OctDigit0,
    /// `'0'..='7'` one or more → `oct_digit1`
    OctDigit1,
    /// `' ' | '\t' | '\n' | '\r'` zero or more → `multispace0`
    Multispace0,
    /// `' ' | '\t' | '\n' | '\r'` one or more → `multispace1`
    Multispace1,
    /// `' ' | '\t'` zero or more → `space0`
    Space0,
    /// `' ' | '\t'` one or more → `space1`
    Space1,
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

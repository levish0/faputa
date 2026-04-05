use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
pub enum Token<'src> {
    // ── Keywords ──
    #[token("let")]
    Let,
    #[token("flag")]
    Flag,
    #[token("counter")]
    Counter,
    #[token("stack")]
    Stack,
    #[token("mode")]
    Mode,
    #[token("guard")]
    Guard,
    #[token("with")]
    With,
    #[token("emit")]
    Emit,
    #[token("when")]
    When,
    #[token("depth_limit")]
    DepthLimit,

    // ── Built-in predicates ──
    #[token("SOI")]
    Soi,
    #[token("EOI")]
    Eoi,
    #[token("ANY")]
    Any,
    #[token("LINE_START")]
    LineStart,
    #[token("LINE_END")]
    LineEnd,

    // ── Literals ──
    #[regex(r#""[^"]*""#, |lex| &lex.source()[lex.span().start + 1..lex.span().end - 1])]
    StringLit(&'src str),

    #[regex(r"'[^']'", |lex| {
        let inner = &lex.source()[lex.span().start + 1..lex.span().end - 1];
        inner.chars().next()
    })]
    CharLit(char),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<u32>().ok())]
    Number(u32),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident(&'src str),

    // ── Operators ──
    #[token("+")]
    Plus,
    #[token("*")]
    Star,
    #[token("?")]
    Question,
    #[token("|")]
    Pipe,
    #[token("&")]
    Amp,
    #[token("!")]
    Bang,
    #[token("=")]
    Eq,
    #[token("+=")]
    PlusEq,
    #[token("..")]
    DotDot,
    #[token(">")]
    Gt,
    #[token("<")]
    Lt,
    #[token(">=")]
    Ge,
    #[token("<=")]
    Le,
    #[token("==")]
    EqEq,
    #[token("!=")]
    BangEq,

    // ── Delimiters ──
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,

    // ── Newline (significant for statement separation) ──
    #[token("\n")]
    Newline,
}

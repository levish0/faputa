use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
#[logos(skip(r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/", allow_greedy = true))]
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
    #[token("inc")]
    Inc,
    #[token("when")]
    When,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("measure")]
    Measure,
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
    // Strings may contain escape sequences: \n \t \r \\ \"
    #[regex(r#""([^"\\]|\\.)*""#, |lex| &lex.source()[lex.span().start + 1..lex.span().end - 1])]
    StringLit(&'src str),

    // Chars may contain escape: '\n', '\t', '\\'
    #[regex(r"'([^'\\]|\\.)'", |lex| {
        let inner = &lex.source()[lex.span().start + 1..lex.span().end - 1];
        unescape_char(inner)
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

    #[token("@")]
    At,
}

fn unescape_char(s: &str) -> Option<char> {
    let mut chars = s.chars();
    match chars.next()? {
        '\\' => match chars.next()? {
            'n' => Some('\n'),
            't' => Some('\t'),
            'r' => Some('\r'),
            '\\' => Some('\\'),
            '\'' => Some('\''),
            _ => None,
        },
        c => Some(c),
    }
}

/// Unescape a string literal body (between the quotes).
pub fn unescape_str(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

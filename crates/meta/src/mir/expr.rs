use crate::ast::GuardCondition;
use crate::hir::{Boundary, CharRange};

#[derive(Debug, Clone, PartialEq)]
pub enum MirExpr {
    Literal(String),
    CharSet(Vec<CharRange>),
    Any,
    Boundary(Boundary),

    RuleRef(usize),
    Seq(Vec<MirExpr>),
    Choice(Vec<MirExpr>),
    Dispatch(Vec<DispatchArm>),

    Repeat {
        expr: Box<MirExpr>,
        min: u32,
        max: Option<u32>,
    },

    Loop {
        body: Box<MirExpr>,
        min: u32,
    },

    Delimited {
        open: Box<MirExpr>,
        body: Box<MirExpr>,
        close: Box<MirExpr>,
    },

    PosLookahead(Box<MirExpr>),
    NegLookahead(Box<MirExpr>),

    WithFlag {
        flag: String,
        body: Box<MirExpr>,
    },

    WithCounter {
        counter: String,
        amount: u32,
        body: Box<MirExpr>,
    },

    When {
        condition: GuardCondition,
        body: Box<MirExpr>,
    },

    DepthLimit {
        limit: u32,
        body: Box<MirExpr>,
    },

    TakeWhile {
        ranges: Vec<CharRange>,
        min: u32,
        max: Option<u32>,
    },

    Scan {
        plain_ranges: Vec<CharRange>,
        specials: Vec<DispatchArm>,
        min: u32,
    },

    SeparatedList {
        first: Box<MirExpr>,
        rest: Box<MirExpr>,
    },

    Labeled {
        expr: Box<MirExpr>,
        label: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchArm {
    pub ranges: Vec<CharRange>,
    pub expr: Box<MirExpr>,
}

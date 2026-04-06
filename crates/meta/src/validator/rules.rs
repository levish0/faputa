use std::collections::{HashMap, HashSet};

use crate::ast::*;

use super::errors::ValidationError;

const BUILTINS: &[&str] = &["SOI", "EOI", "ANY", "LINE_START", "LINE_END"];

/// Collected definitions from the grammar.
pub(crate) struct DefinitionContext {
    pub rules: HashSet<String>,
    pub states: HashMap<String, StateKind>,
}

/// First pass: collect all rule and state definitions, check for duplicates.
pub(crate) fn collect_definitions(
    grammar: &Grammar,
    errors: &mut Vec<ValidationError>,
) -> DefinitionContext {
    let mut rules = HashSet::new();
    let mut states = HashMap::new();

    for item in &grammar.items {
        match item {
            Item::RuleDef(rule) => {
                if BUILTINS.contains(&rule.name.as_str()) {
                    errors.push(ValidationError::ShadowsBuiltin {
                        name: rule.name.clone(),
                    });
                }
                if !rules.insert(rule.name.clone()) {
                    errors.push(ValidationError::DuplicateRule {
                        name: rule.name.clone(),
                    });
                }
            }
            Item::StateDecl(decl) => {
                if states.insert(decl.name.clone(), decl.kind).is_some() {
                    errors.push(ValidationError::DuplicateState {
                        name: decl.name.clone(),
                    });
                }
            }
        }
    }

    DefinitionContext { rules, states }
}

/// Second pass: check that all rule references in expressions point to defined rules.
pub(crate) fn check_references(
    grammar: &Grammar,
    ctx: &DefinitionContext,
    errors: &mut Vec<ValidationError>,
) {
    for item in &grammar.items {
        if let Item::RuleDef(rule) = item {
            check_expr_references(&rule.body.expr, &rule.name, ctx, errors);
        }
    }
}

fn check_expr_references(
    expr: &Expr,
    rule_name: &str,
    ctx: &DefinitionContext,
    errors: &mut Vec<ValidationError>,
) {
    match expr {
        Expr::Ident(name) => {
            if !ctx.rules.contains(name) {
                errors.push(ValidationError::UndefinedRule {
                    name: name.clone(),
                    used_in: rule_name.to_string(),
                });
            }
        }
        Expr::Seq(exprs) | Expr::Choice(exprs) => {
            for e in exprs {
                check_expr_references(e, rule_name, ctx, errors);
            }
        }
        Expr::Repeat { expr, .. }
        | Expr::PosLookahead(expr)
        | Expr::NegLookahead(expr)
        | Expr::Group(expr) => {
            check_expr_references(expr, rule_name, ctx, errors);
        }
        Expr::Labeled { expr, .. } => {
            check_expr_references(expr, rule_name, ctx, errors);
        }
        Expr::With(w) => {
            check_expr_references(&w.body, rule_name, ctx, errors);
        }
        Expr::WithIncrement(w) => {
            check_expr_references(&w.body, rule_name, ctx, errors);
        }
        Expr::When(w) => {
            check_expr_references(&w.body, rule_name, ctx, errors);
        }
        Expr::DepthLimit(d) => {
            check_expr_references(&d.body, rule_name, ctx, errors);
        }
        Expr::StringLit(_) | Expr::CharRange(_, _) | Expr::Builtin(_) => {}
    }
}

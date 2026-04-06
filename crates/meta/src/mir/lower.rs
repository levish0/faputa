use crate::hir::{HirExpr, HirProgram};

use super::{MirExpr, MirProgram, MirRule};

#[tracing::instrument(skip_all, fields(rules = program.rules.len()))]
pub fn lower(program: &HirProgram) -> MirProgram {
    MirProgram {
        state_decls: program.state_decls.clone(),
        rules: program
            .rules
            .iter()
            .map(|rule| {
                let is_entry_point = rule.ref_count == 0;
                let needs_context = is_entry_point || rule.error_label.is_some();
                let mir_rule = MirRule {
                    name: rule.name.clone(),
                    inline: rule.inline,
                    error_label: rule.error_label.clone(),
                    is_entry_point,
                    needs_context,
                    needs_trace: is_entry_point,
                    guards: rule.guards.clone(),
                    emits: rule.emits.clone(),
                    expr: lower_expr(&rule.expr),
                };
                tracing::trace!(
                    rule = %mir_rule.name,
                    entry_point = mir_rule.is_entry_point,
                    needs_context = mir_rule.needs_context,
                    needs_trace = mir_rule.needs_trace,
                    guards = mir_rule.guards.len(),
                    emits = mir_rule.emits.len(),
                    has_error_label = mir_rule.error_label.is_some(),
                    "lowered hir rule to mir"
                );
                mir_rule
            })
            .collect(),
    }
}

fn lower_expr(expr: &HirExpr) -> MirExpr {
    match expr {
        HirExpr::Literal(s) => MirExpr::Literal(s.clone()),
        HirExpr::CharSet(ranges) => MirExpr::CharSet(ranges.clone()),
        HirExpr::Any => MirExpr::Any,
        HirExpr::Boundary(boundary) => MirExpr::Boundary(*boundary),
        HirExpr::RuleRef(idx) => MirExpr::RuleRef(*idx),
        HirExpr::Seq(items) => MirExpr::Seq(items.iter().map(lower_expr).collect()),
        HirExpr::Choice(items) => MirExpr::Choice(items.iter().map(lower_expr).collect()),
        HirExpr::Repeat { expr, min, max } => MirExpr::Repeat {
            expr: Box::new(lower_expr(expr)),
            min: *min,
            max: *max,
        },
        HirExpr::PosLookahead(inner) => MirExpr::PosLookahead(Box::new(lower_expr(inner))),
        HirExpr::NegLookahead(inner) => MirExpr::NegLookahead(Box::new(lower_expr(inner))),
        HirExpr::WithFlag { flag, body } => MirExpr::WithFlag {
            flag: flag.clone(),
            body: Box::new(lower_expr(body)),
        },
        HirExpr::WithCounter {
            counter,
            amount,
            body,
        } => MirExpr::WithCounter {
            counter: counter.clone(),
            amount: *amount,
            body: Box::new(lower_expr(body)),
        },
        HirExpr::When { condition, body } => MirExpr::When {
            condition: condition.clone(),
            body: Box::new(lower_expr(body)),
        },
        HirExpr::DepthLimit { limit, body } => MirExpr::DepthLimit {
            limit: *limit,
            body: Box::new(lower_expr(body)),
        },
        HirExpr::Labeled { expr, label } => MirExpr::Labeled {
            expr: Box::new(lower_expr(expr)),
            label: label.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::{compile, hir};

    use super::lower;

    #[test]
    fn entry_point_gets_trace_and_context() {
        let grammar = compile(r#"value = { 'a'..'z'+ }"#).expect("compile failed");
        let hir = hir::lower(&grammar);
        let hir = hir::optimize(hir);
        let mir = lower(&hir);
        let value = mir.rule(mir.rule_index("value").unwrap()).unwrap();

        assert!(value.is_entry_point);
        assert!(value.needs_context);
        assert!(value.needs_trace);
    }

    #[test]
    fn unlabeled_helper_rule_skips_context_and_trace() {
        let grammar = compile(
            r#"
            word = { ('a'..'z' | 'A'..'Z')+ }
            pair = { word word }
        "#,
        )
        .expect("compile failed");
        let hir = hir::lower(&grammar);
        let hir = hir::optimize(hir);
        let mir = lower(&hir);
        let word = mir.rule(mir.rule_index("word").unwrap()).unwrap();

        assert!(!word.is_entry_point);
        assert!(!word.needs_context);
        assert!(!word.needs_trace);
    }

    #[test]
    fn labeled_helper_rule_keeps_context_without_trace() {
        let grammar = compile(
            r#"
            word = @ "word" { ('a'..'z' | 'A'..'Z')+ }
            pair = { word word }
        "#,
        )
        .expect("compile failed");
        let hir = hir::lower(&grammar);
        let hir = hir::optimize(hir);
        let mir = lower(&hir);
        let word = mir.rule(mir.rule_index("word").unwrap()).unwrap();

        assert!(!word.is_entry_point);
        assert!(word.needs_context);
        assert!(!word.needs_trace);
    }
}

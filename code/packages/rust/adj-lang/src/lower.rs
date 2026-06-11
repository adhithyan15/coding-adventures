//! # Lowering — AST → KnowledgeBase + queries.
//!
//! Translates the AST produced by [`crate::parser`] into a
//! `logic-engine` [`KnowledgeBase`] populated with [`Fact`],
//! [`PriorClause`], [`ContributionClause`], and
//! [`JointContributionClause`] entries. Queries appear in the
//! returned [`LoweredProgram::queries`] vector in source order; the
//! caller decides whether to run them via
//! [`logic_engine::search`] and what to do with the results.
//!
//! ## What the lowerer enforces beyond the parser
//!
//! - At most one `prior` per conclusion. Two priors for the same
//!   atom is a [`LowerError::DuplicatePrior`] (mirroring the
//!   engine's `KbError::ConflictingPriors`, but caught at lowering
//!   time so the diagnostic carries surface line/col).
//! - All three trust-tier names map to the engine's
//!   [`logic_engine::TrustTier`] variants.
//! - `source` may appear at most once per statement; multiple
//!   `source` annotations are a [`LowerError::DuplicateAnnotation`].

use logic_core::{atom as core_atom, compound, float as core_float, Term as CoreTerm};
use logic_engine::{
    compute, CmpOp as EngineCmpOp, ComputeExpr, ComputeOp, ContributionClause, Fact,
    JointContributionClause, KbError, KnowledgeBase, PredicateContributionClause, PriorClause,
    Provenance, TrustTier, UncertaintyMarker,
};

use crate::ast::{
    AggOp, Annotation, ArithOp, CmpOp, Evidence, ExprAst, Program, RelOp, Statement,
    Term as AstTerm, TrustTierName,
};

/// One lowered constraint: `lhs <op> rhs`, with both sides kept as
/// **unevaluated** [`ComputeExpr`] trees (they reference symbols the solver
/// will assign, so they cannot be computed yet — that is the solver's job in
/// track B2).
#[derive(Debug, Clone, PartialEq)]
pub struct LoweredConstraint {
    pub lhs: ComputeExpr,
    pub op: RelOp,
    pub rhs: ComputeExpr,
}

/// The typed constraint system a program builds from its `symbol` /
/// `constrain` / `solve for` / `check` statements (ADJ constraints track B).
/// Track B1 builds and exposes it; the solver backends are wired in B2.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConstraintSystem {
    /// Declared unknowns: `(name, sort)`, where `sort` is a dimensional sort
    /// term (`scalar`, `money(usd)`, …).
    pub symbols: Vec<(String, CoreTerm)>,
    /// The asserted (in)equalities.
    pub constraints: Vec<LoweredConstraint>,
    /// The unknowns a `solve for { … }` asked to solve.
    pub solve_for: Vec<String>,
    /// Whether a `check` (feasibility query) was requested.
    pub check: bool,
}

impl ConstraintSystem {
    /// `true` iff the program declared no constraint machinery at all (the
    /// common case for a pure prior/contributes rulebook).
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
            && self.constraints.is_empty()
            && self.solve_for.is_empty()
            && !self.check
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LowerError {
    DuplicatePrior {
        conclusion_name: String,
    },
    DuplicateAnnotation {
        name: &'static str,
    },
    EngineRejected {
        detail: String,
    },
    /// A `prior <p>` whose probability is not in the open interval
    /// `(0.0, 1.0)`. The engine's `PriorClause::from_probability`
    /// asserts this; we catch it at lowering time so a malformed
    /// rulebook produces a clean diagnostic instead of a process panic.
    InvalidProbability {
        value: f64,
    },
    /// A `contributes <lr>` / `interacts <lr>` whose likelihood ratio is
    /// not strictly positive and finite. LR is a ratio of probabilities,
    /// so `lr <= 0` (or non-finite) is a modeller error — rejected here
    /// rather than panicking in `from_lr`.
    InvalidLikelihoodRatio {
        value: f64,
    },
    /// A `let <name> = <expr>` whose formula could not be evaluated (an
    /// unknown slot, division by zero, an empty aggregation, …). Carries
    /// the engine's [`logic_engine::ComputeError`] rendered for the audit.
    ComputationFailed {
        name: String,
        detail: String,
    },
}

/// The result of lowering — a populated KB, any queries to run, and the
/// (possibly empty) constraint system the program declared.
#[derive(Debug)]
pub struct LoweredProgram {
    pub kb: KnowledgeBase,
    pub queries: Vec<CoreTerm>,
    pub constraints: ConstraintSystem,
}

/// Lower an [`ast::Program`] to a populated KB + queries + constraint system.
pub fn lower(program: &Program) -> Result<LoweredProgram, LowerError> {
    let mut kb = KnowledgeBase::new();
    let mut queries = Vec::new();
    let mut constraints = ConstraintSystem::default();

    for stmt in &program.statements {
        match stmt {
            Statement::Prior {
                probability,
                conclusion,
                annotations,
            } => {
                // Guard the engine's `from_probability` assertion: a
                // probability outside (0, 1) would panic. Reject it as a
                // clean lowering error instead.
                if !(probability.is_finite() && *probability > 0.0 && *probability < 1.0) {
                    return Err(LowerError::InvalidProbability {
                        value: *probability,
                    });
                }
                let prov = annotations_to_provenance(annotations)?;
                let clause = PriorClause::from_probability(lower_term(conclusion), *probability)
                    .with_provenance(prov);
                kb.add_prior(clause).map_err(|e| match e {
                    KbError::ConflictingPriors { conclusion, .. } => LowerError::DuplicatePrior {
                        conclusion_name: format!("{conclusion:?}"),
                    },
                })?;
            }
            Statement::Contributes {
                lr,
                evidence,
                conclusion,
                annotations,
            } => {
                check_lr(*lr)?;
                let prov = annotations_to_provenance(annotations)?;
                match evidence {
                    // Ordinary term evidence → single-source LR contribution.
                    Evidence::Term(t) => {
                        let clause =
                            ContributionClause::from_lr(lower_term(conclusion), lower_term(t), *lr)
                                .with_provenance(prov);
                        kb.add_contribution(clause);
                    }
                    // Numeric predicate evidence → predicate-gated contribution.
                    // The comparison is evaluated on the CPU at decision time;
                    // a saturating `lr` makes the rule deterministic.
                    Evidence::Predicate { slot, op, value } => {
                        let clause = PredicateContributionClause::from_lr(
                            lower_term(conclusion),
                            slot.clone(),
                            lower_cmp_op(*op),
                            *value,
                            *lr,
                        )
                        .with_provenance(prov);
                        kb.add_predicate_contribution(clause);
                    }
                }
            }
            Statement::Interacts {
                lr,
                evidence_set,
                conclusion,
                annotations,
            } => {
                check_lr(*lr)?;
                let prov = annotations_to_provenance(annotations)?;
                let clause = JointContributionClause::from_lr(
                    lower_term(conclusion),
                    evidence_set.iter().map(lower_term).collect(),
                    *lr,
                )
                .with_provenance(prov);
                kb.add_joint_contribution(clause);
            }
            Statement::Observe { term } => {
                kb.add_fact(Fact::certain(lower_term(term)));
            }
            Statement::Query { conclusion } => {
                queries.push(lower_term(conclusion));
            }
            Statement::Let { name, expr } => {
                // Evaluate the formula against the facts (and any earlier
                // `let`s) seen so far — statements lower in source order, so a
                // `let` sees every `observe` above it. The engine builds the
                // derivation tree; the model never computed anything.
                let cexpr = lower_expr(expr);
                let derived = compute(name.clone(), &cexpr, &kb).map_err(|e| {
                    LowerError::ComputationFailed {
                        name: name.clone(),
                        detail: format!("{e:?}"),
                    }
                })?;
                kb.add_derived(derived);
            }
            Statement::Uncertain {
                domain,
                conclusion,
                annotations,
            } => {
                let prov = annotations_to_provenance(annotations)?;
                let marker = UncertaintyMarker::new(
                    lower_term(conclusion),
                    domain.iter().map(lower_term).collect(),
                )
                .with_provenance(prov);
                kb.add_uncertainty_marker(marker);
            }
            // ---- constraint sublanguage (track B) ----
            Statement::Symbol { name, sort } => {
                constraints.symbols.push((name.clone(), lower_term(sort)));
            }
            Statement::Constrain { lhs, op, rhs } => {
                // Keep both sides unevaluated — they mention symbols the solver
                // will assign. lower_expr is a pure ExprAst → ComputeExpr map.
                constraints.constraints.push(LoweredConstraint {
                    lhs: lower_expr(lhs),
                    op: *op,
                    rhs: lower_expr(rhs),
                });
            }
            Statement::SolveFor { names } => {
                constraints.solve_for.extend(names.iter().cloned());
            }
            Statement::Check => {
                constraints.check = true;
            }
        }
    }

    Ok(LoweredProgram {
        kb,
        queries,
        constraints,
    })
}

/// Reject a likelihood ratio that the engine's `from_lr` constructors
/// would panic on (`lr <= 0`) or that is non-finite. Centralised so the
/// `contributes` and `interacts` paths share one guard.
fn check_lr(lr: f64) -> Result<(), LowerError> {
    if lr.is_finite() && lr > 0.0 {
        Ok(())
    } else {
        Err(LowerError::InvalidLikelihoodRatio { value: lr })
    }
}

fn lower_term(t: &AstTerm) -> CoreTerm {
    match t {
        AstTerm::Atom(name) => core_atom(name),
        AstTerm::Num(x) => core_float(*x),
        AstTerm::Compound { functor, args } => {
            compound(functor, args.iter().map(lower_term).collect())
        }
    }
}

/// Lower a surface `let` formula to the engine's [`ComputeExpr`].
fn lower_expr(expr: &ExprAst) -> ComputeExpr {
    match expr {
        ExprAst::Ref(slot) => ComputeExpr::Ref(slot.clone()),
        ExprAst::Lit(x) => ComputeExpr::Lit(*x),
        ExprAst::Bin(op, a, b) => ComputeExpr::Bin(
            lower_arith_op(*op),
            Box::new(lower_expr(a)),
            Box::new(lower_expr(b)),
        ),
        ExprAst::Agg(op, slot) => ComputeExpr::Agg(lower_agg_op(*op), slot.clone()),
    }
}

fn lower_arith_op(op: ArithOp) -> ComputeOp {
    match op {
        ArithOp::Add => ComputeOp::Add,
        ArithOp::Sub => ComputeOp::Sub,
        ArithOp::Mul => ComputeOp::Mul,
        ArithOp::Div => ComputeOp::Div,
    }
}

fn lower_agg_op(op: AggOp) -> ComputeOp {
    match op {
        AggOp::Sum => ComputeOp::Sum,
        AggOp::Count => ComputeOp::Count,
        AggOp::Min => ComputeOp::Min,
        AggOp::Max => ComputeOp::Max,
        AggOp::Avg => ComputeOp::Avg,
    }
}

/// Map the surface comparison operator to the engine's [`EngineCmpOp`].
fn lower_cmp_op(op: CmpOp) -> EngineCmpOp {
    match op {
        CmpOp::Ge => EngineCmpOp::Ge,
        CmpOp::Le => EngineCmpOp::Le,
        CmpOp::Gt => EngineCmpOp::Gt,
        CmpOp::Lt => EngineCmpOp::Lt,
        CmpOp::Eq => EngineCmpOp::Eq,
    }
}

fn annotations_to_provenance(annotations: &[Annotation]) -> Result<Provenance, LowerError> {
    let mut source: Option<String> = None;
    let mut locator: Option<String> = None;
    let mut trust: Option<TrustTier> = None;

    for a in annotations {
        match a {
            Annotation::Source(s) => {
                if source.is_some() {
                    return Err(LowerError::DuplicateAnnotation { name: "source" });
                }
                source = Some(s.clone());
            }
            Annotation::Locator(s) => {
                if locator.is_some() {
                    return Err(LowerError::DuplicateAnnotation { name: "locator" });
                }
                locator = Some(s.clone());
            }
            Annotation::Trust(name) => {
                if trust.is_some() {
                    return Err(LowerError::DuplicateAnnotation { name: "trust" });
                }
                trust = Some(match name {
                    TrustTierName::Consensus => TrustTier::Consensus,
                    TrustTierName::Authoritative => TrustTier::Authoritative,
                    TrustTierName::Empirical => TrustTier::Empirical,
                    TrustTierName::Inferred => TrustTier::Inferred,
                    TrustTierName::Unattributed => TrustTier::Unattributed,
                });
            }
        }
    }

    // If a source is present but no trust tier was specified, default
    // to Authoritative — the common case for cited rulebooks.
    // Otherwise (no source, no tier), default to Unattributed.
    let trust_tier = trust.unwrap_or_else(|| {
        if source.is_some() {
            TrustTier::Authoritative
        } else {
            TrustTier::Unattributed
        }
    });

    Ok(Provenance::new(
        source.unwrap_or_default(),
        locator,
        trust_tier,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile;
    use logic_engine::{search, SearchMode, SearchResult};

    #[test]
    fn lowers_full_acs_rulebook_and_reproduces_adj36_posterior() {
        let src = r#"
            prior 0.10 for acs
              source "Pope JH et al., NEJM 1995;342(16):1163-70"

            contributes 1.5 from pmh(hypertension) to acs
              source "HEART Score; Six AJ et al., Neth Heart J 2008"
              trust empirical

            contributes 1.8 from pmh(smoker) to acs
              source "HEART Score; Six AJ et al., Neth Heart J 2008"
              trust empirical

            contributes 2.5 from symptom_quality(pressure_like) to acs
              source "Panju AA et al., JAMA 1998;280(14):1256-63"

            contributes 2.0 from associated_symptom(diaphoresis) to acs
              source "Panju AA et al., JAMA 1998"

            contributes 0.5 from vital_signs(within_normal_limits) to acs
              source "Panju AA et al., JAMA 1998"

            contributes 0.4 from denied(ecg_acute_st_changes) to acs
              source "Pope JH et al., NEJM 1995"

            interacts 1.3 when symptom_quality(pressure_like)
                           and associated_symptom(diaphoresis)
                           for acs
              source "[empirical] synergy"
              trust empirical

            observe pmh(hypertension)
            observe pmh(smoker)
            observe symptom_quality(pressure_like)
            observe associated_symptom(diaphoresis)
            observe vital_signs(within_normal_limits)
            observe denied(ecg_acute_st_changes)

            ? acs
        "#;
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.queries.len(), 1);
        let query = &lowered.queries[0];
        let result = search(query, &lowered.kb, SearchMode::LRAggregate);
        match result {
            SearchResult::LRAggregateResult { posterior, .. } => {
                assert!(
                    (posterior - 0.281).abs() < 0.005,
                    "expected ≈0.281, got {posterior}"
                );
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_prior_is_rejected() {
        let src = r#"
            prior 0.10 for acs
            prior 0.20 for acs
        "#;
        let err = compile(src).unwrap_err();
        assert!(matches!(
            err,
            crate::CompileError::Lower(LowerError::DuplicatePrior { .. })
        ));
    }

    #[test]
    fn duplicate_source_annotation_is_rejected() {
        let src = r#"
            contributes 1.5 from pmh(htn) to acs
              source "first"
              source "second"
        "#;
        let err = compile(src).unwrap_err();
        assert!(matches!(
            err,
            crate::CompileError::Lower(LowerError::DuplicateAnnotation { name: "source" })
        ));
    }

    #[test]
    fn source_without_trust_defaults_to_authoritative() {
        let src = r#"
            contributes 1.5 from x to y
              source "some paper"
        "#;
        let lowered = compile(src).unwrap();
        let contribs = lowered.kb.contributions_for(&core_atom("y"));
        assert_eq!(contribs.len(), 1);
        assert_eq!(contribs[0].provenance.trust_tier, TrustTier::Authoritative);
    }

    #[test]
    fn no_source_and_no_trust_defaults_to_unattributed() {
        let src = "contributes 1.5 from x to y";
        let lowered = compile(src).unwrap();
        let contribs = lowered.kb.contributions_for(&core_atom("y"));
        assert_eq!(contribs[0].provenance.trust_tier, TrustTier::Unattributed);
    }

    #[test]
    fn observe_without_query_still_compiles() {
        let src = "observe pmh(hypertension)";
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.queries.len(), 0);
    }

    #[test]
    fn uncertain_statement_produces_voi_report_on_aggregation() {
        // The ACS rulebook with a `uncertain {…}` clause for the
        // precipitator, no precipitator observation. The aggregator
        // should return a VOI report listing the three candidate
        // values and the maximum log-odds swing knowing one of
        // them would produce.
        let src = r#"
            prior 0.10 for acs

            contributes 1.5 from pmh(hypertension) to acs
            contributes 2.5 from precipitator(exertional) to acs
            contributes 0.6 from precipitator(rest) to acs
            contributes 0.8 from precipitator(positional) to acs

            observe pmh(hypertension)

            uncertain { precipitator(exertional),
                        precipitator(rest),
                        precipitator(positional) } for acs
              source "patient did not specify"

            ? acs
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let result = search(query, &lowered.kb, SearchMode::LRAggregate);
        match result {
            SearchResult::LRAggregateResult { uncertainties, .. } => {
                assert_eq!(uncertainties.len(), 1);
                let report = &uncertainties[0];
                assert_eq!(report.domain.len(), 3);
                // VOI = ln(2.5) - ln(0.6) ≈ 1.4271
                assert!(
                    (report.voi_logit_range - (2.5_f64.ln() - 0.6_f64.ln())).abs() < 1e-9,
                    "got VOI {}",
                    report.voi_logit_range
                );
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn negative_contributes_lr_is_a_clean_error_not_a_panic() {
        // Regression: a malformed rulebook must not panic the process.
        // `contributes -5 ...` would hit the engine's `assert!(lr > 0.0)`.
        for src in [
            "contributes -5 from x to y",
            "contributes 0 from x to y",
            "interacts -1 when a and b for y",
        ] {
            let err = compile(src).unwrap_err();
            assert!(
                matches!(
                    err,
                    crate::CompileError::Lower(LowerError::InvalidLikelihoodRatio { .. })
                ),
                "expected InvalidLikelihoodRatio for {src:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn out_of_range_prior_is_a_clean_error_not_a_panic() {
        for src in ["prior 2 for x", "prior 0 for x", "prior -0.5 for x"] {
            let err = compile(src).unwrap_err();
            assert!(
                matches!(
                    err,
                    crate::CompileError::Lower(LowerError::InvalidProbability { .. })
                ),
                "expected InvalidProbability for {src:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn non_finite_number_literal_is_rejected_at_parse() {
        // 1e400 overflows f64 to +inf; reject it rather than flow inf on.
        let err = compile("observe gross_income(1e400)").unwrap_err();
        assert!(
            matches!(err, crate::CompileError::Adapt(_)),
            "expected an adapter BadToken error, got {err:?}"
        );
    }

    #[test]
    fn predicate_gated_contribution_fires_end_to_end() {
        // A DETERMINISTIC rule as a saturating LR: "income at/above the
        // filing threshold ⇒ required to file." The model authored the
        // rulebook; the comparison runs in the engine at decision time.
        let src = r#"
            prior 0.10 for required_to_file
            contributes 1000000 from gross_income >= 14600 to required_to_file
              source "IRS Pub 501 (2024), filing threshold single < 65"
              trust authoritative
            observe gross_income(18000)
            ? required_to_file
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        let result = search(query, &lowered.kb, SearchMode::LRAggregate);
        match result {
            SearchResult::LRAggregateResult { posterior, dag, .. } => {
                assert!(posterior > 0.9999, "should saturate, got {posterior}");
                // The proof carries the literal comparison that fired.
                let fired = dag.proofs[0].steps.iter().any(|s| {
                    matches!(
                        s.origin,
                        logic_engine::DerivationOrigin::FromPredicateContribution { .. }
                    )
                });
                assert!(fired, "expected a predicate-contribution step");
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn predicate_fires_over_typed_value_literal_end_to_end() {
        // Step 2: typed value literals. `quantity(18000, usd)` already
        // parses as a nested compound under the predicate grammar; the
        // engine reads its leading magnitude (18000) for the predicate
        // while the `usd` unit travels with the fact. No grammar change.
        let src = r#"
            prior 0.10 for required_to_file
            contributes 1000000 from gross_income >= 14600 to required_to_file
              source "IRS Pub 501 (2024)" trust authoritative
            observe gross_income(quantity(18000, usd))
            ? required_to_file
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        match search(query, &lowered.kb, SearchMode::LRAggregate) {
            SearchResult::LRAggregateResult { posterior, .. } => {
                assert!(posterior > 0.9999, "should saturate, got {posterior}");
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn predicate_below_threshold_stays_at_prior() {
        let src = r#"
            prior 0.10 for required_to_file
            contributes 1000000 from gross_income >= 14600 to required_to_file
            observe gross_income(9000)
            ? required_to_file
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        match search(query, &lowered.kb, SearchMode::LRAggregate) {
            SearchResult::LRAggregateResult { posterior, .. } => {
                assert!((posterior - 0.10).abs() < 1e-9, "got {posterior}");
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn locator_annotation_is_carried_through() {
        let src = r#"
            contributes 1.5 from x to y
              source "guideline"
              locator "§3.2"
        "#;
        let lowered = compile(src).unwrap();
        let contribs = lowered.kb.contributions_for(&core_atom("y"));
        assert_eq!(contribs[0].provenance.locator.as_deref(), Some("§3.2"));
    }

    // ---- `let` + arithmetic (ADJ expansion step 3b) ----

    #[test]
    fn let_arithmetic_computes_a_ratio_with_a_cited_tree() {
        let src = r#"
            observe csf_glucose(quantity(40, mg_dl))
            observe serum_glucose(quantity(100, mg_dl))
            let csf_ratio = csf_glucose / serum_glucose
        "#;
        let lowered = compile(src).unwrap();
        let d = lowered
            .kb
            .derived_for("csf_ratio")
            .expect("csf_ratio should be bound");
        assert!((d.value - 0.4).abs() < 1e-12, "got {}", d.value);
    }

    #[test]
    fn let_derived_value_fires_a_predicate_end_to_end() {
        // The whole point: a predicate fires over a COMPUTED value exactly
        // as over an observed one. Low CSF:serum ratio ⇒ bacterial.
        let src = r#"
            prior 0.30 for bacterial
            observe csf_glucose(40)
            observe serum_glucose(100)
            let csf_ratio = csf_glucose / serum_glucose
            contributes 1000000 from csf_ratio <= 0.5 to bacterial
              source "Spanos 1989" trust authoritative
            ? bacterial
        "#;
        let lowered = compile(src).unwrap();
        let query = &lowered.queries[0];
        match search(query, &lowered.kb, SearchMode::LRAggregate) {
            SearchResult::LRAggregateResult { posterior, .. } => {
                assert!(
                    posterior > 0.9999,
                    "predicate over derived value should fire; got {posterior}"
                );
            }
            other => panic!("expected LRAggregateResult, got {other:?}"),
        }
    }

    #[test]
    fn let_sum_aggregates_repeated_observations() {
        let src = r#"
            observe line_item(12000)
            observe line_item(6000)
            observe line_item(2000)
            let total = sum(line_item)
        "#;
        let lowered = compile(src).unwrap();
        assert!((lowered.kb.derived_for("total").unwrap().value - 20000.0).abs() < 1e-9);
    }

    #[test]
    fn let_respects_operator_precedence_and_parens() {
        // a + b * c  ==  a + (b*c);  (a + b) * c  forces the other grouping.
        let src = r#"
            observe a(2)
            observe b(3)
            observe c(4)
            let unparen = a + b * c
            let paren = (a + b) * c
        "#;
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.kb.derived_for("unparen").unwrap().value, 14.0); // 2 + 12
        assert_eq!(lowered.kb.derived_for("paren").unwrap().value, 20.0); //  5 * 4
    }

    #[test]
    fn let_can_reference_an_earlier_let() {
        let src = r#"
            observe a(3)
            observe b(4)
            let s = a + b
            let d = s * 2
        "#;
        let lowered = compile(src).unwrap();
        assert_eq!(lowered.kb.derived_for("d").unwrap().value, 14.0);
    }

    #[test]
    fn let_over_unknown_slot_is_a_clean_error() {
        let err = compile("let x = nope / 2").unwrap_err();
        assert!(
            matches!(
                err,
                crate::CompileError::Lower(LowerError::ComputationFailed { .. })
            ),
            "got {err:?}"
        );
    }

    // ---- constraint sublanguage (ADJ constraints track B1) ----

    #[test]
    fn symbol_constrain_solve_check_build_a_constraint_system() {
        // A small eligibility set: premium is unknown, bounded above by 2000
        // and below by the observed base_rate; solve for it.
        let src = r#"
            symbol premium : money(usd)
            symbol months  : scalar
            observe base_rate(1200)
            constrain premium <= 2000
            constrain premium >= base_rate
            constrain months >= 6
            solve for { premium, months }
        "#;
        let lowered = compile(src).unwrap();
        let cs = &lowered.constraints;
        assert!(!cs.is_empty());
        assert_eq!(cs.symbols.len(), 2);
        assert_eq!(cs.symbols[0].0, "premium");
        assert!(matches!(
            &cs.symbols[0].1,
            core_compound_money @ _ if format!("{core_compound_money:?}").contains("money")
        ));
        assert_eq!(cs.symbols[1].0, "months");
        assert_eq!(cs.constraints.len(), 3);
        assert_eq!(cs.constraints[0].op, crate::ast::RelOp::Le);
        assert_eq!(cs.constraints[1].op, crate::ast::RelOp::Ge);
        assert_eq!(cs.solve_for, vec!["premium".to_string(), "months".to_string()]);
        assert!(!cs.check);
    }

    #[test]
    fn check_sets_the_feasibility_flag() {
        let lowered = compile("constrain x >= 1\ncheck").unwrap();
        assert!(lowered.constraints.check);
        assert_eq!(lowered.constraints.constraints.len(), 1);
    }

    #[test]
    fn constraint_operands_lower_to_unevaluated_compute_exprs() {
        // `constrain total = a + b * c` — the rhs stays a ComputeExpr tree
        // (not evaluated; it mentions symbols the solver will assign).
        let lowered = compile("constrain total = a + b * 2").unwrap();
        let c = &lowered.constraints.constraints[0];
        assert_eq!(c.op, crate::ast::RelOp::Eq);
        assert!(matches!(c.lhs, logic_engine::ComputeExpr::Ref(_)));
        // rhs is a + (b * 2): an Add whose right operand is a Mul.
        assert!(matches!(c.rhs, logic_engine::ComputeExpr::Bin(logic_engine::ComputeOp::Add, _, _)));
    }

    #[test]
    fn all_relational_operators_parse() {
        for (src, want) in [
            ("constrain a >= 1", crate::ast::RelOp::Ge),
            ("constrain a <= 1", crate::ast::RelOp::Le),
            ("constrain a > 1", crate::ast::RelOp::Gt),
            ("constrain a < 1", crate::ast::RelOp::Lt),
            ("constrain a == 1", crate::ast::RelOp::Eq),
            ("constrain a = 1", crate::ast::RelOp::Eq),
            ("constrain a != 1", crate::ast::RelOp::Ne),
        ] {
            let lowered = compile(src).unwrap();
            assert_eq!(lowered.constraints.constraints[0].op, want, "for {src:?}");
        }
    }

    #[test]
    fn a_pure_rulebook_has_an_empty_constraint_system() {
        let lowered = compile("prior 0.10 for acs\n? acs").unwrap();
        assert!(lowered.constraints.is_empty());
    }
}

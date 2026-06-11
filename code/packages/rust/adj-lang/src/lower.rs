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
    CmpOp as EngineCmpOp, ContributionClause, Fact, JointContributionClause, KbError,
    KnowledgeBase, PredicateContributionClause, PriorClause, Provenance, TrustTier,
    UncertaintyMarker,
};

use crate::ast::{Annotation, CmpOp, Evidence, Program, Statement, Term as AstTerm, TrustTierName};

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
}

/// The result of lowering — a populated KB and any queries to run.
#[derive(Debug)]
pub struct LoweredProgram {
    pub kb: KnowledgeBase,
    pub queries: Vec<CoreTerm>,
}

/// Lower an [`ast::Program`] to a populated KB + queries.
pub fn lower(program: &Program) -> Result<LoweredProgram, LowerError> {
    let mut kb = KnowledgeBase::new();
    let mut queries = Vec::new();

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
        }
    }

    Ok(LoweredProgram { kb, queries })
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
}

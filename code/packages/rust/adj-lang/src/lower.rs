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

use logic_core::{atom as core_atom, compound, Term as CoreTerm};
use logic_engine::{
    ContributionClause, Fact, JointContributionClause, KbError, KnowledgeBase, PriorClause,
    Provenance, TrustTier,
};

use crate::ast::{Annotation, Program, Statement, Term as AstTerm, TrustTierName};

#[derive(Debug, Clone, PartialEq)]
pub enum LowerError {
    DuplicatePrior { conclusion_name: String },
    DuplicateAnnotation { name: &'static str },
    EngineRejected { detail: String },
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
                let prov = annotations_to_provenance(annotations)?;
                let clause = PriorClause::from_probability(lower_term(conclusion), *probability)
                    .with_provenance(prov);
                kb.add_prior(clause).map_err(|e| match e {
                    KbError::ConflictingPriors { conclusion, .. } => {
                        LowerError::DuplicatePrior {
                            conclusion_name: format!("{conclusion:?}"),
                        }
                    }
                })?;
            }
            Statement::Contributes {
                lr,
                evidence,
                conclusion,
                annotations,
            } => {
                let prov = annotations_to_provenance(annotations)?;
                let clause = ContributionClause::from_lr(
                    lower_term(conclusion),
                    lower_term(evidence),
                    *lr,
                )
                .with_provenance(prov);
                kb.add_contribution(clause);
            }
            Statement::Interacts {
                lr,
                evidence_set,
                conclusion,
                annotations,
            } => {
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
        }
    }

    Ok(LoweredProgram { kb, queries })
}

fn lower_term(t: &AstTerm) -> CoreTerm {
    match t {
        AstTerm::Atom(name) => core_atom(name),
        AstTerm::Compound { functor, args } => {
            compound(functor, args.iter().map(lower_term).collect())
        }
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
    fn locator_annotation_is_carried_through() {
        let src = r#"
            contributes 1.5 from x to y
              source "guideline"
              locator "§3.2"
        "#;
        let lowered = compile(src).unwrap();
        let contribs = lowered.kb.contributions_for(&core_atom("y"));
        assert_eq!(
            contribs[0].provenance.locator.as_deref(),
            Some("§3.2")
        );
    }
}

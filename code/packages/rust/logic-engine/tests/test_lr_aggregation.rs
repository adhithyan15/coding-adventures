//! Integration tests for the LP19e LR-aggregation path through the
//! top-level `search` API. Mirrors the ADJ36 ACS chest-pain rulebook
//! semantically — what the rulebook says directly, not the ADJ46
//! awkward synthetic-`contrib`-marker workaround.

use logic_core::{atom, compound};
use logic_engine::{
    counterfactual, search, source_disagreements, BodyLiteral, ContributionClause, Fact,
    JointContributionClause, KnowledgeBase, LrAggregateWarning, PriorClause, Provenance, Rule,
    SearchMode, SearchResult, TrustTier, UncertaintyMarker,
};

fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

/// Build the ACS rulebook directly using LR aggregation primitives.
/// This is what ADJ46's `src/main.rs` *wanted* to write but couldn't
/// — A1, A2, A3, A4, A6 are dissolved at the engine layer.
fn build_acs_kb() -> KnowledgeBase {
    let mut kb = KnowledgeBase::new();
    // Prior: 10% baseline ED chest-pain ACS prevalence (Pope 1995).
    kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
        .unwrap();

    // Demographic contributors (HEART score / Six 2008).
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("pmh", vec![atom("hypertension")]),
        1.5,
    ));
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("pmh", vec![atom("smoker")]),
        1.8,
    ));

    // Symptom quality (Panju 1998).
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("symptom_quality", vec![atom("pressure_like")]),
        2.5,
    ));
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("associated_symptom", vec![atom("diaphoresis")]),
        2.0,
    ));

    // Precipitator (Diamond/Forrester 1979).
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("precipitator", vec![atom("exertional")]),
        2.5,
    ));
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("precipitator", vec![atom("rest")]),
        0.6,
    ));
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("precipitator", vec![atom("positional")]),
        0.8,
    ));

    // Protective.
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("vital_signs", vec![atom("within_normal_limits")]),
        0.5,
    ));
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("denied", vec![atom("ecg_acute_st_changes")]),
        0.4,
    ));

    // Synergy / interaction term.
    kb.add_joint_contribution(JointContributionClause::from_lr(
        atom("acs"),
        vec![
            compound("symptom_quality", vec![atom("pressure_like")]),
            compound("associated_symptom", vec![atom("diaphoresis")]),
        ],
        1.3,
    ));

    kb
}

/// The ADJ36 Jane Doe vignette as observed Facts:
/// 62yo M with HTN + smoker, pressure-like discomfort with diaphoresis,
/// vitals normal, no acute ST changes, no clear precipitator.
fn add_jane_doe(kb: &mut KnowledgeBase) {
    kb.add_fact(Fact::certain(compound("pmh", vec![atom("hypertension")])));
    kb.add_fact(Fact::certain(compound("pmh", vec![atom("smoker")])));
    kb.add_fact(Fact::certain(compound(
        "symptom_quality",
        vec![atom("pressure_like")],
    )));
    kb.add_fact(Fact::certain(compound(
        "associated_symptom",
        vec![atom("diaphoresis")],
    )));
    kb.add_fact(Fact::certain(compound(
        "vital_signs",
        vec![atom("within_normal_limits")],
    )));
    kb.add_fact(Fact::certain(compound(
        "denied",
        vec![atom("ecg_acute_st_changes")],
    )));
    // Note: no precipitator fact — patient said "no clear precipitator."
}

#[test]
fn acs_rulebook_on_jane_doe_reproduces_adj36_posterior() {
    let mut kb = build_acs_kb();
    add_jane_doe(&mut kb);

    let result = search(&atom("acs"), &kb, SearchMode::LRAggregate);
    match result {
        SearchResult::LRAggregateResult {
            posterior,
            warnings,
            dag,
            ..
        } => {
            // ADJ36's published posterior is 0.281 (28.1%). The
            // LP19e engine path should reproduce it within rounding.
            // (ADJ46's hand-aggregated number was 0.2806; the engine
            // is doing the same arithmetic so we expect the same
            // ballpark.)
            assert!(
                approx_eq(posterior, 0.281, 0.005),
                "expected P(acs) ≈ 0.281, got {posterior}"
            );

            // No warnings expected — prior is present, contributions
            // are active, no degenerate LR=1.0.
            assert!(
                warnings.is_empty(),
                "expected no warnings, got {warnings:?}"
            );

            // Proof DAG has exactly one Proof; that proof has the
            // prior step + 7 active atomic contributions + 1 joint:
            // pmh(htn), pmh(smoker), pressure_like, diaphoresis,
            // vital_signs, denied(st_changes), and the joint of
            // pressure_like ⊗ diaphoresis. Precipitator contributors
            // do NOT fire because no precipitator fact is asserted.
            assert_eq!(dag.proofs.len(), 1);
            let proof = &dag.proofs[0];
            assert_eq!(
                proof.steps.len(),
                1 /* prior */ + 6 /* atomic */ + 1 /* joint */
            );

            // posterior_logit and posterior_probability are set on
            // LR-aggregation proofs (the spec says so).
            assert!(proof.posterior_logit.is_some());
            assert!(proof.posterior_probability.is_some());
        }
        other => panic!("expected LRAggregateResult, got {other:?}"),
    }
}

#[test]
fn auto_detect_picks_lr_aggregate_when_conclusion_has_contributions() {
    let mut kb = build_acs_kb();
    add_jane_doe(&mut kb);

    // AutoDetect should route to LRAggregate because `acs` is the
    // target of contribution clauses — even though every Fact in
    // the KB is Certain.
    let result = search(&atom("acs"), &kb, SearchMode::AutoDetect);
    assert!(matches!(result, SearchResult::LRAggregateResult { .. }));
}

#[test]
fn auto_detect_falls_back_to_find_first_when_no_lr_clauses() {
    // Same engine, but `acs` isn't the target of any contributions
    // — AutoDetect goes to FindFirst since the rest of the KB is
    // Certain.
    let mut kb = KnowledgeBase::new();
    kb.add_fact(Fact::certain(atom("p")));

    let result = search(&atom("p"), &kb, SearchMode::AutoDetect);
    assert!(matches!(result, SearchResult::FindFirstResult(Some(_))));
}

#[test]
fn missing_prior_produces_uniform_posterior_with_warning() {
    let mut kb = KnowledgeBase::new();
    // Contribution without a prior — algorithm proceeds at P=0.5
    // and emits a NoPriorDeclared warning per LP19e §"Edge cases."
    kb.add_contribution(ContributionClause::from_lr(atom("c"), atom("ev"), 3.0));
    kb.add_fact(Fact::certain(atom("ev")));

    let result = search(&atom("c"), &kb, SearchMode::LRAggregate);
    match result {
        SearchResult::LRAggregateResult {
            posterior,
            warnings,
            ..
        } => {
            // log(3.0) ≈ 1.0986 with no prior (logit=0) → sigmoid ≈ 0.75
            assert!(approx_eq(posterior, 0.75, 0.005));
            assert!(warnings
                .iter()
                .any(|w| matches!(w, LrAggregateWarning::NoPriorDeclared { .. })));
        }
        other => panic!("expected LRAggregateResult, got {other:?}"),
    }
}

#[test]
fn proof_dag_carries_evidence_fact_ids_through_lr_steps() {
    use logic_engine::DerivationOrigin;

    let mut kb = KnowledgeBase::new();
    kb.add_prior(PriorClause::from_probability(atom("c"), 0.10))
        .unwrap();
    kb.add_contribution(ContributionClause::from_lr(atom("c"), atom("e"), 4.0));
    let fact_id = kb.add_fact(Fact::certain(atom("e")));

    let result = search(&atom("c"), &kb, SearchMode::LRAggregate);
    if let SearchResult::LRAggregateResult { dag, .. } = result {
        let contribution_step = dag.proofs[0]
            .steps
            .iter()
            .find(|s| matches!(s.origin, DerivationOrigin::FromContribution { .. }))
            .expect("a contribution step should be present");
        if let DerivationOrigin::FromContribution {
            evidence_fact_ids, ..
        } = &contribution_step.origin
        {
            assert_eq!(evidence_fact_ids, &vec![fact_id]);
        }
        // and via_facts on the Proof aggregates the evidence ids
        assert!(dag.proofs[0].via_facts.contains(&fact_id));
    } else {
        panic!("expected LRAggregateResult");
    }
}

#[test]
fn rule_derived_evidence_gates_contribution_and_carries_proof() {
    use logic_engine::DerivationOrigin;

    let mut kb = KnowledgeBase::new();
    kb.add_prior(PriorClause::from_probability(atom("bacterial"), 0.10))
        .unwrap();
    kb.add_contribution(ContributionClause::from_lr(
        atom("bacterial"),
        atom("infection_present"),
        10.0,
    ));
    let fever_id = kb.add_fact(Fact::certain(atom("fever")));
    let culture_id = kb.add_fact(Fact::certain(atom("positive_culture")));
    let rule_id = kb.add_rule(Rule::certain(
        atom("infection_present"),
        vec![
            BodyLiteral::Pos(atom("fever")),
            BodyLiteral::Pos(atom("positive_culture")),
        ],
    ));

    let result = search(&atom("bacterial"), &kb, SearchMode::LRAggregate);
    if let SearchResult::LRAggregateResult { dag, posterior, .. } = result {
        // prior odds 1/9, LR 10 => odds 10/9 => posterior 10/19.
        assert!(approx_eq(posterior, 10.0 / 19.0, 1e-12));
        let proof = &dag.proofs[0];
        assert!(proof.via_facts.contains(&fever_id));
        assert!(proof.via_facts.contains(&culture_id));
        assert!(proof.via_rules.contains(&rule_id));
        let contribution_step = proof
            .steps
            .iter()
            .find(|s| matches!(s.origin, DerivationOrigin::FromContribution { .. }))
            .expect("a contribution step should be present");
        if let DerivationOrigin::FromContribution {
            evidence_fact_ids,
            evidence_proof,
            logit_delta,
            ..
        } = &contribution_step.origin
        {
            assert!(evidence_fact_ids.contains(&fever_id));
            assert!(evidence_fact_ids.contains(&culture_id));
            let evidence_proof = evidence_proof
                .as_ref()
                .expect("rule-derived evidence should carry its SLD proof");
            assert!(evidence_proof.via_rules.contains(&rule_id));
            assert!(approx_eq(*logit_delta, 10.0_f64.ln(), 1e-12));
        }
    } else {
        panic!("expected LRAggregateResult");
    }
}

#[test]
fn probabilistic_rule_derived_evidence_attenuates_contribution() {
    use logic_engine::DerivationOrigin;

    let mut kb = KnowledgeBase::new();
    kb.add_prior(PriorClause::from_probability(atom("bacterial"), 0.10))
        .unwrap();
    kb.add_contribution(ContributionClause::from_lr(
        atom("bacterial"),
        atom("infection_present"),
        16.0,
    ));
    kb.add_fact(Fact::certain(atom("fever")));
    let rule_id = kb.add_rule(Rule::with_probability(
        atom("infection_present"),
        vec![BodyLiteral::Pos(atom("fever"))],
        0.5,
    ));

    let result = search(&atom("bacterial"), &kb, SearchMode::LRAggregate);
    if let SearchResult::LRAggregateResult { dag, posterior, .. } = result {
        // ln(16) attenuated by rule confidence 0.5 is ln(4).
        assert!(approx_eq(posterior, 4.0 / 13.0, 1e-12));
        assert!(dag.proofs[0].via_rules.contains(&rule_id));
        let contribution_step = dag.proofs[0]
            .steps
            .iter()
            .find(|s| matches!(s.origin, DerivationOrigin::FromContribution { .. }))
            .expect("a contribution step should be present");
        if let DerivationOrigin::FromContribution { logit_delta, .. } = &contribution_step.origin {
            assert!(approx_eq(*logit_delta, 16.0_f64.ln() * 0.5, 1e-12));
        }
    } else {
        panic!("expected LRAggregateResult");
    }
}

#[test]
fn term_equality_on_compounds_works_for_contribution_lookup() {
    // Confirms the linear-scan KB representation (Vec, not HashMap)
    // is correct for compound-term contributions.
    let mut kb = KnowledgeBase::new();
    kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
        .unwrap();
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("symptom", vec![atom("pressure")]),
        2.0,
    ));
    // The compound term — distinct from atom("symptom") — must NOT
    // satisfy the contribution. Add a fact for the bare atom and
    // confirm posterior stays at prior.
    kb.add_fact(Fact::certain(atom("symptom")));

    let result = search(&atom("acs"), &kb, SearchMode::LRAggregate);
    if let SearchResult::LRAggregateResult { posterior, .. } = result {
        assert!(
            approx_eq(posterior, 0.10, 1e-12),
            "compound vs atom mismatch should leave posterior = prior; got {posterior}"
        );
    } else {
        panic!("expected LRAggregateResult");
    }
}

#[test]
fn provenance_is_recoverable_from_kb_after_aggregation() {
    // Build a tiny rulebook with citations on every clause, run LR
    // aggregation, and confirm that for every step in the proof we
    // can recover the citation by joining the step's clause id back
    // to the KB. This is the ADJ47-B contract: clauses carry
    // provenance, and the audit reader recovers it without a
    // side-table.
    use logic_engine::DerivationOrigin;

    let mut kb = KnowledgeBase::new();
    let prior_id = kb
        .add_prior(
            PriorClause::from_probability(atom("acs"), 0.10).with_provenance(Provenance::cited(
                "Pope JH et al., NEJM 1995;342(16):1163-70",
            )),
        )
        .unwrap();
    let contrib_id = kb.add_contribution(
        ContributionClause::from_lr(
            atom("acs"),
            compound("pmh", vec![atom("hypertension")]),
            1.5,
        )
        .with_provenance(
            Provenance::empirical("HEART Score; Six AJ et al., Neth Heart J 2008;16(6):191-6")
                .with_locator("Table 2"),
        ),
    );
    kb.add_fact(Fact::certain(compound("pmh", vec![atom("hypertension")])));

    let result = search(&atom("acs"), &kb, SearchMode::LRAggregate);
    let dag = if let SearchResult::LRAggregateResult { dag, .. } = result {
        dag
    } else {
        panic!("expected LRAggregateResult")
    };
    let proof = &dag.proofs[0];

    // For each step, recover the provenance via the clause id.
    for step in &proof.steps {
        match &step.origin {
            DerivationOrigin::FromPrior { clause_id, .. } => {
                assert_eq!(*clause_id, prior_id);
                let p = kb.prior_for(&atom("acs")).unwrap();
                assert_eq!(p.provenance.trust_tier, TrustTier::Authoritative);
                assert_eq!(
                    p.provenance.source,
                    "Pope JH et al., NEJM 1995;342(16):1163-70"
                );
            }
            DerivationOrigin::FromContribution { clause_id, .. } => {
                assert_eq!(*clause_id, contrib_id);
                let contribs = kb.contributions_for(&atom("acs"));
                let c = contribs.iter().find(|c| c.id == contrib_id).unwrap();
                assert_eq!(c.provenance.trust_tier, TrustTier::Empirical);
                assert!(c.provenance.source.contains("HEART"));
                assert_eq!(c.provenance.locator.as_deref(), Some("Table 2"));
            }
            _ => {}
        }
    }
}

#[test]
fn unattributed_provenance_is_the_default() {
    // Clauses constructed without `.with_provenance(...)` should
    // round-trip with Provenance::unattributed(). This is the
    // legacy-compatible behaviour: pre-ADJ47-B code that doesn't
    // know about provenance keeps working.
    let mut kb = KnowledgeBase::new();
    kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
        .unwrap();
    let prior = kb.prior_for(&atom("acs")).unwrap();
    assert_eq!(prior.provenance, Provenance::unattributed());
    assert_eq!(prior.provenance.trust_tier, TrustTier::Unattributed);
}

#[test]
fn uncertainty_marker_with_no_observed_domain_value_produces_report() {
    // Build the ACS rulebook (without the precipitator observations)
    // and attach an UncertaintyMarker stating that the patient did
    // not specify their precipitator. The LR aggregator should:
    //   1. Skip all precipitator contributions (no observation).
    //   2. Emit an UncertaintyReport listing the candidate values
    //      and the log-odds delta each would contribute.
    let mut kb = build_acs_kb();
    add_jane_doe(&mut kb);
    kb.add_uncertainty_marker(
        UncertaintyMarker::new(
            atom("acs"),
            vec![
                compound("precipitator", vec![atom("exertional")]),
                compound("precipitator", vec![atom("rest")]),
                compound("precipitator", vec![atom("positional")]),
            ],
        )
        .with_provenance(Provenance::cited("patient did not specify precipitator")),
    );

    let result = search(&atom("acs"), &kb, SearchMode::LRAggregate);
    let (posterior, uncertainties) = match result {
        SearchResult::LRAggregateResult {
            posterior,
            uncertainties,
            ..
        } => (posterior, uncertainties),
        other => panic!("expected LRAggregateResult, got {other:?}"),
    };

    // Posterior should still be ~0.281 (the unmodified ADJ36 number),
    // because the marker reports but doesn't change aggregation.
    assert!(
        (posterior - 0.281).abs() < 0.005,
        "expected ~0.281, got {posterior}"
    );

    // Exactly one uncertainty report — for the precipitator.
    assert_eq!(uncertainties.len(), 1);
    let report = &uncertainties[0];
    assert_eq!(report.conclusion, atom("acs"));
    assert_eq!(report.domain.len(), 3);
    assert_eq!(report.if_observed_logit_delta.len(), 3);

    // VOI: log(2.5) - log(0.6) = ln(2.5/0.6) ≈ 1.4271
    assert!(
        (report.voi_logit_range - (2.5_f64.ln() - 0.6_f64.ln())).abs() < 1e-9,
        "expected VOI ≈ 1.4271, got {}",
        report.voi_logit_range
    );

    // Spot-check individual deltas: the first domain entry is
    // precipitator(exertional), which has contributes 2.5 → log(2.5).
    assert!(
        (report.if_observed_logit_delta[0] - 2.5_f64.ln()).abs() < 1e-9,
        "expected log(2.5) ≈ 0.916, got {}",
        report.if_observed_logit_delta[0]
    );
}

#[test]
fn observing_one_domain_value_suppresses_uncertainty_report() {
    // Same rulebook + marker, but observe precipitator(exertional).
    // The marker should not produce a report because the
    // uncertainty is "resolved" by the observation.
    let mut kb = build_acs_kb();
    add_jane_doe(&mut kb);
    kb.add_fact(Fact::certain(compound(
        "precipitator",
        vec![atom("exertional")],
    )));
    kb.add_uncertainty_marker(UncertaintyMarker::new(
        atom("acs"),
        vec![
            compound("precipitator", vec![atom("exertional")]),
            compound("precipitator", vec![atom("rest")]),
            compound("precipitator", vec![atom("positional")]),
        ],
    ));

    let result = search(&atom("acs"), &kb, SearchMode::LRAggregate);
    if let SearchResult::LRAggregateResult { uncertainties, .. } = result {
        assert!(
            uncertainties.is_empty(),
            "marker should be suppressed when domain is observed; got {uncertainties:?}"
        );
    } else {
        panic!("expected LRAggregateResult");
    }
}

#[test]
fn uncertainty_marker_with_no_matching_contributions_has_zero_voi() {
    // A marker whose domain values are not the target of any
    // ContributionClause for the conclusion. The report should
    // exist (the marker is active because nothing is observed)
    // but every if_observed_logit_delta is 0.0 and VOI is 0.0 —
    // a "rulebook gap" the modeller should know about.
    let mut kb = KnowledgeBase::new();
    kb.add_prior(PriorClause::from_probability(atom("c"), 0.10))
        .unwrap();
    kb.add_contribution(ContributionClause::from_lr(
        atom("c"),
        atom("some_evidence"),
        2.0,
    ));
    // Force this conclusion into the LR-aggregation regime via
    // the participates check — the contribution above does that.
    kb.add_uncertainty_marker(UncertaintyMarker::new(
        atom("c"),
        vec![atom("uncovered_a"), atom("uncovered_b")],
    ));

    let result = search(&atom("c"), &kb, SearchMode::LRAggregate);
    if let SearchResult::LRAggregateResult { uncertainties, .. } = result {
        assert_eq!(uncertainties.len(), 1);
        assert_eq!(uncertainties[0].if_observed_logit_delta, vec![0.0, 0.0]);
        assert_eq!(uncertainties[0].voi_logit_range, 0.0);
    } else {
        panic!("expected LRAggregateResult");
    }
}

#[test]
fn counterfactual_assuming_precipitator_exertional_raises_posterior() {
    // Baseline ACS posterior is ~0.281 with no precipitator info.
    // Knowing the precipitator was exertional adds log(2.5) to the
    // log-odds, so the counterfactual posterior should be higher.
    let mut kb = build_acs_kb();
    add_jane_doe(&mut kb);

    let baseline = search(&atom("acs"), &kb, SearchMode::LRAggregate);
    let baseline_p = match baseline {
        SearchResult::LRAggregateResult { posterior, .. } => posterior,
        _ => panic!(),
    };

    let counter = counterfactual(
        &atom("acs"),
        &kb,
        &[compound("precipitator", vec![atom("exertional")])],
    );

    assert!(
        counter.posterior > baseline_p,
        "exertional precipitator should raise posterior; baseline={baseline_p}, counter={}",
        counter.posterior
    );
    // logit(0.281) + ln(2.5) → sigmoid ≈ 0.494
    assert!(
        (counter.posterior - 0.494).abs() < 0.01,
        "expected ~0.494, got {}",
        counter.posterior
    );
}

#[test]
fn counterfactual_does_not_mutate_the_caller_kb() {
    // After running counterfactual, the original KB must be
    // unchanged — the precipitator fact must not leak back.
    let mut kb = build_acs_kb();
    add_jane_doe(&mut kb);
    let _ = counterfactual(
        &atom("acs"),
        &kb,
        &[compound("precipitator", vec![atom("exertional")])],
    );
    // Re-run aggregation: same as before, no precipitator fact.
    let after = search(&atom("acs"), &kb, SearchMode::LRAggregate);
    let after_p = match after {
        SearchResult::LRAggregateResult { posterior, .. } => posterior,
        _ => panic!(),
    };
    assert!(
        (after_p - 0.281).abs() < 0.005,
        "baseline posterior should be unchanged; got {after_p}"
    );
}

#[test]
fn kickback_suggested_when_uncertainty_band_straddles_decision_threshold() {
    // Patient case with no precipitator → posterior ~0.281, with
    // VOI ranging from log(0.6) to log(2.5). At threshold 0.30, the
    // band ~[0.19, 0.49] straddles the threshold → kickback should
    // suggest.
    let mut kb = build_acs_kb();
    add_jane_doe(&mut kb);
    kb.add_uncertainty_marker(UncertaintyMarker::new(
        atom("acs"),
        vec![
            compound("precipitator", vec![atom("exertional")]),
            compound("precipitator", vec![atom("rest")]),
            compound("precipitator", vec![atom("positional")]),
        ],
    ));
    let result = search(&atom("acs"), &kb, SearchMode::LRAggregate);
    let lr = match result {
        SearchResult::LRAggregateResult {
            dag,
            posterior,
            posterior_logit,
            warnings,
            uncertainties,
        } => logic_engine::LRAggregateResult {
            dag,
            posterior,
            posterior_logit,
            warnings,
            uncertainties,
        },
        _ => panic!(),
    };
    let kb_back = lr.suggest_kickback(0.30);
    assert!(kb_back.is_some(), "expected kickback at threshold 0.30");
    let k = kb_back.unwrap();
    assert!(k.posterior_lo < 0.30);
    assert!(k.posterior_hi > 0.30);
    assert_eq!(k.recommended_resolutions.len(), 1);
}

#[test]
fn no_kickback_when_uncertainty_band_lies_on_one_side_of_threshold() {
    // Same setup but with a threshold of 0.05 — well below the
    // band — kickback should NOT trigger because every resolution
    // would still leave the posterior above the threshold.
    let mut kb = build_acs_kb();
    add_jane_doe(&mut kb);
    kb.add_uncertainty_marker(UncertaintyMarker::new(
        atom("acs"),
        vec![
            compound("precipitator", vec![atom("exertional")]),
            compound("precipitator", vec![atom("rest")]),
            compound("precipitator", vec![atom("positional")]),
        ],
    ));
    let result = search(&atom("acs"), &kb, SearchMode::LRAggregate);
    let lr = match result {
        SearchResult::LRAggregateResult {
            dag,
            posterior,
            posterior_logit,
            warnings,
            uncertainties,
        } => logic_engine::LRAggregateResult {
            dag,
            posterior,
            posterior_logit,
            warnings,
            uncertainties,
        },
        _ => panic!(),
    };
    assert!(lr.suggest_kickback(0.05).is_none());
}

#[test]
fn source_disagreement_detected_when_two_contributions_disagree() {
    // Same evidence, two ContributionClauses with different LRs —
    // simulates AHA saying LR=2.5, ESC saying LR=4.0 for the same
    // finding. The detector should flag them.
    let mut kb = KnowledgeBase::new();
    kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
        .unwrap();
    kb.add_contribution(
        ContributionClause::from_lr(
            atom("acs"),
            compound("symptom", vec![atom("pressure")]),
            2.5,
        )
        .with_provenance(Provenance::cited("AHA 2021")),
    );
    kb.add_contribution(
        ContributionClause::from_lr(
            atom("acs"),
            compound("symptom", vec![atom("pressure")]),
            4.0,
        )
        .with_provenance(Provenance::cited("ESC 2023")),
    );
    let reports = source_disagreements(&kb, &atom("acs"));
    assert_eq!(reports.len(), 1);
    let r = &reports[0];
    assert_eq!(r.evidence_term, compound("symptom", vec![atom("pressure")]));
    assert_eq!(r.source_logit_deltas.len(), 2);
    // disagreement range = ln(4.0) - ln(2.5) ≈ 0.47
    assert!(
        (r.disagreement_logit_range - (4.0_f64.ln() - 2.5_f64.ln())).abs() < 1e-9,
        "got {}",
        r.disagreement_logit_range
    );
}

#[test]
fn no_disagreement_reported_when_only_one_source() {
    let mut kb = KnowledgeBase::new();
    kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
        .unwrap();
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("symptom", vec![atom("pressure")]),
        2.5,
    ));
    assert!(source_disagreements(&kb, &atom("acs")).is_empty());
}

#[test]
fn no_disagreement_reported_when_sources_agree() {
    let mut kb = KnowledgeBase::new();
    kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
        .unwrap();
    // Two ContributionClauses with the same LR — even though they
    // share an evidence term, they don't *disagree*. No report.
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("symptom", vec![atom("pressure")]),
        2.5,
    ));
    kb.add_contribution(ContributionClause::from_lr(
        atom("acs"),
        compound("symptom", vec![atom("pressure")]),
        2.5,
    ));
    assert!(source_disagreements(&kb, &atom("acs")).is_empty());
}

#[test]
fn multiple_priors_for_same_conclusion_are_rejected() {
    let mut kb = KnowledgeBase::new();
    kb.add_prior(PriorClause::from_probability(atom("acs"), 0.10))
        .unwrap();
    let err = kb
        .add_prior(PriorClause::from_probability(atom("acs"), 0.20))
        .unwrap_err();
    // Exhaustive match; if KbError gains variants, this test reminds
    // us to update.
    match err {
        logic_engine::KbError::ConflictingPriors { conclusion, .. } => {
            assert_eq!(conclusion, atom("acs"));
        }
    }
}

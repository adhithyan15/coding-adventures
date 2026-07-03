//! # differential — rank competing hypotheses (the MYCIN operation).
//!
//! [`lr_aggregate`](crate::lr_aggregate::lr_aggregate) answers **one**
//! question: "given the observed evidence, what is the posterior of
//! *this* conclusion?" It is single-hypothesis. But the canonical task
//! of MYCIN — and of any *differential* diagnosis, deal-vs-no-deal call,
//! or charge-selection — is comparative: **of these competing
//! hypotheses, which one, and how confidently over the runner-up?**
//!
//! Before this module the engine could score `acs`, `bacterial_meningitis`,
//! and `viral_meningitis` *independently* (three calls, three posteriors),
//! but nothing in the crate **ranked** them, computed the **margin** between
//! the leader and the runner-up, or told you whether that margin is
//! **robust** to the uncertainties still open in the case. That cross-
//! hypothesis decision is exactly the missing primitive a clinical
//! differential needs, and this module supplies it.
//!
//! ## What it computes
//!
//! Given a set of competing hypothesis terms (in adj-lang, the program's
//! `? h` query lines), [`differential`] runs LR aggregation for each,
//! sorts them by posterior log-odds, and emits a [`DifferentialDecision`]:
//!
//! - **`Determinate`** — the leader out-ranks the runner-up *even under the
//!   worst-case resolution of every open uncertainty* (the leader's VOI band
//!   pushed all the way down, the runner-up's all the way up). The argmax is
//!   safe to act on.
//! - **`Kickback`** — the leader's worst-case and the runner-up's best-case
//!   bands **cross**: an unresolved uncertainty could flip the ranking, so
//!   the framework recommends resolving it before committing. This is the
//!   cross-hypothesis analogue of
//!   [`LRAggregateResult::suggest_kickback`](crate::lr_aggregate::LRAggregateResult::suggest_kickback),
//!   which only looked *within* a single hypothesis.
//!
//! The decision is **argmax + sensitivity** — the ADJ65 uncertainty
//! primitive applied *between* hypotheses, not just inside one — and it is
//! deterministic and CPU-only: no softmax, no temperature, same input +
//! same KB ⇒ same ranking, byte-for-byte. Every ranked hypothesis carries
//! its full [`LRAggregateResult`] (proof DAG included), so the differential
//! is auditable end to end.
//!
//! ## Independence vs. mutual exclusivity
//!
//! The LR model scores each hypothesis from its *own* prior + likelihood
//! ratios — they are independent naive-Bayes posteriors, NOT a normalized
//! multinomial, because the rulebook never declared the hypotheses mutually
//! exclusive. We honour that: the **decision** uses the raw per-hypothesis
//! posteriors. We additionally report a `normalized_share` (posterior ÷ Σ
//! posteriors) as a *convenience* for a differential-style view, explicitly
//! flagged as assuming the listed hypotheses are exhaustive and mutually
//! exclusive — use it for display, not for the decision.

use logic_core::Term;

use crate::lr_aggregate::{lr_aggregate, LRAggregateResult};
use crate::KnowledgeBase;
use crate::UncertaintyMarkerId;

/// One hypothesis's place in the ranked differential, with its full
/// aggregation result (and therefore its proof DAG) preserved.
#[derive(Debug, Clone, PartialEq)]
pub struct RankedHypothesis {
    pub hypothesis: Term,
    /// `sigmoid(posterior_logit)` — the raw per-hypothesis posterior.
    pub posterior: f64,
    pub posterior_logit: f64,
    /// `posterior ÷ Σ posteriors` over the differential. CONVENIENCE
    /// ONLY: meaningful as a "share of belief" iff the hypotheses are
    /// exhaustive and mutually exclusive — which the LR model does not
    /// assume. Never used by the decision; provided for display.
    pub normalized_share: f64,
    /// The full single-hypothesis result, including the proof DAG, the
    /// warnings, and the active uncertainty reports.
    pub result: LRAggregateResult,
}

/// The comparative decision over a ranked differential.
#[derive(Debug, Clone, PartialEq)]
pub enum DifferentialDecision {
    /// No hypotheses were supplied.
    Empty,
    /// The leader out-ranks the runner-up robustly (or there is only one
    /// hypothesis). `margin_logit` is the log-odds gap to the runner-up
    /// (`f64::INFINITY` for a single hypothesis); `margin_posterior` is the
    /// gap in probability space.
    Determinate {
        leader: Term,
        posterior: f64,
        margin_posterior: f64,
        margin_logit: f64,
    },
    /// The leader's worst-case band and the runner-up's best-case band
    /// cross — an open uncertainty (or an exact tie) could flip the
    /// ranking. Resolve the recommended markers before committing.
    Kickback {
        leader: Term,
        runner_up: Term,
        margin_posterior: f64,
        margin_logit: f64,
        reason: String,
        /// Markers (across the leader and runner-up) ranked by VOI,
        /// largest first — resolve these to settle the ranking.
        recommended_resolutions: Vec<UncertaintyMarkerId>,
    },
}

/// A ranked differential plus its comparative decision.
#[derive(Debug, Clone, PartialEq)]
pub struct Differential {
    /// Hypotheses sorted by descending posterior log-odds (stable: ties
    /// keep the input order).
    pub ranked: Vec<RankedHypothesis>,
    pub decision: DifferentialDecision,
}

/// The `(lo_shift, hi_shift)` the active uncertainties could add to a
/// result's running log-odds — the worst-case (all markers resolve down)
/// and best-case (all resolve up) envelope. Mirrors the bounding in
/// [`LRAggregateResult::suggest_kickback`](crate::lr_aggregate::LRAggregateResult::suggest_kickback),
/// reused here to compare *across* hypotheses.
fn voi_band(result: &LRAggregateResult) -> (f64, f64) {
    let mut lo = 0.0;
    let mut hi = 0.0;
    for u in &result.uncertainties {
        if u.if_observed_logit_delta.is_empty() {
            continue;
        }
        let mn = u
            .if_observed_logit_delta
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let mx = u
            .if_observed_logit_delta
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        lo += mn;
        hi += mx;
    }
    (lo, hi)
}

/// Run a differential over `hypotheses` against `kb`.
///
/// Each hypothesis is scored with [`lr_aggregate`]; the results are sorted
/// by descending posterior log-odds (stable on ties), and the top two are
/// compared under their VOI bands to decide [`DifferentialDecision`].
///
/// **Complexity**: `O(H · (C + J·E))` — one linear LR aggregation per
/// hypothesis. No enumeration, no model calls.
pub fn differential(hypotheses: &[Term], kb: &KnowledgeBase) -> Differential {
    if hypotheses.is_empty() {
        return Differential {
            ranked: Vec::new(),
            decision: DifferentialDecision::Empty,
        };
    }

    // Score each hypothesis (preserve full results / proof DAGs).
    let mut scored: Vec<(Term, LRAggregateResult)> = hypotheses
        .iter()
        .map(|h| (h.clone(), lr_aggregate(h, kb)))
        .collect();

    // Stable sort by descending posterior log-odds. `sort_by` is stable in
    // Rust, so equal-logit hypotheses keep their input order — the decision
    // then flags a zero-margin leader as a Kickback rather than picking
    // arbitrarily.
    scored.sort_by(|a, b| {
        b.1.posterior_logit
            .partial_cmp(&a.1.posterior_logit)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let sum_posteriors: f64 = scored.iter().map(|(_, r)| r.posterior).sum();
    let ranked: Vec<RankedHypothesis> = scored
        .iter()
        .map(|(h, r)| RankedHypothesis {
            hypothesis: h.clone(),
            posterior: r.posterior,
            posterior_logit: r.posterior_logit,
            normalized_share: if sum_posteriors > 0.0 {
                r.posterior / sum_posteriors
            } else {
                0.0
            },
            result: r.clone(),
        })
        .collect();

    // Single hypothesis: nothing to compare against — determinate by
    // construction, infinite margin.
    if ranked.len() == 1 {
        let leader = &ranked[0];
        return Differential {
            decision: DifferentialDecision::Determinate {
                leader: leader.hypothesis.clone(),
                posterior: leader.posterior,
                margin_posterior: leader.posterior,
                margin_logit: f64::INFINITY,
            },
            ranked,
        };
    }

    let leader = &scored[0];
    let runner_up = &scored[1];
    let margin_logit = leader.1.posterior_logit - runner_up.1.posterior_logit;
    let margin_posterior = leader.1.posterior - runner_up.1.posterior;

    // Robustness: does the leader still win when its uncertainties resolve
    // as unfavourably as possible AND the runner-up's resolve as favourably
    // as possible? (A conservative outer envelope — each marker optimised
    // independently — so a `Determinate` here never over-claims.)
    let leader_worst = leader.1.posterior_logit + voi_band(&leader.1).0;
    let runner_best = runner_up.1.posterior_logit + voi_band(&runner_up.1).1;

    let decision = if margin_logit > 0.0 && leader_worst > runner_best {
        DifferentialDecision::Determinate {
            leader: leader.0.clone(),
            posterior: leader.1.posterior,
            margin_posterior,
            margin_logit,
        }
    } else {
        let reason = if margin_logit <= 0.0 {
            "top two hypotheses are tied on posterior log-odds".to_string()
        } else {
            "an unresolved uncertainty could lift the runner-up past the leader".to_string()
        };
        // Union of the two contenders' active-uncertainty markers, ranked
        // by VOI (largest range first).
        let mut markers: Vec<(UncertaintyMarkerId, f64)> = leader
            .1
            .uncertainties
            .iter()
            .chain(runner_up.1.uncertainties.iter())
            .map(|u| (u.marker_id, u.voi_logit_range))
            .collect();
        markers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        markers.dedup_by_key(|(id, _)| *id);
        DifferentialDecision::Kickback {
            leader: leader.0.clone(),
            runner_up: runner_up.0.clone(),
            margin_posterior,
            margin_logit,
            reason,
            recommended_resolutions: markers.into_iter().map(|(id, _)| id).collect(),
        }
    };

    Differential { ranked, decision }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lr_aggregate::{ContributionClause, PriorClause, UncertaintyMarker};
    use crate::{Fact, KnowledgeBase};
    use logic_core::{atom, compound};

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    /// A clear leader with no open uncertainty is Determinate, and the
    /// ranking + margin are reported.
    #[test]
    fn clear_leader_is_determinate() {
        let mut kb = KnowledgeBase::new();
        // bacterial: 0.5 prior + strong observed finding (LR 10) → logit 2.30
        kb.add_prior(PriorClause::from_probability(atom("bacterial"), 0.5))
            .unwrap();
        kb.add_contribution(ContributionClause::from_lr(
            atom("bacterial"),
            compound("csf", vec![atom("neutrophilic")]),
            10.0,
        ));
        // viral: 0.5 prior + weak observed finding (LR 2) → logit 0.69
        kb.add_prior(PriorClause::from_probability(atom("viral"), 0.5))
            .unwrap();
        kb.add_contribution(ContributionClause::from_lr(
            atom("viral"),
            compound("csf", vec![atom("neutrophilic")]),
            2.0,
        ));
        kb.add_fact(Fact::certain(compound("csf", vec![atom("neutrophilic")])));

        let diff = differential(&[atom("bacterial"), atom("viral")], &kb);
        assert_eq!(diff.ranked.len(), 2);
        assert_eq!(diff.ranked[0].hypothesis, atom("bacterial"));
        match diff.decision {
            DifferentialDecision::Determinate {
                leader,
                margin_logit,
                ..
            } => {
                assert_eq!(leader, atom("bacterial"));
                // logit gap ≈ ln(10) - ln(2) = ln(5) ≈ 1.609
                assert!(approx_eq(margin_logit, (5.0_f64).ln(), 1e-6));
            }
            other => panic!("expected Determinate, got {other:?}"),
        }
        // normalized_share sums to ~1 over the differential.
        let total: f64 = diff.ranked.iter().map(|r| r.normalized_share).sum();
        assert!(approx_eq(total, 1.0, 1e-9));
        // Each ranked hypothesis kept its proof DAG.
        assert!(diff.ranked.iter().all(|r| !r.result.dag.proofs.is_empty()));
    }

    /// When an unresolved uncertainty could lift the runner-up past the
    /// leader, the differential kicks back rather than committing.
    #[test]
    fn close_call_within_voi_kicks_back() {
        let mut kb = KnowledgeBase::new();
        // Leader bacterial: tiny edge (LR 1.2 observed) → logit ≈ 0.18
        kb.add_prior(PriorClause::from_probability(atom("bacterial"), 0.5))
            .unwrap();
        kb.add_contribution(ContributionClause::from_lr(
            atom("bacterial"),
            atom("seen"),
            1.2,
        ));
        kb.add_fact(Fact::certain(atom("seen")));
        // Leader has an OPEN uncertainty that could pull it DOWN a lot:
        // an unobserved finding with LR 0.2 (log ≈ -1.61).
        kb.add_contribution(ContributionClause::from_lr(
            atom("bacterial"),
            atom("ct_finding_absent"),
            0.2,
        ));
        kb.add_uncertainty_marker(UncertaintyMarker::new(
            atom("bacterial"),
            vec![atom("ct_finding_absent")],
        ));

        // Runner-up viral: just behind (prior only) but has an OPEN
        // uncertainty that could pull it UP a lot (LR 5, log ≈ 1.61).
        kb.add_prior(PriorClause::from_probability(atom("viral"), 0.5))
            .unwrap();
        kb.add_contribution(ContributionClause::from_lr(
            atom("viral"),
            atom("pcr_pos"),
            5.0,
        ));
        kb.add_uncertainty_marker(UncertaintyMarker::new(atom("viral"), vec![atom("pcr_pos")]));

        let diff = differential(&[atom("bacterial"), atom("viral")], &kb);
        // bacterial leads on point estimate...
        assert_eq!(diff.ranked[0].hypothesis, atom("bacterial"));
        // ...but the bands cross, so the engine kicks back.
        match diff.decision {
            DifferentialDecision::Kickback {
                leader,
                runner_up,
                recommended_resolutions,
                ..
            } => {
                assert_eq!(leader, atom("bacterial"));
                assert_eq!(runner_up, atom("viral"));
                assert!(!recommended_resolutions.is_empty());
            }
            other => panic!("expected Kickback, got {other:?}"),
        }
    }

    /// A single-hypothesis differential is determinate with infinite margin.
    #[test]
    fn single_hypothesis_is_determinate() {
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(atom("acs"), 0.3))
            .unwrap();
        let diff = differential(&[atom("acs")], &kb);
        match diff.decision {
            DifferentialDecision::Determinate {
                leader,
                margin_logit,
                ..
            } => {
                assert_eq!(leader, atom("acs"));
                assert!(margin_logit.is_infinite());
            }
            other => panic!("expected Determinate, got {other:?}"),
        }
    }

    /// No hypotheses → Empty.
    #[test]
    fn empty_differential() {
        let kb = KnowledgeBase::new();
        let diff = differential(&[], &kb);
        assert!(diff.ranked.is_empty());
        assert_eq!(diff.decision, DifferentialDecision::Empty);
    }

    /// An exact tie on posterior log-odds kicks back (the engine never
    /// silently picks one of two equal leaders).
    #[test]
    fn exact_tie_kicks_back() {
        let mut kb = KnowledgeBase::new();
        kb.add_prior(PriorClause::from_probability(atom("a"), 0.4))
            .unwrap();
        kb.add_prior(PriorClause::from_probability(atom("b"), 0.4))
            .unwrap();
        let diff = differential(&[atom("a"), atom("b")], &kb);
        assert!(matches!(
            diff.decision,
            DifferentialDecision::Kickback { .. }
        ));
    }
}

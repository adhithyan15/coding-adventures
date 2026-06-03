//! ADJ52 experiment runner — domain-agnostic, with a first-class
//! **counterfactual / value-of-information / kickback** panel.
//!
//! This is the ADJ51 runner plus the part of the thesis that ADJ51
//! specified but never shipped: the engine's `lr_aggregate` already
//! computes, for every active `uncertain { … }` marker whose domain
//! is unobserved, *what each value would contribute if observed*
//! (`UncertaintyReport`). The ADJ51 runner destructured that field as
//! `uncertainties: _` and threw it away. ADJ52 surfaces it as a
//! per-query panel:
//!
//!   - **Counterfactual sensitivity** — for each candidate value in an
//!     unresolved uncertainty's domain, the posterior the answer would
//!     move to *if that value were observed*, flagged when it flips the
//!     decision. This is the "if the biopsy comes back, malignancy
//!     confirms / TB drops to zero" output.
//!   - **Kickback** — when the plausible posterior band (best/worst
//!     case over resolving every uncertainty) straddles the decision
//!     threshold, the framework recommends *escalating* and lists the
//!     uncertainties to resolve, ranked by value-of-information. This
//!     is the resident calling the attending.
//!   - **Source disagreement** — when two cited sources assign
//!     different LRs to the same evidence, the posterior is sensitive
//!     to which authority you trust; surface it rather than hide it.
//!
//! The runner reads `<dir>/03-derived-rulebook.adj` +
//! `<dir>/04-vignette.adj` (same artifact layout as ADJ51), compiles
//! via adj-lang, and runs each query through `logic-engine`'s LR
//! aggregator. `ADJ52_DIR` selects the case directory; it defaults to
//! a bundled fixture that exercises the panel so a bare `cargo run`
//! demonstrates the feature.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use adj_lang::compile;
use logic_core::Term;
use logic_engine::{lr_aggregate, sigmoid, source_disagreements, DerivationOrigin};

const DECISION_THRESHOLD: f64 = 0.30;

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));

    let subdir =
        std::env::var("ADJ52_DIR").unwrap_or_else(|_| "fixtures/uncertainty-demo".to_string());
    let rulebook_path = manifest_dir.join(&subdir).join("03-derived-rulebook.adj");
    let vignette_path = manifest_dir.join(&subdir).join("04-vignette.adj");

    let rulebook = fs::read_to_string(&rulebook_path)
        .unwrap_or_else(|e| panic!("reading rulebook {}: {e}", rulebook_path.display()));
    let vignette = fs::read_to_string(&vignette_path)
        .unwrap_or_else(|e| panic!("reading vignette {}: {e}", vignette_path.display()));
    let combined = format!("{rulebook}\n{vignette}");

    // Mechanical parse of observe lines for the coverage check.
    let observed_terms: HashSet<String> = vignette
        .lines()
        .filter_map(|l| l.trim().strip_prefix("observe "))
        .map(|t| t.trim().to_string())
        .collect();

    println!("================================================================");
    println!("  ADJ52 — counterfactual / VOI / kickback pipeline run");
    println!("================================================================");
    println!();
    println!("Inputs:");
    println!("  rulebook: {}", rulebook_path.display());
    println!("  vignette: {}", vignette_path.display());
    println!("Observed facts in vignette: {}", observed_terms.len());
    println!();

    let lowered = match compile(&combined) {
        Ok(l) => l,
        Err(e) => {
            println!("COMPILE ERROR: {e:?}");
            std::process::exit(1);
        }
    };

    println!("Queries to answer: {}", lowered.queries.len());
    for q in &lowered.queries {
        println!("  ? {}", format_term(q));
    }
    println!();

    for (i, query) in lowered.queries.iter().enumerate() {
        println!("================================================================");
        println!("Query {}/{}: {}", i + 1, lowered.queries.len(), format_term(query));
        println!("================================================================");

        let result = lr_aggregate(query, &lowered.kb);

        println!();
        println!(
            "  Posterior:  P = {:.4}  ({:.1}%)   logodds = {:+.4}",
            result.posterior,
            result.posterior * 100.0,
            result.posterior_logit
        );

        // ---- Fired clauses + coverage (carried over from ADJ51) ----
        let proof = &result.dag.proofs[0];
        let mut matched_obs: HashSet<String> = HashSet::new();
        println!();
        println!("  Fired clauses:");
        let mut shown = 0usize;
        for step in &proof.steps {
            match &step.origin {
                DerivationOrigin::FromPrior {
                    clause_id,
                    prior_logit,
                } => {
                    if let Some(p) = lowered.kb.prior_for(query).filter(|p| p.id == *clause_id) {
                        println!(
                            "    prior  {:+.4}                                source: {}",
                            prior_logit,
                            truncate(&p.provenance.source, 60)
                        );
                        shown += 1;
                    }
                }
                DerivationOrigin::FromContribution {
                    clause_id,
                    logit_delta,
                    ..
                } => {
                    if let Some(c) = lowered
                        .kb
                        .contributions_for(query)
                        .into_iter()
                        .find(|c| c.id == *clause_id)
                    {
                        let ev = format_term(&c.evidence_term);
                        matched_obs.insert(ev.clone());
                        println!(
                            "    {:+.4}  {:42}   src: {}",
                            logit_delta,
                            truncate(&ev, 42),
                            truncate(&c.provenance.source, 40)
                        );
                        shown += 1;
                    }
                }
                DerivationOrigin::FromJointContribution {
                    clause_id,
                    joint_logit_delta,
                    ..
                } => {
                    if let Some(j) = lowered
                        .kb
                        .joint_contributions_for(query)
                        .into_iter()
                        .find(|j| j.id == *clause_id)
                    {
                        let evs: Vec<String> = j.evidence_set.iter().map(format_term).collect();
                        for ev in &evs {
                            matched_obs.insert(ev.clone());
                        }
                        println!(
                            "    {:+.4}  [joint x{}: {}]",
                            joint_logit_delta,
                            evs.len(),
                            truncate(&evs.join(" + "), 80)
                        );
                        shown += 1;
                    }
                }
                _ => {}
            }
        }
        if shown == 0 {
            println!("    (no clauses fired — engine had no rule that matched any observation)");
        }

        let unmatched: Vec<&String> = observed_terms
            .iter()
            .filter(|t| !matched_obs.contains(*t))
            .collect();
        println!();
        println!(
            "  Coverage: {}/{} observed terms matched a clause for this query.",
            observed_terms.len() - unmatched.len(),
            observed_terms.len()
        );

        println!();
        let decision = if result.posterior >= DECISION_THRESHOLD {
            "ABOVE threshold"
        } else {
            "BELOW threshold"
        };
        println!(
            "  Decision at {:.0}%: {}",
            DECISION_THRESHOLD * 100.0,
            decision
        );

        // ---- ADJ52: counterfactual / VOI panel ----
        if !result.uncertainties.is_empty() {
            println!();
            println!("  Counterfactual sensitivity (what would shift this answer):");
            for u in &result.uncertainties {
                let domain_str = u
                    .domain
                    .iter()
                    .map(format_term)
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "    uncertainty: {} over {{ {} }}   VOI range = {:.4} logits",
                    format_term(&u.conclusion),
                    truncate(&domain_str, 60),
                    u.voi_logit_range
                );
                for (val, delta) in u.domain.iter().zip(u.if_observed_logit_delta.iter()) {
                    let p_if = sigmoid(result.posterior_logit + delta);
                    let flips =
                        (result.posterior >= DECISION_THRESHOLD) != (p_if >= DECISION_THRESHOLD);
                    println!(
                        "      if observe {:40}  P {:.4} -> {:.4}{}",
                        truncate(&format_term(val), 40),
                        result.posterior,
                        p_if,
                        if flips { "   <- flips decision" } else { "" }
                    );
                }
            }

            // ---- Kickback: escalate when the band straddles the line ----
            match result.suggest_kickback(DECISION_THRESHOLD) {
                Some(kb) => {
                    println!();
                    println!(
                        "  KICKBACK: plausible posterior band [{:.4}, {:.4}] straddles the {:.0}% threshold -> ESCALATE",
                        kb.posterior_lo,
                        kb.posterior_hi,
                        DECISION_THRESHOLD * 100.0
                    );
                    println!("    resolve these (highest value-of-information first):");
                    for mid in &kb.recommended_resolutions {
                        if let Some(u) = result.uncertainties.iter().find(|u| u.marker_id == *mid) {
                            let domain_str = u
                                .domain
                                .iter()
                                .map(format_term)
                                .collect::<Vec<_>>()
                                .join(", ");
                            println!(
                                "      - {}  (domain {{ {} }})",
                                format_term(&u.conclusion),
                                truncate(&domain_str, 60)
                            );
                        }
                    }
                }
                None => {
                    println!();
                    println!(
                        "  No kickback: resolving the open uncertainties would not move the {:.0}% decision.",
                        DECISION_THRESHOLD * 100.0
                    );
                }
            }
        }

        // ---- Source disagreement: sensitivity to which authority you trust ----
        let disagreements = source_disagreements(&lowered.kb, query);
        if !disagreements.is_empty() {
            println!();
            println!("  Source disagreement (rulebook sources disagree on an LR):");
            for d in &disagreements {
                println!(
                    "    evidence {}: {:.4} logits of spread across {} sources",
                    format_term(&d.evidence_term),
                    d.disagreement_logit_range,
                    d.source_logit_deltas.len()
                );
            }
        }

        if !result.warnings.is_empty() {
            println!();
            println!("  Engine warnings:");
            for w in &result.warnings {
                println!("    {w:?}");
            }
        }
        println!();
    }
}

fn format_term(t: &Term) -> String {
    match t {
        Term::Atom(s) => s.clone(),
        Term::Compound { functor, args } => format!(
            "{}({})",
            functor,
            args.iter().map(format_term).collect::<Vec<_>>().join(", ")
        ),
        other => format!("{other:?}"),
    }
}

/// Truncate to at most `n` characters, char-boundary-safe. Slicing a
/// `str` by byte offset panics if the cut lands inside a multibyte
/// UTF-8 sequence — and rulebook source / citations are arbitrary
/// UTF-8 (accents, em-dashes, CJK). Counting by `chars()` avoids that
/// entirely.
fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let truncated: String = s.chars().take(n.saturating_sub(3)).collect();
    format!("{truncated}...")
}

//! # ADJ49 — M&A deal-completion demo runner.
//!
//! Same architecture as ADJ48 but applied to a Delaware M&A
//! rulebook and three deal vignettes. Decision threshold is 0.50
//! (the conventional "would you pitch this to the IC" gate for a
//! live deal in a sell-side pipeline).
//!
//! Output per case includes:
//! - Posterior P(deal_completes_by_drop_dead).
//! - Every fired contribution with its citation.
//! - Joint interaction terms that fired.
//! - Active uncertainty reports + VOI.
//! - Kickback recommendation at 50% threshold.
//! - Counterfactual: "what if the load-bearing uncertainty resolves
//!   favorably?" computed for the highest-VOI uncertainty.

use std::fs;
use std::path::{Path, PathBuf};

use adj_lang::compile;
use logic_engine::{
    counterfactual, search, source_disagreements, DerivationOrigin, SearchMode, SearchResult,
};
use logic_core::Term;

const DECISION_THRESHOLD: f64 = 0.50;

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let rulebook =
        fs::read_to_string(manifest_dir.join("rulebook.adj")).expect("reading rulebook.adj");
    let mut vignettes: Vec<PathBuf> = fs::read_dir(manifest_dir.join("vignettes"))
        .expect("reading vignettes/")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("adj"))
        .collect();
    vignettes.sort();

    println!("================================================================");
    println!("  ADJ49 — M&A deal completion: investment-banker objection demo");
    println!("================================================================");
    println!();
    println!(
        "Question:       P(deal_completes_by_drop_dead) — does this deal close before parties walk?"
    );
    println!("Rulebook:       rulebook.adj ({} bytes)", rulebook.len());
    println!(
        "Threshold:      {:.0}% — \"would IC approve pursuing on these odds?\"",
        DECISION_THRESHOLD * 100.0
    );
    println!();

    for vpath in &vignettes {
        run_vignette(&rulebook, vpath);
    }

    println!("================================================================");
    println!("  The investment-banker argument");
    println!("================================================================");
    println!("Status-quo AI: a prose memo with no per-claim provenance.");
    println!("The analyst re-reads everything and re-runs every model. No");
    println!("verification leverage. AI saves drafting time but not");
    println!("verification time. Net: marginal.");
    println!();
    println!("Framework output: every claim above is a citation. Every");
    println!("derived posterior is reproducible by running this binary.");
    println!("The analyst verifies inputs once, trusts the engine, and");
    println!("spends saved time on the cases where kickback flagged a");
    println!("decision-relevant uncertainty. That is the productivity");
    println!("delta.");
}

fn run_vignette(rulebook: &str, vpath: &Path) {
    let vignette_src = fs::read_to_string(vpath).expect("reading vignette");
    let combined = format!("{rulebook}\n{vignette_src}");
    let label = vpath.file_stem().unwrap().to_string_lossy();

    println!("----------------------------------------------------------------");
    println!("  Deal: {label}");
    println!("----------------------------------------------------------------");
    print_vignette_header(&vignette_src);

    let lowered = match compile(&combined) {
        Ok(l) => l,
        Err(e) => {
            println!("COMPILE ERROR: {e:?}");
            return;
        }
    };

    if lowered.queries.is_empty() {
        println!("(vignette has no `?` query)");
        return;
    }
    let query = &lowered.queries[0];
    let result = search(query, &lowered.kb, SearchMode::LRAggregate);
    let SearchResult::LRAggregateResult {
        dag,
        posterior,
        posterior_logit,
        warnings,
        uncertainties,
    } = result
    else {
        return;
    };

    println!(
        "Posterior:  P(deal_completes_by_drop_dead) = {:.4}  (logodds = {:+.4})",
        posterior, posterior_logit
    );
    println!();

    let proof = &dag.proofs[0];
    println!("Fired clauses:");
    for step in &proof.steps {
        match &step.origin {
            DerivationOrigin::FromPrior {
                clause_id,
                prior_logit,
            } => {
                if let Some(p) = lowered.kb.prior_for(query).filter(|p| p.id == *clause_id) {
                    println!("  prior {:+.4}    source: {}", prior_logit, p.provenance.source);
                }
            }
            DerivationOrigin::FromContribution {
                clause_id,
                logit_delta,
                ..
            } => {
                let c = lowered
                    .kb
                    .contributions_for(query)
                    .into_iter()
                    .find(|c| c.id == *clause_id);
                if let Some(c) = c {
                    println!(
                        "  {:+.4}      [{}]  source: {}",
                        logit_delta,
                        format_term(&c.evidence_term),
                        c.provenance.source
                    );
                }
            }
            DerivationOrigin::FromJointContribution {
                clause_id,
                joint_logit_delta,
                ..
            } => {
                let j = lowered
                    .kb
                    .joint_contributions_for(query)
                    .into_iter()
                    .find(|j| j.id == *clause_id);
                if let Some(j) = j {
                    println!(
                        "  {:+.4}      [joint, {} terms]  source: {}",
                        joint_logit_delta,
                        j.evidence_set.len(),
                        j.provenance.source
                    );
                }
            }
            _ => {}
        }
    }

    if !uncertainties.is_empty() {
        println!();
        println!("Active uncertainties (VOI ranked):");
        let mut ranked: Vec<_> = uncertainties.iter().collect();
        ranked.sort_by(|a, b| {
            b.voi_logit_range
                .partial_cmp(&a.voi_logit_range)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for u in &ranked {
            println!(
                "  voi = {:+.4} logits over {} candidates",
                u.voi_logit_range,
                u.domain.len()
            );
            for (term, delta) in u.domain.iter().zip(u.if_observed_logit_delta.iter()) {
                println!("      {:>+8.4}   if observed: {}", delta, format_term(term));
            }
        }
    }

    let lr = logic_engine::LRAggregateResult {
        dag: dag.clone(),
        posterior,
        posterior_logit,
        warnings: warnings.clone(),
        uncertainties: uncertainties.clone(),
    };

    if let Some(kickback) = lr.suggest_kickback(DECISION_THRESHOLD) {
        println!();
        println!("KICKBACK RECOMMENDED:");
        println!(
            "  At {:.0}% threshold, posterior {:.4} is decision-sensitive.",
            kickback.decision_threshold * 100.0,
            kickback.posterior
        );
        println!(
            "  Plausible band: [{:.4}, {:.4}].",
            kickback.posterior_lo, kickback.posterior_hi
        );
        println!(
            "  Resolve {} uncertainty markers before committing.",
            kickback.recommended_resolutions.len()
        );
    } else if !uncertainties.is_empty() {
        println!();
        println!(
            "No kickback at {:.0}% — uncertainties exist but the decision is robust.",
            DECISION_THRESHOLD * 100.0
        );
    }

    // Counterfactual: take the highest-VOI uncertainty's most
    // favorable resolution and re-run the aggregation.
    if !uncertainties.is_empty() {
        let best_u = uncertainties
            .iter()
            .max_by(|a, b| {
                a.voi_logit_range
                    .partial_cmp(&b.voi_logit_range)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        // Find the index of the maximum log-odds delta in the domain.
        let (best_idx, _) = best_u
            .if_observed_logit_delta
            .iter()
            .enumerate()
            .max_by(|a, b| {
                a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        let assumed = best_u.domain[best_idx].clone();
        let cf = counterfactual(query, &lowered.kb, std::slice::from_ref(&assumed));
        println!();
        println!("Counterfactual (most favorable resolution):");
        println!(
            "  if {} were observed, posterior → {:.4} (Δ = {:+.4})",
            format_term(&assumed),
            cf.posterior,
            cf.posterior - posterior
        );
    }

    let disagreements = source_disagreements(&lowered.kb, query);
    if !disagreements.is_empty() {
        println!();
        println!("Source disagreements in rulebook:");
        for d in &disagreements {
            println!(
                "  {}: {} sources disagree by {:+.4} logits",
                format_term(&d.evidence_term),
                d.source_logit_deltas.len(),
                d.disagreement_logit_range
            );
        }
    }

    if !warnings.is_empty() {
        println!();
        println!("Warnings:");
        for w in &warnings {
            println!("  {w:?}");
        }
    }
    println!();
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

fn print_vignette_header(src: &str) {
    let mut header = String::new();
    for line in src.lines() {
        let t = line.trim_start();
        if t.starts_with('%') {
            header.push_str(t.trim_start_matches('%').trim_start());
            header.push('\n');
        } else if t.is_empty() && header.is_empty() {
            // Skip leading blanks — the header has not started yet.
        } else {
            // Anything else ends the header: the first non-comment line, or the
            // blank line that terminates a header already in progress. Those two
            // used to be separate arms, but both did the same `break`, so they
            // are one.
            break;
        }
    }
    if !header.is_empty() {
        for line in header.trim().lines() {
            println!("  {line}");
        }
        println!();
    }
}

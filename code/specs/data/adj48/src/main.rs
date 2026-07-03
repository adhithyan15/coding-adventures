//! # ADJ48 — MYCIN-2026 ACS demo runner.
//!
//! Concatenates each vignette with the shared rulebook, compiles
//! through `adj-lang`, runs LR aggregation, and emits a per-case
//! audit document containing:
//!
//! - Posterior P(acs) plus log-odds.
//! - Fired contributions with their citations and individual
//!   log-odds deltas.
//! - Joint interaction terms that fired.
//! - Active uncertainty reports (domain + per-value VOI).
//! - Kickback recommendation at a configurable decision threshold
//!   (30% by default — the conventional admit-vs-discharge gate
//!   for ED chest pain in low-resource settings).
//!
//! The point is not the absolute numbers; it's that every claim in
//! every line of output is grounded in a citation the user can
//! click and verify.

use std::fs;
use std::path::{Path, PathBuf};

use adj_lang::compile;
use logic_engine::{
    search, source_disagreements, DerivationOrigin, SearchMode, SearchResult,
};

const DECISION_THRESHOLD: f64 = 0.30;

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
    println!("  ADJ48 — MYCIN-2026 ACS demo");
    println!("================================================================");
    println!();
    println!(
        "Rulebook: rulebook.adj ({} bytes)",
        rulebook.len()
    );
    println!("Decision threshold: {:.0}% (admit / discharge gate)", DECISION_THRESHOLD * 100.0);
    println!();

    for vpath in &vignettes {
        run_vignette(&rulebook, vpath);
    }

    println!("================================================================");
    println!("  Summary");
    println!("================================================================");
    println!(
        "Vignettes run: {}.  Rulebook is the single source of truth; every",
        vignettes.len()
    );
    println!("contribution in every audit document above is reachable via the");
    println!("source citation. To question any claim, click the citation.");
}

fn run_vignette(rulebook: &str, vpath: &Path) {
    let vignette_src = fs::read_to_string(vpath).expect("reading vignette");
    let combined = format!("{rulebook}\n{vignette_src}");
    let label = vpath.file_stem().unwrap().to_string_lossy();

    println!("----------------------------------------------------------------");
    println!("  Case: {label}");
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
        println!("(vignette has no `?` query — skipping)");
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
        println!("ERROR: expected LR aggregation; got something else");
        return;
    };

    println!("Posterior:  P(acs) = {:.4}  (logodds = {:+.4})", posterior, posterior_logit);
    println!();

    // Walk the proof DAG and print every fired step with its citation.
    let proof = &dag.proofs[0];
    println!("Fired clauses:");
    for step in &proof.steps {
        match &step.origin {
            DerivationOrigin::FromPrior {
                clause_id,
                prior_logit,
            } => {
                let prior = lowered
                    .kb
                    .prior_for(query)
                    .filter(|p| p.id == *clause_id);
                if let Some(p) = prior {
                    println!(
                        "  prior {:+.4}    source: {}",
                        prior_logit, p.provenance.source
                    );
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
                        "  {:+.4}      [{:?}]  source: {}",
                        logit_delta,
                        c.evidence_term,
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
                println!("      {:>+8.4}   if observed: {:?}", delta, term);
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
            "  At decision threshold {:.0}%, the posterior {:.4} is sensitive to",
            kickback.decision_threshold * 100.0,
            kickback.posterior
        );
        println!(
            "  unresolved uncertainty. Plausible band: [{:.4}, {:.4}].",
            kickback.posterior_lo, kickback.posterior_hi
        );
        println!(
            "  Recommend resolving {} uncertainties before committing.",
            kickback.recommended_resolutions.len()
        );
    } else if !uncertainties.is_empty() {
        println!();
        println!(
            "No kickback at threshold {:.0}% — uncertainty does not change the side of the decision.",
            DECISION_THRESHOLD * 100.0
        );
    }

    // Surface any source disagreements in the rulebook for this query.
    let disagreements = source_disagreements(&lowered.kb, query);
    if !disagreements.is_empty() {
        println!();
        println!("Source disagreements in rulebook (for this conclusion):");
        for d in &disagreements {
            println!(
                "  {:?}: {} sources spread by {:+.4} logits",
                d.evidence_term,
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

fn print_vignette_header(src: &str) {
    // Print the lead-in comment block from the vignette (everything
    // up to the first non-comment line). Lets the audit document
    // include the case description verbatim.
    let mut header = String::new();
    for line in src.lines() {
        let t = line.trim_start();
        if t.starts_with('%') {
            header.push_str(t.trim_start_matches('%').trim_start());
            header.push('\n');
        } else if t.is_empty() && header.is_empty() {
            // skip leading blanks
        } else if t.is_empty() && !header.is_empty() {
            break;
        } else {
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

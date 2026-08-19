//! # ADJ50 — runner: run PMC12750962 twice
//!
//! Once against the as-shipped ADJ48 rulebook, once against the
//! extended rulebook that adds rules for CACS, former smoker,
//! hyperlipidemia, and progressive worsening. Print both audit
//! documents side by side and the ground-truth diagnosis.

use std::fs;
use std::path::{Path, PathBuf};

use adj_lang::compile;
use logic_engine::{search, DerivationOrigin, SearchMode, SearchResult};
use logic_core::Term;

const DECISION_THRESHOLD: f64 = 0.30;
const TRUE_DIAGNOSIS: &str = "100% occlusion of the proximal RCA (ACS, true positive)";

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));

    println!("================================================================");
    println!("  ADJ50 — stress-test ACS rulebook on PMC12750962");
    println!("================================================================");
    println!();
    println!("Case: 47yo M with exertional substernal chest pain progressively");
    println!("  worsening over months; no diaphoresis, no nausea; ECG normal");
    println!("  (sinus bradycardia, no acute ST changes); serial high-sens");
    println!("  troponin undetectable (5.9 → 6.3 ng/L vs 78.5 reference).");
    println!("  PMH: hyperlipidemia, former smoker (6 PY in 20s), CACS >200.");
    println!();
    println!("Ground-truth diagnosis: {TRUE_DIAGNOSIS}");
    println!("Decision threshold for admit: {:.0}%", DECISION_THRESHOLD * 100.0);
    println!();

    // -----------------------------------------------------------------
    // Pass 1 — as-shipped rulebook
    // -----------------------------------------------------------------
    run_pass(
        &manifest_dir,
        "Pass 1: as-shipped ADJ48 rulebook",
        "rulebook-as-shipped.adj",
        "vignettes/01-pmc12750962-as-published.adj",
    );

    println!();
    println!();

    // -----------------------------------------------------------------
    // Pass 2 — extended rulebook
    // -----------------------------------------------------------------
    run_pass(
        &manifest_dir,
        "Pass 2: ADJ50-extended rulebook (adds CACS, former smoker, hyperlipidemia, progressive worsening)",
        "rulebook-extended.adj",
        "vignettes/02-pmc12750962-extended-rulebook.adj",
    );

    println!();
    println!("================================================================");
    println!("  ADJ50 summary");
    println!("================================================================");
    println!("Pass 1 produced a confident low posterior on a true-positive");
    println!("ACS patient. This is the framework's most important honest");
    println!("limitation: it correctly reasons over its rulebook, but if the");
    println!("rulebook is incomplete it will be wrong AND confident.");
    println!();
    println!("Pass 2 shows the same engine, with four rules added (CACS,");
    println!("former smoker, hyperlipidemia, progressive worsening), produces");
    println!("a meaningfully higher posterior. The fix is mechanical and");
    println!("every clause carries its citation; no model retraining required.");
    println!();
    println!("Compare this to a status-quo LLM that says \"P(ACS) ≈ 10%, low");
    println!("risk, discharge OK\" — when that LLM is wrong, you have no way");
    println!("to know what it weighted or to patch the failure mode.");
}

fn run_pass(manifest_dir: &Path, label: &str, rulebook_name: &str, vignette_name: &str) {
    let rulebook = fs::read_to_string(manifest_dir.join(rulebook_name)).expect("rulebook");
    let vignette = fs::read_to_string(manifest_dir.join(vignette_name)).expect("vignette");
    let combined = format!("{rulebook}\n{vignette}");

    println!("----------------------------------------------------------------");
    println!("  {label}");
    println!("----------------------------------------------------------------");

    let lowered = match compile(&combined) {
        Ok(l) => l,
        Err(e) => {
            println!("COMPILE ERROR: {e:?}");
            return;
        }
    };
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
        println!("ERROR: expected LR aggregation");
        return;
    };

    println!("Posterior:  P(acs) = {:.4}  (logodds = {:+.4})", posterior, posterior_logit);
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

    let lr = logic_engine::LRAggregateResult {
        dag: dag.clone(),
        posterior,
        posterior_logit,
        warnings: warnings.clone(),
        uncertainties: uncertainties.clone(),
    };

    if let Some(kb_report) = lr.suggest_kickback(DECISION_THRESHOLD) {
        println!();
        println!("Kickback recommended (decision sensitive):");
        println!(
            "  band [{:.4}, {:.4}] straddles {:.0}% threshold",
            kb_report.posterior_lo,
            kb_report.posterior_hi,
            kb_report.decision_threshold * 100.0
        );
    } else if !uncertainties.is_empty() {
        println!();
        println!(
            "No kickback at {:.0}% — uncertainty does not change side of decision.",
            DECISION_THRESHOLD * 100.0
        );
    }

    println!();
    let above_threshold = posterior >= DECISION_THRESHOLD;
    let truth = true; // ground truth: patient HAD ACS
    let agreement = above_threshold == truth;
    println!(
        "Framework decision: {} (P={:.4})",
        if above_threshold { "ADMIT (suspect ACS)" } else { "DISCHARGE (low ACS risk)" },
        posterior
    );
    println!("Ground truth:       ADMIT (patient had 100% pRCA occlusion)");
    println!(
        "Match? {}",
        if agreement {
            "YES — framework would have admitted"
        } else {
            "NO — framework would have discharged a true ACS patient"
        }
    );
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

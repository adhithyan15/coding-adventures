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

    // Rulebook / program separation. A rulebook (accumulating, reusable across
    // cases) and a program (one case's observations + queries) can be supplied
    // independently via ADJ52_RULEBOOK + ADJ52_PROGRAM, so programs swap in and
    // out against an accumulating rulebook. Both may be absolute or relative to
    // the crate manifest dir. Falls back to the single-case ADJ52_DIR layout
    // (03-derived-rulebook.adj + 04-vignette.adj) when the pair is not set.
    let (rulebook_path, vignette_path) =
        match (std::env::var("ADJ52_RULEBOOK"), std::env::var("ADJ52_PROGRAM")) {
            (Ok(rb), Ok(pg)) => (manifest_dir.join(rb), manifest_dir.join(pg)),
            _ => {
                let subdir = std::env::var("ADJ52_DIR")
                    .unwrap_or_else(|_| "fixtures/uncertainty-demo".to_string());
                (
                    manifest_dir.join(&subdir).join("03-derived-rulebook.adj"),
                    manifest_dir.join(&subdir).join("04-vignette.adj"),
                )
            }
        };

    let rulebook = fs::read_to_string(&rulebook_path)
        .unwrap_or_else(|e| panic!("reading rulebook {}: {e}", rulebook_path.display()));
    let vignette = fs::read_to_string(&vignette_path)
        .unwrap_or_else(|e| panic!("reading vignette {}: {e}", vignette_path.display()));
    let mut combined = format!("{rulebook}\n{vignette}");

    // Mechanical parse of observe lines for the coverage check.
    let observed_terms: HashSet<String> = vignette
        .lines()
        .filter_map(|l| l.trim().strip_prefix("observe "))
        .map(|t| t.trim().to_string())
        .collect();

    // ---- ADJ53: latent-mechanism construct (fixes Naive-Bayes over-counting) ----
    // A `% mechanism <M> for <C> lr <L> : <m1>, <m2>, ...` directive declares that
    // the manifestations m1..mk are correlated effects of ONE latent cause M.
    // Rather than summing an independent `contributes` for each (which double-counts
    // correlated evidence and saturates the posterior), the mechanism fires ONCE:
    // if >=1 manifestation is observed, a single synthetic
    // `contributes L from mechanism_present(M) to C` updates the conclusion. We
    // realize it by generating adj-lang that the normal compiler/engine then
    // handles — no engine change. (Phase A home of the ADJ53 `mechanism` construct,
    // ahead of promoting the surface syntax into the adj-lang core grammar.)
    let mechanisms = parse_mechanisms(&combined);
    let mut fired_note: Vec<String> = Vec::new();
    for m in &mechanisms {
        combined.push_str(&format!(
            "\ncontributes {} from mechanism_present({}) to {}\n  source \"ADJ53 latent mechanism\" trust inferred\n",
            m.lr, m.name, m.conclusion
        ));
        let fired = m
            .manifestations
            .iter()
            .any(|man| observed_terms.iter().any(|o| ws_strip(o) == ws_strip(man)));
        if fired {
            combined.push_str(&format!("observe mechanism_present({})\n", m.name));
        }
        fired_note.push(format!(
            "  {} -> {}  (lr {}, {} of {} manifestations observed -> {})",
            m.name,
            m.conclusion,
            m.lr,
            m.manifestations
                .iter()
                .filter(|man| observed_terms.iter().any(|o| ws_strip(o) == ws_strip(man)))
                .count(),
            m.manifestations.len(),
            if fired { "FIRES ONCE" } else { "silent" }
        ));
    }

    println!("================================================================");
    println!("  ADJ52 — counterfactual / VOI / kickback pipeline run");
    println!("================================================================");
    println!();
    println!("Inputs:");
    println!("  rulebook: {}", rulebook_path.display());
    println!("  vignette: {}", vignette_path.display());
    println!("Observed facts in vignette: {}", observed_terms.len());
    if !fired_note.is_empty() {
        println!("Latent mechanisms (ADJ53 — correlated findings counted once):");
        for n in &fired_note {
            println!("{n}");
        }
    }
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

        // ---- ADJ54 H2: open-question discounting (hold residual uncertainty) ----
        // The engine should not report saturated certainty while a decision-relevant
        // confirmatory test bearing on THIS conclusion is still unobserved — the test
        // could still go either way. We build a VOI band over the open uncertainty's
        // outcomes and report a tempered "could-go-either-way" midpoint as the
        // *calibrated* confidence.
        //
        // ANTI-ENTROPY GUARANTEE: this NEVER reorders the differential. The RAW
        // posterior (`P = …`, evidence as observed) is what callers rank on; the
        // tempered value is reporting-only. So H2 can improve calibration but cannot
        // flip a top-1 — by construction it introduces zero correctness regression.
        // (The companion scorer ranks on RAW, calibrates on REPORTED.)
        let open_on_conclusion: Vec<&_> = result
            .uncertainties
            .iter()
            .filter(|u| &u.conclusion == query)
            .collect();
        let (reported_posterior, band) = if open_on_conclusion.is_empty() {
            (result.posterior, None)
        } else {
            // candidate posteriors: the current (evidence-as-observed) one, plus what
            // each unobserved outcome of the bearing uncertainty would move us to.
            let mut ps = vec![result.posterior];
            for u in &open_on_conclusion {
                for delta in &u.if_observed_logit_delta {
                    ps.push(sigmoid(result.posterior_logit + delta));
                }
            }
            let lo = ps.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = ps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (0.5 * (lo + hi), Some((lo, hi))) // midpoint = "the test could go either way"
        };

        println!();
        println!(
            "  Posterior:  P = {:.4}  ({:.1}%)   logodds = {:+.4}",
            result.posterior,
            result.posterior * 100.0,
            result.posterior_logit
        );
        match band {
            Some((lo, hi)) => println!(
                "  Reported (H2 open-question discounted):  P = {:.4}   band [{:.4}, {:.4}]   <- residual held: a recommended confirmatory test for this conclusion is still open",
                reported_posterior, lo, hi
            ),
            None => println!(
                "  Reported (H2 open-question discounted):  P = {:.4}   (no open confirmatory uncertainty bears on this conclusion)",
                reported_posterior
            ),
        }

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

/// One `% mechanism …` directive: a latent cause `name` bearing on `conclusion`
/// with a single likelihood ratio `lr`, whose `manifestations` are its correlated
/// observable effects.
struct MechanismDecl {
    name: String,
    conclusion: String,
    lr: f64,
    manifestations: Vec<String>,
}

fn ws_strip(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Parse `% mechanism <M> for <conclusion> lr <L> : <m1>, <m2>, ...` directives.
/// adj-lang ignores `%` comments, so the runner reads them and realizes the
/// latent-node semantics by generating synthetic adj-lang. This is the ADJ52-
/// runner home of the ADJ53 `mechanism` construct ahead of promoting the surface
/// syntax into the adj-lang core grammar.
fn parse_mechanisms(text: &str) -> Vec<MechanismDecl> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        let Some(rest) = t
            .strip_prefix("% mechanism")
            .or_else(|| t.strip_prefix("%mechanism"))
        else {
            continue;
        };
        let Some((name, after_for)) = rest.split_once(" for ") else {
            continue;
        };
        let Some((conclusion, after_lr)) = after_for.split_once(" lr ") else {
            continue;
        };
        let Some((lr_s, mani)) = after_lr.split_once(':') else {
            continue;
        };
        let Ok(lr) = lr_s.trim().parse::<f64>() else {
            continue;
        };
        let manifestations: Vec<String> = mani
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if manifestations.is_empty() {
            continue;
        }
        out.push(MechanismDecl {
            name: name.trim().to_string(),
            conclusion: conclusion.trim().to_string(),
            lr,
            manifestations,
        });
    }
    out
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

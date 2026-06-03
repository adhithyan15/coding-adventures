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

    // ADJ52 calibration: mutually-exclusive conclusions declared by the deriver
    // via `% @exclusive a, b, c` directive comments are normalized into a
    // coherent differential after the per-query loop. Collect each query's
    // posterior log-odds as we go.
    let exclusive_groups = parse_exclusive_groups(&combined);
    let mut query_logodds: Vec<(String, f64)> = Vec::new();

    for (i, query) in lowered.queries.iter().enumerate() {
        println!("================================================================");
        println!("Query {}/{}: {}", i + 1, lowered.queries.len(), format_term(query));
        println!("================================================================");

        let result = lr_aggregate(query, &lowered.kb);
        query_logodds.push((format_term(query), result.posterior_logit));

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

    print_coherent_differential(&exclusive_groups, &query_logodds);
}

/// Whitespace-stripped key so directive members match `format_term` output even
/// when one side has spaces (e.g. multi-arg compounds rendered as `f(a, b)`).
fn ws_key(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Parse `% @exclusive a, b, c` directive comment lines into groups of member
/// term strings. adj-lang ignores `%` comments, so the runner reads them from
/// the raw text to learn which conclusions the deriver declared mutually
/// exclusive. A group needs >= 2 members to be meaningful.
fn parse_exclusive_groups(text: &str) -> Vec<Vec<String>> {
    let mut groups = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        let body = t
            .strip_prefix("% @exclusive")
            .or_else(|| t.strip_prefix("%@exclusive"));
        if let Some(rest) = body {
            let members: Vec<String> = rest
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if members.len() >= 2 {
                groups.push(members);
            }
        }
    }
    groups
}

/// Softmax each exclusive group's member log-odds into a coherent distribution
/// summing to 1. This competes the hypotheses and tempers the per-hypothesis
/// saturation that made independent posteriors read as several simultaneous
/// ~100%s. Conclusions in no group (e.g. "coexisting" hypotheses) are listed
/// alongside with their raw independent posterior.
fn print_coherent_differential(groups: &[Vec<String>], query_logodds: &[(String, f64)]) {
    if groups.is_empty() {
        return;
    }
    println!("================================================================");
    println!("  Coherent differential (softmax-normalized within exclusive groups)");
    println!("================================================================");

    let mut grouped_keys: HashSet<String> = HashSet::new();
    for (gi, group) in groups.iter().enumerate() {
        let mut members: Vec<(String, f64)> = Vec::new();
        for m in group {
            grouped_keys.insert(ws_key(m));
            if let Some((t, l)) = query_logodds.iter().find(|(t, _)| ws_key(t) == ws_key(m)) {
                members.push((t.clone(), *l));
            }
        }
        if members.is_empty() {
            continue;
        }
        let max = members
            .iter()
            .map(|(_, l)| *l)
            .fold(f64::NEG_INFINITY, f64::max);
        let exps: Vec<f64> = members.iter().map(|(_, l)| (l - max).exp()).collect();
        let sum: f64 = exps.iter().sum();
        let mut ranked: Vec<(String, f64)> = members
            .iter()
            .zip(exps.iter())
            .map(|((t, _), e)| (t.clone(), e / sum))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        println!();
        println!("  Exclusive group {}:", gi + 1);
        for (t, p) in &ranked {
            println!("    {:6.1}%  {}", p * 100.0, t);
        }
    }

    let combinable: Vec<&(String, f64)> = query_logodds
        .iter()
        .filter(|(t, _)| !grouped_keys.contains(&ws_key(t)) && t.starts_with("diagnosis("))
        .collect();
    if !combinable.is_empty() {
        println!();
        println!("  Combinable / non-exclusive (raw independent posterior):");
        for (t, l) in &combinable {
            println!("    {:6.1}%  {}", sigmoid(*l) * 100.0, t);
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

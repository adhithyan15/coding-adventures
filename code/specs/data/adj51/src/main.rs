//! ADJ51 experiment runner — domain-agnostic.
//!
//! Reads a derived adj-lang rulebook plus a vignette generated from
//! the ingester's observations and queries, runs each query through
//! the LR-aggregation engine, and prints the posterior + fired
//! clauses + coverage report (which observed terms participated in a
//! clause vs. were unmatched).

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use adj_lang::compile;
use logic_core::Term;
use logic_engine::{search, DerivationOrigin, SearchMode, SearchResult};

const DECISION_THRESHOLD: f64 = 0.30;

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));

    let subdir = std::env::var("ADJ51_DIR").unwrap_or_else(|_| "experiment".to_string());
    let rulebook_path = manifest_dir.join(&subdir).join("03-derived-rulebook.adj");
    let vignette_path = manifest_dir.join(&subdir).join("04-vignette.adj");

    let rulebook = fs::read_to_string(&rulebook_path).expect("rulebook");
    let vignette = fs::read_to_string(&vignette_path).expect("vignette");
    let combined = format!("{rulebook}\n{vignette}");

    // Mechanical parse of observe lines for the coverage check.
    let observed_terms: HashSet<String> = vignette
        .lines()
        .filter_map(|l| l.trim().strip_prefix("observe "))
        .map(|t| t.trim().to_string())
        .collect();

    println!("================================================================");
    println!("  ADJ51 — domain-agnostic pipeline run");
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
            return;
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

        let result = search(query, &lowered.kb, SearchMode::LRAggregate);
        let SearchResult::LRAggregateResult {
            dag,
            posterior,
            posterior_logit,
            warnings,
            uncertainties: _,
        } = result
        else {
            println!("  ENGINE ERROR: expected LR aggregation result");
            continue;
        };

        println!();
        println!(
            "  Posterior:  P = {:.4}  ({:.1}%)   logodds = {:+.4}",
            posterior,
            posterior * 100.0,
            posterior_logit
        );

        let proof = &dag.proofs[0];
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

        // Coverage
        let unmatched: Vec<&String> = observed_terms.iter().filter(|t| !matched_obs.contains(*t)).collect();
        println!();
        println!(
            "  Coverage: {}/{} observed terms matched a clause for this query.",
            observed_terms.len() - unmatched.len(),
            observed_terms.len()
        );
        if !unmatched.is_empty() {
            let mut um: Vec<String> = unmatched.iter().map(|s| (*s).clone()).collect();
            um.sort();
            println!("  Unmatched observations ({}):", um.len());
            for u in &um {
                println!("    - {}", u);
            }
        }

        println!();
        let decision = if posterior >= DECISION_THRESHOLD {
            "ABOVE threshold"
        } else {
            "BELOW threshold"
        };
        println!(
            "  Decision at {:.0}%: {}",
            DECISION_THRESHOLD * 100.0,
            decision
        );

        if !warnings.is_empty() {
            println!();
            println!("  Engine warnings:");
            for w in &warnings {
                println!("    {:?}", w);
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

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}...", &s[..n.saturating_sub(3)])
    }
}

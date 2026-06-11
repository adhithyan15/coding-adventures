//! `adj-lang-cli` — compile a `.adj` program and emit the decision + a
//! byte-cited proof DAG as JSON.
//!
//! This is the **CPU-bound reasoner** the MYCIN-2026 prototype shells out to.
//! It reads a `.adj` program (a rulebook's `prior`/`contributes`/`interacts`
//! clauses concatenated with a case's `observe`/`?` lines), compiles it through
//! the adj-lang frontend, runs `logic_engine::differential` over the program's
//! queries, and prints JSON:
//!
//! ```json
//! { "queries": [...],
//!   "ranked": [ { "hypothesis", "posterior", "posterior_logit", "normalized_share",
//!                 "proof": [ { "kind":"prior|contribution|interaction|predicate", "logit",
//!                              "evidence?", "slot?", "op?", "threshold?", "observed?",
//!                              "source", "locator", "trust" } ] } ],
//!   "decision": { "type":"determinate|kickback|empty", ... } }
//! ```
//!
//! Every proof step carries the cited `source`/`locator`/`trust` of the clause it
//! fired, so the audit trail is reconstructable without re-running the model.
//! Argument parsing is declarative via `cli-builder`.

use std::fs;
use std::process::ExitCode;

use cli_builder::types::ParserOutput;
use cli_builder::{load_spec_from_str, Parser};

use adj_lang::{compile, decide};
use logic_core::Term;
use logic_engine::{
    DerivationOrigin, DifferentialDecision, KnowledgeBase, LRAggregateResult, Provenance, TrustTier,
};

const SPEC: &str = r#"{
  "cli_builder_spec_version": "1.0",
  "name": "adj-lang-cli",
  "description": "Compile a .adj program and emit decision + byte-cited proof DAG as JSON.",
  "version": "0.1.0",
  "arguments": [
    {"id": "program", "name": "PROGRAM", "description": "Path to a .adj program (rulebook + case)", "type": "string", "required": true}
  ]
}"#;

/// JSON-escape a string body.
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// Emit an f64 as a JSON number, or `null` for non-finite (e.g. an infinite
/// single-hypothesis margin) — JSON has no Infinity.
fn jnum(x: f64) -> String {
    if x.is_finite() {
        format!("{}", x)
    } else {
        "null".to_string()
    }
}

fn trust(t: &TrustTier) -> &'static str {
    match t {
        TrustTier::Consensus => "consensus",
        TrustTier::Authoritative => "authoritative",
        TrustTier::Empirical => "empirical",
        TrustTier::Inferred => "inferred",
        TrustTier::Unattributed => "unattributed",
    }
}

/// The `"source"/"locator"/"trust"` fields of a clause's provenance.
fn prov(p: &Provenance) -> String {
    let loc = match &p.locator {
        Some(l) => format!("\"{}\"", esc(l)),
        None => "null".to_string(),
    };
    format!(
        "\"source\":\"{}\",\"locator\":{},\"trust\":\"{}\"",
        esc(&p.source),
        loc,
        trust(&p.trust_tier)
    )
}

/// Serialize the proof DAG for one hypothesis: walk each step and join its
/// `clause_id` back to the firing clause's evidence term + cited provenance.
fn proof_json(hyp: &Term, kb: &KnowledgeBase, result: &LRAggregateResult) -> String {
    let prior = kb.prior_for(hyp);
    let contribs = kb.contributions_for(hyp);
    let joints = kb.joint_contributions_for(hyp);
    let predicates = kb.predicate_contributions_for(hyp);
    let mut steps: Vec<String> = Vec::new();
    if let Some(proof) = result.dag.proofs.first() {
        for st in &proof.steps {
            match &st.origin {
                DerivationOrigin::FromPrior { prior_logit, .. } => {
                    let pj = prior.map(|p| prov(&p.provenance)).unwrap_or_else(|| {
                        "\"source\":\"\",\"locator\":null,\"trust\":\"unattributed\"".to_string()
                    });
                    steps.push(format!(
                        "{{\"kind\":\"prior\",\"logit\":{},{}}}",
                        jnum(*prior_logit),
                        pj
                    ));
                }
                DerivationOrigin::FromContribution {
                    clause_id,
                    logit_delta,
                    ..
                } => {
                    if let Some(c) = contribs.iter().find(|c| c.id == *clause_id) {
                        steps.push(format!(
                            "{{\"kind\":\"contribution\",\"evidence\":\"{}\",\"logit\":{},{}}}",
                            esc(&format!("{}", c.evidence_term)),
                            jnum(*logit_delta),
                            prov(&c.provenance)
                        ));
                    }
                }
                DerivationOrigin::FromJointContribution {
                    clause_id,
                    joint_logit_delta,
                    ..
                } => {
                    if let Some(j) = joints.iter().find(|j| j.id == *clause_id) {
                        let ev: Vec<String> = j
                            .evidence_set
                            .iter()
                            .map(|t| format!("\"{}\"", esc(&format!("{}", t))))
                            .collect();
                        steps.push(format!(
                            "{{\"kind\":\"interaction\",\"evidence\":[{}],\"logit\":{},{}}}",
                            ev.join(","),
                            jnum(*joint_logit_delta),
                            prov(&j.provenance)
                        ));
                    }
                }
                // A predicate-gated contribution: the audit trail shows the
                // literal comparison that fired (slot, op, threshold, the
                // observed value). The model never computed this — the
                // engine evaluated it on the CPU.
                DerivationOrigin::FromPredicateContribution {
                    clause_id,
                    slot,
                    op,
                    threshold,
                    observed,
                    logit_delta,
                } => {
                    let pv = predicates
                        .iter()
                        .find(|p| p.id == *clause_id)
                        .map(|p| prov(&p.provenance))
                        .unwrap_or_else(|| {
                            "\"source\":\"\",\"locator\":null,\"trust\":\"unattributed\""
                                .to_string()
                        });
                    steps.push(format!(
                        "{{\"kind\":\"predicate\",\"slot\":\"{}\",\"op\":\"{}\",\"threshold\":{},\"observed\":{},\"logit\":{},{}}}",
                        esc(slot),
                        esc(op.symbol()),
                        jnum(*threshold),
                        jnum(*observed),
                        jnum(*logit_delta),
                        pv
                    ));
                }
                _ => {}
            }
        }
    }
    format!("[{}]", steps.join(","))
}

fn main() -> ExitCode {
    let spec = load_spec_from_str(SPEC).expect("internal: invalid CLI spec");
    let parser = Parser::new(spec);
    let argv: Vec<String> = std::env::args().collect();
    let result = match parser.parse(&argv) {
        Ok(ParserOutput::Help(h)) => {
            print!("{}", h.text);
            return ExitCode::SUCCESS;
        }
        Ok(ParserOutput::Version(v)) => {
            println!("{}", v.version);
            return ExitCode::SUCCESS;
        }
        Ok(ParserOutput::Parse(r)) => r,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::from(2);
        }
    };

    let path = result
        .arguments
        .get("program")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("adj-lang-cli: cannot read {}: {}", path, e);
            return ExitCode::from(2);
        }
    };

    let lowered = match compile(&src) {
        Ok(l) => l,
        Err(e) => {
            println!("{{\"error\":\"{}\"}}", esc(&format!("{:?}", e)));
            return ExitCode::from(1);
        }
    };
    let diff = decide(&lowered);

    let mut ranked: Vec<String> = Vec::new();
    for r in &diff.ranked {
        let proof = proof_json(&r.hypothesis, &lowered.kb, &r.result);
        ranked.push(format!(
            "{{\"hypothesis\":\"{}\",\"posterior\":{},\"posterior_logit\":{},\"normalized_share\":{},\"proof\":{}}}",
            esc(&format!("{}", r.hypothesis)),
            jnum(r.posterior),
            jnum(r.posterior_logit),
            jnum(r.normalized_share),
            proof
        ));
    }

    let decision = match &diff.decision {
        DifferentialDecision::Empty => "{\"type\":\"empty\"}".to_string(),
        DifferentialDecision::Determinate { leader, posterior, margin_posterior, margin_logit } => {
            format!(
                "{{\"type\":\"determinate\",\"leader\":\"{}\",\"posterior\":{},\"margin_posterior\":{},\"margin_logit\":{}}}",
                esc(&format!("{}", leader)),
                jnum(*posterior),
                jnum(*margin_posterior),
                jnum(*margin_logit)
            )
        }
        DifferentialDecision::Kickback {
            leader, runner_up, margin_posterior, margin_logit, reason, ..
        } => format!(
            "{{\"type\":\"kickback\",\"leader\":\"{}\",\"runner_up\":\"{}\",\"margin_posterior\":{},\"margin_logit\":{},\"reason\":\"{}\"}}",
            esc(&format!("{}", leader)),
            esc(&format!("{}", runner_up)),
            jnum(*margin_posterior),
            jnum(*margin_logit),
            esc(reason)
        ),
    };

    let queries: Vec<String> = lowered
        .queries
        .iter()
        .map(|q| format!("\"{}\"", esc(&format!("{}", q))))
        .collect();
    println!(
        "{{\"queries\":[{}],\"ranked\":[{}],\"decision\":{}}}",
        queries.join(","),
        ranked.join(","),
        decision
    );
    ExitCode::SUCCESS
}

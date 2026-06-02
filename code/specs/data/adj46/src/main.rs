//! # ADJ46 — ACS chest-pain rulebook + Jane Doe case on the existing
//! `logic-engine`.
//!
//! ## What this binary does
//!
//! It runs the ADJ36 ACS rulebook on the same patient vignette ADJ36
//! used (62yo M, ED for chest discomfort, etc.) but routes the
//! computation through the production `logic-engine` crate rather than
//! the hand-coded Python LR multiplier in `adj36-execute.py`.
//!
//! The point is NOT to be elegant. The point is to be HONEST about
//! every place the existing engine forces an awkward encoding, so that
//! ADJ47 (Adj-Lang) is designed from real evidence rather than from
//! a priori speculation.
//!
//! ## Encoding strategy
//!
//! `logic-engine` is a weighted-model-counting engine: clauses carry
//! probabilities in [0, 1] and the engine multiplies them across a
//! satisfying world to compute P(query). The ACS rulebook needs LR
//! aggregation in log-odds space, which is a different operation. So
//! we:
//!
//! 1. Express each LR contribution as a *deterministic* Rule whose
//!    head is a synthetic `contrib(<id>)` marker and whose body is the
//!    condition under which the contribution fires.
//! 2. Store the actual LR magnitudes and citations in a parallel
//!    side-table keyed by the synthetic id.
//! 3. Query `?- contrib(X)` under `SearchMode::EnumerateAll` to
//!    enumerate which contributions fire on this case.
//! 4. Walk the resulting `ProofDAG`, look up the LR for each fired
//!    contribution, aggregate in log-odds space, emit the audit
//!    document.
//!
//! Every step where this encoding feels wrong is noted in
//! `AWKWARDNESS.md` in this directory.

use std::collections::HashMap;

use logic_core::{atom, compound, var, Term};
use logic_engine::{
    search, BodyLiteral, Fact, KnowledgeBase, Rule, RuleId, SearchMode, SearchResult,
};

// ---------------------------------------------------------------------------
// Side-tables for everything the engine cannot represent
// ---------------------------------------------------------------------------

/// An LR-style contribution to the posterior. The `id` is the synthetic
/// atom used as the rule head; `log_lr` is the natural log of the LR;
/// `citation` is the audit-trail string.
///
/// **Awkwardness A1, A2:** this struct exists only because the engine
/// has no native concept of LR magnitudes or provenance fields on
/// clauses. In Adj-Lang every clause has (lr, source) as first-class.
#[derive(Debug, Clone)]
struct LrEntry {
    id: &'static str,
    log_lr: f64,
    citation: &'static str,
    /// `true` if this is an interaction term layered on top of atomic
    /// contributions. **Awkwardness A4:** the engine has no
    /// `interaction` clause kind; we tag it on the side.
    is_interaction: bool,
}

/// All the LR contributions in the ACS rulebook, plus the joint term.
fn lr_table() -> HashMap<&'static str, LrEntry> {
    let entries = vec![
        // Demographic
        LrEntry {
            id: "c_pmh_htn",
            log_lr: 1.5_f64.ln(),
            citation: "HEART Score; Six AJ et al., Neth Heart J 2008;16(6):191-6. [empirical]",
            is_interaction: false,
        },
        LrEntry {
            id: "c_pmh_smoker",
            log_lr: 1.8_f64.ln(),
            citation: "HEART Score; Six AJ et al., Neth Heart J 2008;16(6):191-6. [empirical]",
            is_interaction: false,
        },
        // Symptom quality
        LrEntry {
            id: "c_sympq_pressure",
            log_lr: 2.5_f64.ln(),
            citation: "Panju AA et al., JAMA 1998;280(14):1256-63. Pooled LR for 'pressure' descriptor (range 1.5-3.0).",
            is_interaction: false,
        },
        LrEntry {
            id: "c_assoc_diaphoresis",
            log_lr: 2.0_f64.ln(),
            citation: "Panju AA et al., JAMA 1998. Pooled LR for diaphoresis present (range 1.7-2.7).",
            is_interaction: false,
        },
        // Precipitator
        LrEntry {
            id: "c_precip_exertional",
            log_lr: 2.5_f64.ln(),
            citation: "Diamond GA, Forrester JS. NEJM 1979;300(24):1350-8. Typical-angina pillar (range 2.0-4.0).",
            is_interaction: false,
        },
        LrEntry {
            id: "c_precip_rest",
            log_lr: 0.6_f64.ln(),
            citation: "Diamond/Forrester NEJM 1979. Rest pain mildly protective in undifferentiated ED population.",
            is_interaction: false,
        },
        LrEntry {
            id: "c_precip_positional",
            log_lr: 0.8_f64.ln(),
            citation: "[empirical] Positional pain is often MSK/pleuritic.",
            is_interaction: false,
        },
        // Protective contributors
        LrEntry {
            id: "c_vitals_wnl",
            log_lr: 0.5_f64.ln(),
            citation: "Panju AA et al., JAMA 1998. Normal vitals roughly halve ACS likelihood.",
            is_interaction: false,
        },
        LrEntry {
            id: "c_ecg_no_st",
            log_lr: 0.4_f64.ln(),
            citation: "Pope JH et al., NEJM 1995;342(16):1163-70. ECG w/o acute ST changes lowers but does not exclude ACS.",
            is_interaction: false,
        },
        // Interaction term (Awkwardness A4)
        LrEntry {
            id: "c_joint_press_diaph",
            log_lr: 1.3_f64.ln(),
            citation: "[empirical] Synergy: pressure-like pain WITH diaphoresis is more diagnostic than the product of individual LRs.",
            is_interaction: true,
        },
    ];
    entries.into_iter().map(|e| (e.id, e)).collect()
}

// ---------------------------------------------------------------------------
// Encoding the ACS rulebook in the logic-engine API
// ---------------------------------------------------------------------------

/// Build the rulebook. **Awkwardness A1, A3, A10:** the bodies of
/// these rules are the only thing the engine actually sees. The LR
/// magnitudes live in [`lr_table`]; the prior lives below in
/// [`PRIOR_P_ACS`]; the surface syntax is hand-written Rust.
fn build_rulebook(kb: &mut KnowledgeBase) -> HashMap<RuleId, &'static str> {
    let mut rule_id_to_contrib: HashMap<RuleId, &'static str> = HashMap::new();

    let mut add = |kb: &mut KnowledgeBase, contrib_id: &'static str, body: Vec<BodyLiteral>| {
        let head = compound("contrib", vec![atom(contrib_id)]);
        let rid = kb.add_rule(Rule::certain(head, body));
        rule_id_to_contrib.insert(rid, contrib_id);
    };

    // PMH
    add(kb, "c_pmh_htn",
        vec![BodyLiteral::Pos(compound("pmh", vec![atom("hypertension")]))]);
    add(kb, "c_pmh_smoker",
        vec![BodyLiteral::Pos(compound("pmh", vec![atom("smoker")]))]);

    // Symptom quality
    add(kb, "c_sympq_pressure",
        vec![BodyLiteral::Pos(compound("symptom_quality", vec![atom("pressure_like")]))]);
    add(kb, "c_assoc_diaphoresis",
        vec![BodyLiteral::Pos(compound("associated_symptom", vec![atom("diaphoresis")]))]);

    // Precipitator — three competing values (Awkwardness A5)
    add(kb, "c_precip_exertional",
        vec![BodyLiteral::Pos(compound("precipitator", vec![atom("exertional")]))]);
    add(kb, "c_precip_rest",
        vec![BodyLiteral::Pos(compound("precipitator", vec![atom("rest")]))]);
    add(kb, "c_precip_positional",
        vec![BodyLiteral::Pos(compound("precipitator", vec![atom("positional")]))]);

    // Protective
    add(kb, "c_vitals_wnl",
        vec![BodyLiteral::Pos(compound("vital_signs", vec![atom("within_normal_limits")]))]);
    add(kb, "c_ecg_no_st",
        vec![BodyLiteral::Pos(compound("denied", vec![atom("ecg_acute_st_changes")]))]);

    // Joint (Awkwardness A4) — fires only when BOTH atomic conditions hold
    add(kb, "c_joint_press_diaph",
        vec![
            BodyLiteral::Pos(compound("symptom_quality", vec![atom("pressure_like")])),
            BodyLiteral::Pos(compound("associated_symptom", vec![atom("diaphoresis")])),
        ]);

    rule_id_to_contrib
}

/// **Awkwardness A3:** prior is stored as a bare constant outside the
/// engine because `Probability` is world-state probability, not
/// Bayesian-prior log-odds.
const PRIOR_P_ACS: f64 = 0.10;

// ---------------------------------------------------------------------------
// The patient case (ADJ36 vignette: 62yo M with chest discomfort)
// ---------------------------------------------------------------------------

/// **Awkwardness A5:** the case says "no clear precipitator." That is
/// an *uncertainty marker over a domain*, not a fact. The engine has
/// no native uncertainty atom, so we encode it as three competing
/// probabilistic facts with uniform 1/3 prior weight. This:
///   (a) loses the user's "I don't know" annotation,
///   (b) double-counts: the engine will sum LR contributions for all
///       three precipitators when only one can be true.
/// In ADJ46 we accept this and document it; ADJ47 fixes it.
fn add_case_facts(kb: &mut KnowledgeBase) {
    kb.add_fact(Fact::certain(compound("pmh", vec![atom("hypertension")])));
    kb.add_fact(Fact::certain(compound("pmh", vec![atom("smoker")])));
    kb.add_fact(Fact::certain(compound("symptom_quality", vec![atom("pressure_like")])));
    kb.add_fact(Fact::certain(compound("associated_symptom", vec![atom("diaphoresis")])));
    kb.add_fact(Fact::certain(compound("vital_signs", vec![atom("within_normal_limits")])));
    kb.add_fact(Fact::certain(compound("denied", vec![atom("ecg_acute_st_changes")])));
    // Precipitator is uncertain — omit entirely. None of the three
    // precip facts are asserted. Aggregator treats this as "no
    // precipitator contribution fires," which is a defensible (and
    // probably correct) interpretation of "no clear precipitator."
}

// ---------------------------------------------------------------------------
// Aggregation: walk the proof DAG, apply LR math by hand
// ---------------------------------------------------------------------------

fn sigmoid(logodds: f64) -> f64 {
    1.0 / (1.0 + (-logodds).exp())
}

fn logit(p: f64) -> f64 {
    (p / (1.0 - p)).ln()
}

#[derive(Debug)]
struct FiredContribution {
    id: &'static str,
    log_lr: f64,
    citation: &'static str,
    is_interaction: bool,
}

fn aggregate(
    fired: &[FiredContribution],
    prior_p: f64,
) -> (f64, f64, f64) {
    let prior_logodds = logit(prior_p);
    let sum_log_lr: f64 = fired.iter().map(|f| f.log_lr).sum();
    let post_logodds = prior_logodds + sum_log_lr;
    let post_p = sigmoid(post_logodds);
    (prior_logodds, post_logodds, post_p)
}

// ---------------------------------------------------------------------------
// Audit document — what an attending would see
// ---------------------------------------------------------------------------

fn print_audit(
    case_label: &str,
    prior_p: f64,
    fired: &[FiredContribution],
    prior_logodds: f64,
    post_logodds: f64,
    post_p: f64,
) {
    println!("================================================================");
    println!("  ADJ46 — ACS posterior estimate, audit document");
    println!("================================================================");
    println!("Case:  {case_label}");
    println!();
    println!("Prior:  P(acs) = {:.4}  (logodds = {:+.4})", prior_p, prior_logodds);
    println!("       Source: Pope JH et al., NEJM 1995;342(16):1163-70.");
    println!();
    println!("Fired contributions ({} of {} possible rules):",
             fired.len(), lr_table().len());
    println!();
    println!("  {:<24} {:>8} {:>10}   {}", "id", "LR", "log(LR)", "citation");
    println!("  {}", "-".repeat(110));
    let mut atomic_log_sum = 0.0_f64;
    let mut interact_log_sum = 0.0_f64;
    for f in fired {
        let lr = f.log_lr.exp();
        let kind = if f.is_interaction { "  (interaction)" } else { "" };
        println!("  {:<24} {:>8.3} {:>+10.4}   {}{}",
                 f.id, lr, f.log_lr, f.citation, kind);
        if f.is_interaction {
            interact_log_sum += f.log_lr;
        } else {
            atomic_log_sum += f.log_lr;
        }
    }
    println!();
    println!("Sum of log(LR) over atomic contributions   = {:+.4}", atomic_log_sum);
    println!("Sum of log(LR) over interaction terms      = {:+.4}", interact_log_sum);
    println!("Total log-odds shift                       = {:+.4}",
             atomic_log_sum + interact_log_sum);
    println!();
    println!("Posterior:  logodds = {:+.4}  →  P(acs) = {:.4}",
             post_logodds, post_p);
    println!();
    println!("================================================================");
    println!("  Awkwardness flags raised during this query");
    println!("================================================================");
    println!("  A1  Likelihood ratios stored in a side-table (engine has no LR type)");
    println!("  A2  Provenance stored in a side-table (clauses have no provenance field)");
    println!("  A3  Prior stored as a bare constant (engine probability ≠ Bayesian prior)");
    println!("  A4  Joint contribution encoded as a multi-body rule but not flagged as interaction");
    println!("  A5  'no clear precipitator' has no encoding — omitted (lossy)");
    println!("  A6  WMC posterior discarded — aggregated LRs by hand");
    println!("  A7  No kickback variant: harness has to invent its own threshold");
    println!("  A8  Counterfactuals not run — would require KB clone + rerun");
    println!("  A9  Source disagreement not modeled — picked one number per rule");
    println!("  A10 Surface syntax is hand-written Rust, not a rulebook DSL");
    println!();
    println!("See AWKWARDNESS.md in this directory for the full design log.");
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let mut kb = KnowledgeBase::new();
    let rule_id_to_contrib = build_rulebook(&mut kb);
    add_case_facts(&mut kb);

    // Query: enumerate all `contrib(X)` derivations under EnumerateAll.
    let x = var("X");
    let query = compound("contrib", vec![Term::Var(x.clone())]);

    let result = search(&query, &kb, SearchMode::EnumerateAll);

    let dag = match result {
        SearchResult::EnumerateAllResult { dag, .. } => dag,
        SearchResult::FindFirstResult(_) => {
            panic!("EnumerateAll requested but engine short-circuited to FindFirst");
        }
    };

    // For each proof in the DAG, recover which contrib id it derived.
    // **Awkwardness A6 again:** the engine gives us `via_rules: Vec<RuleId>`
    // per proof, and we have to join that against our `rule_id_to_contrib`
    // map manually to learn what the proof was about.
    let lrt = lr_table();
    let mut fired: Vec<FiredContribution> = Vec::new();
    let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();

    for proof in &dag.proofs {
        for rid in &proof.via_rules {
            if let Some(contrib_id) = rule_id_to_contrib.get(rid) {
                if seen.insert(contrib_id) {
                    if let Some(entry) = lrt.get(contrib_id) {
                        fired.push(FiredContribution {
                            id: entry.id,
                            log_lr: entry.log_lr,
                            citation: entry.citation,
                            is_interaction: entry.is_interaction,
                        });
                    }
                }
            }
        }
    }

    // Stable order for the audit doc.
    fired.sort_by(|a, b| a.id.cmp(&b.id));

    let (prior_lo, post_lo, post_p) = aggregate(&fired, PRIOR_P_ACS);

    print_audit(
        "62yo M, ED for chest discomfort x 2h; pressure-like, mild \
         diaphoresis; no clear precipitator; PMH: HTN, smoker; vitals \
         normal; ECG: no acute ST changes.",
        PRIOR_P_ACS,
        &fired,
        prior_lo,
        post_lo,
        post_p,
    );

    // Cross-check against ADJ36's published posterior of ~28.1%.
    // If we land within 0.5% absolute, the encoding is reproducing
    // the same math the original adj36-execute.py did.
    let adj36_target = 0.281;
    let delta = (post_p - adj36_target).abs();
    println!("ADJ36 reference posterior:  P(acs) = {:.4}", adj36_target);
    println!("This binary's posterior:    P(acs) = {:.4}", post_p);
    println!("Absolute delta:             {:.4} ({})", delta,
             if delta < 0.005 { "OK — encoding reproduces ADJ36" }
             else            { "MISMATCH — investigate" });
}

//! The `explain` renderer — a human-readable projection of the reasoning trace
//! (ADJ-REASON-MATH §E.8, RS-4 PR-E).
//!
//! Everything else the CLI prints is a *machine* trail: byte-cited JSON that
//! `adj-verify` re-checks. That is necessary and it is not an explanation a
//! person reads. This module is the missing human view. It is governed by the
//! §E.8 invariants, and the one that matters most is **P1 — projection only**:
//! this code *reads* the derivation trees the engine already built and never
//! re-runs the engine or asserts anything not already in the trace. The
//! explanation can therefore never say more than the proof.
//!
//! # Staging (this slice = PR-E1: the DERIVATIONS surface)
//!
//! §E.8.1 linearizes a query into premises → derivations → inference →
//! adjudication → abstention. This first slice renders the **derivations**: for
//! every `let`/formula value the engine bound, the arithmetic is shown
//! operand-by-operand down to its cited leaves — the *how* of each computed
//! number (the §E.8.4 shape). The premises / inference / adjudication / abstention
//! sections are added by later PR-E slices; they append to the same output.
//!
//! # Invariants honored here
//!
//! - **P1 projection-only** — reads `KnowledgeBase::derived_bindings`; no engine
//!   re-run, no new value computed.
//! - **P2 provenance on every line** — a computed value carries its applied
//!   `formula`'s citation; a leaf grounded in an observed fact carries that
//!   fact's `source`/`locator`/`trust`, or renders an explicit `[unattributed]`
//!   when the fact bears no attribution. A *literal* constant written into the
//!   formula asserts nothing new (it is shown inline in its parent expression and
//!   carries no citation — that is honest, not a gap).
//! - **P4 determinism** — bindings are walked in first-seen order (mirroring the
//!   JSON `derived` section), values via `f64`/exact `Display` which is stable,
//!   and the output carries no timestamp, map-iteration order, or locale. The
//!   same KB renders byte-identical text on every run and platform.
//! - **P6 addressed structure** — each operand of an op renders on its own line,
//!   indented one level deeper, so a line in the prose maps back to exactly one
//!   node in the derivation tree.

use adj_lang::{LoweredStateMachine, StateMachineOutcome, StateMachineRun};
use logic_engine::compute::{DerivationNode, RoundSpec};
use logic_engine::differential::{Differential, DifferentialDecision};
use logic_engine::proof_dag::DerivationOrigin;
use logic_engine::{KnowledgeBase, Provenance, RoundingMode, TrustTier};

/// Render the human-readable explanation of a decided query.
///
/// Composes the §E.8.1 surfaces in order: **derivations** (the arithmetic behind
/// each computed value), **inference** (the ordered proof steps behind each
/// hypothesis — prior, rule-derived premises, likelihood-ratio contributions,
/// negation-as-failure), and **adjudication** (the ranked leader and the
/// comparative decision, or the honest kickback when the ranking is not robust).
/// Premises are shown inline within the inference steps — each cited step names
/// the fact or clause that licensed it.
///
/// Projection-only (P1): the inputs are the already-populated `kb` and the
/// already-decided `diff`; nothing is re-run. A pure computation (a `let` with no
/// differential evidence) renders only its derivations; a pure differential
/// renders only its inference + adjudication. Determinism (P4) rides on the
/// stable walk order of both.
pub fn explain(
    kb: &KnowledgeBase,
    diff: &Differential,
    state_machine_runs: &[(&LoweredStateMachine, StateMachineRun)],
) -> String {
    let mut sections: Vec<String> = Vec::new();
    let derivations = render_derivations(kb);
    if !derivations.is_empty() {
        sections.push(derivations);
    }
    // Inference + adjudication only apply when the differential carries actual
    // evidence (a prior or a contribution produced a proof step). A bare
    // computation / recall query has empty proofs and renders no differential —
    // so a `let`-only program stays derivations-only, unchanged from PR-E1.
    let has_inference = diff
        .ranked
        .iter()
        .any(|r| r.result.dag.proofs.first().is_some_and(|p| !p.steps.is_empty()));
    if has_inference {
        let inference = render_inference(kb, diff);
        if !inference.is_empty() {
            sections.push(inference);
        }
        sections.push(render_adjudication(kb, diff));
    }
    // ADJ-STATEMACHINE RS-3c: the run narrative — each machine's ordered,
    // provenance-cited steps ending in its typed terminal outcome. Projection-only
    // (P1): the runs were computed once in `main`; this only reads them. Omitted
    // when the program declared no `statemachine`, so a program without one renders
    // exactly as before.
    let runs = render_state_machines(state_machine_runs);
    if !runs.is_empty() {
        sections.push(runs);
    }
    sections.join("\n\n")
}

/// Render the state-machine run surface (ADJ-STATEMACHINE §3–§4, RS-3c) — for each
/// machine, the ordered transitions it fired (each citing the machine's
/// provenance and naming any facts it asserted), ending in the typed terminal
/// outcome line. Projection-only and deterministic (P1/P4): a machine renders
/// byte-identical text on every run. Returns "" when there are no machines.
fn render_state_machines(runs: &[(&LoweredStateMachine, StateMachineRun)]) -> String {
    if runs.is_empty() {
        return String::new();
    }
    let mut out: Vec<String> = Vec::new();
    for (sm, run) in runs {
        out.push(format!("Run of {}:", sm.name));
        for st in &run.steps {
            // The asserted facts, when the transition carried `do assert …`.
            let asserted = if st.asserted.is_empty() {
                String::new()
            } else {
                format!("  (asserted {})", st.asserted.join(", "))
            };
            out.push(format!(
                "  state {}: transition on {} to {} [{}]{}",
                st.from_state,
                st.guard,
                st.target,
                fmt_prov(&st.provenance),
                asserted
            ));
        }
        // The typed terminal outcome — the "abstain with a reason" line (§4).
        let outcome = match &run.outcome {
            StateMachineOutcome::Halted { state, result } => {
                format!("  => Halted at {}, yields {}", state, result.render())
            }
            StateMachineOutcome::StepBudgetExceeded { steps, budget, .. } => {
                format!("  => StepBudgetExceeded after {steps} steps (budget {budget})")
            }
            StateMachineOutcome::NonTerminating { state } => {
                format!("  => NonTerminating (cycle at {state})")
            }
            StateMachineOutcome::Stuck { state } => format!("  => Stuck in {state}"),
        };
        out.push(outcome);
    }
    out.join("\n")
}

/// Render the derivations surface — the arithmetic behind each `let`/formula
/// value, operand-by-operand to cited leaves (PR-E1). Returns "" when the
/// program bound no derived values.
fn render_derivations(kb: &KnowledgeBase) -> String {
    // First-seen order, latest value per name — identical to `derived_json`, so
    // the human view and the JSON view agree on which bindings exist and in what
    // order. Determinism (P4) rides on this being a deterministic walk.
    let all = kb.derived_bindings();
    let mut order: Vec<&str> = Vec::new();
    for d in all {
        if !order.contains(&d.name.as_str()) {
            order.push(d.name.as_str());
        }
    }

    let mut out: Vec<String> = Vec::new();
    for name in &order {
        let Some(d) = kb.derived_for(name) else {
            continue;
        };
        // The exact-first display (NUM-5): all digits when the value has a finite
        // decimal expansion, else the labeled-lossy f64 — matching `value_json`.
        let value = d
            .exact
            .as_ref()
            .and_then(|e| e.to_exact_decimal_string())
            .unwrap_or_else(|| fmt_num(d.value));
        // P2: a value produced by applying a provenanced `formula` carries the
        // formula's citation (why the formula is trusted). A plain `let` has no
        // library claim; its audit trail is the derivation tree itself.
        let cited = match &d.provenance {
            Some(p) => format!("   <= {}", fmt_prov(p)),
            None => String::new(),
        };
        out.push(format!("{} = {} [{}]{}", d.name, value, d.dim.tag(), cited));
        expand(&d.tree, 1, kb, &mut out);
        out.push(String::new()); // blank line between bindings
    }
    // Drop the trailing separator so the output has no dangling blank line.
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    if out.is_empty() {
        return String::new();
    }
    format!("Derivations:\n{}", out.join("\n"))
}

/// Render the inference surface — the ordered proof steps behind each hypothesis
/// that the differential actually proved. Mirrors the total-walk discipline of
/// the JSON `trace_steps_json`: the match over `DerivationOrigin` is exhaustive,
/// so a new step kind must be handled here or the crate fails to compile.
/// Projection-only: reads `r.result.dag`, re-runs nothing.
fn render_inference(kb: &KnowledgeBase, diff: &Differential) -> String {
    let mut out: Vec<String> = Vec::new();
    for r in &diff.ranked {
        let Some(proof) = r.result.dag.proofs.first() else {
            continue;
        };
        if proof.steps.is_empty() {
            continue;
        }
        // The graded clauses that can fire for this hypothesis — used to resolve
        // each contribution step back to the CLAUSE that licensed it (its cited
        // `source`/`trust`), which is the "why this likelihood ratio applies"
        // citation, distinct from the observed evidence a step consumed.
        let contribs = kb.contributions_for(&r.hypothesis);
        let joints = kb.joint_contributions_for(&r.hypothesis);
        let predicates = kb.predicate_contributions_for(&r.hypothesis);
        out.push(format!("Inference for {}:", r.hypothesis));
        for st in &proof.steps {
            // Depth addresses the step (P6): a rule's body steps are one level
            // deeper than the rule step that introduced them.
            let ind = "  ".repeat(st.depth + 1);
            let line = match &st.origin {
                DerivationOrigin::FromFact(id) => {
                    format!("{ind}{} [fact] {}", st.goal, cite_fact(kb, *id))
                }
                DerivationOrigin::FromRule(id) => {
                    let cite = kb
                        .find_rule_by_id(*id)
                        .map(|rule| format!("[{}]", fmt_prov(&rule.provenance)))
                        .unwrap_or_else(|| "[unattributed]".to_string());
                    format!("{ind}{} [rule] {cite}", st.goal)
                }
                // Negation-as-failure quotes nothing: what licensed the step is
                // the *absence* of any proof for the negated goal (§E.5).
                DerivationOrigin::FromNegation { goal } => {
                    format!("{ind}{} [negation: no proof exists for {}]", st.goal, goal)
                }
                DerivationOrigin::FromPrior { prior_logit, .. } => {
                    let cite = kb
                        .prior_for(&r.hypothesis)
                        .map(|p| format!("[{}]", fmt_prov(&p.provenance)))
                        .unwrap_or_else(|| "[unattributed]".to_string());
                    format!(
                        "{ind}prior on {} = logit {} {cite}",
                        st.goal,
                        fmt_num(*prior_logit)
                    )
                }
                DerivationOrigin::FromContribution {
                    clause_id,
                    evidence_fact_ids,
                    logit_delta,
                    ..
                } => {
                    let via = contribs
                        .iter()
                        .find(|c| c.id == *clause_id)
                        .map(|c| format!("[{}]", fmt_prov(&c.provenance)))
                        .unwrap_or_else(|| "[unattributed]".to_string());
                    format!(
                        "{ind}{} contributes logit {} via {via} [evidence: {}]",
                        st.goal,
                        fmt_num(*logit_delta),
                        cite_facts(kb, evidence_fact_ids)
                    )
                }
                DerivationOrigin::FromJointContribution {
                    clause_id,
                    evidence_fact_ids,
                    joint_logit_delta,
                    ..
                } => {
                    let via = joints
                        .iter()
                        .find(|j| j.id == *clause_id)
                        .map(|j| format!("[{}]", fmt_prov(&j.provenance)))
                        .unwrap_or_else(|| "[unattributed]".to_string());
                    format!(
                        "{ind}{} interaction logit {} via {via} [evidence: {}]",
                        st.goal,
                        fmt_num(*joint_logit_delta),
                        cite_facts(kb, evidence_fact_ids)
                    )
                }
                // The literal comparison that fired is the justification: the
                // reader sees `observed <op> threshold`, recomputable, no
                // model-computed number. The clause that set the threshold
                // carries the citation for why it applies.
                DerivationOrigin::FromPredicateContribution {
                    clause_id,
                    slot,
                    op,
                    threshold,
                    observed,
                    logit_delta,
                } => {
                    let via = predicates
                        .iter()
                        .find(|p| p.id == *clause_id)
                        .map(|p| format!(" via [{}]", fmt_prov(&p.provenance)))
                        .unwrap_or_default();
                    format!(
                        "{ind}{} {} {} (observed {}) contributes logit {}{via}",
                        slot,
                        op.symbol(),
                        fmt_num(*threshold),
                        fmt_num(*observed),
                        fmt_num(*logit_delta)
                    )
                }
            };
            out.push(line);
        }
    }
    out.join("\n")
}

/// Render the adjudication surface — the ranked hypotheses and the comparative
/// decision. P3: the leader line shows the trust tier propagated as the `min`
/// (weakest link) over its proof's cited clauses. A `Kickback` is rendered as
/// the honest "cannot commit" verdict it is, with its reason — the differential's
/// own form of abstention.
fn render_adjudication(kb: &KnowledgeBase, diff: &Differential) -> String {
    let mut out: Vec<String> = vec!["Decision:".to_string()];
    for r in &diff.ranked {
        out.push(format!(
            "  {} — posterior {} (logit {})",
            r.hypothesis,
            fmt_num(r.posterior),
            fmt_num(r.posterior_logit)
        ));
    }
    match &diff.decision {
        DifferentialDecision::Empty => {
            out.push("  abstained: no hypotheses were supplied".to_string());
        }
        DifferentialDecision::Determinate {
            leader,
            posterior,
            margin_posterior,
            ..
        } => {
            let trust = min_trust_for(kb, diff, leader);
            out.push(format!(
                "  => {} (determinate; posterior {}, margin {}; trust {})",
                leader,
                fmt_num(*posterior),
                fmt_num(*margin_posterior),
                trust
            ));
        }
        DifferentialDecision::Kickback {
            leader,
            runner_up,
            margin_posterior,
            reason,
            ..
        } => {
            out.push(format!(
                "  => KICKBACK: {} vs {} (margin {}) — {} [resolve the flagged uncertainty before committing]",
                leader,
                runner_up,
                fmt_num(*margin_posterior),
                reason
            ));
        }
    }
    out.join("\n")
}

/// The weakest (`min`) trust tier over the cited clauses of `hyp`'s proof — P3's
/// "trust propagates as the weakest link." Resolves the prior, each rule, and
/// each contribution's evidence facts; an unresolved or unattributed clause pins
/// the result at `unattributed`.
fn min_trust_for(kb: &KnowledgeBase, diff: &Differential, hyp: &logic_core::Term) -> &'static str {
    let Some(r) = diff.ranked.iter().find(|r| &r.hypothesis == hyp) else {
        return "unattributed";
    };
    let Some(proof) = r.result.dag.proofs.first() else {
        return "unattributed";
    };
    // Rank: higher = stronger; the min (weakest) wins.
    let rank = |t: &TrustTier| match t {
        TrustTier::Consensus => 4u8,
        TrustTier::Authoritative => 3,
        TrustTier::Empirical => 2,
        TrustTier::Inferred => 1,
        TrustTier::Unattributed => 0,
    };
    let mut weakest: Option<TrustTier> = None;
    let mut fold = |t: TrustTier| {
        weakest = Some(match weakest.take() {
            Some(w) if rank(&w) <= rank(&t) => w,
            _ => t,
        });
    };
    // Fold the trust of the graded KNOWLEDGE the reasoning applied — the prior,
    // each rule, and each contribution/interaction/predicate clause. A raw
    // observation is an input, not a knowledge claim, so it does not pin the
    // reasoning's tier here; its own provenance is shown inline per step (P2).
    let contrib_clauses = kb.contributions_for(hyp);
    let joint_clauses = kb.joint_contributions_for(hyp);
    let predicate_clauses = kb.predicate_contributions_for(hyp);
    for st in &proof.steps {
        match &st.origin {
            DerivationOrigin::FromRule(id) => {
                if let Some(rule) = kb.find_rule_by_id(*id) {
                    fold(rule.provenance.trust_tier);
                }
            }
            DerivationOrigin::FromPrior { .. } => {
                if let Some(p) = kb.prior_for(hyp) {
                    fold(p.provenance.trust_tier);
                }
            }
            DerivationOrigin::FromContribution { clause_id, .. } => {
                if let Some(c) = contrib_clauses.iter().find(|c| c.id == *clause_id) {
                    fold(c.provenance.trust_tier);
                }
            }
            DerivationOrigin::FromJointContribution { clause_id, .. } => {
                if let Some(j) = joint_clauses.iter().find(|j| j.id == *clause_id) {
                    fold(j.provenance.trust_tier);
                }
            }
            DerivationOrigin::FromPredicateContribution { clause_id, .. } => {
                if let Some(p) = predicate_clauses.iter().find(|p| p.id == *clause_id) {
                    fold(p.provenance.trust_tier);
                }
            }
            // A directly-consumed fact is an observation/input; negation cites no
            // clause. Neither pins the reasoning's knowledge tier.
            DerivationOrigin::FromFact(_) | DerivationOrigin::FromNegation { .. } => {}
        }
    }
    match weakest {
        Some(t) => fmt_trust(&t),
        None => "unattributed",
    }
}

/// A fact's citation `[source … trust …]`, or `[unattributed]`.
fn cite_fact(kb: &KnowledgeBase, id: logic_engine::FactId) -> String {
    kb.fact(id)
        .map(|f| fmt_leaf_prov(&f.provenance))
        .unwrap_or_else(|| "[unattributed]".to_string())
}

/// The citations for a list of evidence facts, in order — each `[source …]` or
/// `[unattributed]`, joined for one line.
fn cite_facts(kb: &KnowledgeBase, ids: &[logic_engine::FactId]) -> String {
    if ids.is_empty() {
        return "[none]".to_string();
    }
    ids.iter()
        .map(|id| cite_fact(kb, *id))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The inline label by which a parent operation refers to this operand: an
/// atom's name, a constant's value, or a nested result (parenthesized). The
/// child's own line — printed by [`expand`] — carries the detail.
fn label(n: &DerivationNode) -> String {
    match n {
        DerivationNode::Leaf { slot, .. } => slot.clone(),
        DerivationNode::DerivedRef { name, .. } => name.clone(),
        DerivationNode::Lit { value } => fmt_num(*value),
        DerivationNode::Op { result, .. } => format!("({})", fmt_num(*result)),
        DerivationNode::Round { result, .. }
        | DerivationNode::ToScientific { result, .. }
        | DerivationNode::ToPercent { result, .. }
        | DerivationNode::ToCurrency { result, .. } => fmt_num(*result),
    }
}

/// Whether this node earns its own expanded line. A literal constant asserts
/// nothing new (§E.8: it is shown inline in its parent and quotes nothing), so
/// it is the one node kind that does not expand; everything else does.
fn expands(n: &DerivationNode) -> bool {
    !matches!(n, DerivationNode::Lit { .. })
}

/// Emit the lines for one derivation node at `depth`, then recurse into the
/// operands that expand. The match is **total** over `DerivationNode` (no
/// wildcard): a new node kind must be handled here or the crate fails to
/// compile — the same totality discipline the JSON walker enforces (§E.8.1).
fn expand(n: &DerivationNode, depth: usize, kb: &KnowledgeBase, out: &mut Vec<String>) {
    let ind = "  ".repeat(depth);
    match n {
        DerivationNode::Op {
            op,
            operands,
            result,
        } => {
            let sep = format!(" {} ", op.symbol());
            let expr = operands.iter().map(label).collect::<Vec<_>>().join(&sep);
            out.push(format!("{ind}{} = {}", fmt_num(*result), expr));
            for c in operands {
                if expands(c) {
                    expand(c, depth + 1, kb, out);
                }
            }
        }
        DerivationNode::Leaf {
            slot,
            value,
            fact_id,
        } => {
            let prov = kb
                .fact(*fact_id)
                .map(|f| fmt_leaf_prov(&f.provenance))
                .unwrap_or_else(|| "[unattributed]".to_string());
            out.push(format!("{ind}{slot} = {}   {}", fmt_num(*value), prov));
        }
        DerivationNode::DerivedRef { name, value } => {
            out.push(format!("{ind}{name} = {}   (derived above)", fmt_num(*value)));
        }
        // A `Lit` operand never reaches here (guarded by `expands`); handled for
        // totality — a bare literal used as the whole tree just shows its value.
        DerivationNode::Lit { value } => {
            out.push(format!("{ind}{}", fmt_num(*value)));
        }
        DerivationNode::Round {
            spec,
            mode,
            operand,
            result,
        } => {
            out.push(format!(
                "{ind}{} = round({}, {}) [{}]",
                fmt_num(*result),
                label(operand),
                fmt_round_spec(spec),
                fmt_mode(*mode)
            ));
            if expands(operand) {
                expand(operand, depth + 1, kb, out);
            }
        }
        DerivationNode::ToScientific {
            figures,
            mode,
            rendered,
            operand,
            result,
        } => {
            out.push(format!(
                "{ind}{} (\"{}\") = to_scientific({}, {} sig figs) [{}]",
                fmt_num(*result),
                rendered,
                label(operand),
                figures,
                fmt_mode(*mode)
            ));
            if expands(operand) {
                expand(operand, depth + 1, kb, out);
            }
        }
        DerivationNode::ToPercent {
            places,
            mode,
            rendered,
            operand,
            result,
        } => {
            out.push(format!(
                "{ind}{} (\"{}\") = to_percent({}, {} places) [{}]",
                fmt_num(*result),
                rendered,
                label(operand),
                places,
                fmt_mode(*mode)
            ));
            if expands(operand) {
                expand(operand, depth + 1, kb, out);
            }
        }
        DerivationNode::ToCurrency {
            code,
            places,
            mode,
            rendered,
            operand,
            result,
        } => {
            out.push(format!(
                "{ind}{} (\"{}\") = to_currency({}, {}, {} places) [{}]",
                fmt_num(*result),
                rendered,
                label(operand),
                code,
                places,
                fmt_mode(*mode)
            ));
            if expands(operand) {
                expand(operand, depth + 1, kb, out);
            }
        }
    }
}

/// A node value as stable text. Rust's `f64` `Display` is deterministic and
/// drops a trailing `.0` (so `3.0` prints `3`, matching the JSON `value`), which
/// is exactly the stability P4 requires.
fn fmt_num(v: f64) -> String {
    format!("{}", v)
}

/// The provenance of a leaf's grounding fact, or `[unattributed]`. P2: a fact
/// with no `source` or an `Unattributed` trust tier is not silently blank — it
/// is marked, so an uncited magnitude is visible as such.
fn fmt_leaf_prov(p: &Provenance) -> String {
    if p.source.is_empty() || p.trust_tier == TrustTier::Unattributed {
        "[unattributed]".to_string()
    } else {
        format!("[{}]", fmt_prov(p))
    }
}

/// `source "S" locator "L" trust T` — the citation fields, with `locator`
/// omitted when the clause has none.
fn fmt_prov(p: &Provenance) -> String {
    let loc = match &p.locator {
        Some(l) => format!(" locator \"{}\"", l),
        None => String::new(),
    };
    format!(
        "source \"{}\"{} trust {}",
        p.source,
        loc,
        fmt_trust(&p.trust_tier)
    )
}

/// The stable spelling of a trust tier (mirrors the JSON `trust` field).
fn fmt_trust(t: &TrustTier) -> &'static str {
    match t {
        TrustTier::Consensus => "consensus",
        TrustTier::Authoritative => "authoritative",
        TrustTier::Empirical => "empirical",
        TrustTier::Inferred => "inferred",
        TrustTier::Unattributed => "unattributed",
    }
}

/// The rounding precision as text: decimal places for `round_to`, significant
/// figures for `round_sig`.
fn fmt_round_spec(spec: &RoundSpec) -> String {
    match spec {
        RoundSpec::Places(p) => format!("{p} places"),
        RoundSpec::SigFigures(n) => format!("{n} sig figs"),
    }
}

/// The stable spelling of a rounding mode (mirrors `rounding_mode_name`).
fn fmt_mode(mode: RoundingMode) -> &'static str {
    match mode {
        RoundingMode::Down => "down",
        RoundingMode::Up => "up",
        RoundingMode::Floor => "floor",
        RoundingMode::Ceiling => "ceiling",
        RoundingMode::HalfUp => "half_up",
        RoundingMode::HalfDown => "half_down",
        RoundingMode::HalfEven => "half_even",
    }
}

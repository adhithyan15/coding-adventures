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

use logic_engine::compute::{DerivationNode, RoundSpec};
use logic_engine::{KnowledgeBase, Provenance, RoundingMode, TrustTier};

/// Render the human-readable explanation of a decided knowledge base.
///
/// Returns the empty string when the program bound no derived values — this
/// slice explains the derivations surface only, so a pure-differential or
/// pure-recall program has nothing to render here yet (later PR-E slices add
/// those sections). Projection-only: the only input is the already-populated
/// `kb`.
pub fn explain(kb: &KnowledgeBase) -> String {
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
    out.join("\n")
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

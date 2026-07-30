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
//!   "decision": { "type":"determinate|kickback|empty", ... },
//!   "solve": { "outcome":"solved", "assignments":[{"name","value"}],
//!              "from_constraints":[...] } }   // present only when the program
//!                                              // declared a constraint system
//! ```
//!
//! Every proof step carries the cited `source`/`locator`/`trust` of the clause it
//! fired, so the audit trail is reconstructable without re-running the model.
//! Argument parsing is declarative via `cli-builder`.

use std::fs;
use std::process::ExitCode;

use cli_builder::types::ParserOutput;
use cli_builder::{load_spec_from_str, Parser};

use adj_constraint_solver::{
    check, optimize, solve, FeasibilityOutcome, OptimizeOutcome, SolveOutcome,
};
use adj_lang::{
    compile_with_imports, decide, run_state_machine, ImportLimits, LoweredRangeLookup,
    LoweredStateMachine, StateMachineOutcome, StateMachineRun, YieldValue,
};
use adj_lang_cli::{esc, payload, query_echo, sensitive_input, FsProvider};
use logic_core::{atom, compound, var, LogicVar, Term};
use logic_engine::govern::Standing;
use logic_engine::{
    enumerate_all, enumerate_governing, numeric_exact_magnitude, DerivationOrigin,
    DifferentialDecision, Fact, GovernStatus, KnowledgeBase, LRAggregateResult, Proof, ProofDAG,
    Provenance, TrustTier,
};

mod explain;

const SPEC: &str = r#"{
  "cli_builder_spec_version": "1.0",
  "name": "adj-lang-cli",
  "description": "Compile a .adj program and emit decision + byte-cited proof DAG as JSON.",
  "version": "0.1.0",
  "arguments": [
    {"id": "program", "name": "PROGRAM", "description": "Path to a .adj program (rulebook + case)", "type": "string", "required": true}
  ],
  "flags": [
    {"id": "explain", "long": "explain", "description": "Render a deterministic, human-readable explanation of the reasoning instead of the JSON trail (ADJ-REASON-MATH §E.8). Projection-only: no engine re-run.", "type": "boolean"}
  ]
}"#;

/// Emit an f64 as a JSON number, or `null` for non-finite (e.g. an infinite
/// single-hypothesis margin) — JSON has no Infinity.
fn jnum(x: f64) -> String {
    if x.is_finite() {
        format!("{}", x)
    } else {
        "null".to_string()
    }
}

/// Render a derived value's `"value"` field, **exact-first** (ADJ-EXACT-NUMBERS NX-4).
///
/// When a computation stayed inside exact rational arithmetic (NX-3) *and* the result has a
/// finite base-10 expansion, we print ALL of its digits — the same policy NX-2 gave a stored
/// `Number::Exact` recall binding. So a stored 39-digit π fed through a shipped `formula` renders
/// its doubled value to all 39 fractional digits here, instead of the ~16 an `f64` carries.
///
/// The `f64` (`jnum`) is the labeled-lossy fallback, used only when there is no exact sidecar or
/// when the value *repeats* (e.g. `1/3`), which no finite decimal can hold. The emitted digits
/// remain a JSON number literal, so the field's type is unchanged for every existing consumer;
/// only its precision grows for values that were previously truncated.
fn value_json(d: &logic_engine::compute::Derived) -> String {
    if let Some(exact) = &d.exact {
        if let Some(s) = exact.to_exact_decimal_string() {
            return s;
        }
    }
    jnum(d.value)
}

/// Render a compute derivation tree — **how** a `let`/formula value was
/// actually calculated, step by arithmetic step.
///
/// # Why this matters more than it looks
///
/// The engine has always built this tree. It builds one for *every* `let` and
/// every formula application, and until now it was **discarded at the JSON
/// boundary**: the CLI printed the final number and its dimension, and the
/// record of how that number came to be was dropped on the floor. So a reader
/// could see `bmi = 22.86` and its citation, but could not see that it was
/// `70 / (1.75 ^ 2)`, nor which observed facts supplied the 70 and the 1.75.
///
/// The leaves are the important part. A `Leaf` carries a real `FactId`, so it
/// resolves to that fact's `source`/`locator`/`trust` — which means **an
/// arithmetic result is traceable all the way down to bytes**, one operand at a
/// time. That bridge already existed in the engine; this function is what
/// finally lets anyone outside the process walk across it.
///
/// Node kinds, and what each asserts:
/// - `leaf`   — asserts a fact (an observed magnitude), so it **quotes**.
/// - `ref`    — points at another named derived value; its own tree explains it.
/// - `lit`    — a constant written in the formula; asserts nothing new.
/// - `op`     — arithmetic over operands; asserts nothing new, and is honest
///   because its operands are (§E.3: a step that computes over already-cited
///   inputs quotes nothing — its justification is its operands').
fn derivation_tree_json(
    node: &logic_engine::compute::DerivationNode,
    kb: &KnowledgeBase,
) -> String {
    use logic_engine::compute::DerivationNode as D;
    match node {
        D::Leaf {
            slot,
            value,
            fact_id,
        } => {
            let pv = kb
                .fact(*fact_id)
                .map(|f| prov(&f.provenance))
                .unwrap_or_else(|| UNRESOLVED_PROV.to_string());
            format!(
                "{{\"node\":\"leaf\",\"slot\":\"{}\",\"value\":{},{}}}",
                esc(slot),
                jnum(*value),
                pv
            )
        }
        D::DerivedRef { name, value } => format!(
            "{{\"node\":\"ref\",\"name\":\"{}\",\"value\":{}}}",
            esc(name),
            jnum(*value)
        ),
        D::Lit { value } => format!("{{\"node\":\"lit\",\"value\":{}}}", jnum(*value)),
        D::Op {
            op,
            operands,
            result,
        } => {
            let kids: Vec<String> = operands
                .iter()
                .map(|o| derivation_tree_json(o, kb))
                .collect();
            format!(
                "{{\"node\":\"op\",\"op\":\"{}\",\"value\":{},\"operands\":[{}]}}",
                esc(op.symbol()),
                jnum(*result),
                kids.join(",")
            )
        }
        // A `round_to(x, n)` narrowing (NUM-6a): the audit exposes the precision,
        // the rounding mode, the rounded `value`, and the operand subtree it
        // narrowed — so a checker can re-round the operand's exact value under the
        // recorded mode and confirm the rendering (ADJ-NUMERIC-SUBSTRATE §4.3).
        D::Round {
            spec,
            mode,
            operand,
            operand_exact: _,
            result,
        } => {
            // The precision field names the KIND of narrowing: `places` for
            // `round_to` (decimal places), `sig_figures` for `round_sig`.
            let precision = match spec {
                logic_engine::compute::RoundSpec::Places(p) => format!("\"places\":{p}"),
                logic_engine::compute::RoundSpec::SigFigures(n) => {
                    format!("\"sig_figures\":{n}")
                }
            };
            format!(
                "{{\"node\":\"round\",{},\"mode\":\"{}\",\"value\":{},\"operand\":{}}}",
                precision,
                rounding_mode_name(*mode),
                jnum(*result),
                derivation_tree_json(operand, kb)
            )
        }
        // A `to_scientific(x, figures)` rendering (NUM-6c): the audit exposes the
        // significant-figure count, the rounding mode, the `rendered` boundary string,
        // the narrowed numeric `value`, and the operand subtree it narrowed — so a
        // checker can re-narrow the operand's exact value to `figures` significant
        // figures and confirm the rendered form (ADJ-NUMERIC-SUBSTRATE §4.3).
        D::ToScientific {
            figures,
            mode,
            rendered,
            operand,
            operand_exact: _,
            result,
        } => format!(
            "{{\"node\":\"to_scientific\",\"figures\":{},\"mode\":\"{}\",\"rendered\":\"{}\",\"value\":{},\"operand\":{}}}",
            figures,
            rounding_mode_name(*mode),
            esc(rendered),
            jnum(*result),
            derivation_tree_json(operand, kb)
        ),
        // A `to_percent(x, places)` rendering (NUM-6c): the audit exposes the decimal-place
        // count, the rounding mode, the `rendered` `d.dd%` string, the narrowed numeric
        // `value` (the fraction the percentage denotes), and the operand subtree — so a
        // checker can re-scale and re-round the operand's exact value and confirm the
        // rendered form (ADJ-NUMERIC-SUBSTRATE §4.3).
        D::ToPercent {
            places,
            mode,
            rendered,
            operand,
            operand_exact: _,
            result,
        } => format!(
            "{{\"node\":\"to_percent\",\"places\":{},\"mode\":\"{}\",\"rendered\":\"{}\",\"value\":{},\"operand\":{}}}",
            places,
            rounding_mode_name(*mode),
            esc(rendered),
            jnum(*result),
            derivation_tree_json(operand, kb)
        ),
        // A `to_currency(x, code, places)` rendering (NUM-6c): the audit exposes the currency
        // code, the decimal-place count, the rounding mode, the `rendered` `CODE d.dd` string,
        // the narrowed numeric `value` (the rounded amount), and the operand subtree — so a
        // checker can re-round the operand's exact value and confirm the rendered form
        // (ADJ-NUMERIC-SUBSTRATE §4.3).
        D::ToCurrency {
            code,
            places,
            mode,
            rendered,
            operand,
            operand_exact: _,
            result,
        } => format!(
            "{{\"node\":\"to_currency\",\"code\":\"{}\",\"places\":{},\"mode\":\"{}\",\"rendered\":\"{}\",\"value\":{},\"operand\":{}}}",
            esc(code),
            places,
            rounding_mode_name(*mode),
            esc(rendered),
            jnum(*result),
            derivation_tree_json(operand, kb)
        ),
    }
}

/// The stable JSON spelling of a rounding mode for the audit trail — a checker
/// keys off these, so they are fixed identifiers, not `Debug`.
fn rounding_mode_name(mode: logic_engine::RoundingMode) -> &'static str {
    use logic_engine::RoundingMode as M;
    match mode {
        M::Down => "down",
        M::Up => "up",
        M::Floor => "floor",
        M::Ceiling => "ceiling",
        M::HalfUp => "half_up",
        M::HalfDown => "half_down",
        M::HalfEven => "half_even",
    }
}

/// Render the `let`-bound derived values as a JSON array, one object per
/// distinct binding name, each carrying the engine-computed magnitude plus the
/// [`Dimension`](logic_engine::dimension::Dimension) tag the engine *inferred*
/// for it (`"km/h"`, `"mol/l"`, `"usd"`, `"scalar"`, …).
///
/// This is the audit channel for dimensional analysis: a downstream reader (a
/// grader, a UI, a proof checker) sees not just *80* but *80 km/h*, so it can
/// reject a numerically-right-but-unit-wrong answer. The dimension is not
/// asserted by the model — it is the result of [`Dimension::combine`] applied
/// at every binary op while the `let` was evaluated (so `quantity(240, km) /
/// quantity(3, h)` reports `km/h`, never a guess).
///
/// A rebinding leaves two entries in the engine's table (latest wins for
/// lookups); we mirror that here by emitting only the most-recently-bound value
/// per name, preserving first-seen order so the output is stable.
fn derived_json(kb: &KnowledgeBase) -> String {
    let all = kb.derived_bindings();
    // First-seen order, but the value/dim are the LATEST binding for that name.
    let mut order: Vec<&str> = Vec::new();
    for d in all {
        if !order.contains(&d.name.as_str()) {
            order.push(d.name.as_str());
        }
    }
    let objs: Vec<String> = order
        .iter()
        .filter_map(|name| kb.derived_for(name))
        .map(|d| {
            // The exact value is now an arbitrary-precision `BigRational` (NUM-5), so its
            // numerator/denominator can exceed JSON's safe integer range — emit them as
            // **strings** so no precision is lost at the boundary (the whole point of the exact
            // channel). `BigInteger`'s `Display` is the plain decimal form.
            let exact = match &d.exact {
                Some(r) => format!(
                    ",\"exact\":{{\"num\":\"{}\",\"den\":\"{}\"}}",
                    r.numerator(),
                    r.denominator()
                ),
                None => String::new(),
            };
            // A value produced by APPLYING a provenanced `formula`
            // (ADJ-FORMULA-LIBRARIES rung-0) carries the formula's cited
            // `source`/`locator`/`trust` — the audit channel proving WHY the
            // formula is trusted, alongside the derivation tree proving HOW the
            // number was computed. A plain `let` has no such library claim, so
            // the field is omitted (output byte-for-byte unchanged there).
            let provenance = match &d.provenance {
                Some(p) => format!(",{}", prov(p)),
                None => String::new(),
            };
            // RS-4: the derivation tree the engine already built for this
            // value. Previously computed and then dropped here — emitting it is
            // what turns "the engine says 22.86" into "here is the arithmetic,
            // and here is the observed fact behind each operand."
            let derivation = format!(",\"derivation\":{}", derivation_tree_json(&d.tree, kb));
            format!(
                "{{\"name\":\"{}\",\"value\":{},\"dim\":\"{}\"{}{}{}}}",
                esc(&d.name),
                value_json(d),
                esc(&d.dim.tag()),
                exact,
                provenance,
                derivation
            )
        })
        .collect();
    format!("[{}]", objs.join(","))
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

/// The `"source"/"locator"/"trust"/"corroborations"` fields of a clause's
/// provenance. `corroborations` (ADJ-A9) is an array of co-equal citations for
/// the same fact — empty in the common single-citation case.
fn prov(p: &Provenance) -> String {
    let loc = match &p.locator {
        Some(l) => format!("\"{}\"", esc(l)),
        None => "null".to_string(),
    };
    let corro = p
        .corroborations
        .iter()
        .map(|c| {
            format!(
                "{{\"source\":\"{}\",\"locator\":\"{}\"}}",
                esc(&c.source),
                esc(&c.locator)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "\"source\":\"{}\",\"locator\":{},\"trust\":\"{}\",\"corroborations\":[{}]",
        esc(&p.source),
        loc,
        trust(&p.trust_tier),
        corro
    )
}

/// The provenance rendered for a step whose clause could not be resolved in the
/// KB. This should not happen — a step names a clause the engine just used —
/// but if it ever does, the trail must say "unattributed", never invent one.
const UNRESOLVED_PROV: &str =
    "\"source\":\"\",\"locator\":null,\"trust\":\"unattributed\",\"corroborations\":[]";

/// Render one proof as an **ordered, addressed, self-contained** list of
/// reasoning steps — the `ReasoningTrace` of `ADJ-REASON-MATH.md` §E.
///
/// # What each of those three words buys you
///
/// - **Ordered.** `Proof.steps` is a preorder walk of the derivation, so
///   `step` 0, 1, 2 … *is* the order the engine reasoned in. Contrast the
///   `via_facts` list every other citation surface renders from: that one is
///   sorted by fact id and deduplicated, which is why those surfaces can show
///   you *which* sources were used but never *in what order* — a bag, not a
///   narrative.
/// - **Addressed.** Each step carries `step` (its index) and `depth` (its
///   nesting). Parent = the nearest preceding step one level shallower, so a
///   reader can rebuild the tree without re-deriving any rule's arity.
/// - **Self-contained.** Each step inlines the *resolved* `source`/`locator`/
///   `trust` of the clause it fired, not a `FactId` pointer. The trace can then
///   travel — to a reviewer, to another machine, into an ADJ07 trail — and still
///   be readable without the knowledge base that produced it.
///
/// # Why the match below has no wildcard arm
///
/// It is **total** over `DerivationOrigin`, deliberately. The previous renderer
/// ended in `_ => {}`, which silently discarded every likelihood-ratio step:
/// a probabilistic conclusion would render a short, tidy, *incomplete* trail,
/// and nothing in the output marked the omission. A trail with a hole is worse
/// than no trail, because it looks complete. Adding a variant to
/// `DerivationOrigin` must now break this function's compilation — that failure
/// is the feature.
/// Render a typed **abstention reason** (`ADJ-REASON-MATH.md` §E.4).
///
/// # Why a boolean was not enough
///
/// Every abstention used to be the same value: `"abstained": true`. That
/// collapses genuinely different situations into one, and two of them are
/// *opposites*:
///
/// - **`below_table_domain`** — the question was well-formed and the source
///   simply does not cover it. The table is being honest, and the caller now
///   knows the domain it fell outside of.
/// - **`non_numeric_key`** — the question was malformed. Nothing is wrong with
///   the table; the caller is wrong.
///
/// Those emitted **byte-identical JSON**. No consumer could tell "your question
/// is outside what this source covers" from "your question was invalid", which
/// makes an abstention unactionable — you cannot tell whether to widen the
/// source or fix the query.
///
/// `search_limit_exceeded` is the third and subtlest: the engine did not
/// establish an absence at all, it *stopped looking*. Reporting that as
/// "no grounded support" would be a claim about the world derived from a
/// resource limit.
///
/// The rendered object is additive — `"abstained": true` is still emitted
/// beside it, so existing consumers are untouched.
fn abstention_json(reason: &AbstentionReason) -> String {
    match reason {
        AbstentionReason::NoGroundedSupport { goal } => format!(
            "{{\"reason\":\"no_grounded_support\",\"goal\":\"{}\",\"explanation\":\"the knowledge base contains no derivation of this goal\"}}",
            payload(goal)
        ),
        AbstentionReason::BelowTableDomain {
            table,
            key,
            min_key,
        } => format!(
            "{{\"reason\":\"below_table_domain\",\"table\":\"{}\",\"key\":\"{}\",\"min_key\":\"{}\",\"explanation\":\"the query falls below the lowest breakpoint this source defines; the source does not cover it\"}}",
            payload(table),
            payload(key),
            payload(min_key)
        ),
        AbstentionReason::AboveTableDomain {
            table,
            key,
            max_key,
        } => format!(
            "{{\"reason\":\"above_table_domain\",\"table\":\"{}\",\"key\":\"{}\",\"max_key\":\"{}\",\"explanation\":\"the query falls above the highest breakpoint this source defines; interpolation would extrapolate past what the source measured\"}}",
            payload(table),
            payload(key),
            payload(max_key)
        ),
        AbstentionReason::NonNumericKey { table, column, key } => format!(
            "{{\"reason\":\"non_numeric_key\",\"table\":\"{}\",\"column\":\"{}\",\"key\":\"{}\",\"explanation\":\"a range lookup needs a numeric key; this one could not be read as a number\"}}",
            payload(table),
            payload(column),
            payload(key)
        ),
        AbstentionReason::SearchLimitExceeded { goal } => format!(
            "{{\"reason\":\"search_limit_exceeded\",\"goal\":\"{}\",\"explanation\":\"the proof search hit its depth or width limit and stopped; this is NOT evidence that no proof exists\"}}",
            payload(goal)
        ),
    }
}

/// The typed reasons an ADJ query can decline to answer.
///
/// Closed on purpose: a new way to abstain must be named here and rendered,
/// rather than quietly reusing a neighbouring reason or falling back to the
/// bare boolean.
enum AbstentionReason {
    /// Nothing in the knowledge base derives the goal.
    NoGroundedSupport { goal: String },
    /// The key is below the table's lowest breakpoint — the source's domain
    /// starts above it.
    BelowTableDomain {
        table: String,
        key: String,
        min_key: String,
    },
    /// The key is above the table's highest breakpoint (ADJ-TABLES RS-5d). An
    /// `interpolated` lookup needs a breakpoint on *both* sides of the query; above
    /// the last one there is nothing to interpolate toward, so — rather than
    /// extrapolate past what the source measured — it abstains. (A `range` lookup
    /// treats the top breakpoint as an open band and never hits this.)
    AboveTableDomain {
        table: String,
        key: String,
        max_key: String,
    },
    /// A range lookup was handed a key that is not a number.
    NonNumericKey {
        table: String,
        column: String,
        key: String,
    },
    /// The search stopped at a resolution limit. **Not** an absence.
    SearchLimitExceeded { goal: String },
}

fn trace_steps_json(proof: &Proof, kb: &KnowledgeBase) -> String {
    let mut steps = Vec::new();
    for (i, st) in proof.steps.iter().enumerate() {
        // Every step is addressed the same way, whatever its kind.
        let head = format!(
            "\"step\":{},\"depth\":{},\"goal\":\"{}\"",
            i,
            st.depth,
            esc(&format!("{}", st.goal))
        );
        let body = match &st.origin {
            // ---- Deduction -------------------------------------------------
            DerivationOrigin::FromFact(id) => {
                let pv = kb
                    .fact(*id)
                    .map(|f| prov(&f.provenance))
                    .unwrap_or_else(|| UNRESOLVED_PROV.to_string());
                format!("\"kind\":\"fact\",{head},{pv}")
            }
            DerivationOrigin::FromRule(id) => {
                let pv = kb
                    .find_rule_by_id(*id)
                    .map(|r| prov(&r.provenance))
                    .unwrap_or_else(|| UNRESOLVED_PROV.to_string());
                format!("\"kind\":\"rule\",{head},{pv}")
            }
            // ---- Negation as failure --------------------------------------
            // No clause fired, so there is no citation to quote: what justified
            // this step is the *absence* of any proof for `goal`. A re-checker
            // verifies it by re-running that goal and asserting it still has
            // none (§E.5).
            DerivationOrigin::FromNegation { goal } => {
                format!(
                    "\"kind\":\"negation\",{head},\"absent_goal\":\"{}\",\"justification\":\"no proof exists for the negated goal\"",
                    esc(&format!("{goal}"))
                )
            }
            // ---- Probability (likelihood-ratio aggregation) ----------------
            // These four were the ones the old wildcard dropped.
            DerivationOrigin::FromPrior {
                clause_id,
                prior_logit,
            } => {
                format!(
                    "\"kind\":\"prior\",{head},\"clause_id\":{},\"prior_logit\":{}",
                    clause_id.0,
                    jnum(*prior_logit)
                )
            }
            DerivationOrigin::FromContribution {
                clause_id,
                evidence_fact_ids,
                logit_delta,
                ..
            } => {
                format!(
                    "\"kind\":\"contribution\",{head},\"clause_id\":{},\"logit_delta\":{},\"evidence\":{}",
                    clause_id.0,
                    jnum(*logit_delta),
                    fact_citations_json(evidence_fact_ids, kb)
                )
            }
            DerivationOrigin::FromJointContribution {
                clause_id,
                evidence_fact_ids,
                joint_logit_delta,
                ..
            } => {
                format!(
                    "\"kind\":\"interaction\",{head},\"clause_id\":{},\"logit_delta\":{},\"evidence\":{}",
                    clause_id.0,
                    jnum(*joint_logit_delta),
                    fact_citations_json(evidence_fact_ids, kb)
                )
            }
            // The literal comparison that fired is the provenance here: the
            // reader sees `observed <op> threshold` and can recompute it. No
            // number in this step came from a model.
            DerivationOrigin::FromPredicateContribution {
                clause_id,
                slot,
                op,
                threshold,
                observed,
                logit_delta,
            } => {
                format!(
                    "\"kind\":\"predicate\",{head},\"clause_id\":{},\"slot\":\"{}\",\"op\":\"{}\",\"threshold\":{},\"observed\":{},\"logit_delta\":{}",
                    clause_id.0,
                    esc(slot),
                    esc(op.symbol()),
                    jnum(*threshold),
                    jnum(*observed),
                    jnum(*logit_delta)
                )
            }
        };
        steps.push(format!("{{{body}}}"));
    }
    format!("[{}]", steps.join(","))
}

/// Resolve a list of `FactId`s to their inline citations, in the order given.
fn fact_citations_json(ids: &[logic_engine::FactId], kb: &KnowledgeBase) -> String {
    let objs: Vec<String> = ids
        .iter()
        .map(|id| {
            let pv = kb
                .fact(*id)
                .map(|f| prov(&f.provenance))
                .unwrap_or_else(|| UNRESOLVED_PROV.to_string());
            format!("{{{pv}}}")
        })
        .collect();
    format!("[{}]", objs.join(","))
}

/// Serialize the proof DAG for one hypothesis: walk each step and join its
/// `clause_id` back to the firing clause's evidence term + cited provenance.
/// `certs` maps a constraint STATUS atom to its solver certificate JSON (E3); a
/// contribution step whose evidence *is* such a status gets the certificate
/// attached under it as `"solver": …`, so the verdict's proof descends into the
/// solver's result (the IIS, the assignment, the optimum).
fn proof_json(
    hyp: &Term,
    kb: &KnowledgeBase,
    result: &LRAggregateResult,
    certs: &[(&'static str, String)],
) -> String {
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
                        "\"source\":\"\",\"locator\":null,\"trust\":\"unattributed\",\"corroborations\":[]".to_string()
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
                    evidence_proof,
                    ..
                } => {
                    if let Some(c) = contribs.iter().find(|c| c.id == *clause_id) {
                        let ev = format!("{}", c.evidence_term);
                        // E3: if this contribution fired from a constraint STATUS
                        // atom, descend into the solver certificate beneath it.
                        let solver = certs
                            .iter()
                            .find(|(k, _)| *k == ev.as_str())
                            .map(|(_, cert)| format!(",\"solver\":{}", cert))
                            .unwrap_or_default();
                        let evidence_proof = evidence_proof
                            .as_ref()
                            .map(|proof| {
                                format!(",\"evidence_proof\":{}", trace_steps_json(proof, kb))
                            })
                            .unwrap_or_default();
                        steps.push(format!(
                            "{{\"kind\":\"contribution\",\"evidence\":\"{}\",\"logit\":{},{}{}{}}}",
                            esc(&ev),
                            jnum(*logit_delta),
                            prov(&c.provenance),
                            evidence_proof,
                            solver
                        ));
                    }
                }
                DerivationOrigin::FromJointContribution {
                    clause_id,
                    joint_logit_delta,
                    evidence_proofs,
                    ..
                } => {
                    if let Some(j) = joints.iter().find(|j| j.id == *clause_id) {
                        let ev: Vec<String> = j
                            .evidence_set
                            .iter()
                            .map(|t| format!("\"{}\"", esc(&format!("{}", t))))
                            .collect();
                        let evidence_proofs = if evidence_proofs.is_empty() {
                            String::new()
                        } else {
                            format!(
                                ",\"evidence_proofs\":[{}]",
                                evidence_proofs
                                    .iter()
                                    .map(|proof| trace_steps_json(proof, kb))
                                    .collect::<Vec<_>>()
                                    .join(",")
                            )
                        };
                        steps.push(format!(
                            "{{\"kind\":\"interaction\",\"evidence\":[{}],\"logit\":{},{}{}}}",
                            ev.join(","),
                            jnum(*joint_logit_delta),
                            prov(&j.provenance),
                            evidence_proofs
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
                            "\"source\":\"\",\"locator\":null,\"trust\":\"unattributed\",\"corroborations\":[]"
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

/// Map the constraint-engine outcomes to `(status atom, certificate JSON)` pairs
/// (ADJ constraints E2 + E3). The status atom, injected as an observed fact,
/// feeds the differential — it fires an existing
/// `contributes <lr> from <status> to <verdict>` clause (E2, feed-a-verdict).
/// The certificate JSON is the solver's full result for that status (the IIS
/// `core` for `infeasible`, the assignments for `solved`, the value + binding
/// constraints for `optimal`, …); E3 attaches it *under* the proof step that
/// fired, so the whole adjudication is one auditable tree.
///
/// Deduplicated and order-stable. An `Unknown` / `Unsupported` /
/// `NoUniqueSolution` outcome yields NOTHING — the engine never launders an
/// undecided constraint into a verdict (the one-engine invariant).
fn status_certificates(
    solve: &Option<SolveOutcome>,
    check: &Option<FeasibilityOutcome>,
    optimize: &Option<OptimizeOutcome>,
) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::new();
    let add = |out: &mut Vec<(&'static str, String)>, s: &'static str, cert: String| {
        if !out.iter().any(|(k, _)| *k == s) {
            out.push((s, cert));
        }
    };
    if let Some(o) = check {
        match o {
            FeasibilityOutcome::Sat { .. } | FeasibilityOutcome::SatReal { .. } => {
                add(&mut out, "feasible", check_json(o))
            }
            FeasibilityOutcome::Unsat { .. } => add(&mut out, "infeasible", check_json(o)),
            FeasibilityOutcome::Unknown { .. } => {}
        }
    }
    if let Some(o @ (SolveOutcome::Solved { .. } | SolveOutcome::SolvedRoots { .. })) = solve {
        add(&mut out, "solved", solve_json(o));
    }
    if let Some(o) = optimize {
        match o {
            OptimizeOutcome::Optimal { .. } => add(&mut out, "optimal", optimize_json(o)),
            OptimizeOutcome::Infeasible { .. } => add(&mut out, "infeasible", optimize_json(o)),
            OptimizeOutcome::Unbounded => add(&mut out, "unbounded", optimize_json(o)),
            OptimizeOutcome::Unknown { .. } => {}
        }
    }
    out
}

/// The filesystem-backed [`ImportProvider`] — the **trust boundary** for
/// `import`. The `adj-lang` library does no I/O; this is the only thing that
/// touches the disk, so every path-safety check lives here:
///
/// - Canonical ids are absolute, symlink-resolved real paths
///   (`fs::canonicalize`), so two spellings of the same file dedupe and a
///   symlink cannot smuggle in a second identity.
/// - Import literals must be **relative** — an absolute literal is refused.
/// - The resolved real path must stay within `root` (the directory of the
///   top-level program). A `../…` escape or a symlink pointing outside `root`
///   is refused — `import` cannot read arbitrary files on the host.
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

    // §E.8: opt-in human-readable explanation instead of the JSON trail.
    let explain = result
        .flags
        .get("explain")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Resolve the program (and any `import`s) through the filesystem provider.
    // The sandbox root is the directory of the program file; no `import` may
    // read outside it. The canonical id of the root file seeds the resolver.
    let root_id = match fs::canonicalize(path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("adj-lang-cli: cannot read {}: {}", path, e);
            return ExitCode::from(2);
        }
    };
    let root_dir = match root_id.parent() {
        Some(d) => d.to_path_buf(),
        None => {
            eprintln!("adj-lang-cli: {} has no parent directory", path);
            return ExitCode::from(2);
        }
    };
    let provider = FsProvider { root: root_dir };
    let mut lowered = match compile_with_imports(
        &root_id.to_string_lossy(),
        &provider,
        ImportLimits::default(),
    ) {
        Ok(l) => l,
        Err(e) => {
            println!("{{\"error\":\"{}\"}}", esc(&format!("{:?}", e)));
            return ExitCode::from(1);
        }
    };

    // Partition the queries (MYCIN-2026 REL-3). A query containing a `$variable`
    // is a RELATIONAL RECALL binding query — resolved by SLD enumeration to a
    // `"recall"` section (bindings + the citing edge's provenance). A ground
    // hypothesis query flows to the differential as before. We snapshot all query
    // strings first (so the `"queries"` echo lists every query), then route the
    // binding queries out of `lowered.queries` so `decide` only sees hypotheses.
    let all_query_strs: Vec<String> = lowered
        .queries
        .iter()
        .map(|q| format!("\"{}\"", esc(&format!("{}", q))))
        .collect();
    let binding_queries: Vec<Term> = lowered
        .queries
        .iter()
        .filter(|q| contains_var(q))
        .cloned()
        .collect();
    lowered.queries.retain(|q| !contains_var(q));

    // ---- Constraint engine FIRST, so its verdict can feed the differential
    // (ADJ constraints E2 — feed-a-verdict). The constraint outcomes are
    // computed up front; each maps to a STATUS atom (`feasible` / `infeasible`
    // / `solved` / `optimal` / `unbounded`) that we inject as an observed fact
    // into the KB *before* `decide` runs. An existing
    // `contributes <lr> from <status> to <verdict>` clause then fires in the
    // differential — composing solver result → verdict through the ordinary
    // contribution machinery, no new engine logic.
    let solve_outcome =
        (!lowered.constraints.is_empty()).then(|| solve(&lowered.constraints, &lowered.kb));
    let check_outcome = lowered
        .constraints
        .check
        .then(|| check(&lowered.constraints, &lowered.kb));
    let optimize_outcome = lowered
        .constraints
        .objective
        .is_some()
        .then(|| optimize(&lowered.constraints, &lowered.kb));

    let certs = status_certificates(&solve_outcome, &check_outcome, &optimize_outcome);
    for (status, _) in &certs {
        lowered.kb.add_fact(Fact::certain(atom(*status)));
    }

    let diff = decide(&lowered);

    // ADJ-STATEMACHINE RS-3c: run every `statemachine` the program declared. Each
    // run is deterministic and TOTAL — it returns one typed outcome (Halted /
    // StepBudgetExceeded / NonTerminating / Stuck) and never hangs (the declared
    // step budget caps the loop). The driver reasons over a working CLONE of the
    // KB, so a machine's `assert`s never leak into the rest of the program. Empty
    // for a program with no `statemachine`, so all existing output stays
    // byte-identical (the section is omitted below).
    let state_machine_runs: Vec<(&LoweredStateMachine, StateMachineRun)> = lowered
        .state_machines
        .iter()
        .map(|sm| (sm, run_state_machine(sm, &lowered.kb)))
        .collect();

    let mut ranked: Vec<String> = Vec::new();
    for r in &diff.ranked {
        let proof = proof_json(&r.hypothesis, &lowered.kb, &r.result, &certs);
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

    // The `"queries"` echo lists every query the program declared (ground +
    // binding), captured before the partition above.
    // Same gate as every other echo — the `queries` array would otherwise
    // reprint in full exactly what the abstention object redacts.
    let queries: Vec<String> = if sensitive_input() {
        all_query_strs
            .iter()
            .map(|_| "\"[redacted]\"".to_string())
            .collect()
    } else {
        all_query_strs
    };

    // Relational recall: each binding query resolves to its bindings + the citing
    // edge's provenance (or abstains with an empty answer set). 0 answer-time
    // model calls — pure SLD over the grounded knowledge graph.
    let recall: Vec<String> = binding_queries
        .iter()
        .map(|q| recall_json(q, &lowered.kb))
        .collect();
    let recall_section = if recall.is_empty() {
        String::new()
    } else {
        format!(",\"recall\":[{}]", recall.join(","))
    };

    // ADJ73 governance: the precedence-resolved view of each binding query — every answer
    // tagged governing / defeated(by) / conflict_peer. For non-functional predicates this
    // mirrors `recall` (all governing); for a functional predicate with `priority:` tiers it
    // shows the override chain. 0 answer-time model calls.
    let governing: Vec<String> = binding_queries
        .iter()
        .map(|q| governing_json(q, &lowered.kb))
        .collect();
    let governing_section = if governing.is_empty() {
        String::new()
    } else {
        format!(",\"governing\":[{}]", governing.join(","))
    };

    // ADJ-TABLES RS-5c/RS-5d: table lookups. `mode range` reads the table as a step
    // function (bracketed value + the matched breakpoint row's citation, or abstains
    // below the domain); `mode interpolated` reads it as a piecewise-linear function
    // (exact linear blend of the two bracketing rows, both citations, or abstains
    // outside the domain). Both are 0 answer-time model calls — exact rational
    // arithmetic over the CAS-grounded rows. Omitted when the program declares no
    // `? lookup …`, so existing output is byte-for-byte unchanged.
    let lookups: Vec<String> = lowered
        .range_lookups
        .iter()
        .map(|rl| match rl.mode.as_str() {
            "interpolated" => interpolated_lookup_json(rl, &lowered.kb),
            "nearest" => nearest_lookup_json(rl, &lowered.kb),
            _ => range_lookup_json(rl, &lowered.kb),
        })
        .collect();
    let lookup_section = if lookups.is_empty() {
        String::new()
    } else {
        format!(",\"lookups\":[{}]", lookups.join(","))
    };

    // Render the constraint sections from the outcomes computed above (the
    // solvers are not re-run). Absent a constraint system / `check` / objective,
    // the respective key is omitted entirely.
    let solve_section = match &solve_outcome {
        Some(o) => format!(",\"solve\":{}", solve_json(o)),
        None => String::new(),
    };
    let check_section = match &check_outcome {
        Some(o) => format!(",\"check\":{}", check_json(o)),
        None => String::new(),
    };
    let optimize_section = match &optimize_outcome {
        Some(o) => format!(",\"optimize\":{}", optimize_json(o)),
        None => String::new(),
    };

    // `let`-bound derived values with their inferred dimensions. Omitted when
    // the program binds nothing (the common rulebook/recall case), so existing
    // output is byte-for-byte unchanged unless a `let` is present.
    let derived = derived_json(&lowered.kb);
    let derived_section = if derived == "[]" {
        String::new()
    } else {
        format!(",\"derived\":{}", derived)
    };

    // ADJ-STATEMACHINE RS-3c: the state-machine run section — each machine's typed
    // outcome, its ordered provenanced steps, and the machine's own citation.
    // Omitted (empty string) when the program declared no `statemachine`, so every
    // existing program's output is byte-for-byte unchanged.
    let state_machines_section = if state_machine_runs.is_empty() {
        String::new()
    } else {
        let items: Vec<String> = state_machine_runs
            .iter()
            .map(|(sm, run)| state_machine_json(sm, run, &lowered.kb))
            .collect();
        format!(",\"state_machines\":[{}]", items.join(","))
    };

    // `--explain` (ADJ-REASON-MATH §E.8): render the human-readable view of the
    // reasoning instead of the JSON trail. Projection-only — it reads the same
    // `lowered.kb` the JSON above was built from and re-runs nothing. The JSON
    // remains the primary, complete artifact (default output); `--explain` is the
    // opt-in human view onto it.
    if explain {
        // ADJ-ARGUMENT-IR ADR-6: the SLD proof chain behind each binding query —
        // the argument's premises → connective → conclusion. Re-resolve each
        // binding query to the same proof DAG the `recall` section was built from
        // (projection-only: `enumerate_all` is deterministic and side-effect
        // free), so `--explain` can render the derivation as an argument. Empty
        // for a program with no binding query, leaving all other output unchanged.
        let argument_chains: Vec<(Term, ProofDAG)> = binding_queries
            .iter()
            .map(|q| (q.clone(), enumerate_all(q, &lowered.kb)))
            .collect();
        println!(
            "{}",
            explain::explain(&lowered.kb, &diff, &state_machine_runs, &argument_chains)
        );
        return ExitCode::SUCCESS;
    }

    println!(
        "{{\"queries\":[{}],\"ranked\":[{}],\"decision\":{}{}{}{}{}{}{}{}{}}}",
        queries.join(","),
        ranked.join(","),
        decision,
        solve_section,
        check_section,
        optimize_section,
        recall_section,
        governing_section,
        lookup_section,
        derived_section,
        state_machines_section
    );
    ExitCode::SUCCESS
}

/// Render one state-machine run (ADJ-STATEMACHINE RS-3c) as JSON: the machine's
/// name, its typed outcome, the ordered provenanced steps, and the machine's own
/// cited `source`/`locator`/`trust` envelope (which every transition inherits).
fn state_machine_json(
    sm: &LoweredStateMachine,
    run: &StateMachineRun,
    kb: &KnowledgeBase,
) -> String {
    let steps: Vec<String> = run
        .steps
        .iter()
        .map(|st| {
            let asserted: Vec<String> = st
                .asserted
                .iter()
                .map(|a| format!("\"{}\"", esc(a)))
                .collect();
            format!(
                "{{\"from_state\":\"{}\",\"guard\":\"{}\",\"target\":\"{}\",\"asserted\":[{}],{}}}",
                esc(&st.from_state),
                esc(&st.guard),
                esc(&st.target),
                asserted.join(","),
                prov(&st.provenance)
            )
        })
        .collect();
    format!(
        "{{\"name\":\"{}\",\"outcome\":{},\"steps\":[{}],{}}}",
        esc(&sm.name),
        state_machine_outcome_json(&run.outcome, kb),
        steps.join(","),
        prov(&sm.provenance)
    )
}

/// Render a state-machine's typed terminal outcome (ADJ-STATEMACHINE §4) as JSON.
/// The `type` field is the stable discriminant a checker keys off.
fn state_machine_outcome_json(outcome: &StateMachineOutcome, kb: &KnowledgeBase) -> String {
    match outcome {
        StateMachineOutcome::Halted { state, result } => format!(
            "{{\"type\":\"halted\",\"state\":\"{}\",\"result\":{}}}",
            esc(state),
            yield_json(result, kb)
        ),
        StateMachineOutcome::StepBudgetExceeded {
            steps,
            budget,
            state,
        } => format!(
            "{{\"type\":\"step_budget_exceeded\",\"steps\":{},\"budget\":{},\"state\":\"{}\"}}",
            steps,
            budget,
            esc(state)
        ),
        StateMachineOutcome::NonTerminating { state } => format!(
            "{{\"type\":\"non_terminating\",\"state\":\"{}\"}}",
            esc(state)
        ),
        StateMachineOutcome::Stuck { state } => {
            format!("{{\"type\":\"stuck\",\"state\":\"{}\"}}", esc(state))
        }
    }
}

/// Render a halt's yielded value (ADJ-STATEMACHINE §3). A numeric yield carries its
/// exact-first value AND its derivation tree (byte-traceable exactly like a `let`);
/// a symbolic yield (`at_target`) is the bare finding name.
fn yield_json(y: &YieldValue, kb: &KnowledgeBase) -> String {
    match y {
        YieldValue::Numeric(d) => format!(
            "{{\"kind\":\"numeric\",\"value\":{},\"derivation\":{}}}",
            value_json(d),
            derivation_tree_json(&d.tree, kb)
        ),
        YieldValue::Symbol(s) => {
            format!("{{\"kind\":\"symbol\",\"symbol\":\"{}\"}}", esc(s))
        }
    }
}

/// True if a query goal contains a logic variable — i.e. it is a relational
/// recall *binding* query rather than a ground hypothesis query.
fn contains_var(t: &Term) -> bool {
    match t {
        Term::Var(_) => true,
        Term::Compound { args, .. } => args.iter().any(contains_var),
        _ => false,
    }
}

/// Collect the distinct logic variables of a goal, in first-appearance order, so
/// each binding can be labelled by the variable's surface name.
fn collect_vars(t: &Term, out: &mut Vec<LogicVar>) {
    match t {
        Term::Var(v) => {
            if !out.iter().any(|x| x == v) {
                out.push(v.clone());
            }
        }
        Term::Compound { args, .. } => {
            for a in args {
                collect_vars(a, out);
            }
        }
        _ => {}
    }
}

/// Resolve a relational recall binding query against the grounded knowledge graph
/// and render it as JSON: every answer (the variable bindings) plus the citing
/// edge's provenance — the proof. An empty answer set is honest **abstention**
/// (`"abstained": true`): no grounded edge supports an answer, so none is
/// fabricated. 0 answer-time model calls — pure SLD over the CAS-grounded facts.
fn recall_json(query: &Term, kb: &KnowledgeBase) -> String {
    let dag = enumerate_all(query, kb);
    let mut vars: Vec<LogicVar> = Vec::new();
    collect_vars(query, &mut vars);
    let mut answers: Vec<String> = Vec::new();
    for proof in &dag.proofs {
        let binds: Vec<String> = vars
            .iter()
            .map(|v| {
                let name = v.display_name.clone().unwrap_or_default();
                format!(
                    "\"{}\":\"{}\"",
                    esc(&name),
                    esc(&format!("{}", proof.bindings.walk_var(v)))
                )
            })
            .collect();
        // The citing edge(s): each fact this proof relied on, with its provenance.
        let cites: Vec<String> = proof
            .via_facts
            .iter()
            .filter_map(|fid| kb.fact(*fid))
            .map(|f| format!("{{{}}}", prov(&f.provenance)))
            .collect();
        // RS-4: `citations` is a SET (sorted, deduped `via_facts`) — it answers
        // "what did this rely on?" but cannot answer "in what order, and how?".
        // `steps` is the ordered derivation. Both are emitted: existing consumers
        // keep their field, auditors get the narrative.
        answers.push(format!(
            "{{\"bindings\":{{{}}},\"citations\":[{}],\"steps\":{}}}",
            binds.join(","),
            cites.join(","),
            trace_steps_json(proof, kb)
        ));
    }
    // An empty answer set is reported with the REASON it is empty. `truncated`
    // is checked first: a search that stopped early established no absence, so
    // calling it "no grounded support" would convert a resource limit into a
    // claim about the knowledge base.
    let abstention = if dag.proofs.is_empty() {
        let goal = format!("{query}");
        let reason = if dag.truncated {
            AbstentionReason::SearchLimitExceeded { goal }
        } else {
            AbstentionReason::NoGroundedSupport { goal }
        };
        format!(",\"abstention\":{}", abstention_json(&reason))
    } else {
        String::new()
    };
    format!(
        "{{\"query\":\"{}\",\"answers\":[{}],\"abstained\":{}{}}}",
        query_echo(&format!("{query}")),
        answers.join(","),
        dag.proofs.is_empty(),
        abstention
    )
}

/// Resolve a RANGE / BRACKET lookup (ADJ-TABLES RS-5c) against the grounded
/// table and render it as JSON. The table's rows are its facts, so enumerating
/// `<table>($c0, …, $cn)` binds every column of every row **and** yields that
/// row's citing fact (`via_facts`) — the same machinery exact recall uses. Among
/// the rows whose key column is `<= key_value`, the tactic selects the one with
/// the greatest key (the breakpoint the query falls in) and returns its value
/// column WITH that row's citation. The comparison rides the exact `BigRational`
/// order (`ExactRational::as_ratio()` — the identical total order the engine's
/// `CmpOp` exact path uses), so there is no `f64` hop in the decision. A query
/// below the smallest key has no key `<=` it and honestly **abstains** ("below
/// the table's domain"), never a fabricated classification. 0 answer-time model
/// calls — pure comparison over the CAS-grounded rows.
fn range_lookup_json(rl: &LoweredRangeLookup, kb: &KnowledgeBase) -> String {
    // Enumerate every row of the table by unifying an all-fresh-vars goal against
    // the table relation. `cols[i]` is bound, per proof, to row i's cell in
    // column i; the proof's `via_facts` is that row's citing fact.
    let cols: Vec<LogicVar> = (0..rl.arity).map(|i| var(&format!("c{i}"))).collect();
    let goal = compound(
        rl.table.clone(),
        cols.iter().map(|v| Term::Var(v.clone())).collect(),
    );
    let dag = enumerate_all(&goal, kb);

    let query_str = format!(
        "lookup {} {} = {} mode {} give {}",
        rl.table, rl.key_col, rl.key_value, rl.mode, rl.value_col
    );

    // The query value, as an exact rational — the right-hand side of every
    // breakpoint comparison.
    let q = match numeric_exact_magnitude(&rl.key_value) {
        Some(x) => x,
        None => {
            // The lowerer guarantees a numeric literal, so this is unreachable in
            // practice; abstain rather than panic if a non-numeric ever arrives.
            return format!(
                "{{\"query\":\"{}\",\"mode\":\"{}\",\"answers\":[],\"abstained\":true,\"abstention\":{}}}",
                query_echo(&query_str),
                esc(&rl.mode),
                abstention_json(&AbstentionReason::NonNumericKey {
                    table: rl.table.clone(),
                    column: rl.key_col.clone(),
                    key: format!("{}", rl.key_value),
                })
            );
        }
    };

    // Among all rows, keep those whose key column is `<= q`, then pick the one
    // with the GREATEST key — the breakpoint the query falls into. Comparison is
    // exact (`BigRational::cmp`), never `f64`.
    let mut best: Option<(&Proof, Term, Term, logic_engine::compute::ExactRational)> = None;
    for proof in &dag.proofs {
        let key_term = proof.bindings.walk_var(&cols[rl.key_index]);
        let Some(k) = numeric_exact_magnitude(&key_term) else {
            continue; // a non-numeric key cell is impossible post-lowering; skip defensively.
        };
        if k.as_ratio() > q.as_ratio() {
            continue; // key is above the query — not a candidate breakpoint.
        }
        let value_term = proof.bindings.walk_var(&cols[rl.value_index]);
        let take = match &best {
            None => true,
            Some((_, _, _, best_k)) => k.as_ratio() > best_k.as_ratio(),
        };
        if take {
            best = Some((proof, key_term, value_term, k));
        }
    }

    match best {
        Some((proof, key_term, value_term, _)) => {
            let cites: Vec<String> = proof
                .via_facts
                .iter()
                .filter_map(|fid| kb.fact(*fid))
                .map(|f| format!("{{{}}}", prov(&f.provenance)))
                .collect();
            // The answer names the value column (the binding) AND the matched
            // breakpoint key, so the audit shows WHICH bracket the query fell in.
            format!(
                "{{\"query\":\"{}\",\"mode\":\"{}\",\"answers\":[{{\"bindings\":{{\"{}\":\"{}\",\"{}\":\"{}\"}},\"citations\":[{}],\"steps\":{}}}],\"abstained\":false}}",
                query_echo(&query_str),
                esc(&rl.mode),
                esc(&rl.value_col),
                esc(&format!("{value_term}")),
                esc(&rl.key_col),
                esc(&format!("{key_term}")),
                cites.join(","),
                trace_steps_json(proof, kb)
            )
        }
        None => {
            // No breakpoint is `<= q`, so the query sits BELOW the table's
            // floor. Report the floor itself: an abstention that names the
            // domain you fell outside is actionable ("this source starts at
            // 0"), whereas a bare `true` leaves the caller guessing whether
            // the table is wrong, the key is wrong, or the source simply does
            // not reach that far.
            //
            // If the search truncated we cannot even claim that — we did not
            // enumerate every row, so the floor we computed may not be the
            // real one. That case reports the limit instead.
            let min_key = dag
                .proofs
                .iter()
                .filter_map(|pr| {
                    let kt = pr.bindings.walk_var(&cols[rl.key_index]);
                    numeric_exact_magnitude(&kt).map(|k| (kt, k))
                })
                .min_by(|(_, a), (_, b)| {
                    a.as_ratio()
                        .partial_cmp(b.as_ratio())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(kt, _)| format!("{kt}"))
                .unwrap_or_else(|| "(empty table)".to_string());
            let reason = if dag.truncated {
                AbstentionReason::SearchLimitExceeded {
                    goal: query_str.clone(),
                }
            } else {
                AbstentionReason::BelowTableDomain {
                    table: rl.table.clone(),
                    key: format!("{}", rl.key_value),
                    min_key,
                }
            };
            format!(
                "{{\"query\":\"{}\",\"mode\":\"{}\",\"answers\":[],\"abstained\":true,\"abstention\":{}}}",
                query_echo(&query_str),
                esc(&rl.mode),
                abstention_json(&reason)
            )
        }
    }
}

/// Resolve a NEAREST lookup (ADJ-TABLES RS-5f) against the grounded table and render
/// it as JSON. Where a `range` lookup returns the greatest key `<= q` (a step / floor)
/// and `interpolated` blends between the two bracketing rows, `nearest` snaps the query
/// to the single row whose key is CLOSEST to `q` — nearest-neighbour lookup. It returns
/// that row's value cell VERBATIM (like `range`, and unlike `interpolated`, so the value
/// column may hold a category label, not just a number), together with the matched key
/// and that one row's citation.
///
/// This is the tactic for tables where neither flooring nor linear blending is right:
/// snapping a measurement to the closest tabulated standard, nearest-rank selection, or
/// a discrete lookup grid where the between-points region has no defined value. The key
/// column must be numeric (enforced at lowering, shared with `range`/`interpolated`); the
/// value column is returned as-is.
///
/// Distance is exact: `|k - q|` is computed as a `BigRational` (`sub` then `abs`), never
/// via `f64`, and candidates are compared on that exact distance. **Ties break to the
/// SMALLER key**, deterministically — if `q` sits exactly halfway between two keys, the
/// lower key wins, so the answer is reproducible and never depends on row order.
///
/// Two honest edges:
/// - **empty table**: with no rows there is no nearest key, so it abstains
///   (`no_grounded_support`) rather than inventing one.
/// - **truncated search**: if enumeration hit a resolution limit we may not have seen the
///   truly nearest row, so snapping to the closest row we *did* see could be wrong; it
///   abstains with `search_limit_exceeded` instead of a possibly-non-nearest key.
///
/// 0 answer-time model calls — pure exact comparison over the CAS-grounded rows.
fn nearest_lookup_json(rl: &LoweredRangeLookup, kb: &KnowledgeBase) -> String {
    use logic_engine::compute::ExactRational;

    // Enumerate every row (same all-fresh-vars unification as the other tactics).
    let cols: Vec<LogicVar> = (0..rl.arity).map(|i| var(&format!("c{i}"))).collect();
    let goal = compound(
        rl.table.clone(),
        cols.iter().map(|v| Term::Var(v.clone())).collect(),
    );
    let dag = enumerate_all(&goal, kb);

    let query_str = format!(
        "lookup {} {} = {} mode {} give {}",
        rl.table, rl.key_col, rl.key_value, rl.mode, rl.value_col
    );

    let abstain = |reason: AbstentionReason| -> String {
        format!(
            "{{\"query\":\"{}\",\"mode\":\"{}\",\"answers\":[],\"abstained\":true,\"abstention\":{}}}",
            query_echo(&query_str),
            esc(&rl.mode),
            abstention_json(&reason)
        )
    };

    // The query value, as an exact rational.
    let q = match numeric_exact_magnitude(&rl.key_value) {
        Some(x) => x,
        None => {
            // The lowerer guarantees a numeric literal; abstain rather than panic.
            return abstain(AbstentionReason::NonNumericKey {
                table: rl.table.clone(),
                column: rl.key_col.clone(),
                key: format!("{}", rl.key_value),
            });
        }
    };

    // A truncated scan may have hidden the truly nearest row, so no row we saw can be
    // claimed nearest — abstain rather than snap to a possibly-non-nearest key.
    if dag.truncated {
        return abstain(AbstentionReason::SearchLimitExceeded {
            goal: query_str.clone(),
        });
    }

    // Among ALL rows, keep the one minimizing the exact distance `|k - q|`. Ties break
    // to the SMALLER key so the choice is deterministic and order-independent.
    let mut best: Option<(&Proof, Term, Term, ExactRational)> = None;
    for proof in &dag.proofs {
        let key_term = proof.bindings.walk_var(&cols[rl.key_index]);
        let Some(k) = numeric_exact_magnitude(&key_term) else {
            continue; // a non-numeric key cell is impossible post-lowering; skip defensively.
        };
        let value_term = proof.bindings.walk_var(&cols[rl.value_index]);
        let take = match &best {
            None => true,
            Some((_, _, _, best_k)) => {
                // `d_*` are exact `BigRational` distances; comparison is exact.
                let d_new = k.as_ratio().sub(q.as_ratio()).abs();
                let d_best = best_k.as_ratio().sub(q.as_ratio()).abs();
                d_new < d_best || (d_new == d_best && k.as_ratio() < best_k.as_ratio())
            }
        };
        if take {
            best = Some((proof, key_term, value_term, k));
        }
    }

    match best {
        Some((proof, key_term, value_term, _)) => {
            let cites: Vec<String> = proof
                .via_facts
                .iter()
                .filter_map(|fid| kb.fact(*fid))
                .map(|f| format!("{{{}}}", prov(&f.provenance)))
                .collect();
            // The answer names the value column (the binding) AND the matched key, so
            // the audit shows WHICH row the query snapped to.
            format!(
                "{{\"query\":\"{}\",\"mode\":\"{}\",\"answers\":[{{\"bindings\":{{\"{}\":\"{}\",\"{}\":\"{}\"}},\"citations\":[{}],\"steps\":{}}}],\"abstained\":false}}",
                query_echo(&query_str),
                esc(&rl.mode),
                esc(&rl.value_col),
                esc(&format!("{value_term}")),
                esc(&rl.key_col),
                esc(&format!("{key_term}")),
                cites.join(","),
                trace_steps_json(proof, kb)
            )
        }
        // No rows at all — there is no nearest key to snap to.
        None => abstain(AbstentionReason::NoGroundedSupport {
            goal: query_str.clone(),
        }),
    }
}

/// Resolve an INTERPOLATED lookup (ADJ-TABLES RS-5d) against the grounded table and
/// render it as JSON. Where a `range` lookup reads the table as a *step* function,
/// `interpolated` reads it as a *piecewise-linear* one: it finds the two breakpoint
/// rows that bracket the query — the greatest key `k0 <= q` and the smallest key
/// `k1 >= q` — and returns the exact linear blend
///
/// ```text
///     v = v0 + (v1 - v0) * (q - k0) / (k1 - k0)
/// ```
///
/// with BOTH bracketing rows' citations riding along, so the interpolated answer is
/// traceable to the two measured points it sits between (nomograms, growth charts,
/// calibration curves). Every step is exact `BigRational` arithmetic — no `f64` hop —
/// so a terminating blend renders all its digits and a repeating one renders as the
/// reduced fraction, never a rounded float.
///
/// Three honest edges:
/// - **exact hit** (`q` equals a breakpoint key, so `k0 == k1`): the blend is
///   degenerate (`0/0`), so it is short-circuited to that row's value with its single
///   citation — no fabricated division.
/// - **below / above the domain**: interpolation needs a breakpoint on *both* sides;
///   outside `[min, max]` it abstains (`below_table_domain` / `above_table_domain`)
///   rather than extrapolate past what the source measured.
/// - **truncated search**: if enumeration hit a resolution limit we may not have seen
///   the true bracketing rows, so any interpolation could be wrong; we abstain with
///   `search_limit_exceeded` instead of blending against a possibly-incomplete scan.
///
/// 0 answer-time model calls — pure exact arithmetic over the CAS-grounded rows.
fn interpolated_lookup_json(rl: &LoweredRangeLookup, kb: &KnowledgeBase) -> String {
    use logic_engine::compute::ExactRational;

    let cols: Vec<LogicVar> = (0..rl.arity).map(|i| var(&format!("c{i}"))).collect();
    let goal = compound(
        rl.table.clone(),
        cols.iter().map(|v| Term::Var(v.clone())).collect(),
    );
    let dag = enumerate_all(&goal, kb);

    let query_str = format!(
        "lookup {} {} = {} mode {} give {}",
        rl.table, rl.key_col, rl.key_value, rl.mode, rl.value_col
    );

    // Render an exact rational for a JSON string binding: prefer a terminating
    // decimal (all digits), else the reduced fraction (still exact, never a float).
    let render = |x: &ExactRational| -> String {
        x.to_exact_decimal_string()
            .unwrap_or_else(|| format!("{}/{}", x.numerator(), x.denominator()))
    };
    let cites_of = |proof: &Proof| -> Vec<String> {
        proof
            .via_facts
            .iter()
            .filter_map(|fid| kb.fact(*fid))
            .map(|f| format!("{{{}}}", prov(&f.provenance)))
            .collect()
    };
    let abstain = |reason: AbstentionReason| -> String {
        format!(
            "{{\"query\":\"{}\",\"mode\":\"{}\",\"answers\":[],\"abstained\":true,\"abstention\":{}}}",
            query_echo(&query_str),
            esc(&rl.mode),
            abstention_json(&reason)
        )
    };

    let q = match numeric_exact_magnitude(&rl.key_value) {
        Some(x) => x,
        None => {
            return abstain(AbstentionReason::NonNumericKey {
                table: rl.table.clone(),
                column: rl.key_col.clone(),
                key: format!("{}", rl.key_value),
            })
        }
    };

    // A truncated scan may have missed the true bracketing rows, which would make
    // any interpolation wrong — abstain rather than blend against a partial table.
    if dag.truncated {
        return abstain(AbstentionReason::SearchLimitExceeded {
            goal: query_str.clone(),
        });
    }

    // Scan every row once, keeping the tightest bracket on each side of `q`:
    // `lower` = the row with the greatest key `<= q`; `upper` = the row with the
    // smallest key `>= q`. Comparison and storage are exact.
    let mut lower: Option<(&Proof, ExactRational, ExactRational)> = None;
    let mut upper: Option<(&Proof, ExactRational, ExactRational)> = None;
    for proof in &dag.proofs {
        let key_term = proof.bindings.walk_var(&cols[rl.key_index]);
        let Some(k) = numeric_exact_magnitude(&key_term) else {
            continue; // non-numeric key is impossible post-lowering; skip defensively.
        };
        let value_term = proof.bindings.walk_var(&cols[rl.value_index]);
        let Some(v) = numeric_exact_magnitude(&value_term) else {
            continue; // non-numeric value is impossible post-lowering (checked); skip.
        };
        if k.as_ratio() <= q.as_ratio() {
            let take = match &lower {
                None => true,
                Some((_, lk, _)) => k.as_ratio() > lk.as_ratio(),
            };
            if take {
                lower = Some((proof, k.clone(), v.clone()));
            }
        }
        if k.as_ratio() >= q.as_ratio() {
            let take = match &upper {
                None => true,
                Some((_, uk, _)) => k.as_ratio() < uk.as_ratio(),
            };
            if take {
                upper = Some((proof, k, v));
            }
        }
    }

    // The min/max keys, for an out-of-domain abstention's audit payload.
    let extremal = |pick_max: bool| -> String {
        dag.proofs
            .iter()
            .filter_map(|pr| numeric_exact_magnitude(&pr.bindings.walk_var(&cols[rl.key_index])))
            .fold(None, |acc: Option<ExactRational>, k| match acc {
                None => Some(k),
                Some(a) => {
                    let keep = if pick_max {
                        k.as_ratio() > a.as_ratio()
                    } else {
                        k.as_ratio() < a.as_ratio()
                    };
                    Some(if keep { k } else { a })
                }
            })
            .map(|k| render(&k))
            .unwrap_or_else(|| "(empty table)".to_string())
    };

    match (lower, upper) {
        // Below the lowest breakpoint — nothing to interpolate down toward.
        (None, _) => abstain(AbstentionReason::BelowTableDomain {
            table: rl.table.clone(),
            key: format!("{}", rl.key_value),
            min_key: extremal(false),
        }),
        // Above the highest breakpoint — nothing to interpolate up toward.
        (_, None) => abstain(AbstentionReason::AboveTableDomain {
            table: rl.table.clone(),
            key: format!("{}", rl.key_value),
            max_key: extremal(true),
        }),
        (Some((lp, k0, v0)), Some((up, k1, v1))) => {
            // Exact hit on a breakpoint (`k0 == k1 == q`): the blend is `0/0`, so
            // return that row's value verbatim with its single citation.
            if k0.as_ratio() == k1.as_ratio() {
                let cites = cites_of(lp);
                return format!(
                    "{{\"query\":\"{}\",\"mode\":\"{}\",\"answers\":[{{\"bindings\":{{\"{}\":\"{}\",\"{}\":\"{}\"}},\"brackets\":{{\"exact\":{{\"{}\":\"{}\",\"{}\":\"{}\"}}}},\"citations\":[{}],\"steps\":{}}}],\"abstained\":false}}",
                    query_echo(&query_str),
                    esc(&rl.mode),
                    esc(&rl.value_col),
                    esc(&render(&v0)),
                    esc(&rl.key_col),
                    esc(&render(&q)),
                    esc(&rl.key_col),
                    esc(&render(&k0)),
                    esc(&rl.value_col),
                    esc(&render(&v0)),
                    cites.join(","),
                    trace_steps_json(lp, kb)
                );
            }
            // Linear blend, all exact: v = v0 + (v1 - v0) * (q - k0) / (k1 - k0).
            // The denominator is non-zero (k1 > k0 here), so every step is defined.
            let blended = (|| {
                let dv = v1.sub(&v0)?;
                let dq = q.sub(&k0)?;
                let dk = k1.sub(&k0)?;
                let frac = dq.div(&dk)?;
                let scaled = dv.mul(&frac)?;
                v0.add(&scaled)
            })();
            let v = match blended {
                Some(v) => v,
                None => {
                    // Exact arithmetic only fails on a zero denominator, already
                    // excluded above; abstain rather than emit a wrong number.
                    return abstain(AbstentionReason::SearchLimitExceeded {
                        goal: query_str.clone(),
                    });
                }
            };
            let mut cites = cites_of(lp);
            cites.extend(cites_of(up));
            format!(
                "{{\"query\":\"{}\",\"mode\":\"{}\",\"answers\":[{{\"bindings\":{{\"{}\":\"{}\",\"{}\":\"{}\"}},\"brackets\":{{\"lower\":{{\"{}\":\"{}\",\"{}\":\"{}\"}},\"upper\":{{\"{}\":\"{}\",\"{}\":\"{}\"}}}},\"citations\":[{}]}}],\"abstained\":false}}",
                query_echo(&query_str),
                esc(&rl.mode),
                esc(&rl.value_col),
                esc(&render(&v)),
                esc(&rl.key_col),
                esc(&render(&q)),
                esc(&rl.key_col),
                esc(&render(&k0)),
                esc(&rl.value_col),
                esc(&render(&v0)),
                esc(&rl.key_col),
                esc(&render(&k1)),
                esc(&rl.value_col),
                esc(&render(&v1)),
                cites.join(",")
            )
        }
    }
}

/// Render the ADJ73 *governance* of a binding query (defeasible precedence): every distinct
/// answer tagged `governing` / `defeated` (by which term) / `conflict_peer`, plus its precedence
/// standing. For a predicate that is NOT declared `functional`, every answer is `governing`
/// (no conflict) — so this section is the precedence-resolved view, alongside the raw `recall`.
/// 0 answer-time model calls (pure SLD + a resolution post-pass over the grounded graph).
fn governing_json(query: &Term, kb: &KnowledgeBase) -> String {
    let res = enumerate_governing(query, kb);
    let mut vars: Vec<LogicVar> = Vec::new();
    collect_vars(query, &mut vars);
    let answers: Vec<String> = res
        .answers
        .iter()
        .map(|a| {
            // Bindings come from one representative proof of this answer (all proofs of a
            // distinct answer share the same variable bindings by construction).
            let binds: Vec<String> = a
                .proof_indices
                .first()
                .map(|&i| {
                    let proof = &res.dag.proofs[i];
                    vars.iter()
                        .map(|v| {
                            format!(
                                "\"{}\":\"{}\"",
                                esc(&v.display_name.clone().unwrap_or_default()),
                                esc(&format!("{}", proof.bindings.walk_var(v)))
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let (status, defeated_by) = match &a.status {
                GovernStatus::Governing => ("governing", String::new()),
                GovernStatus::ConflictPeer => ("conflict_peer", String::new()),
                GovernStatus::Defeated { by } => (
                    "defeated",
                    format!(",\"defeated_by\":\"{}\"", esc(&format!("{}", by))),
                ),
            };
            let standing = match a.priority {
                Standing::Asserted => "asserted".to_string(),
                Standing::Rule(p) => format!("{p:?}").to_lowercase(),
            };
            // ADJ73 PR-B: surface the grounded CONTEXT this answer is decided in (the context of
            // its highest-standing deriving rule), so the audit reader sees *which* context
            // governed (federal vs state, ninth_circuit vs district_court) — not just that one
            // term beat another. Omitted for a context-free derivation.
            let context = match &a.context {
                Some(c) => format!(",\"context\":\"{}\"", esc(c)),
                None => String::new(),
            };
            format!(
                "{{\"term\":\"{}\",\"bindings\":{{{}}},\"status\":\"{}\",\"standing\":\"{}\"{}{}}}",
                esc(&format!("{}", a.term)),
                binds.join(","),
                status,
                standing,
                context,
                defeated_by
            )
        })
        .collect();
    // `has_conflict()` answers "did I SEE a conflict?". On a truncated search
    // that is not the same question as "is there one?" — reporting `false`
    // there would be an affirmative claim reached by failing to find a rival,
    // which proves nothing if the search gave up before looking. This is the
    // same laundering PR-C removes from recall and lookup; leaving it in one
    // renderer would have kept the hole open.
    let (conflict, abstention) = match res.conflict_status() {
        logic_engine::ConflictStatus::Conflict => ("true", String::new()),
        logic_engine::ConflictStatus::NoConflict => ("false", String::new()),
        logic_engine::ConflictStatus::Unknown => (
            "null",
            format!(
                ",\"abstention\":{}",
                abstention_json(&AbstentionReason::SearchLimitExceeded {
                    goal: format!("{query}"),
                })
            ),
        ),
    };
    format!(
        "{{\"query\":\"{}\",\"answers\":[{}],\"has_conflict\":{}{}}}",
        query_echo(&format!("{query}")),
        answers.join(","),
        conflict,
        abstention
    )
}

/// Render a [`SolveOutcome`] as JSON. A solved system lists each unknown's
/// value plus the constraints that determined it (provenance); an
/// out-of-scope or singular system reports why, never a fabricated answer.
fn solve_json(outcome: &SolveOutcome) -> String {
    match outcome {
        SolveOutcome::Solved {
            assignments,
            from_constraints,
        } => {
            let vars: Vec<String> = assignments
                .iter()
                .map(|(name, value)| {
                    format!("{{\"name\":\"{}\",\"value\":{}}}", esc(name), jnum(*value))
                })
                .collect();
            let cites: Vec<String> = from_constraints.iter().map(|i| i.to_string()).collect();
            format!(
                "{{\"outcome\":\"solved\",\"assignments\":[{}],\"from_constraints\":[{}]}}",
                vars.join(","),
                cites.join(",")
            )
        }
        SolveOutcome::SolvedRoots {
            var,
            roots,
            from_constraints,
        } => {
            let rs: Vec<String> = roots.iter().map(|r| jnum(*r)).collect();
            let cites: Vec<String> = from_constraints.iter().map(|i| i.to_string()).collect();
            format!(
                "{{\"outcome\":\"solved_roots\",\"var\":\"{}\",\"roots\":[{}],\"from_constraints\":[{}]}}",
                esc(var),
                rs.join(","),
                cites.join(",")
            )
        }
        SolveOutcome::NoUniqueSolution => "{\"outcome\":\"no_unique_solution\"}".to_string(),
        SolveOutcome::Unsupported { reason } => {
            format!(
                "{{\"outcome\":\"unsupported\",\"reason\":\"{}\"}}",
                esc(reason)
            )
        }
    }
}

/// Render a [`FeasibilityOutcome`] as JSON. `sat` carries an **integer** witness
/// (from the linear-integer tactic); `sat_real` carries a **rational** witness
/// rendered as numbers (from the Fourier–Motzkin / QF_LRA layer, track C1);
/// `unsat` carries the indices of the constraints whose conjunction is
/// contradictory; `unknown` reports why feasibility could not be decided (a
/// `!=`, a nonlinear term, or a system too large for the bounded slice).
fn check_json(outcome: &FeasibilityOutcome) -> String {
    match outcome {
        FeasibilityOutcome::Sat { assignments } => {
            let vars: Vec<String> = assignments
                .iter()
                .map(|(name, value)| format!("{{\"name\":\"{}\",\"value\":{}}}", esc(name), value))
                .collect();
            format!(
                "{{\"outcome\":\"sat\",\"assignments\":[{}]}}",
                vars.join(",")
            )
        }
        FeasibilityOutcome::SatReal { assignments } => {
            let vars: Vec<String> = assignments
                .iter()
                .map(|(name, value)| {
                    format!("{{\"name\":\"{}\",\"value\":{}}}", esc(name), jnum(*value))
                })
                .collect();
            format!(
                "{{\"outcome\":\"sat_real\",\"assignments\":[{}]}}",
                vars.join(",")
            )
        }
        FeasibilityOutcome::Unsat { core } => {
            let idx: Vec<String> = core.iter().map(|i| i.to_string()).collect();
            format!("{{\"outcome\":\"unsat\",\"core\":[{}]}}", idx.join(","))
        }
        FeasibilityOutcome::Unknown { reason } => {
            format!("{{\"outcome\":\"unknown\",\"reason\":\"{}\"}}", esc(reason))
        }
    }
}

/// Render an [`OptimizeOutcome`] (a `minimize`/`maximize` LP result) as JSON.
/// `optimal` carries the optimal `value`, the achieving `assignments`, and the
/// `binding` constraint indices (the provenance of the bound); `unbounded` /
/// `infeasible` / `unknown` report the degenerate cases without a fake number.
fn optimize_json(outcome: &OptimizeOutcome) -> String {
    match outcome {
        OptimizeOutcome::Optimal {
            value,
            assignments,
            binding,
        } => {
            let vars: Vec<String> = assignments
                .iter()
                .map(|(name, v)| format!("{{\"name\":\"{}\",\"value\":{}}}", esc(name), jnum(*v)))
                .collect();
            let bind: Vec<String> = binding.iter().map(|i| i.to_string()).collect();
            format!(
                "{{\"outcome\":\"optimal\",\"value\":{},\"assignments\":[{}],\"binding\":[{}]}}",
                jnum(*value),
                vars.join(","),
                bind.join(",")
            )
        }
        OptimizeOutcome::Unbounded => "{\"outcome\":\"unbounded\"}".to_string(),
        OptimizeOutcome::Infeasible { core } => {
            let idx: Vec<String> = core.iter().map(|i| i.to_string()).collect();
            format!(
                "{{\"outcome\":\"infeasible\",\"core\":[{}]}}",
                idx.join(",")
            )
        }
        OptimizeOutcome::Unknown { reason } => {
            format!("{{\"outcome\":\"unknown\",\"reason\":\"{}\"}}", esc(reason))
        }
    }
}

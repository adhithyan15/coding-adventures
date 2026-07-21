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
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cli_builder::types::ParserOutput;
use cli_builder::{load_spec_from_str, Parser};

use adj_constraint_solver::{
    check, optimize, solve, FeasibilityOutcome, OptimizeOutcome, SolveOutcome,
};
use adj_lang::{compile_with_imports, decide, ImportLimits, ImportProvider, LoweredRangeLookup};
use logic_core::{atom, compound, var, LogicVar, Term};
use logic_engine::govern::Standing;
use logic_engine::{
    enumerate_all, enumerate_governing, numeric_exact_magnitude, DerivationOrigin,
    DifferentialDecision, Fact, GovernStatus, KnowledgeBase, LRAggregateResult, Proof, Provenance,
    TrustTier,
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
struct FsProvider {
    /// The sandbox root: canonicalized directory of the top-level program. No
    /// import may resolve outside this subtree.
    root: PathBuf,
}

impl FsProvider {
    /// Canonicalize `p` and confirm it lies within the sandbox `root`.
    fn checked_canonical(&self, p: &Path) -> Result<String, String> {
        let abs = fs::canonicalize(p).map_err(|e| format!("{}: {e}", p.display()))?;
        if !abs.starts_with(&self.root) {
            return Err(format!(
                "{} escapes the import root {}",
                abs.display(),
                self.root.display()
            ));
        }
        Ok(abs.to_string_lossy().into_owned())
    }
}

impl ImportProvider for FsProvider {
    fn resolve(&self, importer: &str, literal: &str) -> Result<String, String> {
        if Path::new(literal).is_absolute() {
            return Err(format!("import path must be relative, got {literal:?}"));
        }
        // Reject NUL and other obviously hostile bytes before touching the FS.
        if literal.contains('\0') {
            return Err("import path contains a NUL byte".to_string());
        }
        let importer_dir = Path::new(importer)
            .parent()
            .ok_or_else(|| format!("importer {importer:?} has no parent directory"))?;
        self.checked_canonical(&importer_dir.join(literal))
    }

    fn load(&self, canonical: &str) -> Result<String, String> {
        // `canonical` came from `resolve`/the root, already inside `root`; read
        // it. (Re-checking would re-canonicalize an already-canonical path.)
        fs::read_to_string(canonical).map_err(|e| format!("{canonical}: {e}"))
    }
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
    let queries: Vec<String> = all_query_strs;

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

    // ADJ-TABLES RS-5c: range / bracket lookups over `table`s read as step
    // functions. Each resolves to its bracketed value + the matched breakpoint
    // row's citation (or abstains below the table's domain). 0 answer-time model
    // calls — exact comparison over the CAS-grounded rows. Omitted when the
    // program declares no `? lookup … mode range …`, so existing output is
    // byte-for-byte unchanged.
    let lookups: Vec<String> = lowered
        .range_lookups
        .iter()
        .map(|rl| range_lookup_json(rl, &lowered.kb))
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

    println!(
        "{{\"queries\":[{}],\"ranked\":[{}],\"decision\":{}{}{}{}{}{}{}{}}}",
        queries.join(","),
        ranked.join(","),
        decision,
        solve_section,
        check_section,
        optimize_section,
        recall_section,
        governing_section,
        lookup_section,
        derived_section
    );
    ExitCode::SUCCESS
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
    format!(
        "{{\"query\":\"{}\",\"answers\":[{}],\"abstained\":{}}}",
        esc(&format!("{}", query)),
        answers.join(","),
        dag.proofs.is_empty()
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
                "{{\"query\":\"{}\",\"mode\":\"{}\",\"answers\":[],\"abstained\":true}}",
                esc(&query_str),
                esc(&rl.mode)
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
                esc(&query_str),
                esc(&rl.mode),
                esc(&rl.value_col),
                esc(&format!("{value_term}")),
                esc(&rl.key_col),
                esc(&format!("{key_term}")),
                cites.join(","),
                trace_steps_json(proof, kb)
            )
        }
        None => format!(
            "{{\"query\":\"{}\",\"mode\":\"{}\",\"answers\":[],\"abstained\":true}}",
            esc(&query_str),
            esc(&rl.mode)
        ),
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
    format!(
        "{{\"query\":\"{}\",\"answers\":[{}],\"has_conflict\":{}}}",
        esc(&format!("{}", query)),
        answers.join(","),
        res.has_conflict()
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

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
use adj_lang::{compile_with_imports, decide, ImportLimits, ImportProvider};
use logic_core::{atom, LogicVar, Term};
use logic_engine::{
    enumerate_all, DerivationOrigin, DifferentialDecision, Fact, KnowledgeBase, LRAggregateResult,
    Provenance, TrustTier,
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
                        let ev = format!("{}", c.evidence_term);
                        // E3: if this contribution fired from a constraint STATUS
                        // atom, descend into the solver certificate beneath it.
                        let solver = certs
                            .iter()
                            .find(|(k, _)| *k == ev.as_str())
                            .map(|(_, cert)| format!(",\"solver\":{}", cert))
                            .unwrap_or_default();
                        steps.push(format!(
                            "{{\"kind\":\"contribution\",\"evidence\":\"{}\",\"logit\":{},{}{}}}",
                            esc(&ev),
                            jnum(*logit_delta),
                            prov(&c.provenance),
                            solver
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
    let mut add = |out: &mut Vec<(&'static str, String)>, s: &'static str, cert: String| {
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

    println!(
        "{{\"queries\":[{}],\"ranked\":[{}],\"decision\":{}{}{}{}{}}}",
        queries.join(","),
        ranked.join(","),
        decision,
        solve_section,
        check_section,
        optimize_section,
        recall_section
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
        answers.push(format!(
            "{{\"bindings\":{{{}}},\"citations\":[{}]}}",
            binds.join(","),
            cites.join(",")
        ));
    }
    format!(
        "{{\"query\":\"{}\",\"answers\":[{}],\"abstained\":{}}}",
        esc(&format!("{}", query)),
        answers.join(","),
        dag.proofs.is_empty()
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

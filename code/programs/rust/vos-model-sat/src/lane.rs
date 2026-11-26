//! # The lane: solve, certify, control, witness, collapse, and the committed table
//!
//! VOS-040b (VOS31 sections 7 to 9).  [`analyse`] is the whole of `gen` and `check` short of
//! touching the disk: it reads the contract crate's files, runs every gate, and either returns
//! the table text or a list of [`Finding`]s (`MS01` to `MS10`).  Nothing is trusted from a
//! committed file: the table is regenerated and compared, never read as a status.
//!
//! ```text
//!   contract crate files ──> MS01 inputs ──> MS02 cross-checks (VOS31 section 6)
//!                                  │
//!                                  v   for each lemma of lemmas.part, for each N of the ladder
//!    ┌──────────────────────────────────────────────────────────────────────────────────┐
//!    │ honest encoder:  assert NOT holds -> CNF -> solve with proof                     │
//!    │     SAT / over budget                      MS03 / MS05                           │
//!    │     DRAT rejected by the default limits    MS04                                  │
//!    │     check_work over 2^31                   MS05                                  │
//!    │ collapse re-derived (constant-fold from the AIG, propagation-only from the CNF)  │
//!    │     UNSAT without conflicts and no reason  MS08                                  │
//!    │ mutant encoder: assert NOT holds -> must be SAT, model replayed                  │
//!    │     UNSAT / unreplayable                   MS06                                  │
//!    │ implication lemmas (read from the AST): premise alone must be SAT, replayed      │
//!    │     not witnessed                          MS07                                  │
//!    └──────────────────────────────────────────────────────────────────────────────────┘
//!                                  │ no finding at all
//!                                  v
//!                       the table text (solver-independent fields only)
//!                                  │ `check` only
//!                                  v
//!                MS09 compared byte for byte with warrant/lemmas.sat
//! ```
//!
//! ## What is committed and what is gated (VOS31 8.1)
//!
//! The table holds only fields that depend on no solver: sizes, the CNF digest, verdicts, the
//! collapse reason and the control and witness verdicts.  Conflicts, `check_work` and
//! acceptance by the checker are **gated** (they fail the run when out of bounds) and printed in
//! the uncommitted report, but no table byte depends on them.
//!
//! ## A label is earned, never typed
//!
//! The labels of [`crate::labels`] say "SAT-certified".  [`Row`] is only built, at the end of
//! [`obligation`], after a DRAT proof was accepted under the default limits **and** the control
//! passed; [`certified_label`] is the one place in the crate that reads a label for a row, and a
//! test scans the sources to keep it so.

use std::collections::BTreeSet;

use bit_blaster::aig::{Aig, Edge};
use bit_blaster::tseitin::Tseitin;
use cdcl_sat::{write_dimacs, Cnf};
use coding_adventures_sha256::sha256_hex;

use crate::crosscheck::Evidence;
use crate::labels::{label_collapsed, label_for, label_token, CollapseReason, LABEL_TRIVIAL};
use crate::lemmas::{lemma_circuit, LemmaCircuit, LemmaId};
use crate::model::{Encoder, SchedModel};
use crate::mutants::Mutant;
use crate::oracle::{claims_of, ContractFiles};
use crate::solve::{
    check_default, model_satisfies, propagation_refutes, solve_cnf, Budget, Verdict,
};

/// The width ladder of VOS31 section 4 (owner decision, 2026-11-25).
pub const LADDER: [usize; 4] = [8, 16, 32, 64];

/// The first line of the report of a run that skipped the `vos.toml` opt-in check (the
/// `--allow-missing-sat-key` bridge of VOS-040b).  The table is pinned byte for byte by VOS31 8.2,
/// so the marker is in the report only: a bridged table is identical to an unbridged one, and the
/// report and standard error are where the bridge is visible.
pub const BRIDGED_REPORT_LINE: &str = "BRIDGED RUN: the vos.toml opt-in check (MS01) was skipped (--allow-missing-sat-key, VOS-040b; removed by VOS-040c)";

/// The most lemmas a contract may have (the limit `check-contract` enforces: CK03).
pub const MAX_LEMMAS: usize = 64;

/// One rule failure: a code `MS01` to `MS10`, and what is wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// `MS01` to `MS10` (VOS31 section 9).
    pub code: &'static str,
    /// What is wrong, naming the lemma and width where there is one.
    pub message: String,
}

impl Finding {
    fn new(code: &'static str, message: impl Into<String>) -> Finding {
        Finding {
            code,
            message: message.into(),
        }
    }
}

/// How the lane builds its encoders and which bounds it enforces.  [`LaneConfig::real`] is the
/// only configuration the CLI uses.  The fields are private to this module: the hooks that let a
/// negative control hand the lane a wrong encoder, a wrong budget or a corrupted proof exist
/// only behind the `test-hooks` cargo feature (enabled for this crate's own tests through a
/// dev-dependency), so no released build can reach them.
pub struct LaneConfig<'a> {
    ladder: &'a [usize],
    budget: Budget,
    honest: &'a dyn Fn(usize) -> Box<dyn SchedModel>,
    mutant: &'a dyn Fn(LemmaId, usize) -> Box<dyn SchedModel>,
    require_opt_in: bool,
    keep_proofs: bool,
    proof_hook: &'a dyn Fn(&str) -> String,
}

fn real_honest(n: usize) -> Box<dyn SchedModel> {
    Box::new(Encoder { n })
}

fn real_mutant(lemma: LemmaId, n: usize) -> Box<dyn SchedModel> {
    Box::new(Mutant { lemma, n })
}

fn identity(s: &str) -> String {
    s.to_string()
}

impl LaneConfig<'static> {
    /// The real lane: the ladder, the route budget, the honest encoder and the real mutants;
    /// `vos.toml` must opt in; DIMACS and DRAT texts are not kept.
    pub fn real() -> LaneConfig<'static> {
        LaneConfig {
            ladder: &LADDER,
            budget: Budget::ROUTE,
            honest: &real_honest,
            mutant: &real_mutant,
            require_opt_in: true,
            keep_proofs: false,
            proof_hook: &identity,
        }
    }
}

impl<'a> LaneConfig<'a> {
    /// Keep each obligation's DIMACS and DRAT text in [`Analysis::proofs`] (for `--emit-drat`).
    /// Off by default: the texts are large and are dropped as soon as they are checked.
    pub fn keep_proofs(mut self) -> LaneConfig<'a> {
        self.keep_proofs = true;
        self
    }

    /// The `--allow-missing-sat-key` bridge: do not require `sat = "model"` in `vos.toml`.
    /// Until VOS-040c teaches `vos-link` the key its loader refuses a `vos.toml` that carries
    /// it.  A bridged run says so in its report and on standard error; VOS-040c deletes this.
    pub fn bridge_missing_sat_key(mut self) -> LaneConfig<'a> {
        self.require_opt_in = false;
        self
    }
}

/// Test-only hooks (cargo feature `test-hooks`): each returns a config with one part replaced.
#[cfg(feature = "test-hooks")]
impl<'a> LaneConfig<'a> {
    /// Solve at these widths.
    pub fn with_ladder<'b>(self, ladder: &'b [usize]) -> LaneConfig<'b>
    where
        'a: 'b,
    {
        LaneConfig { ladder, ..self }
    }

    /// Enforce this budget.
    pub fn with_budget(mut self, budget: Budget) -> LaneConfig<'a> {
        self.budget = budget;
        self
    }

    /// Use this honest encoder.
    pub fn with_honest<'b>(self, f: &'b dyn Fn(usize) -> Box<dyn SchedModel>) -> LaneConfig<'b>
    where
        'a: 'b,
    {
        LaneConfig { honest: f, ..self }
    }

    /// Use this mutant (control) encoder.
    pub fn with_mutant<'b>(
        self,
        f: &'b dyn Fn(LemmaId, usize) -> Box<dyn SchedModel>,
    ) -> LaneConfig<'b>
    where
        'a: 'b,
    {
        LaneConfig { mutant: f, ..self }
    }

    /// Rewrite every DRAT text before it is checked.
    pub fn with_proof_hook<'b>(self, f: &'b dyn Fn(&str) -> String) -> LaneConfig<'b>
    where
        'a: 'b,
    {
        LaneConfig {
            proof_hook: f,
            ..self
        }
    }

    /// Require (or not) the `sat = "model"` opt-in.
    pub fn with_require_opt_in(mut self, yes: bool) -> LaneConfig<'a> {
        self.require_opt_in = yes;
        self
    }
}

/// One certified obligation: lemma L at width N.  Its fields are private to this module and the
/// only constructor call is at the end of [`obligation`], so a row (and the label it carries)
/// exists only after every gate passed.
#[derive(Clone, Debug)]
pub struct Row {
    lemma: LemmaId,
    n: usize,
    and_count: usize,
    vars: usize,
    clauses: usize,
    cnf_sha256: String,
    collapsed: Option<CollapseReason>,
    premise_witnessed: bool,
    label: String,
    conflicts: u64,
    check_work: u64,
}

impl Row {
    /// The lemma.
    pub fn lemma(&self) -> LemmaId {
        self.lemma
    }
    /// The width.
    pub fn n(&self) -> usize {
        self.n
    }
    /// CNF variables.
    pub fn vars(&self) -> usize {
        self.vars
    }
    /// CNF clauses.
    pub fn clauses(&self) -> usize {
        self.clauses
    }
    /// The re-derived collapse reason, if the obligation collapsed.
    pub fn collapsed(&self) -> Option<CollapseReason> {
        self.collapsed
    }
    /// Whether a premise witness was produced.
    pub fn premise_witnessed(&self) -> bool {
        self.premise_witnessed
    }
    /// The label (VOS31 section 11).
    pub fn label(&self) -> &str {
        &self.label
    }
    /// Gated, never committed: conflicts the solver used.
    pub fn conflicts(&self) -> u64 {
        self.conflicts
    }
    /// Gated, never committed: the checker's work.
    pub fn check_work(&self) -> u64 {
        self.check_work
    }
}

/// The label of a certified row: the one place a label is chosen.  Called only from
/// [`obligation`] after the proof was accepted and the control passed.
fn certified_label(lemma: LemmaId, n: usize, collapsed: Option<CollapseReason>) -> String {
    match (lemma, collapsed) {
        (LemmaId::ClosureAtM, Some(CollapseReason::ConstantFold)) => LABEL_TRIVIAL.to_string(),
        (_, Some(reason)) => label_collapsed(n, reason),
        (_, None) => label_for(n),
    }
}

impl Row {
    /// A row is counted unless it is the trivial lemma (VOS31 7.5).
    pub fn counted(&self) -> bool {
        self.label != LABEL_TRIVIAL
    }
}

/// What [`analyse`] found.
#[derive(Debug, Default)]
pub struct Analysis {
    /// Every finding; empty means every gate passed.
    pub findings: Vec<Finding>,
    /// The uncommitted report: one line per obligation, with the gated numbers.
    pub report: Vec<String>,
    /// The table text, present only when there is no finding.
    pub table: Option<String>,
    /// `(file stem, DIMACS, DRAT)` of each certified obligation, for `--emit-drat`.
    pub proofs: Vec<(String, String, String)>,
    /// The certified rows, for the tests and the report (empty when there is a finding).
    pub rows: Vec<Row>,
}

// ---------------------------------------------------------------------------------------
// One obligation
// ---------------------------------------------------------------------------------------

/// `Not holds`, as a CNF with its Tseitin map.
fn negated_cnf(aig: &Aig, edge: Edge) -> (Cnf, Tseitin) {
    let mut cnf = Cnf::new(0);
    let mut ts = Tseitin::new();
    ts.assert_true(aig, !edge, &mut cnf);
    (cnf, ts)
}

/// The circuit inputs a solver model gives, input by input (an input outside every asserted
/// cone is unconstrained and read as false).
pub fn model_inputs(aig: &Aig, ts: &Tseitin, model: &[bool]) -> Vec<bool> {
    (0..aig.num_inputs())
        .map(|j| ts.input_value(aig, j, model).unwrap_or(false))
        .collect()
}

/// The case number of `inputs` (the numbering of [`crate::lemmas`]): input `j` is bit `j`.
pub fn case_number(inputs: &[bool]) -> u64 {
    inputs
        .iter()
        .enumerate()
        .fold(0u64, |c, (j, &b)| c | (u64::from(b) << j))
}

/// Replay a SAT model: it must satisfy the CNF by a plain re-evaluation, and `edge` must have
/// the value `want` on the inputs it gives.  `Err` says which half failed.
pub fn replay(
    aig: &Aig,
    ts: &Tseitin,
    cnf: &Cnf,
    model: &[bool],
    edge: Edge,
    want: bool,
) -> Result<Vec<bool>, String> {
    if !model_satisfies(cnf, model) {
        return Err("the solver's model does not satisfy the CNF".to_string());
    }
    let inputs = model_inputs(aig, ts, model);
    if aig.eval(edge, &inputs) != want {
        return Err(format!(
            "the model does not replay through the circuit (the edge is not {want})"
        ));
    }
    Ok(inputs)
}

/// Build, solve and certify lemma `lemma` at width `n`, running every gate of VOS31 7.2.
/// Findings are appended to `findings`; a [`Row`] comes back only if none was.
fn obligation(
    lemma: LemmaId,
    n: usize,
    is_implication: bool,
    cfg: &LaneConfig,
    ev: &Evidence,
    findings: &mut Vec<Finding>,
    proofs: &mut Vec<(String, String, String)>,
) -> Option<Row> {
    let who = format!("{} at N = {n}", lemma.name());
    let before = findings.len();
    let mut fail = |code: &'static str, msg: String| findings.push(Finding::new(code, msg));

    // The honest obligation.
    let model = (cfg.honest)(n);
    let mut aig = Aig::new();
    let c: LemmaCircuit = lemma_circuit(lemma, model.as_ref(), &mut aig);
    let (cnf, _) = negated_cnf(&aig, c.holds);
    let dimacs = write_dimacs(&cnf);
    let cnf_sha256 = sha256_hex(dimacs.as_bytes());
    let and_count = aig.num_ands();

    let solved = solve_cnf(&cnf, cfg.budget.max_conflicts, true);
    let mut check_work = 0u64;
    let mut drat_text = String::new();
    match &solved.verdict {
        Verdict::Sat(_) => fail(
            "MS03",
            format!("{who}: the negated lemma is SATISFIABLE: the encoder has a counterexample"),
        ),
        Verdict::Unknown => fail(
            "MS05",
            format!(
                "{who}: over the route budget of VOS30 8.6.1: more than {} conflicts",
                cfg.budget.max_conflicts
            ),
        ),
        Verdict::Unsat => {
            if solved.conflicts > cfg.budget.max_conflicts {
                fail(
                    "MS05",
                    format!(
                        "{who}: {} conflicts, over the route budget of VOS30 8.6.1 ({})",
                        solved.conflicts, cfg.budget.max_conflicts
                    ),
                );
            }
            match &solved.drat {
                None => fail("MS04", format!("{who}: the solver produced no DRAT proof")),
                Some(drat) => {
                    let drat = (cfg.proof_hook)(drat);
                    let chk = check_default(&cnf, &drat);
                    match chk.work {
                        Some(w) if chk.accepted => {
                            check_work = w;
                            if w > cfg.budget.max_check_work {
                                fail(
                                    "MS05",
                                    format!(
                                        "{who}: check_work {w} is over the route budget of VOS30 8.6.1 ({})",
                                        cfg.budget.max_check_work
                                    ),
                                );
                            }
                        }
                        _ => fail(
                            "MS04",
                            format!(
                                "{who}: the proof is rejected under the default limits: {}",
                                chk.reason.unwrap_or_default()
                            ),
                        ),
                    }
                    drat_text = drat;
                }
            }
        }
    }

    // The collapse, re-derived: from the AIG, then from the CNF by the lane's own propagator.
    let collapsed = if c.holds == Edge::constant(true) {
        Some(CollapseReason::ConstantFold)
    } else if propagation_refutes(&cnf) {
        Some(CollapseReason::PropagationOnly)
    } else {
        None
    };
    if solved.verdict == Verdict::Unsat && solved.conflicts == 0 && collapsed.is_none() {
        fail(
            "MS08",
            format!(
                "{who}: refuted with no conflict, but neither constant-fold nor propagation-only re-derives"
            ),
        );
    }

    // The control: the mutant's negation must be SAT, and the model must replay.
    {
        let mutant = (cfg.mutant)(lemma, n);
        let mut maig = Aig::new();
        let mc = lemma_circuit(lemma, mutant.as_ref(), &mut maig);
        let (mcnf, mts) = negated_cnf(&maig, mc.holds);
        match solve_cnf(&mcnf, cfg.budget.max_conflicts, false).verdict {
            Verdict::Sat(m) => match replay(&maig, &mts, &mcnf, &m, mc.holds, false) {
                Err(e) => fail("MS06", format!("{who}: the control does not replay: {e}")),
                Ok(inputs) => {
                    if n == crate::crosscheck::EVALUATOR_N {
                        let case = case_number(&inputs) as usize;
                        let in_set = ev
                            .mutants
                            .iter()
                            .find(|(l, _)| *l == lemma)
                            .and_then(|(_, r)| r.lemmas.get(lemma.name()))
                            .and_then(|f| f.failing.get(case))
                            .copied();
                        if in_set != Some(true) {
                            fail(
                                "MS06",
                                format!(
                                    "{who}: the control's counterexample (case {case}) is not a failing case of the mutant document in the Warrant evaluator"
                                ),
                            );
                        }
                    }
                }
            },
            Verdict::Unsat => fail(
                "MS06",
                format!("{who}: the control is UNSAT: the lemma's mutant is not caught, so the lemma proved nothing"),
            ),
            Verdict::Unknown => fail(
                "MS06",
                format!("{who}: the control ran over the route budget of VOS30 8.6.1"),
            ),
        }
    }

    // The premise witness, for the lemmas the PARSER says are implications.
    let mut premise_witnessed = false;
    if is_implication {
        match c.premise {
            None => fail(
                "MS07",
                format!("{who}: lemmas.part says this claim is an implication, but the encoder has no premise"),
            ),
            Some(p) => {
                let mut paig = Aig::new();
                let pm = (cfg.honest)(n);
                let pc = lemma_circuit(lemma, pm.as_ref(), &mut paig);
                let pcnf_edge = pc.premise.unwrap_or(Edge::constant(false));
                let mut pcnf = Cnf::new(0);
                let mut pts = Tseitin::new();
                pts.assert_true(&paig, pcnf_edge, &mut pcnf);
                let _ = p;
                match solve_cnf(&pcnf, cfg.budget.max_conflicts, false).verdict {
                    Verdict::Sat(m) => match replay(&paig, &pts, &pcnf, &m, pcnf_edge, true) {
                        Ok(_) => premise_witnessed = true,
                        Err(e) => fail("MS07", format!("{who}: the premise witness does not replay: {e}")),
                    },
                    _ => fail(
                        "MS07",
                        format!("{who}: the premise alone is not satisfiable: the implication is vacuous"),
                    ),
                }
            }
        }
    } else if c.premise.is_some() {
        fail(
            "MS07",
            format!("{who}: the encoder has a premise but lemmas.part says this claim is not an implication"),
        );
    }

    if findings.len() != before {
        return None;
    }
    if cfg.keep_proofs {
        proofs.push((format!("{}-n{n}", lemma.name()), dimacs, drat_text));
    }
    Some(Row {
        lemma,
        n,
        and_count,
        vars: cnf.num_vars,
        clauses: cnf.clauses.len(),
        cnf_sha256,
        collapsed,
        premise_witnessed,
        label: certified_label(lemma, n, collapsed),
        conflicts: solved.conflicts,
        check_work,
    })
}

// ---------------------------------------------------------------------------------------
// The table
// ---------------------------------------------------------------------------------------

fn collapsed_word(c: Option<CollapseReason>) -> &'static str {
    c.map_or("no", CollapseReason::as_str)
}

/// The text of `warrant/lemmas.sat` (VOS31 8.2) for `rows`, which are in lemma order and then
/// ascending N.  Only solver-independent fields appear.
pub(crate) fn render_table(files: &ContractFiles, rows: &[Row], budget: Budget) -> String {
    let mut s = String::from("vos-sat 1\n");
    s.push_str(&format!(
        "contract-part {}\n",
        sha256_hex(files.contract_text.as_bytes())
    ));
    s.push_str(&format!(
        "lemmas-part {}\n",
        sha256_hex(files.lemmas_text.as_bytes())
    ));
    s.push_str(&format!(
        "route-budget conflicts {} check-work {}\n",
        budget.max_conflicts, budget.max_check_work
    ));
    for r in rows {
        s.push_str(&format!(
            "lemma {} n {} label {} and {} vars {} clauses {} cnf {} verdict unsat collapsed {} control sat premise {}\n",
            r.lemma.name(),
            r.n,
            label_token(&r.label),
            r.and_count,
            r.vars,
            r.clauses,
            r.cnf_sha256,
            collapsed_word(r.collapsed),
            if r.premise_witnessed { "witnessed" } else { "none" },
        ));
    }
    let trivial = rows.iter().any(|r| r.label == LABEL_TRIVIAL);
    if trivial {
        s.push_str("trivial closure_at_m\n");
    }
    s.push_str("end\n");
    s
}

/// The first line at which `a` and `b` differ, as `line N: committed X, regenerated Y`.
pub fn first_difference(committed: &str, regenerated: &str) -> String {
    let mut a = committed.lines();
    let mut b = regenerated.lines();
    let mut line = 1;
    loop {
        match (a.next(), b.next()) {
            (None, None) => return "the texts differ only in a trailing newline".to_string(),
            (x, y) if x != y => {
                return format!(
                    "line {line}: committed `{}`, regenerated `{}`",
                    x.unwrap_or("<end of file>"),
                    y.unwrap_or("<end of file>")
                )
            }
            _ => line += 1,
        }
    }
}

/// The collapse reasons a committed table CLAIMS: `(lemma, n, reason word)` for every row whose
/// `collapsed` word is not `no`.  Read as text, never trusted: `check` re-derives each.
pub fn recorded_collapses(table: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for line in table.lines() {
        let w: Vec<&str> = line.split(' ').collect();
        if w.first() != Some(&"lemma") || w.len() < 4 {
            continue;
        }
        let at = |key: &str| w.iter().position(|x| *x == key).and_then(|i| w.get(i + 1));
        if let (Some(n), Some(reason)) = (at("n"), at("collapsed")) {
            if *reason != "no" {
                out.push((w[1].to_string(), n.to_string(), reason.to_string()));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------
// The whole analysis
// ---------------------------------------------------------------------------------------

/// Whether `vos.toml` says `sat = "model"`: a flat file, one `key = value` per line.
pub fn says_sat_model(vos_toml: &str) -> bool {
    vos_toml.lines().any(|l| {
        let l = l.split('#').next().unwrap_or("");
        match l.split_once('=') {
            Some((k, v)) => k.trim() == "sat" && v.trim() == "\"model\"",
            None => false,
        }
    })
}

/// Run every gate of VOS31 sections 6, 7.2 and 7.5 on `files` and, if all pass, build the
/// table.  `Err` is "no verdict" (the evaluator could not run); every other failure is a
/// [`Finding`].
pub fn analyse(files: &ContractFiles, cfg: &LaneConfig) -> Result<Analysis, String> {
    analyse_with(files, cfg, None)
}

/// [`analyse`] on evidence the caller already gathered (the evaluator is an interpreter: about
/// 25 s), or `None` to gather it here.
pub fn analyse_with(
    files: &ContractFiles,
    cfg: &LaneConfig,
    evidence: Option<&Evidence>,
) -> Result<Analysis, String> {
    let mut out = Analysis::default();

    // MS01: the inputs.
    match (&files.vos_toml, cfg.require_opt_in) {
        (_, false) => out.report.push(BRIDGED_REPORT_LINE.to_string()),
        (Some(t), true) if says_sat_model(t) => {}
        (Some(_), true) => out.findings.push(Finding::new(
            "MS01",
            "vos.toml does not say `sat = \"model\"`: the lane is opt-in per contract (VOS31 10.3)",
        )),
        (None, true) => out
            .findings
            .push(Finding::new("MS01", "vos.toml: not found")),
    }
    for &n in cfg.ladder {
        if n == 0 || n > crate::MAX_N {
            out.findings.push(Finding::new(
                "MS01",
                format!("the width N = {n} is outside 1 ..= {}", crate::MAX_N),
            ));
        }
    }
    let claims = claims_of(&files.base_document())?;
    if claims.len() > MAX_LEMMAS {
        out.findings.push(Finding::new(
            "MS01",
            format!(
                "lemmas.part has {} claims, over the limit of {MAX_LEMMAS}",
                claims.len()
            ),
        ));
    }
    if !out.findings.is_empty() {
        return Ok(out);
    }

    // MS10: every claim of lemmas.part must have an encoder (hence a row), and there must be one.
    if claims.is_empty() {
        out.findings.push(Finding::new(
            "MS10",
            "lemmas.part has no lemma: none ran is a failure",
        ));
    }
    let mut lemmas: Vec<(LemmaId, bool)> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (name, implies) in &claims {
        if !seen.insert(name.as_str()) {
            // Two claims of one name would give one lemma two rows (and two premise verdicts).
            out.findings.push(Finding::new(
                "MS10",
                format!("the lemma `{name}` is declared more than once in lemmas.part"),
            ));
            continue;
        }
        match LemmaId::from_name(name) {
            Ok(id) => lemmas.push((id, *implies)),
            Err(_) => out.findings.push(Finding::new(
                "MS10",
                format!("the lemma `{name}` of lemmas.part has no encoder, so no row"),
            )),
        }
    }
    // A duplicate claim name is not something the evaluator can be asked about: stop here.
    if out
        .findings
        .iter()
        .any(|f| f.message.contains("more than once"))
    {
        return Ok(out);
    }
    let present: BTreeSet<LemmaId> = lemmas.iter().map(|(l, _)| *l).collect();
    for l in LemmaId::ALL {
        if !present.contains(&l) {
            out.findings.push(Finding::new(
                "MS10",
                format!("the encoder's lemma `{}` is not in lemmas.part", l.name()),
            ));
        }
    }

    // MS02: the encoder against the evaluator and the reference (VOS31 section 6).
    let gathered;
    let ev = match evidence {
        Some(e) => e,
        None => {
            gathered = crate::crosscheck::gather(files)?;
            &gathered
        }
    };
    let (lines, ok) = crate::run_cross_checks_with(ev);
    if !ok {
        out.findings.push(Finding::new(
            "MS02",
            format!("the encoder cross-check failed: {}", lines.join(" | ")),
        ));
    }

    // MS03 to MS08: the obligations.
    let mut rows: Vec<Row> = Vec::new();
    for &(lemma, implies) in &lemmas {
        for &n in cfg.ladder {
            if let Some(r) = obligation(
                lemma,
                n,
                implies,
                cfg,
                ev,
                &mut out.findings,
                &mut out.proofs,
            ) {
                out.report.push(format!(
                    "{} N = {}: unsat, conflicts {}, check_work {}, DRAT accepted, collapsed {}, control sat, premise {}; {}{}",
                    r.lemma.name(),
                    r.n,
                    r.conflicts,
                    r.check_work,
                    collapsed_word(r.collapsed),
                    if r.premise_witnessed { "witnessed" } else { "none" },
                    r.label,
                    if r.counted() { "" } else { " (not counted)" },
                ));
                rows.push(r);
            }
        }
    }
    if out.findings.is_empty() {
        let counted = rows.iter().filter(|r| r.counted()).count();
        out.report.push(format!(
            "{} obligations certified and counted, {} trivial and not counted",
            counted,
            rows.len() - counted
        ));
        out.table = Some(render_table(files, &rows, cfg.budget));
        out.rows = rows;
    } else {
        out.proofs.clear();
    }
    Ok(out)
}

/// `check`: [`analyse`], then compare the regenerated table byte for byte with the committed
/// one (`MS09`), and re-derive every collapse reason the committed table claims (`MS08`).
pub fn check(files: &ContractFiles, cfg: &LaneConfig) -> Result<Analysis, String> {
    check_with(files, cfg, None)
}

/// [`check`] on evidence the caller already gathered.
pub fn check_with(
    files: &ContractFiles,
    cfg: &LaneConfig,
    evidence: Option<&Evidence>,
) -> Result<Analysis, String> {
    let mut a = analyse_with(files, cfg, evidence)?;
    let Some(regenerated) = a.table.clone() else {
        return Ok(a);
    };
    match &files.sat_table {
        None => a.findings.push(Finding::new(
            "MS09",
            "warrant/lemmas.sat is absent: run `vos-model-sat gen`",
        )),
        Some(committed) => {
            // A recorded reason must hold on the regenerated obligation.
            let derived = recorded_collapses(&regenerated);
            for claim in recorded_collapses(committed) {
                let known = CollapseReason::parse(&claim.2).is_some();
                if !known || !derived.contains(&claim) {
                    a.findings.push(Finding::new(
                        "MS08",
                        format!(
                            "{} at N = {}: the committed table records collapse `{}`, which does not re-derive on the regenerated obligation",
                            claim.0, claim.1, claim.2
                        ),
                    ));
                }
            }
            if committed != &regenerated {
                a.findings.push(Finding::new(
                    "MS09",
                    format!(
                        "warrant/lemmas.sat is stale: {}",
                        first_difference(committed, &regenerated)
                    ),
                ));
            }
        }
    }
    if !a.findings.is_empty() {
        a.table = None;
    }
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(conflicts: u64, check_work: u64) -> Row {
        Row {
            lemma: LemmaId::TopIsTheMaximum,
            n: 8,
            and_count: 5,
            vars: 6,
            clauses: 7,
            cnf_sha256: "ab".repeat(32),
            collapsed: None,
            premise_witnessed: false,
            label: label_for(8),
            conflicts,
            check_work,
        }
    }

    #[test]
    fn the_table_does_not_depend_on_conflicts_or_check_work() {
        // VOS31 8.1: conflicts, check_work and acceptance are gated and never committed.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .unwrap()
            .join("packages")
            .join("rust")
            .join("vos-sched-contract");
        let files = crate::oracle::load(&dir, None).unwrap();
        let a = render_table(&files, &[row(1, 2)], Budget::ROUTE);
        let b = render_table(&files, &[row(7_777, 9_999_999)], Budget::ROUTE);
        assert_eq!(a, b);
        assert!(a.contains(" and 5 vars 6 clauses 7 cnf "));
        assert!(!a.contains("7777") && !a.contains("9999999"));
    }

    #[test]
    fn certified_label_is_the_one_chooser() {
        assert_eq!(
            certified_label(LemmaId::ClosureAtM, 16, Some(CollapseReason::ConstantFold)),
            LABEL_TRIVIAL
        );
        assert!(certified_label(LemmaId::PolicyAdmitsOnlyTop, 8, None)
            .starts_with("lemma, SAT-certified at N = 8"));
    }
}

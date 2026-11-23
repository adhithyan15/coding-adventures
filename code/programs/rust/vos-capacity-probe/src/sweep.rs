//! # `sweep`: SAT sweeping with certified lemmas (experiment B5, VOS30 section 8.6.3)
//!
//! A monolithic miter asks one SAT call to prove two circuits equal.  When the circuits are
//! built from the *same* pieces joined in a different order, most of their internal nodes have
//! a twin on the other side, and the cheap way to prove the whole is to prove the twins one by
//! one, from the inputs towards the outputs, each proof assuming the ones before it.  That is
//! **SAT sweeping** (fraiging).  This module is a small, deterministic, *certified* one: every
//! merge it makes is a DRAT-checked UNSAT proof, so a green row is evidence and not a claim.
//!
//! ```text
//!   original AIG                     reduced AIG `red` (built node by node)
//!   ------------                     --------------------------------------
//!   n  = And(a, b)       ------->    img(n) = red.and(img(a), img(b))
//!
//!   if some earlier node r has the same simulation signature as n (up to complement):
//!        img(n) == img(r) ^ phase  already?   yes -> merged for free (structural hashing)
//!                                             no  -> a LEMMA: prove it, then set
//!                                                    img(n) := img(r) ^ phase
//! ```
//!
//! After the last node every node's image is a node of `red`, with the proved equivalences
//! substituted, so the two output vectors have collapsed wherever they could.  What remains is
//! the **final obligation**: the premises together with "some output bit differs", over `red`.
//! `red` computes exactly the functions the original computes (a lemma only ever replaces a
//! node by a *proved-equal* one), so the final obligation is UNSAT exactly when the original
//! miter is.
//!
//! ## Candidates: simulation signatures
//!
//! 256 seeded random patterns (four calls of [`Aig::simulate`], 64 patterns each) give every
//! node a four-word signature.  Two nodes with equal signatures *up to complement* (the
//! signature is normalised so its first bit is 0, and the flip is remembered as a `phase`) are
//! candidates; the **representative** of a class is its lowest node, and a node is only ever
//! compared with the representative.  Signatures are evidence, not proof: that is exactly what
//! the lemma is for.
//!
//! ## A lemma, and the cut frontier
//!
//! A lemma "`x == y`" is a fresh CNF over the cones of `x` and `y` in `red`, asserting
//! `x != y`.  Because earlier merges are already substituted, the cones are small (the two
//! sides share everything below the previous equivalence points).  To keep them smaller still,
//! the cone is **cut at a frontier**: AND nodes at breadth-first depth `D` from the two roots
//! (start `D = 8`) are replaced by fresh free inputs.
//!
//! * UNSAT with free cut inputs is a proof (a free input over-approximates the node it
//!   replaces), so the lemma is proved.
//! * SAT with a cut is **possibly spurious**: the cut inputs are unconstrained, so the model
//!   may not be realisable.  The depth doubles (8, 16, 32, ...), counted as an `escalation`,
//!   until the cut is empty.
//! * SAT on the full cone is a *real* counterexample: the candidates were wrong.  It is added
//!   as a new simulation pattern (one extra signature bit per counterexample), the classes are
//!   re-split, and the node is retried against its new candidate.
//!
//! ## Three proof modes
//!
//! * [`ProofMode::Certified`]: every lemma and the final obligation are solved with DRAT and
//!   checked under `Limits::default()`.  This is the experiment's row.
//! * [`ProofMode::Solver`]: the same sweep with no proof (the G3 control only needs the
//!   *answer*: a mutated program must end SAT, and its model is checked elsewhere).
//! * [`ProofMode::SignatureOnly`]: merges on signature equality with **no proof at all**.  It is
//!   **UNSOUND** and exists only as the negative-control instrument: a pair of circuits that
//!   differ on one input in 2^32 agree on all 256 patterns, this mode merges them and "proves"
//!   them equal, and the test shows the other two modes refuting it.  Nothing else uses it.
//!
//! ## Counters (VOS30 section 9.6)
//!
//! `candidates`: cone nodes that had a representative with a different image (each one costs a
//! lemma attempt).  `lemmas`: UNSAT lemma proofs.  `refuted`: full-cone SAT answers.
//! `escalations`: depth doublings.

use crate::exp::obligation_deadline;
use crate::inst::Instance;
use crate::prog::Rng;
use crate::solve::{check_default, solve_cnf, Cap, SolveOpts, SolveOut, Verdict};
use bit_blaster::aig::{Aig, Edge, Node};
use bit_blaster::tseitin::Tseitin;
use cdcl_sat::Cnf;
use coding_adventures_sha256::Sha256Hasher;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// The first cut depth of a lemma.
pub const FRONTIER_DEPTH: usize = 8;
/// Simulation words (64 patterns each) before any counterexample.
pub const SIM_WORDS: usize = 4;

/// How the sweep justifies a merge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofMode {
    /// A DRAT proof per lemma and for the final obligation, each checked under the default limits.
    Certified,
    /// The same sweep with the solver's answers trusted and no proof (the G3 control).
    Solver,
    /// Merge on signature equality with no proof.  **Unsound**: the negative-control instrument.
    SignatureOnly,
}

/// Options of one sweep.
#[derive(Clone, Copy, Debug)]
pub struct SweepOpts {
    /// The proof mode.
    pub mode: ProofMode,
    /// Seed of the simulation patterns and of the counterexample fill.
    pub seed: u64,
    /// The row's deadline.
    pub row_deadline: Instant,
    /// The wall of one obligation (each solve ends at the row deadline or this long after it
    /// started, whichever is first).
    pub obligation_wall: Duration,
    /// An extra conflict cap for every solve (the SAT control uses it).
    pub conflict_cap: Option<u64>,
}

/// What a sweep concluded about the miter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SweepVerdict {
    /// The final obligation is UNSAT: the sides agree (certified, in `Certified` mode).
    Unsat,
    /// The final obligation is SAT: one value per input of the AIG, a counterexample.
    Sat(Vec<bool>),
    /// A budget ended a solve.
    Unknown(Cap),
}

/// Everything a sweep measured.
#[derive(Clone, Debug)]
pub struct SweepReport {
    /// The verdict.
    pub verdict: SweepVerdict,
    /// Cone nodes with a representative of a different image (lemma attempts).
    pub candidates: u64,
    /// UNSAT lemma proofs.
    pub lemmas: u64,
    /// Full-cone SAT answers: wrong candidates.
    pub refuted: u64,
    /// Depth doublings after a possibly spurious cut SAT.
    pub escalations: u64,
    /// Every solve, lemma attempts and final.
    pub solves: u64,
    /// The most conflicts any UNSAT lemma or the final obligation used.
    pub worst_conflicts: u64,
    /// Conflicts over all solves.
    pub total_conflicts: u64,
    /// Propagations over all solves.
    pub total_propagations: u64,
    /// Solver milliseconds over all solves.
    pub total_solve_ms: f64,
    /// The slowest single solve, milliseconds.
    pub worst_solve_ms: f64,
    /// The largest checker work of any proof, when one was checked.
    pub max_check_work: Option<u64>,
    /// Checker milliseconds over all proofs.
    pub check_ms: f64,
    /// Every proof was accepted under the default limits (vacuously true with no proof).
    pub all_accepted: bool,
    /// A proof was *wrong* (not merely over a limit): a solver bug.
    pub invalid: bool,
    /// SHA-256 of the proofs, in order (`Certified` only).
    pub drat_sha256: Option<String>,
    /// The largest single proof, bytes.
    pub max_drat_bytes: u64,
    /// The first reason a proof was rejected, or another remark.
    pub note: String,
}

impl SweepReport {
    fn new() -> SweepReport {
        SweepReport {
            verdict: SweepVerdict::Unsat,
            candidates: 0,
            lemmas: 0,
            refuted: 0,
            escalations: 0,
            solves: 0,
            worst_conflicts: 0,
            total_conflicts: 0,
            total_propagations: 0,
            total_solve_ms: 0.0,
            worst_solve_ms: 0.0,
            max_check_work: None,
            check_ms: 0.0,
            all_accepted: true,
            invalid: false,
            drat_sha256: None,
            max_drat_bytes: 0,
            note: String::new(),
        }
    }
}

/// `e` with its inverter flipped when `c`.
fn flip(e: Edge, c: bool) -> Edge {
    if c {
        !e
    } else {
        e
    }
}

/// The 256 seeded patterns of the sweep: for every node, its `SIM_WORDS` words of simulated
/// values.  Public so a test can show that a pair of circuits is indistinguishable *by exactly
/// these patterns*.
pub fn simulation_signatures(aig: &Aig, seed: u64) -> Vec<Vec<u64>> {
    let mut rng = Rng::new(seed, 0x5157);
    let mut words: Vec<Vec<u64>> = vec![Vec::with_capacity(SIM_WORDS + 1); aig.num_nodes()];
    for _ in 0..SIM_WORDS {
        let inputs: Vec<u64> = (0..aig.num_inputs()).map(|_| rng.next_u64()).collect();
        let values = aig.simulate(&inputs);
        for (w, v) in words.iter_mut().zip(values) {
            w.push(v);
        }
    }
    words
}

/// The simulation signatures of every node of an AIG, and the counterexample patterns added
/// since.
///
/// Word `0..SIM_WORDS` are random; word `SIM_WORDS + j` packs counterexamples `64j .. 64j+63`
/// (bit `i` of the word is counterexample `64j + i`).  Unused counterexample bits are 0 in
/// every node, and the normalisation masks them, so a half-filled word cannot split a class.
struct Signatures {
    words: Vec<Vec<u64>>,
    cx: usize,
}

impl Signatures {
    /// Simulate `SIM_WORDS` rounds of seeded random patterns.
    fn new(aig: &Aig, seed: u64) -> Signatures {
        Signatures {
            words: simulation_signatures(aig, seed),
            cx: 0,
        }
    }

    /// Add one counterexample (a value per input) as a new pattern.
    fn add(&mut self, aig: &Aig, inputs: &[bool]) {
        let (j, bit) = (self.cx / 64, self.cx % 64);
        let one = 1u64 << bit;
        let words: Vec<u64> = inputs.iter().map(|&b| if b { one } else { 0 }).collect();
        let values = aig.simulate(&words);
        for (w, v) in self.words.iter_mut().zip(values) {
            if w.len() == SIM_WORDS + j {
                w.push(0);
            }
            w[SIM_WORDS + j] |= v & one;
        }
        self.cx += 1;
    }

    /// The valid-bit mask of word `i`.
    fn mask(&self, i: usize) -> u64 {
        if i < SIM_WORDS {
            return !0;
        }
        let j = i - SIM_WORDS;
        let used = self.cx.saturating_sub(64 * j).min(64);
        if used == 64 {
            !0
        } else {
            (1u64 << used) - 1
        }
    }

    /// The signature of `n` up to complement: normalised so that pattern 0 is 0, and the phase
    /// (whether the raw signature was complemented to get there).
    fn normalised(&self, n: usize) -> (Vec<u64>, bool) {
        let w = &self.words[n];
        let phase = w[0] & 1 == 1;
        let key = w
            .iter()
            .enumerate()
            .map(|(i, &x)| (if phase { !x } else { x }) & self.mask(i))
            .collect();
        (key, phase)
    }
}

/// The result of proving `x == y` in `red`.
enum Lemma {
    /// UNSAT: proved.
    Proved,
    /// A real counterexample: one value per input of the AIG.
    Refuted(Vec<bool>),
    /// A budget ended the solve.
    Unknown(Cap),
}

/// The state of one sweep.
struct Sweeper<'a> {
    inst: &'a Instance,
    opts: &'a SweepOpts,
    /// The reduced AIG (inputs one to one with the original's, in order).
    red: Aig,
    rep: SweepReport,
    proofs: Sha256Hasher,
    cx_rng: Rng,
}

impl Sweeper<'_> {
    fn deadline(&self) -> Instant {
        obligation_deadline(
            self.opts.row_deadline,
            Instant::now(),
            self.opts.obligation_wall,
        )
    }

    /// Solve `cnf`, account for it, and for a certified UNSAT check its proof.  The verdict comes
    /// back with whether the proof (if any) was accepted.
    fn solve(&mut self, cnf: &Cnf) -> (Verdict, bool) {
        let proof = self.opts.mode == ProofMode::Certified;
        let out = solve_cnf(
            cnf,
            &SolveOpts {
                deadline: self.deadline(),
                proof,
                conflict_cap: self.opts.conflict_cap,
            },
        );
        let accepted = account(&mut self.rep, &mut self.proofs, cnf, &out, proof);
        (out.verdict, accepted)
    }

    /// Prove `x == y` over `red`, cutting at a frontier and doubling its depth on a possibly
    /// spurious SAT.
    fn prove(&mut self, x: Edge, y: Edge) -> Lemma {
        if self.opts.mode == ProofMode::SignatureOnly {
            return Lemma::Proved; // UNSOUND by design: see the module docs.
        }
        let mut depth = FRONTIER_DEPTH;
        loop {
            let cut = LemmaCnf::build(&self.red, x, y, depth);
            let (verdict, _accepted) = self.solve(&cut.cnf);
            match verdict {
                Verdict::Unsat => {
                    // The merge is made even after a rejected proof that is not `invalid`:
                    // `solve` already recorded it (`all_accepted = false`), which turns the
                    // row red through `pass_default_limits = false`, as for the other
                    // experiments, and `invalid` marks the row crashed.  Merging anyway keeps
                    // the rest of the sweep measured.
                    self.rep.lemmas += 1;
                    return Lemma::Proved;
                }
                Verdict::Unknown(c) => return Lemma::Unknown(c),
                Verdict::Sat(model) => {
                    if !cut.complete {
                        self.rep.escalations += 1;
                        depth = depth.saturating_mul(2);
                        continue;
                    }
                    return Lemma::Refuted(self.fill_counterexample(&cut, &model));
                }
            }
        }
    }

    /// A full input assignment from the model of a complete lemma CNF: the cone's inputs from
    /// the model, every other input a seeded random bit (it cannot matter to the lemma, and
    /// varied bits split more classes).
    fn fill_counterexample(&mut self, cut: &LemmaCnf, model: &[bool]) -> Vec<bool> {
        let n = self.inst.aig.num_inputs();
        let mut a: Vec<bool> = (0..n).map(|_| self.cx_rng.next_u64() & 1 == 1).collect();
        for (k, &red_input) in cut.input_of.iter().enumerate() {
            if let Some(i) = red_input {
                a[i] = cut.ts.input_value(&cut.aig, k, model).unwrap_or(false);
            }
        }
        a
    }

    /// The final obligation: premises AND some output pair differs, over `red`.
    fn final_obligation(&mut self, img: &[Edge]) -> SweepVerdict {
        let ie = |e: Edge| flip(img[e.node()], e.is_complemented());
        let inst = self.inst;
        let mut diffs = Vec::with_capacity(inst.code_out.len());
        for (c, r) in inst.code_out.iter().zip(&inst.ref_out) {
            let d = self.red.xor(ie(*c), ie(*r));
            diffs.push(d);
        }
        let miter = self.red.or_all(&diffs);
        let mut cnf = Cnf::new(0);
        let mut ts = Tseitin::new();
        for p in &inst.premises {
            ts.assert_true(&self.red, ie(*p), &mut cnf);
        }
        ts.assert_true(&self.red, miter, &mut cnf);
        let (verdict, _) = self.solve(&cnf);
        match verdict {
            Verdict::Unsat => SweepVerdict::Unsat,
            Verdict::Unknown(c) => SweepVerdict::Unknown(c),
            Verdict::Sat(model) => {
                let a = (0..self.red.num_inputs())
                    .map(|i| ts.input_value(&self.red, i, &model).unwrap_or(false))
                    .collect();
                SweepVerdict::Sat(a)
            }
        }
    }
}

/// The CNF of one lemma: a small AIG copy of the cones of two `red` edges, cut at a frontier.
struct LemmaCnf {
    cnf: Cnf,
    ts: Tseitin,
    aig: Aig,
    /// For each input of the small AIG, the AIG input index it stands for (`None`: a free cut node).
    input_of: Vec<Option<usize>>,
    /// No AND node was cut: a SAT answer is a real counterexample.
    complete: bool,
}

impl LemmaCnf {
    /// Copy the cone of `x` and `y` down to breadth-first depth `depth`; the AND nodes at that
    /// depth become fresh free inputs.  Asserts `x != y`.
    fn build(red: &Aig, x: Edge, y: Edge, depth: usize) -> LemmaCnf {
        // Breadth-first minimum depth from the two roots.
        let mut at: HashMap<usize, usize> = HashMap::new();
        let mut frontier = vec![x.node(), y.node()];
        for &n in &frontier {
            at.insert(n, 0);
        }
        let mut d = 0usize;
        while !frontier.is_empty() && d < depth {
            let mut next = Vec::new();
            for &n in &frontier {
                if let Node::And(a, b) = red.node(n) {
                    for m in [a.node(), b.node()] {
                        if let std::collections::hash_map::Entry::Vacant(v) = at.entry(m) {
                            v.insert(d + 1);
                            next.push(m);
                        }
                    }
                }
            }
            frontier = next;
            d += 1;
        }
        let mut nodes: Vec<usize> = at.keys().copied().collect();
        nodes.sort_unstable(); // node index order is topological
        let mut aig = Aig::new();
        let mut map: HashMap<usize, Edge> = HashMap::with_capacity(nodes.len());
        let mut input_of: Vec<Option<usize>> = Vec::new();
        let mut complete = true;
        for &n in &nodes {
            let e = match red.node(n) {
                Node::False => Edge::FALSE,
                Node::Input(i) => {
                    input_of.push(Some(i));
                    aig.input()
                }
                Node::And(a, b) => {
                    if at[&n] >= depth {
                        complete = false;
                        input_of.push(None);
                        aig.input()
                    } else {
                        let ea = flip(map[&a.node()], a.is_complemented());
                        let eb = flip(map[&b.node()], b.is_complemented());
                        aig.and(ea, eb)
                    }
                }
            };
            map.insert(n, e);
        }
        let mx = flip(map[&x.node()], x.is_complemented());
        let my = flip(map[&y.node()], y.is_complemented());
        let diff = aig.xor(mx, my);
        let mut cnf = Cnf::new(0);
        let mut ts = Tseitin::new();
        ts.assert_true(&aig, diff, &mut cnf);
        LemmaCnf {
            cnf,
            ts,
            aig,
            input_of,
            complete,
        }
    }
}

/// Sweep the miter of `inst` (see the module docs) and report.
pub fn sweep(inst: &Instance, opts: &SweepOpts) -> SweepReport {
    let aig = &inst.aig;
    let n_nodes = aig.num_nodes();
    // The cone of everything the final obligation mentions.
    let mut in_cone = vec![false; n_nodes];
    let mut stack: Vec<usize> = inst
        .code_out
        .iter()
        .chain(&inst.ref_out)
        .chain(&inst.premises)
        .map(|e| e.node())
        .collect();
    while let Some(i) = stack.pop() {
        if std::mem::replace(&mut in_cone[i], true) {
            continue;
        }
        if let Node::And(a, b) = aig.node(i) {
            stack.push(a.node());
            stack.push(b.node());
        }
    }
    // The constant takes part whether or not a cone reaches it: a node whose signature is
    // constant has it as representative.
    in_cone[0] = true;
    let mut sw = Sweeper {
        inst,
        opts,
        red: Aig::new(),
        rep: SweepReport::new(),
        proofs: Sha256Hasher::new(),
        cx_rng: Rng::new(opts.seed, 0xC8),
    };
    // Inputs map one to one, in order, so an input assignment is the same on both AIGs.
    let mut img: Vec<Edge> = vec![Edge::FALSE; n_nodes];
    for i in 0..aig.num_inputs() {
        let node = aig.input_edge(i).expect("input exists").node();
        img[node] = sw.red.input();
    }
    let mut sigs = Signatures::new(aig, opts.seed);
    // The representative of each normalised signature: (node, its phase).
    let mut reps: HashMap<Vec<u64>, (usize, bool)> = HashMap::new();
    let (k0, p0) = sigs.normalised(0);
    reps.insert(k0, (0, p0));
    let mut counted = vec![false; n_nodes];
    'nodes: for n in 1..n_nodes {
        if !in_cone[n] {
            continue;
        }
        if let Node::And(a, b) = aig.node(n) {
            let ea = flip(img[a.node()], a.is_complemented());
            let eb = flip(img[b.node()], b.is_complemented());
            img[n] = sw.red.and(ea, eb);
        }
        loop {
            let (key, phase) = sigs.normalised(n);
            let Some(&(r, rphase)) = reps.get(&key) else {
                reps.insert(key, (n, phase));
                continue 'nodes;
            };
            let target = flip(img[r], phase ^ rphase);
            if img[n] == target {
                continue 'nodes; // the same node of `red` already: merged for free
            }
            if !std::mem::replace(&mut counted[n], true) {
                sw.rep.candidates += 1;
            }
            match sw.prove(img[n], target) {
                Lemma::Proved => {
                    img[n] = target;
                    continue 'nodes;
                }
                Lemma::Unknown(c) => {
                    sw.rep.verdict = SweepVerdict::Unknown(c);
                    return finish(sw);
                }
                Lemma::Refuted(cx) => {
                    sw.rep.refuted += 1;
                    sigs.add(aig, &cx);
                    // Re-split the classes over the nodes already processed.
                    reps.clear();
                    for (m, _) in in_cone.iter().enumerate().take(n).filter(|(_, &c)| c) {
                        let (k, ph) = sigs.normalised(m);
                        reps.entry(k).or_insert((m, ph));
                    }
                    let (key2, _) = sigs.normalised(n);
                    if reps.get(&key2).is_some_and(|&(r2, _)| r2 == r) {
                        // The counterexample must separate n from r; if it did not, the
                        // simulation disagrees with the solver and nothing can be trusted.
                        sw.rep.invalid = true;
                        sw.rep.note =
                            "a counterexample did not split its class (simulation and solver disagree)"
                                .into();
                        sw.rep.verdict = SweepVerdict::Unknown(Cap::Conflicts);
                        return finish(sw);
                    }
                }
            }
        }
    }
    sw.rep.verdict = sw.final_obligation(&img);
    finish(sw)
}

fn finish(sw: Sweeper<'_>) -> SweepReport {
    let mut rep = sw.rep;
    if rep.solves > 0 && sw.opts.mode == ProofMode::Certified {
        rep.drat_sha256 = Some(sw.proofs.hex_digest());
    }
    rep
}

/// Evaluate every node of `aig` on one input assignment: the value of each node (not edge).
/// Used to rebuild a full Tseitin model from a sweep counterexample.
pub fn node_values(aig: &Aig, inputs: &[bool]) -> Vec<bool> {
    let words: Vec<u64> = inputs.iter().map(|&b| b as u64).collect();
    aig.simulate(&words).iter().map(|v| v & 1 == 1).collect()
}

/// Account for one solve in `r`: the counters, and in certified mode the proof check.  Pure in the
/// sense that it touches only its arguments, so a test can drive it with a hand-made `SolveOut`.
/// Returns whether the solve's proof (if any) was accepted.  A certified UNSAT with no DRAT text
/// is refused (see [`refuse_missing_proof`]).
fn account(
    r: &mut SweepReport,
    proofs: &mut Sha256Hasher,
    cnf: &Cnf,
    out: &SolveOut,
    proof: bool,
) -> bool {
    r.solves += 1;
    r.total_conflicts += out.conflicts;
    r.total_propagations += out.propagations;
    r.total_solve_ms += out.solve_ms;
    r.worst_solve_ms = r.worst_solve_ms.max(out.solve_ms);
    let mut accepted = true;
    if out.verdict == Verdict::Unsat {
        r.worst_conflicts = r.worst_conflicts.max(out.conflicts);
        // Security review: in certified mode an UNSAT answer with no DRAT text is refused
        // (it used to count as vacuously accepted); see `refuse_missing_proof`.
        if proof && out.drat.is_none() {
            accepted = refuse_missing_proof(r);
        }
        if let Some(d) = &out.drat {
            proofs.update(d.as_bytes());
            r.max_drat_bytes = r.max_drat_bytes.max(d.len() as u64);
            let chk = check_default(cnf, d);
            r.check_ms += chk.ms;
            if let Some(w) = chk.work {
                r.max_check_work = Some(r.max_check_work.unwrap_or(0).max(w));
            }
            if !chk.accepted {
                accepted = false;
                r.all_accepted = false;
                if chk.invalid {
                    r.invalid = true;
                }
                if r.note.is_empty() {
                    r.note = chk.reason.unwrap_or_default();
                }
            }
        }
    }
    accepted
}

/// What a certified sweep does with an UNSAT answer that came back without a proof (security
/// review of VOS-028c3b).  It refuses it: the answer is not accepted, `all_accepted` goes false
/// and the note says why, unless an earlier note is already there.  Before this, a missing proof
/// was counted as accepted, so a certified sweep could pass on bare UNSAT claims.  Returns
/// `false`, the value of `accepted` for this solve.
fn refuse_missing_proof(r: &mut SweepReport) -> bool {
    r.all_accepted = false;
    if r.note.is_empty() {
        r.note = "no proof for an UNSAT answer in certified mode".into();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Negative control: the old code left `all_accepted` true for a missing proof.
    #[test]
    fn a_missing_proof_is_not_accepted() {
        // Drive the accounting `Sweeper::solve` itself calls, with an UNSAT that has no DRAT.
        let cnf = Cnf::new(0);
        let out = SolveOut {
            verdict: Verdict::Unsat,
            conflicts: 1,
            propagations: 1,
            solve_ms: 0.0,
            drat: None,
        };
        let mut r = SweepReport::new();
        let mut h = Sha256Hasher::new();
        assert!(!account(&mut r, &mut h, &cnf, &out, true));
        assert!(!r.all_accepted);
        assert_eq!(r.note, "no proof for an UNSAT answer in certified mode");
        // An earlier note is kept.
        let mut r = SweepReport::new();
        r.note = "first".into();
        assert!(!account(&mut r, &mut h, &cnf, &out, true));
        assert_eq!(r.note, "first");
        // Outside certified mode the same answer is not refused.
        let mut r = SweepReport::new();
        assert!(account(&mut r, &mut h, &cnf, &out, false));
        assert!(r.all_accepted);
    }
}

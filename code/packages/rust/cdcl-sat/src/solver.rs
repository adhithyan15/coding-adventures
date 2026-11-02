//! # The CDCL solver
//!
//! This module is the whole search engine. It is long because CDCL has
//! several cooperating parts, but each part is one function, and they are in
//! the order a reader meets them in a textbook:
//!
//! 1. **Adding clauses** ([`Solver::add_clause`]) — level-0 simplification.
//! 2. **Assignment and the trail** ([`Solver::enqueue`],
//!    [`Solver::cancel_until`]).
//! 3. **Unit propagation** with two watched literals ([`Solver::propagate`]).
//! 4. **Conflict analysis**: first UIP and minimisation
//!    ([`Solver::analyze`]).
//! 5. **Failed assumptions** ([`Solver::analyze_final`]).
//! 6. **Heuristics**: VSIDS bumping and decay, phase saving, learned-clause
//!    reduction ([`Solver::reduce_db`]).
//! 7. **The search loop** ([`Solver::search`]) and the restart driver
//!    ([`Solver::solve_with_assumptions`]).
//!
//! ## The big picture
//!
//! A CNF formula is an AND of clauses; a clause is an OR of literals. The
//! solver builds an assignment one literal at a time:
//!
//! ```text
//!   decide   pick an unassigned variable, guess a value      (a new level)
//!   propagate   every clause with one unassigned literal left and the rest
//!               false forces that literal                    (same level)
//!   conflict    some clause has every literal false
//!       → analyze: find a short clause the formula implies that explains
//!         the conflict ("learn" it)
//!       → backjump: undo decisions until the learned clause can fire
//! ```
//!
//! The solver answers **SAT** when every variable is assigned with no
//! conflict, and **UNSAT** when a conflict happens at decision level 0, where
//! nothing was guessed, so the formula itself is contradictory.
//!
//! ## Where proofs come from (VER11)
//!
//! Every clause the solver learns is implied by the formula *and* is checkable
//! by reverse unit propagation (RUP). VER11 adds a proof sink that records
//! each learned clause, each deletion and the final empty clause in DRAT
//! form. The four events it needs are each produced in exactly one place
//! here: `add_clause` (originals and their level-0 simplification),
//! `record_learnt` (lemmas), `reduce_db` (deletions) and `search` (the empty
//! clause, when a conflict reaches level 0).

use crate::heap::VarHeap;
use crate::lit::{LBool, Lit, Var};
use crate::luby::luby;
use crate::proof::{DratProof, ProofStep};

// ---------------------------------------------------------------------------
// Public result and statistics types
// ---------------------------------------------------------------------------

/// The answer to a `solve` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveResult {
    /// A satisfying assignment exists; read it with [`Solver::model`].
    Sat,
    /// No assignment satisfies the clauses (and assumptions, if any). If
    /// assumptions were given, [`Solver::failed_assumptions`] says which.
    Unsat,
    /// The conflict budget ran out before an answer was found.
    Unknown,
}

/// Counters describing the work a solver has done. Useful for tests,
/// benchmarks and budget tuning; nothing in the search reads them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Number of conflicts analysed.
    pub conflicts: u64,
    /// Number of decisions made (assumptions included).
    pub decisions: u64,
    /// Number of literals assigned by unit propagation.
    pub propagations: u64,
    /// Number of restarts.
    pub restarts: u64,
    /// Number of learned clauses (unit learned clauses included).
    pub learned: u64,
    /// Number of learned clauses deleted by database reduction.
    pub deleted: u64,
    /// Number of literals removed from learned clauses by minimisation.
    pub minimized_literals: u64,
}

// ---------------------------------------------------------------------------
// Internal storage
// ---------------------------------------------------------------------------

/// Index of a clause in the solver's clause arena.
type ClauseRef = u32;

/// "No reason": the variable is a decision, an assumption, or a level-0
/// unit.
const NO_REASON: ClauseRef = ClauseRef::MAX;

/// A stored clause of length ≥ 2. (Unit clauses are never stored: they are
/// assigned at level 0 the moment they are added or learned.)
#[derive(Debug, Clone)]
struct Clause {
    /// The literals. Positions 0 and 1 are the two watched literals.
    lits: Vec<Lit>,
    /// Learned (deletable) rather than original.
    learnt: bool,
    /// Literal block distance at learning time: how many distinct decision
    /// levels its literals spanned. Low LBD ("glue") clauses are kept.
    lbd: u32,
    /// Bumped when the clause takes part in conflict analysis.
    activity: f64,
    /// Tombstone: the slot is free and must not be used.
    deleted: bool,
}

/// An entry in a watch list.
///
/// `watches[l]` lists the clauses that watch literal `l`; they are visited
/// when `l` becomes false. The **blocker** is some other literal of the
/// clause: if it is already true the clause is satisfied and the solver skips
/// it without touching the clause's memory, which is most of the time.
#[derive(Debug, Clone, Copy)]
struct Watcher {
    cref: ClauseRef,
    blocker: Lit,
}

/// The value of `lit` under `assigns`. A free function, not a method, so it
/// can be called while other fields of the solver are mutably borrowed.
#[inline]
fn lit_value(assigns: &[LBool], lit: Lit) -> LBool {
    assigns[lit.var().index()].negate_if(lit.is_negated())
}

// Tuning constants. The values are MiniSat's, which most CDCL solvers start
// from; nothing here depends on them for correctness.
/// VSIDS decay: every conflict, all variable activities shrink by this factor
/// (implemented by growing the bump increment instead).
const VAR_DECAY: f64 = 0.95;
/// Clause activity decay.
const CLAUSE_DECAY: f64 = 0.999;
/// Conflicts per Luby unit: restart `i` comes after `RESTART_UNIT · luby(i)`.
const RESTART_UNIT: u64 = 100;
/// Activities above this are rescaled to stay inside `f64` range.
const RESCALE_LIMIT: f64 = 1e100;
/// Clause activities above this are rescaled.
const CLAUSE_RESCALE_LIMIT: f64 = 1e20;
/// The learned-clause limit starts at `max(originals / 3, this)`.
const MIN_LEARNT_LIMIT: f64 = 2000.0;
/// The learned-clause limit grows by this factor at each reduction.
const LEARNT_LIMIT_GROWTH: f64 = 1.1;

// ---------------------------------------------------------------------------
// The solver
// ---------------------------------------------------------------------------

/// A conflict-driven clause-learning SAT solver.
///
/// ```
/// use cdcl_sat::{Solver, SolveResult, Var};
///
/// let mut s = Solver::new();
/// let a = s.new_var();
/// let b = s.new_var();
/// s.add_clause(&[a.positive(), b.positive()]);   //  a ∨ b
/// s.add_clause(&[a.negative()]);            // ¬a
/// assert_eq!(s.solve(), SolveResult::Sat);
/// assert_eq!(s.model_value(b), Some(true));
///
/// // Under the assumption ¬b the formula is UNSAT, and ¬b is to blame.
/// assert_eq!(s.solve_with_assumptions(&[b.negative()]), SolveResult::Unsat);
/// assert_eq!(s.failed_assumptions(), &[b.negative()]);
/// ```
#[derive(Debug, Clone)]
pub struct Solver {
    // --- clause database ---
    /// Clause arena; `ClauseRef` indexes it.
    clauses: Vec<Clause>,
    /// Freed arena slots, reused before the arena grows.
    free_slots: Vec<ClauseRef>,
    /// References of the live learned clauses.
    learnts: Vec<ClauseRef>,
    /// Number of live original (non-learned) stored clauses.
    num_original: usize,
    /// `watches[lit.code()]`: clauses watching `lit`.
    watches: Vec<Vec<Watcher>>,

    // --- assignment ---
    /// Current value of each variable.
    assigns: Vec<LBool>,
    /// Decision level at which each variable was assigned.
    level: Vec<u32>,
    /// The clause that forced each variable, or `NO_REASON`.
    reason: Vec<ClauseRef>,
    /// Assigned literals in order.
    trail: Vec<Lit>,
    /// `trail_lim[d]` = trail length when decision level `d + 1` began.
    trail_lim: Vec<usize>,
    /// Next trail position unit propagation has not yet processed.
    qhead: usize,

    // --- heuristics ---
    /// VSIDS activity per variable.
    activity: Vec<f64>,
    /// Current activity bump.
    var_inc: f64,
    /// Decision order.
    order: VarHeap,
    /// Saved phase: `true` means the variable was last assigned false, so
    /// the next decision on it will be the negative literal. Starts `true`.
    saved_negated: Vec<bool>,
    /// Current clause-activity bump.
    cla_inc: f64,
    /// Learned-clause count that triggers a reduction.
    max_learnts: f64,

    // --- conflict-analysis scratch space (kept to avoid reallocating) ---
    seen: Vec<bool>,
    analyze_stack: Vec<Lit>,
    analyze_toclear: Vec<Lit>,

    // --- status and results ---
    /// False once the clauses alone are known to be UNSAT.
    ok: bool,
    /// Assumptions for the current call.
    assumptions: Vec<Lit>,
    /// Model after a SAT answer.
    model: Vec<bool>,
    /// Failed assumptions after an UNSAT-under-assumptions answer.
    failed: Vec<Lit>,
    /// Remaining conflicts allowed, if the caller set a budget.
    conflict_budget: Option<u64>,
    stats: Stats,

    // --- proof logging (VER11 §4) ---
    /// The DRAT proof being recorded, if logging is on.
    proof: Option<DratProof>,
    /// Whether any clause has ever been added. Logging must start before
    /// the first one, or level-0 simplifications would go unrecorded.
    touched: bool,
}

impl Default for Solver {
    fn default() -> Self {
        Solver::new()
    }
}

impl Solver {
    /// A solver with no variables and no clauses.
    pub fn new() -> Solver {
        Solver {
            clauses: Vec::new(),
            free_slots: Vec::new(),
            learnts: Vec::new(),
            num_original: 0,
            watches: Vec::new(),
            assigns: Vec::new(),
            level: Vec::new(),
            reason: Vec::new(),
            trail: Vec::new(),
            trail_lim: Vec::new(),
            qhead: 0,
            activity: Vec::new(),
            var_inc: 1.0,
            order: VarHeap::default(),
            saved_negated: Vec::new(),
            cla_inc: 1.0,
            max_learnts: MIN_LEARNT_LIMIT,
            seen: Vec::new(),
            analyze_stack: Vec::new(),
            analyze_toclear: Vec::new(),
            ok: true,
            assumptions: Vec::new(),
            model: Vec::new(),
            failed: Vec::new(),
            conflict_budget: None,
            stats: Stats::default(),
            proof: None,
            touched: false,
        }
    }

    // -----------------------------------------------------------------------
    // Variables
    // -----------------------------------------------------------------------

    /// Create a fresh variable.
    pub fn new_var(&mut self) -> Var {
        let v = Var::from_index(self.assigns.len());
        self.assigns.push(LBool::Undef);
        self.level.push(0);
        self.reason.push(NO_REASON);
        self.activity.push(0.0);
        self.saved_negated.push(true);
        self.seen.push(false);
        self.watches.push(Vec::new());
        self.watches.push(Vec::new());
        self.order.insert(v.index(), &self.activity);
        v
    }

    /// Make sure variables `0..n` exist, creating any that are missing.
    pub fn reserve_vars(&mut self, n: usize) {
        while self.assigns.len() < n {
            self.new_var();
        }
    }

    /// Number of variables.
    pub fn num_vars(&self) -> usize {
        self.assigns.len()
    }

    /// Number of live stored clauses (original and learned). Unit clauses
    /// are assignments, not stored clauses, and are not counted.
    pub fn num_clauses(&self) -> usize {
        self.num_original + self.learnts.len()
    }

    /// Number of live learned clauses.
    pub fn num_learnts(&self) -> usize {
        self.learnts.len()
    }

    /// Work counters.
    pub fn stats(&self) -> Stats {
        self.stats
    }

    /// False once the clauses alone (without assumptions) are known UNSAT.
    /// After that every `solve` answers `Unsat` immediately.
    pub fn is_ok(&self) -> bool {
        self.ok
    }

    /// Start recording a DRAT proof (VER11 §4). Must be called before the
    /// first clause is added, because simplifications made while adding
    /// clauses are part of the proof; returns `false` (and records nothing)
    /// if clauses were already added.
    ///
    /// ```
    /// use cdcl_sat::{Solver, SolveResult, Var};
    ///
    /// let mut s = Solver::new();
    /// assert!(s.enable_proof());
    /// let x = Var::from_index(0);
    /// s.add_clause(&[x.positive()]);
    /// s.add_clause(&[x.negative()]);
    /// assert_eq!(s.solve(), SolveResult::Unsat);
    /// assert_eq!(s.proof().unwrap().to_drat_text(), "0\n");
    /// ```
    pub fn enable_proof(&mut self) -> bool {
        if self.touched {
            return false;
        }
        self.proof.get_or_insert_with(DratProof::default);
        true
    }

    /// The DRAT proof recorded so far, if logging was enabled. After an
    /// `Unsat` answer without assumptions it ends with the empty clause and
    /// `drat-check` can certify it against the clauses added.
    pub fn proof(&self) -> Option<&DratProof> {
        self.proof.as_ref()
    }

    /// Take the recorded proof out of the solver. Logging continues into a
    /// fresh, empty proof.
    pub fn take_proof(&mut self) -> Option<DratProof> {
        self.proof.as_mut().map(std::mem::take)
    }

    /// Record a lemma, if logging.
    fn log_add(&mut self, lits: &[Lit]) {
        if let Some(p) = self.proof.as_mut() {
            p.steps.push(ProofStep::Add(lits.to_vec()));
        }
    }

    /// Limit the number of conflicts the **next** `solve` calls may use in
    /// total. `None` removes the limit. When the budget runs out, `solve`
    /// returns [`SolveResult::Unknown`] and the solver remains usable.
    pub fn set_conflict_budget(&mut self, budget: Option<u64>) {
        self.conflict_budget = budget;
    }

    // -----------------------------------------------------------------------
    // 1. Adding clauses
    // -----------------------------------------------------------------------

    /// Add a clause (an OR of literals). Variables that do not exist yet are
    /// created: **every** variable up to the largest index mentioned, at
    /// about 100 bytes each. Variable indices are the caller's to bound;
    /// a caller handling untrusted input should number variables densely
    /// (as `constraint-engine` does with [`Solver::new_var`]) or check
    /// indices first, because one literal of index 2^31 asks for hundreds
    /// of gigabytes.
    ///
    /// Returns `false` if the solver now knows the clauses are UNSAT (the
    /// clause was empty after simplification, or contradicted a level-0
    /// fact). Adding more clauses after that is allowed and does nothing.
    ///
    /// The clause is simplified against what is already known at level 0
    /// (VER09 §4.7):
    ///
    /// ```text
    ///   (x ∨ ¬x ∨ …)        a tautology: always true, dropped
    ///   (x ∨ …) with x true  already satisfied, dropped
    ///   (x ∨ y) with x false shortened to (y)
    ///   ()                   empty: the formula is UNSAT
    ///   (y)                  a unit: y is assigned at level 0
    /// ```
    pub fn add_clause(&mut self, lits: &[Lit]) -> bool {
        // Clauses are only added between searches, at level 0.
        self.cancel_until(0);
        self.touched = true;
        if !self.ok {
            return false;
        }
        if let Some(max) = lits.iter().map(|l| l.var().index()).max() {
            self.reserve_vars(max + 1);
        }

        // Sort so that duplicates and complementary pairs are adjacent:
        // x and ¬x have codes 2v and 2v+1.
        let mut c: Vec<Lit> = lits.to_vec();
        c.sort_unstable();
        c.dedup();
        let mut kept: Vec<Lit> = Vec::with_capacity(c.len());
        for (i, &l) in c.iter().enumerate() {
            if i > 0 && c[i - 1] == !l {
                return true; // tautology
            }
            match lit_value(&self.assigns, l) {
                LBool::True => return true, // already satisfied
                LBool::False => {}          // false at level 0: drop it
                LBool::Undef => kept.push(l),
            }
        }
        // The checker sees the clause as given. If level-0 facts shortened
        // it, the shorter clause is a lemma (RUP: the dropped literals are
        // false by units the checker can propagate too). An empty result is
        // the empty clause, whether or not anything was dropped.
        if kept.len() < c.len() || kept.is_empty() {
            self.log_add(&kept);
        }

        match kept.len() {
            0 => {
                self.ok = false;
                false
            }
            1 => {
                self.enqueue(kept[0], NO_REASON);
                // Propagate now, so a contradiction among units is found
                // before the next clause is simplified against them.
                if self.propagate().is_some() {
                    self.ok = false;
                    self.log_add(&[]);
                }
                self.ok
            }
            _ => {
                let cref = self.alloc_clause(kept, false, 0);
                self.attach(cref);
                self.num_original += 1;
                true
            }
        }
    }

    /// Put a clause in the arena, reusing a freed slot if there is one.
    fn alloc_clause(&mut self, lits: Vec<Lit>, learnt: bool, lbd: u32) -> ClauseRef {
        let clause = Clause { lits, learnt, lbd, activity: 0.0, deleted: false };
        if let Some(slot) = self.free_slots.pop() {
            self.clauses[slot as usize] = clause;
            slot
        } else {
            let slot = self.clauses.len();
            assert!(slot < NO_REASON as usize, "clause arena exhausted");
            self.clauses.push(clause);
            slot as ClauseRef
        }
    }

    /// Start watching a clause's first two literals.
    fn attach(&mut self, cref: ClauseRef) {
        let c = &self.clauses[cref as usize].lits;
        let (l0, l1) = (c[0], c[1]);
        self.watches[l0.code()].push(Watcher { cref, blocker: l1 });
        self.watches[l1.code()].push(Watcher { cref, blocker: l0 });
    }

    // -----------------------------------------------------------------------
    // 2. Assignment and the trail
    // -----------------------------------------------------------------------

    fn decision_level(&self) -> usize {
        self.trail_lim.len()
    }

    fn value(&self, lit: Lit) -> LBool {
        lit_value(&self.assigns, lit)
    }

    /// Make `lit` true at the current decision level, with the given reason.
    /// The caller guarantees `lit` is unassigned.
    fn enqueue(&mut self, lit: Lit, reason: ClauseRef) {
        let v = lit.var().index();
        debug_assert_eq!(self.assigns[v], LBool::Undef);
        self.assigns[v] = LBool::from_bool(!lit.is_negated());
        self.level[v] = self.decision_level() as u32;
        self.reason[v] = reason;
        self.trail.push(lit);
    }

    fn new_decision_level(&mut self) {
        self.trail_lim.push(self.trail.len());
    }

    /// Backtrack: unassign everything above decision level `target`.
    ///
    /// Each unassigned variable remembers the polarity it had (phase saving)
    /// and goes back into the decision heap. The watch lists need no repair:
    /// the two-watched-literal invariant survives unassignment.
    fn cancel_until(&mut self, target: usize) {
        if self.decision_level() <= target {
            return;
        }
        let keep = self.trail_lim[target];
        for i in (keep..self.trail.len()).rev() {
            let lit = self.trail[i];
            let v = lit.var().index();
            self.assigns[v] = LBool::Undef;
            self.reason[v] = NO_REASON;
            self.saved_negated[v] = lit.is_negated();
            self.order.insert(v, &self.activity);
        }
        self.trail.truncate(keep);
        self.trail_lim.truncate(target);
        self.qhead = keep;
    }

    // -----------------------------------------------------------------------
    // 3. Unit propagation with two watched literals
    // -----------------------------------------------------------------------

    /// Propagate every assignment on the trail not yet processed. Returns the
    /// conflicting clause, if one became all-false.
    ///
    /// When literal `p` becomes true, `¬p` becomes false, and only the
    /// clauses watching `¬p` can have become unit or conflicting. For each:
    ///
    /// ```text
    ///   blocker true?             satisfied, skip               (fast path)
    ///   make ¬p the second watch
    ///   first watch true?         satisfied, keep watching ¬p
    ///   some other literal not false?  watch it instead of ¬p
    ///   else first watch unassigned?   unit: assign it
    ///   else                           conflict
    /// ```
    fn propagate(&mut self) -> Option<ClauseRef> {
        let mut conflict = None;
        while self.qhead < self.trail.len() {
            let p = self.trail[self.qhead];
            self.qhead += 1;
            let false_lit = !p;
            // Take the watch list out so clauses can be inspected and other
            // watch lists extended while this one is rewritten in place.
            let mut ws = std::mem::take(&mut self.watches[false_lit.code()]);
            let mut i = 0;
            let mut j = 0;
            while i < ws.len() {
                let w = ws[i];
                i += 1;
                if lit_value(&self.assigns, w.blocker) == LBool::True {
                    ws[j] = w;
                    j += 1;
                    continue;
                }
                let cref = w.cref;
                let lits = &mut self.clauses[cref as usize].lits;
                // Normalise: the false literal goes to position 1.
                if lits[0] == false_lit {
                    lits.swap(0, 1);
                }
                debug_assert_eq!(lits[1], false_lit);
                let first = lits[0];
                let keep = Watcher { cref, blocker: first };
                if first != w.blocker && lit_value(&self.assigns, first) == LBool::True {
                    ws[j] = keep;
                    j += 1;
                    continue;
                }
                // Look for a replacement watch.
                let mut moved = false;
                for k in 2..lits.len() {
                    if lit_value(&self.assigns, lits[k]) != LBool::False {
                        lits.swap(1, k);
                        let new_watch = lits[1];
                        self.watches[new_watch.code()].push(keep);
                        moved = true;
                        break;
                    }
                }
                if moved {
                    continue; // no longer watches false_lit: drop from ws
                }
                // Every literal but `first` is false.
                ws[j] = keep;
                j += 1;
                if lit_value(&self.assigns, first) == LBool::False {
                    conflict = Some(cref);
                    self.qhead = self.trail.len();
                    // Keep the watchers not yet visited.
                    while i < ws.len() {
                        ws[j] = ws[i];
                        j += 1;
                        i += 1;
                    }
                } else {
                    self.stats.propagations += 1;
                    self.enqueue(first, cref);
                }
            }
            ws.truncate(j);
            self.watches[false_lit.code()] = ws;
            if conflict.is_some() {
                break;
            }
        }
        conflict
    }

    // -----------------------------------------------------------------------
    // 4. Conflict analysis
    // -----------------------------------------------------------------------

    /// Derive a learned clause from a conflict (VER09 §4.3).
    ///
    /// Returns the learned clause with the asserting (UIP) literal first and
    /// a literal of the backjump level second, the backjump level, and the
    /// clause's LBD.
    ///
    /// The walk resolves the conflict clause with reasons of current-level
    /// literals, newest first, until exactly one current-level literal is
    /// left: the first unique implication point. Lower-level literals go
    /// straight into the learned clause; level-0 literals are dropped
    /// because they are false in every assignment the solver will consider.
    fn analyze(&mut self, conflict: ClauseRef) -> (Vec<Lit>, usize, u32) {
        let current = self.decision_level() as u32;
        // out_learnt[0] is a placeholder for the UIP literal.
        let mut learnt: Vec<Lit> = vec![Lit::from_code(0)];
        let mut path_count = 0usize; // current-level literals still to resolve
        let mut index = self.trail.len();
        let mut confl = conflict;
        let mut uip: Option<Lit> = None;

        loop {
            self.bump_clause(confl);
            // For a reason clause, position 0 is the literal it implied
            // (that is `uip`, already handled), so skip it.
            let start = if uip.is_none() { 0 } else { 1 };
            let len = self.clauses[confl as usize].lits.len();
            for k in start..len {
                let q = self.clauses[confl as usize].lits[k];
                let v = q.var().index();
                if !self.seen[v] && self.level[v] > 0 {
                    self.bump_var(v);
                    self.seen[v] = true;
                    if self.level[v] >= current {
                        path_count += 1;
                    } else {
                        learnt.push(q);
                    }
                }
            }
            // Next: the newest trail literal marked as seen.
            loop {
                index -= 1;
                if self.seen[self.trail[index].var().index()] {
                    break;
                }
            }
            let p = self.trail[index];
            uip = Some(p);
            self.seen[p.var().index()] = false;
            path_count -= 1;
            if path_count == 0 {
                break;
            }
            confl = self.reason[p.var().index()];
            debug_assert_ne!(confl, NO_REASON, "a non-UIP current-level literal has a reason");
        }
        learnt[0] = !uip.expect("analysis visits at least one literal");

        // --- Minimisation (MiniSat's recursive form) ---
        //
        // A literal is redundant if it is implied by the other literals of the
        // learned clause: following its reason backwards only ever reaches
        // literals already in the clause (or level-0 facts). Dropping it keeps
        // the clause implied and RUP.
        self.analyze_toclear.clear();
        self.analyze_toclear.extend_from_slice(&learnt);
        let abstract_levels = learnt[1..]
            .iter()
            .fold(0u32, |acc, l| acc | self.abstract_level(l.var()));
        let before = learnt.len();
        let mut j = 1;
        for i in 1..learnt.len() {
            let l = learnt[i];
            if self.reason[l.var().index()] == NO_REASON || !self.lit_redundant(l, abstract_levels) {
                learnt[j] = l;
                j += 1;
            }
        }
        learnt.truncate(j);
        self.stats.minimized_literals += (before - learnt.len()) as u64;
        for k in 0..self.analyze_toclear.len() {
            let v = self.analyze_toclear[k].var().index();
            self.seen[v] = false;
        }

        // --- Backjump level: the highest level among the other literals ---
        let backjump = if learnt.len() == 1 {
            0
        } else {
            let mut max_i = 1;
            for i in 2..learnt.len() {
                if self.level[learnt[i].var().index()] > self.level[learnt[max_i].var().index()] {
                    max_i = i;
                }
            }
            learnt.swap(1, max_i);
            self.level[learnt[1].var().index()] as usize
        };

        let lbd = self.compute_lbd(&learnt);
        (learnt, backjump, lbd)
    }

    /// A 32-bit summary of a variable's level, used to cut minimisation short:
    /// a literal whose level is not among the clause's levels cannot be
    /// implied by the clause's literals.
    fn abstract_level(&self, v: Var) -> u32 {
        1 << (self.level[v.index()] & 31)
    }

    /// Is `p` implied by literals already marked `seen` (VER09 §4.3)?
    /// Marks every literal it proves redundant along the way, and undoes
    /// its marks if the answer is no.
    fn lit_redundant(&mut self, p: Lit, abstract_levels: u32) -> bool {
        self.analyze_stack.clear();
        self.analyze_stack.push(p);
        let top = self.analyze_toclear.len();
        while let Some(q) = self.analyze_stack.pop() {
            let cref = self.reason[q.var().index()];
            debug_assert_ne!(cref, NO_REASON);
            let len = self.clauses[cref as usize].lits.len();
            for k in 1..len {
                let l = self.clauses[cref as usize].lits[k];
                let v = l.var().index();
                if self.seen[v] || self.level[v] == 0 {
                    continue;
                }
                if self.reason[v] != NO_REASON && (self.abstract_level(l.var()) & abstract_levels) != 0 {
                    self.seen[v] = true;
                    self.analyze_stack.push(l);
                    self.analyze_toclear.push(l);
                } else {
                    for t in top..self.analyze_toclear.len() {
                        let tv = self.analyze_toclear[t].var().index();
                        self.seen[tv] = false;
                    }
                    self.analyze_toclear.truncate(top);
                    return false;
                }
            }
        }
        true
    }

    /// Number of distinct decision levels among `lits`.
    fn compute_lbd(&self, lits: &[Lit]) -> u32 {
        let mut levels: Vec<u32> = lits.iter().map(|l| self.level[l.var().index()]).collect();
        levels.sort_unstable();
        levels.dedup();
        levels.len() as u32
    }

    /// Store a learned clause and assert its first literal. The solver has
    /// already backjumped to the level where the clause is unit.
    fn record_learnt(&mut self, learnt: Vec<Lit>, lbd: u32) {
        self.stats.learned += 1;
        self.log_add(&learnt);
        if learnt.len() == 1 {
            self.enqueue(learnt[0], NO_REASON);
        } else {
            let first = learnt[0];
            let cref = self.alloc_clause(learnt, true, lbd);
            self.attach(cref);
            self.learnts.push(cref);
            self.bump_clause(cref);
            self.enqueue(first, cref);
        }
    }

    // -----------------------------------------------------------------------
    // 5. Failed assumptions
    // -----------------------------------------------------------------------

    /// Assumption `p` turned out false. Find which assumptions forced that
    /// (VER09 §4.6), by following reasons back from `¬p` to the decisions
    /// they rest on. Above level 0, while assumptions are being placed, every
    /// decision is an assumption, so the decisions reached are the core.
    fn analyze_final(&mut self, p: Lit) {
        self.failed.clear();
        self.failed.push(p);
        if self.decision_level() == 0 {
            return; // ¬p is a level-0 fact: p fails on its own
        }
        self.seen[p.var().index()] = true;
        for i in (self.trail_lim[0]..self.trail.len()).rev() {
            let lit = self.trail[i];
            let v = lit.var().index();
            if !self.seen[v] {
                continue;
            }
            let r = self.reason[v];
            if r == NO_REASON {
                // A decision: an assumption that is part of the core. (If it
                // is `¬p` itself, `p` and `¬p` were both assumed.)
                self.failed.push(lit);
            } else {
                let len = self.clauses[r as usize].lits.len();
                for k in 1..len {
                    let l = self.clauses[r as usize].lits[k];
                    if self.level[l.var().index()] > 0 {
                        self.seen[l.var().index()] = true;
                    }
                }
            }
            self.seen[v] = false;
        }
        self.seen[p.var().index()] = false;
        self.failed.sort_unstable();
        self.failed.dedup();
    }

    // -----------------------------------------------------------------------
    // 6. Heuristics
    // -----------------------------------------------------------------------

    /// VSIDS: raise `v`'s activity by the current increment.
    fn bump_var(&mut self, v: usize) {
        self.activity[v] += self.var_inc;
        if self.activity[v] > RESCALE_LIMIT {
            // Scale everything down; relative order is unchanged.
            for a in self.activity.iter_mut() {
                *a *= 1.0 / RESCALE_LIMIT;
            }
            self.var_inc *= 1.0 / RESCALE_LIMIT;
        }
        self.order.increased(v, &self.activity);
    }

    /// Decay all variable activities, by growing the increment instead of
    /// touching every variable: bumps made later count for more.
    fn decay_var_activity(&mut self) {
        self.var_inc /= VAR_DECAY;
    }

    fn bump_clause(&mut self, cref: ClauseRef) {
        let c = &mut self.clauses[cref as usize];
        if !c.learnt {
            return;
        }
        c.activity += self.cla_inc;
        if c.activity > CLAUSE_RESCALE_LIMIT {
            for &l in &self.learnts {
                self.clauses[l as usize].activity *= 1.0 / CLAUSE_RESCALE_LIMIT;
            }
            self.cla_inc *= 1.0 / CLAUSE_RESCALE_LIMIT;
        }
    }

    fn decay_clause_activity(&mut self) {
        self.cla_inc /= CLAUSE_DECAY;
    }

    /// The next decision literal: the most active unassigned variable, in its
    /// saved phase. `None` when every variable is assigned.
    fn pick_branch_lit(&mut self) -> Option<Lit> {
        while let Some(v) = self.order.pop_max(&self.activity) {
            if self.assigns[v] == LBool::Undef {
                return Some(Var::from_index(v).lit(self.saved_negated[v]));
            }
        }
        None
    }

    /// Is the clause the reason for a current assignment? Such a clause must
    /// not be deleted: conflict analysis may need it.
    fn locked(&self, cref: ClauseRef) -> bool {
        let first = self.clauses[cref as usize].lits[0];
        self.reason[first.var().index()] == cref && self.value(first) == LBool::True
    }

    /// Delete the worse half of the learned clauses (VER09 §4.5).
    ///
    /// "Worse" means higher LBD, then lower activity. Kept regardless: clauses
    /// that are currently reasons, binary clauses, and glue clauses
    /// (LBD ≤ 2), which experience shows stay useful for the whole search.
    fn reduce_db(&mut self) {
        let mut order = std::mem::take(&mut self.learnts);
        {
            let clauses = &self.clauses;
            order.sort_by(|&a, &b| {
                let (ca, cb) = (&clauses[a as usize], &clauses[b as usize]);
                cb.lbd
                    .cmp(&ca.lbd)
                    .then(ca.activity.partial_cmp(&cb.activity).unwrap_or(std::cmp::Ordering::Equal))
            });
        }
        let target = order.len() / 2;
        let mut removed = 0;
        let mut kept = Vec::with_capacity(order.len());
        for cref in order {
            let c = &self.clauses[cref as usize];
            let deletable = removed < target && c.lbd > 2 && c.lits.len() > 2 && !self.locked(cref);
            if deletable {
                if let Some(p) = self.proof.as_mut() {
                    p.steps.push(ProofStep::Delete(self.clauses[cref as usize].lits.clone()));
                }
                self.clauses[cref as usize].deleted = true;
                removed += 1;
            } else {
                kept.push(cref);
            }
        }
        self.learnts = kept;
        if removed > 0 {
            // Detach deleted clauses from every watch list, then free their
            // slots. Watches must go first: a freed slot can be reused.
            let clauses = &self.clauses;
            for ws in self.watches.iter_mut() {
                ws.retain(|w| !clauses[w.cref as usize].deleted);
            }
            for (i, c) in self.clauses.iter_mut().enumerate() {
                if c.deleted && !c.lits.is_empty() {
                    c.lits = Vec::new();
                    self.free_slots.push(i as ClauseRef);
                }
            }
            self.stats.deleted += removed as u64;
        }
        self.max_learnts *= LEARNT_LIMIT_GROWTH;
    }

    // -----------------------------------------------------------------------
    // 7. Search
    // -----------------------------------------------------------------------

    /// Run CDCL until a model, a proof of UNSAT, `max_conflicts` conflicts
    /// (time to restart) or the budget runs out. Returns `True` (SAT),
    /// `False` (UNSAT, possibly only under the assumptions) or `Undef`
    /// (restart or budget).
    fn search(&mut self, max_conflicts: u64) -> LBool {
        let mut conflicts_here = 0u64;
        loop {
            if let Some(confl) = self.propagate() {
                self.stats.conflicts += 1;
                conflicts_here += 1;
                if let Some(b) = self.conflict_budget.as_mut() {
                    *b = b.saturating_sub(1);
                }
                if self.decision_level() == 0 {
                    // A conflict with nothing guessed: the formula is UNSAT.
                    self.ok = false;
                    self.log_add(&[]);
                    return LBool::False;
                }
                let (learnt, backjump, lbd) = self.analyze(confl);
                self.cancel_until(backjump);
                self.record_learnt(learnt, lbd);
                self.decay_var_activity();
                self.decay_clause_activity();
                continue;
            }

            if conflicts_here >= max_conflicts || self.conflict_budget == Some(0) {
                self.cancel_until(0);
                return LBool::Undef;
            }
            if self.learnts.len() as f64 >= self.max_learnts {
                self.reduce_db();
            }

            // Assumptions first, one per decision level.
            let mut next = None;
            while self.decision_level() < self.assumptions.len() {
                let a = self.assumptions[self.decision_level()];
                match self.value(a) {
                    LBool::True => self.new_decision_level(), // already holds: empty level
                    LBool::False => {
                        self.analyze_final(a);
                        return LBool::False;
                    }
                    LBool::Undef => {
                        next = Some(a);
                        break;
                    }
                }
            }
            let decision = match next {
                Some(a) => a,
                None => match self.pick_branch_lit() {
                    Some(l) => l,
                    None => return LBool::True, // everything assigned, no conflict
                },
            };
            self.stats.decisions += 1;
            self.new_decision_level();
            self.enqueue(decision, NO_REASON);
        }
    }

    /// Solve the clauses added so far.
    pub fn solve(&mut self) -> SolveResult {
        self.solve_with_assumptions(&[])
    }

    /// Solve the clauses added so far, with the given literals assumed true
    /// for this call only (VER09 §4.6). Learned clauses are kept for later
    /// calls. As with [`Solver::add_clause`], assumptions on new variables
    /// create every variable up to the largest index.
    pub fn solve_with_assumptions(&mut self, assumptions: &[Lit]) -> SolveResult {
        self.model.clear();
        self.failed.clear();
        if !self.ok {
            return SolveResult::Unsat;
        }
        if let Some(max) = assumptions.iter().map(|l| l.var().index()).max() {
            self.reserve_vars(max + 1);
        }
        self.assumptions = assumptions.to_vec();
        if self.max_learnts <= MIN_LEARNT_LIMIT {
            self.max_learnts = (self.num_original as f64 / 3.0).max(MIN_LEARNT_LIMIT);
        }

        let mut status = LBool::Undef;
        let mut restart = 0u64;
        while status == LBool::Undef {
            if self.conflict_budget == Some(0) {
                break;
            }
            let limit = RESTART_UNIT.saturating_mul(luby(restart));
            status = self.search(limit);
            if status == LBool::Undef {
                restart += 1;
                self.stats.restarts += 1;
            }
        }

        let result = match status {
            LBool::True => {
                self.model = self.assigns.iter().map(|&v| v == LBool::True).collect();
                SolveResult::Sat
            }
            LBool::False => SolveResult::Unsat,
            LBool::Undef => SolveResult::Unknown,
        };
        self.cancel_until(0);
        self.assumptions.clear();
        result
    }

    // -----------------------------------------------------------------------
    // Results
    // -----------------------------------------------------------------------

    /// The model from the last `Sat` answer: one value per variable. Empty
    /// after any other answer.
    pub fn model(&self) -> &[bool] {
        &self.model
    }

    /// The model's value for `v`, or `None` if there is no model or `v` was
    /// created after it.
    pub fn model_value(&self, v: Var) -> Option<bool> {
        self.model.get(v.index()).copied()
    }

    /// After an `Unsat` answer under assumptions: the assumptions that were
    /// enough to make it UNSAT, sorted. The clause made of their negations is
    /// implied by the clauses. Empty if the clauses are UNSAT on their own.
    pub fn failed_assumptions(&self) -> &[Lit] {
        &self.failed
    }

    /// Variables assigned at level 0: facts that hold in every model.
    pub fn fixed_literals(&self) -> Vec<Lit> {
        let end = self.trail_lim.first().copied().unwrap_or(self.trail.len());
        self.trail[..end].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lits(s: &mut Solver, dimacs: &[i64]) -> Vec<Lit> {
        let v: Vec<Lit> = dimacs.iter().map(|&d| Lit::from_dimacs(d).unwrap()).collect();
        if let Some(m) = v.iter().map(|l| l.var().index()).max() {
            s.reserve_vars(m + 1);
        }
        v
    }

    fn add(s: &mut Solver, dimacs: &[i64]) -> bool {
        let c = lits(s, dimacs);
        s.add_clause(&c)
    }

    #[test]
    fn empty_formula_is_sat() {
        let mut s = Solver::new();
        assert_eq!(s.solve(), SolveResult::Sat);
        assert!(s.model().is_empty());
        let mut s = Solver::default();
        s.new_var();
        assert_eq!(s.solve(), SolveResult::Sat);
        assert_eq!(s.model().len(), 1);
    }

    #[test]
    fn empty_clause_is_unsat_and_stays_unsat() {
        let mut s = Solver::new();
        assert!(!s.add_clause(&[]));
        assert!(!s.is_ok());
        assert_eq!(s.solve(), SolveResult::Unsat);
        assert!(!add(&mut s, &[1]));
        assert_eq!(s.solve(), SolveResult::Unsat);
        assert!(s.model().is_empty());
    }

    #[test]
    fn level_zero_simplification() {
        let mut s = Solver::new();
        assert!(add(&mut s, &[1, -1, 2])); // tautology: dropped
        assert_eq!(s.num_clauses(), 0);
        assert!(add(&mut s, &[1])); // unit
        assert!(add(&mut s, &[1, 2, 3])); // satisfied: dropped
        assert_eq!(s.num_clauses(), 0);
        assert!(add(&mut s, &[-1, 2, 3])); // ¬1 false: becomes (2 ∨ 3)
        assert_eq!(s.num_clauses(), 1);
        assert!(add(&mut s, &[2, 2, 3])); // duplicates collapse
        assert!(!add(&mut s, &[-1])); // contradicts the unit
        assert_eq!(s.solve(), SolveResult::Unsat);
        assert_eq!(s.fixed_literals(), vec![Lit::from_dimacs(1).unwrap()]);
    }

    #[test]
    fn contradicting_units_through_propagation() {
        let mut s = Solver::new();
        assert!(add(&mut s, &[-1, 2]));
        assert!(add(&mut s, &[-2, 3]));
        assert!(add(&mut s, &[1]));
        assert!(!add(&mut s, &[-3]));
        assert_eq!(s.solve(), SolveResult::Unsat);
    }

    #[test]
    fn simple_sat_model_is_correct() {
        let mut s = Solver::new();
        add(&mut s, &[1, 2]);
        add(&mut s, &[-1, 2]);
        add(&mut s, &[-2, 3]);
        assert_eq!(s.solve(), SolveResult::Sat);
        let m = s.model();
        assert!(m[1] && m[2]);
        assert_eq!(s.model_value(Var::from_index(1)), Some(true));
        assert_eq!(s.model_value(Var::from_index(9)), None);
    }

    #[test]
    fn all_four_two_variable_clauses_are_unsat() {
        let mut s = Solver::new();
        add(&mut s, &[1, 2]);
        add(&mut s, &[1, -2]);
        add(&mut s, &[-1, 2]);
        add(&mut s, &[-1, -2]);
        assert_eq!(s.solve(), SolveResult::Unsat);
        assert!(s.failed_assumptions().is_empty());
        assert!(s.stats().conflicts >= 1);
    }

    #[test]
    fn assumptions_and_failed_core() {
        let mut s = Solver::new();
        add(&mut s, &[-1, 2]); // 1 → 2
        add(&mut s, &[-2, 3]); // 2 → 3
        let a = lits(&mut s, &[1, -3, 4]);
        assert_eq!(s.solve_with_assumptions(&a), SolveResult::Unsat);
        let core = s.failed_assumptions().to_vec();
        assert!(core.contains(&a[0]) && core.contains(&a[1]));
        assert!(!core.contains(&a[2]), "4 is irrelevant: {core:?}");
        // Without assumptions it is SAT again: nothing was learned as a fact.
        assert_eq!(s.solve(), SolveResult::Sat);
        assert!(s.failed_assumptions().is_empty());
    }

    #[test]
    fn contradictory_assumptions() {
        let mut s = Solver::new();
        s.new_var();
        let x = Var::from_index(0);
        assert_eq!(s.solve_with_assumptions(&[x.positive(), x.negative()]), SolveResult::Unsat);
        assert_eq!(s.failed_assumptions(), &[x.positive(), x.negative()]);
    }

    #[test]
    fn assumption_false_at_level_zero() {
        let mut s = Solver::new();
        add(&mut s, &[-1]);
        let a = lits(&mut s, &[2, 1]);
        assert_eq!(s.solve_with_assumptions(&a), SolveResult::Unsat);
        assert_eq!(s.failed_assumptions(), &[a[1]]);
    }

    #[test]
    fn repeated_assumption_already_true() {
        let mut s = Solver::new();
        add(&mut s, &[-1, 2]);
        let a = lits(&mut s, &[1, 2, 1]);
        assert_eq!(s.solve_with_assumptions(&a), SolveResult::Sat);
        assert!(s.model()[0] && s.model()[1]);
    }

    #[test]
    fn assumptions_create_missing_variables() {
        let mut s = Solver::new();
        let far = Var::from_index(5).positive();
        assert_eq!(s.solve_with_assumptions(&[far]), SolveResult::Sat);
        assert_eq!(s.num_vars(), 6);
        assert_eq!(s.model_value(Var::from_index(5)), Some(true));
    }

    /// Pigeonhole: n+1 pigeons, n holes. Needs real search and learning.
    fn pigeonhole(s: &mut Solver, holes: usize) {
        let pigeons = holes + 1;
        let var = |p: usize, h: usize| Var::from_index(p * holes + h);
        for p in 0..pigeons {
            let c: Vec<Lit> = (0..holes).map(|h| var(p, h).positive()).collect();
            s.add_clause(&c);
        }
        for h in 0..holes {
            for p in 0..pigeons {
                for q in (p + 1)..pigeons {
                    s.add_clause(&[var(p, h).negative(), var(q, h).negative()]);
                }
            }
        }
    }

    #[test]
    fn pigeonhole_is_unsat_and_exercises_learning() {
        let mut s = Solver::new();
        pigeonhole(&mut s, 6);
        assert_eq!(s.solve(), SolveResult::Unsat);
        let st = s.stats();
        assert!(st.learned > 0 && st.decisions > 0 && st.propagations > 0);
    }

    #[test]
    fn conflict_budget_returns_unknown_and_solver_stays_usable() {
        let mut s = Solver::new();
        pigeonhole(&mut s, 7);
        s.set_conflict_budget(Some(10));
        assert_eq!(s.solve(), SolveResult::Unknown);
        assert!(s.model().is_empty());
        assert_eq!(s.solve(), SolveResult::Unknown); // budget already spent
        s.set_conflict_budget(None);
        assert_eq!(s.solve(), SolveResult::Unsat);
    }

    #[test]
    fn reduce_db_deletes_and_reuses_slots() {
        // Force very frequent reductions on a hard instance, and check the
        // answer is still right.
        let mut s = Solver::new();
        pigeonhole(&mut s, 7);
        s.max_learnts = 10.0;
        s.num_original = 0; // keep solve from raising the limit
        assert_eq!(s.solve(), SolveResult::Unsat);
        assert!(s.stats().deleted > 0, "{:?}", s.stats());
        assert!(s.num_learnts() > 0);
    }

    /// The clauses a test added, as DIMACS text for the checker.
    fn dimacs_of(clauses: &[Vec<i64>], vars: usize) -> String {
        let mut t = format!("p cnf {vars} {}\n", clauses.len());
        for c in clauses {
            for l in c {
                t.push_str(&format!("{l} "));
            }
            t.push_str("0\n");
        }
        t
    }

    #[test]
    fn reduce_db_deletions_appear_in_a_checked_proof() {
        let holes = 7;
        let pigeons = holes + 1;
        let mut clauses: Vec<Vec<i64>> = Vec::new();
        let x = |p: usize, h: usize| (p * holes + h + 1) as i64;
        for p in 0..pigeons {
            clauses.push((0..holes).map(|h| x(p, h)).collect());
        }
        for h in 0..holes {
            for p in 0..pigeons {
                for q in p + 1..pigeons {
                    clauses.push(vec![-x(p, h), -x(q, h)]);
                }
            }
        }
        let mut s = Solver::new();
        assert!(s.enable_proof());
        for c in &clauses {
            add(&mut s, c);
        }
        s.max_learnts = 10.0;
        s.num_original = 0;
        assert_eq!(s.solve(), SolveResult::Unsat);
        let proof = s.proof().unwrap();
        assert!(proof.steps.iter().any(|st| matches!(st, ProofStep::Delete(_))));
        let ok = drat_check::check(&dimacs_of(&clauses, pigeons * holes), &proof.to_drat_text())
            .expect("the kernel accepts the solver's proof");
        assert!(ok.deletions > 0);
    }

    #[test]
    fn proof_logging_switches() {
        let mut s = Solver::new();
        assert!(s.proof().is_none());
        assert!(s.take_proof().is_none());
        add(&mut s, &[1, 2]);
        assert!(!s.enable_proof(), "too late: a clause was added");
        assert!(s.proof().is_none());

        let mut s = Solver::new();
        assert!(s.enable_proof());
        assert!(s.enable_proof()); // idempotent before any clause
        add(&mut s, &[1]);
        add(&mut s, &[-1, 2]); // shortened to (2): logged
        let taken = s.take_proof().unwrap();
        assert_eq!(taken.steps, vec![ProofStep::Add(vec![Lit::from_dimacs(2).unwrap()])]);
        assert_eq!(s.proof().unwrap().steps, vec![]);
        // Empty input clause: the empty clause is logged.
        add(&mut s, &[]);
        assert!(s.proof().unwrap().derives_empty_clause());
    }

    #[test]
    fn unit_conflict_while_adding_is_logged() {
        let clauses = vec![vec![-1, 2], vec![-2, 3], vec![1], vec![-3]];
        let mut s = Solver::new();
        s.enable_proof();
        for c in &clauses {
            add(&mut s, c);
        }
        assert_eq!(s.solve(), SolveResult::Unsat);
        let text = s.proof().unwrap().to_drat_text();
        drat_check::check(&dimacs_of(&clauses, 3), &text).expect("certified");
    }

    #[test]
    fn activity_rescaling_preserves_behaviour() {
        let mut s = Solver::new();
        pigeonhole(&mut s, 5);
        s.var_inc = RESCALE_LIMIT / 2.0;
        s.cla_inc = CLAUSE_RESCALE_LIMIT / 2.0;
        assert_eq!(s.solve(), SolveResult::Unsat);
        assert!(s.var_inc < RESCALE_LIMIT);
        assert!(s.cla_inc < CLAUSE_RESCALE_LIMIT);
    }

    #[test]
    fn lbd_counts_distinct_levels() {
        let mut s = Solver::new();
        s.reserve_vars(3);
        s.level = vec![2, 2, 5];
        let c = vec![Var::from_index(0).positive(), Var::from_index(1).positive(), Var::from_index(2).negative()];
        assert_eq!(s.compute_lbd(&c), 2);
    }
}

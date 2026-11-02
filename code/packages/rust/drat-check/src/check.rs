//! # The checking core
//!
//! This file is the trusted kernel (VER11 §3). Everything the verified
//! shipping program labels `proved` at the SAT level rests on it, so it is
//! written to be read in full.
//!
//! ## What it does
//!
//! It keeps a **clause database**: the formula's clauses, plus every lemma
//! accepted so far, minus every clause deleted so far. It walks the proof
//! forward:
//!
//! ```text
//!   add C     accept if C is RUP, else if C is RAT; then put C in the database
//!   delete C  remove one copy of C (it must be there)
//!   add ()    the empty clause: if it is RUP, the formula is UNSAT. Done.
//! ```
//!
//! ## RUP, by example
//!
//! Database: `(a ∨ b)`, `(¬b ∨ c)`, `(¬a ∨ c)`. Lemma: `(c)`.
//!
//! ```text
//!   assume the lemma false:   c = false
//!   (¬b ∨ c) is now unit  →   b = false
//!   (¬a ∨ c) is now unit  →   a = false
//!   (a ∨ b)  is all false →   conflict
//! ```
//!
//! So every model of the database makes `c` true: the lemma is implied,
//! and adding it changes nothing about satisfiability.
//!
//! ## RAT, by example
//!
//! Database: `(¬x ∨ a)`. Lemma `(x)` is not RUP: assuming `x` false
//! propagates nothing. Is it RAT on `x`? The only clause containing `¬x` is
//! `(¬x ∨ a)`, so the one resolvent to check is `(x ∨ a)`. Assuming it false
//! propagates nothing either, so `(x)` is **rejected**.
//!
//! Add the unit `(a)` to the database and try again. Now assuming `(x ∨ a)`
//! false clashes with the unit `(a)`, so the resolvent is RUP and `(x)` is
//! **accepted**. Any model of the database can be changed to make `x` true
//! without breaking a clause, so adding `(x)` keeps a satisfiable database
//! satisfiable.
//!
//! ## Unit propagation, kept simple
//!
//! Every RUP check starts from **nothing assigned**: it assigns the negated
//! lemma, assigns every unit clause in the database, and propagates with two
//! watched literals. Then it unassigns everything. There is no trail kept
//! between checks, so deleting a clause can never leave a stale implication
//! behind. That costs some speed, and buys a kernel with no tricky state.
//!
//! ## A work budget, which can only reject
//!
//! Both inputs may be hostile, and checking is polynomial but not cheap:
//! one RAT step costs a RUP check per partner clause. So every unit of work
//! (a watch visited, a literal scanned, a unit re-assigned) is counted
//! against a budget. When it runs out, every further check answers "no
//! conflict", which can only make lemmas fail, and the run ends with
//! `ResourceLimit`. Running out can turn an acceptance into a rejection,
//! never the other way round, so the budget cannot touch soundness.

use std::collections::HashMap;

use crate::parse::{neg, var, Lit, Step, StepKind};

/// Identifier of a clause in the database.
type ClauseId = usize;

/// The clause database and the scratch assignment used by RUP checks.
#[derive(Debug, Default)]
pub(crate) struct Database {
    /// Clause literals. The first two of a clause with ≥ 2 literals are its
    /// watched literals; propagation reorders them.
    clauses: Vec<Vec<Lit>>,
    /// Whether each clause is still present (not deleted).
    alive: Vec<bool>,
    /// `watches[lit]`: clauses watching `lit`. May hold deleted clauses,
    /// which propagation drops when it meets them.
    watches: Vec<Vec<ClauseId>>,
    /// Clauses with exactly one literal. May hold deleted clauses.
    units: Vec<ClauseId>,
    /// Number of live empty clauses.
    empties: usize,
    /// Sorted, de-duplicated literal set → live clauses with that set.
    /// Used to find the clause a `d` step deletes.
    by_content: HashMap<Vec<Lit>, Vec<ClauseId>>,
    /// Scratch assignment per variable: 0 unassigned, 1 true, -1 false.
    value: Vec<i8>,
    /// Literals made true during the current check, in order.
    trail: Vec<Lit>,
    /// Work done so far, in literal visits.
    work: u64,
    /// The most work allowed.
    max_work: u64,
}

/// The truth value of `lit` under `value`: 1 true, -1 false, 0 unassigned.
fn lit_value(value: &[i8], lit: Lit) -> i8 {
    let v = value[var(lit)];
    if lit & 1 == 1 {
        -v
    } else {
        v
    }
}

/// Sorted and de-duplicated: the identity of a clause as a set.
fn content_key(lits: &[Lit]) -> Vec<Lit> {
    let mut key = lits.to_vec();
    key.sort_unstable();
    key.dedup();
    key
}

impl Database {
    /// An empty database over variables `0..num_vars` that may do at most
    /// `max_work` units of work.
    pub(crate) fn new(num_vars: usize, max_work: u64) -> Database {
        Database {
            watches: vec![Vec::new(); 2 * num_vars],
            value: vec![0; num_vars],
            max_work,
            ..Database::default()
        }
    }

    /// Charge `units` of work. Returns false once the budget is exhausted.
    fn spend(&mut self, units: usize) -> bool {
        self.work = self.work.saturating_add(units as u64);
        self.work <= self.max_work
    }

    /// Whether the work budget has run out.
    pub(crate) fn exhausted(&self) -> bool {
        self.work > self.max_work
    }

    /// Put a clause in the database.
    pub(crate) fn add(&mut self, lits: &[Lit]) {
        let key = content_key(lits);
        let id = self.clauses.len();
        match key.len() {
            0 => self.empties += 1,
            1 => self.units.push(id),
            _ => {
                self.watches[key[0] as usize].push(id);
                self.watches[key[1] as usize].push(id);
            }
        }
        self.by_content.entry(key.clone()).or_default().push(id);
        self.clauses.push(key);
        self.alive.push(true);
    }

    /// Remove one copy of a clause. Returns false if it is not present.
    pub(crate) fn delete(&mut self, lits: &[Lit]) -> bool {
        let key = content_key(lits);
        let Some(id) = self.by_content.get_mut(&key).and_then(|ids| ids.pop()) else {
            return false;
        };
        self.alive[id] = false;
        if key.is_empty() {
            self.empties -= 1;
        }
        true
    }

    /// Make `lit` true for the current check.
    fn assign(&mut self, lit: Lit) {
        self.value[var(lit)] = if lit & 1 == 1 { -1 } else { 1 };
        self.trail.push(lit);
    }

    /// Unit propagation from the current trail. Returns true on conflict,
    /// and false when there is none **or the work budget ran out**.
    ///
    /// For each true literal `p`, visit the clauses watching `¬p`, which
    /// just became false. A clause keeps watching `¬p` only if no other
    /// literal can take over; then it is unit (assign its other watch) or,
    /// if that watch is false too, in conflict.
    fn propagate(&mut self) -> bool {
        let mut head = 0;
        while head < self.trail.len() {
            let false_lit = neg(self.trail[head]);
            head += 1;
            let mut ws = std::mem::take(&mut self.watches[false_lit as usize]);
            let mut keep = 0;
            let mut conflict = false;
            let mut out_of_work = false;
            let mut i = 0;
            while i < ws.len() {
                let id = ws[i];
                if !self.spend(1 + self.clauses[id].len()) {
                    out_of_work = true;
                    break;
                }
                i += 1;
                if !self.alive[id] {
                    continue; // deleted: drop the watch
                }
                let c = &mut self.clauses[id];
                if c[0] == false_lit {
                    c.swap(0, 1);
                }
                let other = c[0];
                if lit_value(&self.value, other) == 1 {
                    ws[keep] = id;
                    keep += 1;
                    continue; // satisfied
                }
                if let Some(k) = (2..c.len()).find(|&k| lit_value(&self.value, c[k]) != -1) {
                    c.swap(1, k);
                    self.watches[c[1] as usize].push(id);
                    continue; // moved to a new watch
                }
                ws[keep] = id;
                keep += 1;
                if lit_value(&self.value, other) == -1 {
                    conflict = true;
                    break;
                }
                self.assign(other);
            }
            // Keep the unvisited watches too, if we stopped early.
            while i < ws.len() {
                ws[keep] = ws[i];
                keep += 1;
                i += 1;
            }
            ws.truncate(keep);
            self.watches[false_lit as usize] = ws;
            if conflict {
                return true;
            }
            if out_of_work {
                return false;
            }
        }
        false
    }

    /// Is `lemma` RUP: does assuming it false lead to a conflict by unit
    /// propagation over the database?
    pub(crate) fn is_rup(&mut self, lemma: &[Lit]) -> bool {
        let mut conflict = self.empties > 0;
        for &l in lemma {
            if conflict {
                break;
            }
            match lit_value(&self.value, l) {
                1 => conflict = true, // lemma holds l and ¬l: a tautology
                -1 => {}              // already assumed false
                _ => self.assign(neg(l)),
            }
        }
        if !conflict && self.spend(self.units.len()) {
            self.units.retain(|&id| self.alive[id]);
            for k in 0..self.units.len() {
                let u = self.clauses[self.units[k]][0];
                match lit_value(&self.value, u) {
                    -1 => {
                        conflict = true;
                        break;
                    }
                    0 => self.assign(u),
                    _ => {}
                }
            }
        }
        if !conflict && !self.exhausted() {
            conflict = self.propagate();
        }
        for &l in &self.trail {
            self.value[var(l)] = 0;
        }
        self.trail.clear();
        conflict
    }

    /// Is `lemma` RAT on its first literal `p`: for every live clause `D`
    /// containing `¬p`, is the resolvent `lemma ∪ (D ∖ {¬p})` RUP?
    pub(crate) fn is_rat(&mut self, lemma: &[Lit]) -> bool {
        let Some(&pivot) = lemma.first() else {
            return false; // the empty clause has no pivot
        };
        let scan: usize = self.clauses.iter().map(|c| 1 + c.len()).sum();
        if !self.spend(scan) {
            return false;
        }
        let partners: Vec<ClauseId> = (0..self.clauses.len())
            .filter(|&id| self.alive[id] && self.clauses[id].contains(&neg(pivot)))
            .collect();
        partners.into_iter().all(|id| {
            let mut resolvent = lemma.to_vec();
            resolvent.extend(self.clauses[id].iter().copied().filter(|&l| l != neg(pivot)));
            self.is_rup(&resolvent)
        })
    }
}

/// The outcome of checking a parsed proof against a parsed formula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The empty clause was derived at step `step`.
    Refuted {
        /// Index of the step that added the empty clause.
        step: usize,
        /// Lemmas accepted (including the empty clause).
        lemmas: usize,
        /// Of those, how many needed RAT.
        rat_lemmas: usize,
        /// Deletions performed.
        deletions: usize,
        /// Work spent.
        work: u64,
    },
    /// The work budget ran out while checking step `step`.
    ResourceLimit {
        /// Index of the step being checked.
        step: usize,
    },
    /// Step `step` adds a lemma that is neither RUP nor RAT.
    NotImplied {
        /// Index of the offending step.
        step: usize,
    },
    /// Step `step` deletes a clause that is not in the database.
    DeleteAbsent {
        /// Index of the offending step.
        step: usize,
    },
    /// Every step checked, but the empty clause was never added.
    NoEmptyClause,
}

/// Replay `steps` against `formula` (VER11 §3.1), within `max_work`.
pub(crate) fn run(num_vars: usize, formula: &[Vec<Lit>], steps: &[Step], max_work: u64) -> Outcome {
    let mut db = Database::new(num_vars, max_work);
    for clause in formula {
        db.add(clause);
    }
    let (mut lemmas, mut rat_lemmas, mut deletions) = (0, 0, 0);
    for (step, s) in steps.iter().enumerate() {
        match s.kind {
            StepKind::Delete => {
                if !db.delete(&s.lits) {
                    return Outcome::DeleteAbsent { step };
                }
                deletions += 1;
            }
            StepKind::Add => {
                if db.is_rup(&s.lits) {
                    lemmas += 1;
                } else if db.is_rat(&s.lits) {
                    lemmas += 1;
                    rat_lemmas += 1;
                } else if db.exhausted() {
                    return Outcome::ResourceLimit { step };
                } else {
                    return Outcome::NotImplied { step };
                }
                if s.lits.is_empty() {
                    return Outcome::Refuted { step, lemmas, rat_lemmas, deletions, work: db.work };
                }
                db.add(&s.lits);
            }
        }
    }
    Outcome::NoEmptyClause
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DIMACS-style helper: `c(&[1, -2])` is the clause (x1 ∨ ¬x2).
    fn c(d: &[i64]) -> Vec<Lit> {
        d.iter()
            .map(|&x| (((x.unsigned_abs() - 1) as u32) << 1) | (x < 0) as u32)
            .collect()
    }

    fn add(d: &[i64]) -> Step {
        Step { kind: StepKind::Add, lits: c(d), line: 0 }
    }

    fn del(d: &[i64]) -> Step {
        Step { kind: StepKind::Delete, lits: c(d), line: 0 }
    }

    #[test]
    fn module_rup_example() {
        let mut db = Database::new(3, u64::MAX);
        for cl in [&[1, 2][..], &[-2, 3], &[-1, 3]] {
            db.add(&c(cl));
        }
        assert!(db.is_rup(&c(&[3])));
        assert!(!db.is_rup(&c(&[-3])));
        assert!(!db.is_rup(&c(&[1])));
        // Checks leave nothing assigned behind.
        assert!(db.value.iter().all(|&v| v == 0));
    }

    #[test]
    fn module_rat_example() {
        let mut db = Database::new(2, u64::MAX);
        db.add(&c(&[-1, 2]));
        assert!(!db.is_rup(&c(&[1])));
        assert!(!db.is_rat(&c(&[1])));
        db.add(&c(&[2]));
        assert!(db.is_rat(&c(&[1])));
        assert!(!db.is_rat(&[]));
    }

    #[test]
    fn tautologies_and_repeated_literals_are_rup() {
        let mut db = Database::new(2, u64::MAX);
        assert!(db.is_rup(&c(&[1, -1])));
        db.add(&c(&[1, 2]));
        assert!(db.is_rup(&c(&[1, 1, 2])));
    }

    #[test]
    fn units_and_empty_clauses_in_the_database() {
        let mut db = Database::new(2, u64::MAX);
        db.add(&c(&[1]));
        db.add(&c(&[-1]));
        assert!(db.is_rup(&[])); // the two units clash
        assert!(db.delete(&c(&[-1])));
        assert!(!db.is_rup(&[]));
        db.add(&[]);
        assert!(db.is_rup(&c(&[2])));
        assert!(db.delete(&[]));
        assert!(!db.delete(&[]));
        assert!(!db.is_rup(&c(&[2])));
    }

    #[test]
    fn deletion_matches_sets_and_removes_one_copy() {
        let mut db = Database::new(3, u64::MAX);
        db.add(&c(&[1, 2, 3]));
        db.add(&c(&[3, 2, 1]));
        assert!(db.delete(&c(&[2, 3, 1, 1])));
        assert!(db.delete(&c(&[1, 2, 3])));
        assert!(!db.delete(&c(&[1, 2, 3])));
        assert!(!db.delete(&c(&[1, 2])));
    }

    #[test]
    fn deleted_clauses_stop_propagating() {
        let mut db = Database::new(2, u64::MAX);
        db.add(&c(&[1, 2]));
        db.add(&c(&[1, -2]));
        assert!(db.is_rup(&c(&[1])));
        assert!(db.delete(&c(&[1, -2])));
        assert!(!db.is_rup(&c(&[1])));
        assert!(!db.is_rup(&c(&[1]))); // the stale watch was dropped
    }

    #[test]
    fn run_refutes_and_counts() {
        // (1 ∨ 2)(1 ∨ ¬2)(¬1 ∨ 2)(¬1 ∨ ¬2): learn (1), then the empty clause.
        let f = vec![c(&[1, 2]), c(&[1, -2]), c(&[-1, 2]), c(&[-1, -2])];
        let proof = vec![add(&[1]), del(&[1, 2]), add(&[]), add(&[5])];
        assert_eq!(
            run(2, &f, &proof, u64::MAX),
            Outcome::Refuted { step: 2, lemmas: 2, rat_lemmas: 0, deletions: 1, work: 13 }
        );
    }

    #[test]
    fn run_rejections() {
        let f = vec![c(&[1, 2]), c(&[-1, 2])];
        // (x1) would pass: it is RAT, because its only resolvent (x1 ∨ x2) is
        // already a clause. (¬x2) is neither RUP nor RAT.
        assert_eq!(run(2, &f, &[add(&[-2])], u64::MAX), Outcome::NotImplied { step: 0 });
        assert_eq!(run(2, &f, &[add(&[2]), add(&[])], u64::MAX), Outcome::NotImplied { step: 1 });
        assert_eq!(run(2, &f, &[del(&[1, -2])], u64::MAX), Outcome::DeleteAbsent { step: 0 });
        assert_eq!(run(2, &f, &[add(&[2])], u64::MAX), Outcome::NoEmptyClause);
    }

    #[test]
    fn running_out_of_work_rejects_and_never_accepts() {
        let f = vec![c(&[1, 2]), c(&[1, -2]), c(&[-1, 2]), c(&[-1, -2])];
        let proof = vec![add(&[1]), add(&[])];
        // The full budget accepts; the exact cost reproduces it.
        let Outcome::Refuted { work, .. } = run(2, &f, &proof, u64::MAX) else { panic!() };
        assert_eq!(run(2, &f, &proof, work), run(2, &f, &proof, u64::MAX));
        // Any less and the same valid proof is rejected, never accepted.
        for budget in 0..work {
            assert!(matches!(run(2, &f, &proof, budget), Outcome::ResourceLimit { .. }), "budget {budget}");
        }
        // A RAT step charges its partner scan.
        let f = vec![c(&[-1, 2]), c(&[2])];
        assert!(matches!(run(2, &f, &[add(&[1])], 1), Outcome::ResourceLimit { step: 0 }));
        assert_eq!(run(2, &f, &[add(&[1])], u64::MAX), Outcome::NoEmptyClause);
    }

    #[test]
    fn run_accepts_rat_extension() {
        // A fresh variable x3 defined as ¬x1 by RAT steps (extended
        // resolution), then a refutation that does not need it.
        let f = vec![c(&[1]), c(&[-1])];
        let proof = vec![add(&[3, 1]), add(&[-3, -1]), add(&[])];
        assert_eq!(
            run(3, &f, &proof, u64::MAX),
            Outcome::Refuted { step: 2, lemmas: 3, rat_lemmas: 0, deletions: 0, work: 6 }
        );
        let f = vec![c(&[1, 2])];
        let proof = vec![add(&[3, 1]), add(&[-3, -1])];
        assert_eq!(run(3, &f, &proof, u64::MAX), Outcome::NoEmptyClause);
        let mut db = Database::new(3, u64::MAX);
        db.add(&c(&[1, 2]));
        assert!(db.is_rat(&c(&[3, 1])));
    }
}

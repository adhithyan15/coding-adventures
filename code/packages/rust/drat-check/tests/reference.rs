//! # The kernel against a naive reference checker
//!
//! The kernel uses two watched literals, which are fast and easy to get
//! subtly wrong. This test re-implements DRAT checking in the most obvious way
//! possible: unit propagation by scanning every clause again and again until
//! nothing changes. The two must give the **same verdict, at the same step**,
//! on thousands of random proofs, valid and invalid, with additions, RAT
//! lemmas, deletions and empty clauses.
//!
//! A second test checks soundness directly: for formulas that brute force
//! shows are satisfiable, no proof may ever be accepted, however it was
//! built.

use drat_check::{check, check_with_limits, Limits, Rejection};

/// xorshift64, deterministic and dependency-free.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn lit(&mut self, vars: u64) -> i64 {
        let v = 1 + self.below(vars) as i64;
        if self.below(2) == 0 {
            v
        } else {
            -v
        }
    }
    fn clause(&mut self, vars: u64, max_len: u64) -> Vec<i64> {
        (0..self.below(max_len + 1)).map(|_| self.lit(vars)).collect()
    }
}

// ---------------------------------------------------------------------------
// The reference checker: DIMACS integers, full-scan propagation.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    Accepted,
    NotImplied(usize),
    DeleteAbsent(usize),
    NoEmptyClause,
}

#[derive(Clone)]
enum RefStep {
    Add(Vec<i64>),
    Delete(Vec<i64>),
}

/// Unit propagation by repeated full scans. `assumed` are literals taken as
/// true. Returns true on conflict.
fn naive_conflict(db: &[Vec<i64>], assumed: &[i64]) -> bool {
    let mut truth: Vec<i64> = Vec::new();
    for &l in assumed {
        if truth.contains(&-l) {
            return true;
        }
        if !truth.contains(&l) {
            truth.push(l);
        }
    }
    loop {
        let mut changed = false;
        for clause in db {
            if clause.iter().any(|l| truth.contains(l)) {
                continue;
            }
            let open: Vec<i64> = clause.iter().copied().filter(|l| !truth.contains(&-l)).collect();
            match open.first() {
                None => return true,
                Some(&u) if open.iter().all(|&x| x == u) => {
                    truth.push(u);
                    changed = true;
                }
                _ => {}
            }
        }
        if !changed {
            return false;
        }
    }
}

fn naive_rup(db: &[Vec<i64>], lemma: &[i64]) -> bool {
    let negated: Vec<i64> = lemma.iter().map(|l| -l).collect();
    naive_conflict(db, &negated)
}

fn naive_rat(db: &[Vec<i64>], lemma: &[i64]) -> bool {
    let Some(&p) = lemma.first() else { return false };
    db.iter().filter(|d| d.contains(&-p)).all(|d| {
        let mut r = lemma.to_vec();
        r.extend(d.iter().copied().filter(|&l| l != -p));
        naive_rup(db, &r)
    })
}

fn set(c: &[i64]) -> Vec<i64> {
    let mut s = c.to_vec();
    s.sort_unstable();
    s.dedup();
    s
}

fn naive_check(formula: &[Vec<i64>], proof: &[RefStep]) -> Verdict {
    let mut db: Vec<Vec<i64>> = formula.to_vec();
    for (i, step) in proof.iter().enumerate() {
        match step {
            RefStep::Delete(c) => {
                let Some(pos) = db.iter().position(|d| set(d) == set(c)) else {
                    return Verdict::DeleteAbsent(i);
                };
                db.remove(pos);
            }
            RefStep::Add(c) => {
                if !(naive_rup(&db, c) || naive_rat(&db, c)) {
                    return Verdict::NotImplied(i);
                }
                if c.is_empty() {
                    return Verdict::Accepted;
                }
                db.push(c.clone());
            }
        }
    }
    Verdict::NoEmptyClause
}

// ---------------------------------------------------------------------------
// Rendering and the kernel's verdict in the same shape.
// ---------------------------------------------------------------------------

fn formula_text(vars: u64, clauses: &[Vec<i64>]) -> String {
    let mut s = format!("p cnf {vars} {}\n", clauses.len());
    for c in clauses {
        for l in c {
            s.push_str(&format!("{l} "));
        }
        s.push_str("0\n");
    }
    s
}

fn proof_text(proof: &[RefStep]) -> String {
    let mut s = String::new();
    for step in proof {
        let c = match step {
            RefStep::Add(c) => c,
            RefStep::Delete(c) => {
                s.push_str("d ");
                c
            }
        };
        for l in c {
            s.push_str(&format!("{l} "));
        }
        s.push_str("0\n");
    }
    s
}

fn kernel_check(vars: u64, formula: &[Vec<i64>], proof: &[RefStep]) -> Verdict {
    match check(&formula_text(vars, formula), &proof_text(proof)) {
        Ok(_) => Verdict::Accepted,
        Err(Rejection::NotImplied { step, .. }) => Verdict::NotImplied(step),
        Err(Rejection::DeleteAbsent { step, .. }) => Verdict::DeleteAbsent(step),
        Err(Rejection::NoEmptyClause) => Verdict::NoEmptyClause,
        Err(Rejection::Parse(e)) => panic!("generated input failed to parse: {e}"),
        Err(Rejection::ResourceLimit { .. }) => panic!("a tiny proof exhausted the default work budget"),
    }
}

/// A random proof that mostly makes progress: candidate lemmas the reference
/// accepts are kept nine times in ten, and invalid steps (rejected lemmas,
/// deletions of absent clauses) are mixed in rarely, so proofs run long
/// before failing, if they fail.
fn random_proof(rng: &mut Rng, vars: u64, formula: &[Vec<i64>]) -> Vec<RefStep> {
    let mut db = formula.to_vec();
    let mut proof = Vec::new();
    for _ in 0..30 {
        match rng.below(10) {
            0 | 1 if !db.is_empty() => {
                let i = rng.below(db.len() as u64) as usize;
                let mut c = db.remove(i);
                c.reverse(); // deletion must match by set, not by order
                proof.push(RefStep::Delete(c));
            }
            2 if rng.below(8) == 0 => proof.push(RefStep::Delete(rng.clause(vars, 3))),
            3 if rng.below(16) == 0 => proof.push(RefStep::Add(Vec::new())),
            _ => {
                // Lemmas may mention one variable past the formula, so RAT
                // extension steps happen.
                let c = rng.clause(vars + 1, 3);
                let ok = naive_rup(&db, &c) || naive_rat(&db, &c);
                if ok || rng.below(20) == 0 {
                    proof.push(RefStep::Add(c.clone()));
                    db.push(c);
                }
            }
        }
    }
    if rng.below(2) == 0 {
        proof.push(RefStep::Add(Vec::new()));
    }
    proof
}

#[test]
fn kernel_agrees_with_reference_on_random_proofs() {
    let mut rng = Rng(0xD8A7_C4EC_0000_0001);
    let mut tally = [0usize; 4];
    for round in 0..4000 {
        let vars = 2 + rng.below(6);
        let n = rng.below(4 * vars);
        let formula: Vec<Vec<i64>> = (0..n).map(|_| rng.clause(vars, 3)).collect();
        let proof = random_proof(&mut rng, vars, &formula);
        let expected = naive_check(&formula, &proof);
        let got = kernel_check(vars, &formula, &proof);
        assert_eq!(
            got,
            expected,
            "round {round}\nformula:\n{}proof:\n{}",
            formula_text(vars, &formula),
            proof_text(&proof)
        );
        tally[match expected {
            Verdict::Accepted => 0,
            Verdict::NotImplied(_) => 1,
            Verdict::DeleteAbsent(_) => 2,
            Verdict::NoEmptyClause => 3,
        }] += 1;
    }
    // Every verdict must actually have been exercised.
    assert!(tally.iter().all(|&t| t > 100), "{tally:?}");
}

fn brute_force_sat(vars: u64, formula: &[Vec<i64>]) -> bool {
    (0u64..1 << vars).any(|bits| {
        formula.iter().all(|c| {
            c.iter().any(|&l| {
                let value = bits >> (l.unsigned_abs() - 1) & 1 == 1;
                value == (l > 0)
            })
        })
    })
}

#[test]
fn no_proof_is_accepted_for_a_satisfiable_formula() {
    let mut rng = Rng(0x50DA_0D00_5EED_0002);
    let mut tried = 0;
    while tried < 2000 {
        let vars = 2 + rng.below(8);
        let n = rng.below(3 * vars);
        let formula: Vec<Vec<i64>> = (0..n).map(|_| rng.clause(vars, 3)).collect();
        if !brute_force_sat(vars, &formula) {
            continue;
        }
        tried += 1;
        let mut proof = random_proof(&mut rng, vars, &formula);
        proof.push(RefStep::Add(Vec::new()));
        assert_ne!(
            kernel_check(vars, &formula, &proof),
            Verdict::Accepted,
            "accepted a proof for a satisfiable formula\n{}{}",
            formula_text(vars, &formula),
            proof_text(&proof)
        );
    }
}

/// The security review's slow-proof construction: a RAT lemma whose pivot
/// has thousands of partner clauses, added and deleted over and over. With
/// no budget this ran for minutes; with one it is cut off and rejected.
#[test]
fn a_proof_built_to_be_slow_hits_the_work_budget() {
    let n = 2000;
    let mut formula = String::new();
    let mut clauses = 0;
    let mut body = String::new();
    for i in 0..n {
        body.push_str(&format!("{} 0\n", 3 + i)); // filler units
        clauses += 1;
    }
    body.push_str("2 0\n"); // the unit (u)
    clauses += 1;
    for i in 0..n {
        body.push_str(&format!("-1 2 {} 0\n", 3 + n + i)); // (¬p ∨ u ∨ y_i)
        clauses += 1;
    }
    formula.push_str(&format!("p cnf {} {clauses}\n", 2 + 2 * n));
    formula.push_str(&body);
    let proof = "1 0\nd 1 0\n".repeat(1000);
    let limits = Limits { max_work: 50_000_000, ..Limits::default() };
    match check_with_limits(&formula, &proof, limits) {
        Err(Rejection::ResourceLimit { step, .. }) => assert!(step < 2000),
        other => panic!("expected the work budget to stop it, got {other:?}"),
    }
}

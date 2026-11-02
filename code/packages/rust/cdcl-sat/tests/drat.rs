//! # Every UNSAT answer is certified by the trusted kernel (VER11 §5)
//!
//! The solver is not trusted; `drat-check` is. This file closes the loop:
//!
//! 1. **Positive corpus.** Every UNSAT formula from the VER09 differential
//!    corpus (random formulas, pigeonhole) is solved with proof logging on,
//!    and the kernel must accept the proof.
//! 2. **Negative corpus.** Each real proof is then damaged in ways that make
//!    it invalid, and the kernel must reject every damaged version:
//!    - truncated: the empty clause dropped, or the text cut mid-step;
//!    - reordered: the empty clause moved first, or a lemma moved ahead of
//!      the lemmas it depends on;
//!    - tampered: a literal flipped (counted only when an independent naive
//!      checker confirms the result is invalid), or a deletion of a clause
//!      that is not there;
//!    - replayed against a **satisfiable** weakening of its formula, where
//!      no proof may ever be accepted.
//!
//! "Invalid" is never decided by the kernel itself. It is decided by
//! construction (truncation, satisfiable formulas) or by the naive checker
//! below, which shares no code with the kernel.

mod common;

use cdcl_sat::{write_dimacs, Cnf, Lit, ProofStep, SolveResult, Solver};
use common::{brute_force_sat, pigeonhole, random_cnf, Rng};
use drat_check::{check, ParseError, Rejection};

// ---------------------------------------------------------------------------
// A naive, independent DRAT checker over DIMACS integers.
// ---------------------------------------------------------------------------

/// Unit propagation by repeated full scans, from the literals in `assumed`.
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
        for c in db {
            if c.iter().any(|l| truth.contains(l)) {
                continue;
            }
            let open: Vec<i64> = c.iter().copied().filter(|l| !truth.contains(&-l)).collect();
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

fn naive_implied(db: &[Vec<i64>], lemma: &[i64]) -> bool {
    let rup = |c: &[i64]| naive_conflict(db, &c.iter().map(|l| -l).collect::<Vec<_>>());
    if rup(lemma) {
        return true;
    }
    let Some(&p) = lemma.first() else { return false };
    db.iter().filter(|d| d.contains(&-p)).all(|d| {
        let mut r = lemma.to_vec();
        r.extend(d.iter().copied().filter(|&l| l != -p));
        rup(&r)
    })
}

fn same_set(a: &[i64], b: &[i64]) -> bool {
    let norm = |c: &[i64]| {
        let mut v = c.to_vec();
        v.sort_unstable();
        v.dedup();
        v
    };
    norm(a) == norm(b)
}

/// Does the naive checker accept `proof` for `formula`?
fn naive_accepts(formula: &[Vec<i64>], proof: &[(bool, Vec<i64>)]) -> bool {
    let mut db = formula.to_vec();
    for (delete, c) in proof {
        if *delete {
            match db.iter().position(|d| same_set(d, c)) {
                Some(i) => {
                    db.remove(i);
                }
                None => return false,
            }
        } else {
            if !naive_implied(&db, c) {
                return false;
            }
            if c.is_empty() {
                return true;
            }
            db.push(c.clone());
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

fn ints(c: &[Lit]) -> Vec<i64> {
    c.iter().map(|l| l.to_dimacs()).collect()
}

fn steps(proof: &cdcl_sat::DratProof) -> Vec<(bool, Vec<i64>)> {
    proof
        .steps
        .iter()
        .map(|s| match s {
            ProofStep::Add(c) => (false, ints(c)),
            ProofStep::Delete(c) => (true, ints(c)),
        })
        .collect()
}

fn text(steps: &[(bool, Vec<i64>)]) -> String {
    let mut out = String::new();
    for (delete, c) in steps {
        if *delete {
            out.push_str("d ");
        }
        for l in c {
            out.push_str(&format!("{l} "));
        }
        out.push_str("0\n");
    }
    out
}

/// Solve with logging on; return the proof steps if UNSAT.
fn unsat_proof(cnf: &Cnf) -> Option<Vec<(bool, Vec<i64>)>> {
    let mut s = Solver::from_cnf_with_proof(cnf);
    match s.solve() {
        SolveResult::Unsat => {
            let p = s.proof().expect("logging was on");
            assert!(p.derives_empty_clause(), "an UNSAT proof must end in the empty clause");
            Some(steps(p))
        }
        _ => None,
    }
}

/// The UNSAT part of the VER09 differential corpus, plus pigeonhole.
fn unsat_corpus() -> Vec<Cnf> {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut out = Vec::new();
    while out.len() < 600 {
        let vars = 3 + rng.below(10);
        let clauses = (vars as f64 * (4.0 + 3.0 * (rng.below(100) as f64 / 100.0))) as usize;
        let cnf = random_cnf(&mut rng, vars, clauses);
        if !brute_force_sat(&cnf, &[]) {
            out.push(cnf);
        }
    }
    out.extend((1..=6).map(pigeonhole));
    out
}

/// Harder formulas: pure 3-CNF with 30–50 variables at five clauses per
/// variable, past the phase transition, so almost all are UNSAT and their
/// proofs are long enough to damage in many places. Too big for brute
/// force; the solver's UNSAT answers here are trusted only because the
/// kernel certifies them in the test that uses them.
fn hard_unsat_corpus() -> Vec<Cnf> {
    let mut rng = Rng(0x0005_EED5_0000_0003);
    let mut out = Vec::new();
    while out.len() < 60 {
        let vars = 30 + rng.below(21);
        let mut cnf = Cnf::new(vars);
        for _ in 0..5 * vars {
            let c: Vec<Lit> = (0..3).map(|_| rng.lit(vars)).collect();
            cnf.add_clause(&c);
        }
        if Solver::from_cnf(&cnf).solve() == SolveResult::Unsat {
            out.push(cnf);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 1. Positive corpus
// ---------------------------------------------------------------------------

#[test]
fn every_unsat_answer_is_certified() {
    let mut lemmas = 0;
    for (i, cnf) in unsat_corpus().iter().enumerate() {
        let proof = unsat_proof(cnf).unwrap_or_else(|| panic!("formula {i} is UNSAT by brute force"));
        let accepted = check(&write_dimacs(cnf), &text(&proof))
            .unwrap_or_else(|e| panic!("formula {i}: kernel rejected the solver's proof: {e}"));
        lemmas += accepted.lemmas;
    }
    assert!(lemmas > 2000, "the corpus should make the solver learn: {lemmas}");
}

#[test]
fn the_largest_pigeonhole_proof_is_certified() {
    let cnf = pigeonhole(7);
    let proof = unsat_proof(&cnf).expect("PHP(8,7) is UNSAT");
    check(&write_dimacs(&cnf), &text(&proof)).expect("certified");
}

// ---------------------------------------------------------------------------
// 2. Negative corpus
// ---------------------------------------------------------------------------

#[test]
fn damaged_proofs_are_all_rejected() {
    let mut rng = Rng(0x0BAD_900F);
    let mut rejected = [0usize; 6];
    let corpus: Vec<Cnf> = unsat_corpus().into_iter().step_by(3).chain(hard_unsat_corpus()).collect();
    for (i, cnf) in corpus.iter().enumerate() {
        let formula_text = write_dimacs(cnf);
        let formula: Vec<Vec<i64>> = cnf.clauses.iter().map(|c| ints(c)).collect();
        let proof = unsat_proof(cnf).expect("UNSAT");
        check(&formula_text, &text(&proof)).unwrap_or_else(|e| panic!("formula {i}: valid proof rejected: {e}"));
        let last = proof.len() - 1;
        assert_eq!(proof[last], (false, vec![]), "the empty clause comes last");
        let refutable_by_propagation = naive_conflict(&formula, &[]);

        // Truncated: drop the empty clause.
        let r = check(&formula_text, &text(&proof[..last]));
        assert_eq!(r, Err(Rejection::NoEmptyClause), "formula {i}");
        rejected[0] += 1;

        // Truncated: cut in the middle of a step.
        if let Some((_, c)) = proof[..last].iter().rev().find(|(_, c)| !c.is_empty()) {
            let mut t = text(&proof[..last]);
            for l in c {
                t.push_str(&format!("{l} "));
            }
            assert!(
                matches!(check(&formula_text, &t), Err(Rejection::Parse(ParseError::Truncated { .. }))),
                "formula {i}"
            );
            rejected[1] += 1;
        }

        // Reordered: the empty clause first.
        if !refutable_by_propagation {
            let mut moved = vec![(false, vec![])];
            moved.extend_from_slice(&proof[..last]);
            assert_eq!(check(&formula_text, &text(&moved)), Err(Rejection::NotImplied { step: 0, line: 1 }));
            rejected[2] += 1;
        }

        // Reordered: a lemma that does not follow from the formula alone,
        // moved ahead of the lemmas it was derived from.
        if let Some(k) = (0..last).find(|&k| !proof[k].0 && !naive_implied(&formula, &proof[k].1)) {
            let mut moved = vec![proof[k].clone()];
            moved.extend(proof.iter().enumerate().filter(|&(j, _)| j != k).map(|(_, s)| s.clone()));
            assert_eq!(
                check(&formula_text, &text(&moved)),
                Err(Rejection::NotImplied { step: 0, line: 1 }),
                "formula {i}"
            );
            rejected[3] += 1;
        }

        // Tampered: flip one literal of one lemma; counted when the naive
        // checker confirms the damage.
        for _ in 0..6 {
            let adds: Vec<usize> = (0..last).filter(|&k| !proof[k].0 && !proof[k].1.is_empty()).collect();
            if adds.is_empty() {
                break;
            }
            let k = adds[rng.below(adds.len())];
            let mut bad = proof.clone();
            let pos = rng.below(bad[k].1.len());
            bad[k].1[pos] = -bad[k].1[pos];
            if !naive_accepts(&formula, &bad) {
                assert!(check(&formula_text, &text(&bad)).is_err(), "formula {i}: tampered proof accepted");
                rejected[4] += 1;
            }
        }

        // Tampered: delete a clause that is not there.
        let mut absent = formula[0].clone();
        absent.push(cnf.num_vars as i64 + 1);
        let mut bad = vec![(true, absent)];
        bad.extend_from_slice(&proof);
        assert_eq!(check(&formula_text, &text(&bad)), Err(Rejection::DeleteAbsent { step: 0, line: 1 }));
        rejected[5] += 1;
    }
    eprintln!("damaged proofs rejected, by kind: {rejected:?}");
    // Every kind of damage was actually exercised, many times.
    assert!(rejected.iter().all(|&n| n > 50), "{rejected:?}");
}

#[test]
fn proofs_are_rejected_against_satisfiable_weakenings() {
    let mut checked = 0;
    for cnf in unsat_corpus().iter().filter(|c| c.num_vars <= 12) {
        let proof = unsat_proof(cnf).expect("UNSAT");
        // Drop clauses from the end until the formula becomes satisfiable.
        let mut weak = cnf.clone();
        while !brute_force_sat(&weak, &[]) {
            weak.clauses.pop();
        }
        assert!(
            check(&write_dimacs(&weak), &text(&proof)).is_err(),
            "a proof was accepted for a satisfiable formula"
        );
        checked += 1;
    }
    assert!(checked > 500);
}

#[test]
fn unsat_under_assumptions_is_not_a_refutation() {
    // (x1 → x2) with x1 and ¬x2 assumed is UNSAT, but the formula is not:
    // the proof has no empty clause and the kernel says so.
    let mut s = Solver::new();
    s.enable_proof();
    let x1 = Lit::from_dimacs(1).unwrap();
    let x2 = Lit::from_dimacs(2).unwrap();
    s.add_clause(&[!x1, x2]);
    assert_eq!(s.solve_with_assumptions(&[x1, !x2]), SolveResult::Unsat);
    let p = s.proof().unwrap();
    assert!(!p.derives_empty_clause());
    assert_eq!(check("p cnf 2 1\n-1 2 0\n", &p.to_drat_text()), Err(Rejection::NoEmptyClause));
}

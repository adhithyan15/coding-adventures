//! # Differential tests: the solver against exhaustive enumeration
//!
//! The solver is not trusted (VER00 §7.5), but a wrong solver wastes proofs,
//! so it is checked against the dumbest possible oracle: try all `2^n`
//! assignments. With `n ≤ 12` that is at most 4096 assignments per formula,
//! cheap enough to do for thousands of formulas.
//!
//! Formulas are drawn around the 3-SAT phase transition (about 4.26 clauses
//! per variable), where random instances are hardest and SAT and UNSAT are
//! roughly equally likely, so both answers get exercised. Every SAT answer's
//! model is re-checked against the formula with code that shares nothing
//! with the solver (`Cnf::is_satisfied_by`).

use cdcl_sat::{parse_dimacs, write_dimacs, Cnf, Lit, SolveResult, Solver, Var};

mod common;

use common::{brute_force_sat, pigeonhole, random_cnf, Rng};

#[test]
fn random_formulas_agree_with_brute_force() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let (mut sat, mut unsat) = (0, 0);
    for round in 0..3000 {
        let vars = 1 + rng.below(12);
        let clauses = (vars as f64 * (2.0 + 4.0 * (rng.below(100) as f64 / 100.0))) as usize;
        let cnf = random_cnf(&mut rng, vars, clauses);
        let mut s = Solver::from_cnf(&cnf);
        let expected = brute_force_sat(&cnf, &[]);
        match s.solve() {
            SolveResult::Sat => {
                assert!(expected, "round {round}: solver SAT, brute force UNSAT\n{}", write_dimacs(&cnf));
                assert!(cnf.is_satisfied_by(s.model()), "round {round}: bad model\n{}", write_dimacs(&cnf));
                sat += 1;
            }
            SolveResult::Unsat => {
                assert!(!expected, "round {round}: solver UNSAT, brute force SAT\n{}", write_dimacs(&cnf));
                unsat += 1;
            }
            SolveResult::Unknown => panic!("no budget was set"),
        }
    }
    // Both answers must actually have been exercised.
    assert!(sat > 300 && unsat > 300, "sat={sat} unsat={unsat}");
}

#[test]
fn assumptions_agree_with_brute_force_and_cores_are_sound() {
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    let mut cores = 0;
    for round in 0..1500 {
        let vars = 2 + rng.below(11);
        let cnf = random_cnf(&mut rng, vars, vars * 3);
        let assumptions: Vec<Lit> = (0..1 + rng.below(4)).map(|_| rng.lit(vars)).collect();
        let mut s = Solver::from_cnf(&cnf);
        let expected = brute_force_sat(&cnf, &assumptions);
        match s.solve_with_assumptions(&assumptions) {
            SolveResult::Sat => {
                assert!(expected, "round {round}");
                let m = s.model();
                assert!(cnf.is_satisfied_by(m));
                assert!(assumptions.iter().all(|l| m[l.var().index()] != l.is_negated()));
            }
            SolveResult::Unsat => {
                assert!(!expected, "round {round}");
                let core = s.failed_assumptions().to_vec();
                // The core is drawn from the assumptions…
                assert!(core.iter().all(|l| assumptions.contains(l)), "round {round}: {core:?}");
                // …and is itself enough for UNSAT.
                assert!(!brute_force_sat(&cnf, &core), "round {round}: core {core:?} is not UNSAT");
                if !core.is_empty() {
                    cores += 1;
                }
            }
            SolveResult::Unknown => panic!("no budget was set"),
        }
        // The same solver answers the unassumed question correctly afterwards.
        let plain = s.solve();
        assert_eq!(plain == SolveResult::Sat, brute_force_sat(&cnf, &[]), "round {round} (no assumptions)");
    }
    assert!(cores > 100, "only {cores} non-trivial cores");
}

#[test]
fn incremental_clause_batches_agree_with_brute_force() {
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    for round in 0..300 {
        let vars = 3 + rng.below(10);
        let mut so_far = Cnf::new(vars);
        let mut s = Solver::new();
        s.reserve_vars(vars);
        for batch in 0..8 {
            let count = 1 + rng.below(vars);
            let more = random_cnf(&mut rng, vars, count);
            for c in &more.clauses {
                so_far.add_clause(c);
                s.add_clause(c);
            }
            let expected = brute_force_sat(&so_far, &[]);
            let got = s.solve();
            assert_eq!(got == SolveResult::Sat, expected, "round {round} batch {batch}");
            if got == SolveResult::Sat {
                assert!(so_far.is_satisfied_by(s.model()));
            }
            // Probe with a random assumption between batches, as BMC does.
            let a = rng.lit(vars);
            let expected_a = brute_force_sat(&so_far, &[a]);
            assert_eq!(s.solve_with_assumptions(&[a]) == SolveResult::Sat, expected_a);
        }
    }
}

#[test]
fn pigeonhole_instances_are_unsat() {
    for holes in 1..=7 {
        let cnf = pigeonhole(holes);
        let mut s = Solver::from_cnf(&cnf);
        assert_eq!(s.solve(), SolveResult::Unsat, "PHP({}, {holes})", holes + 1);
    }
}

#[test]
fn pigeonhole_with_one_hole_fewer_pigeon_is_sat() {
    // n pigeons in n holes is satisfiable; check the model is a real
    // placement.
    for holes in 1..=7 {
        let pigeons = holes;
        let x = |p: usize, h: usize| Var::from_index(p * holes + h);
        let mut cnf = Cnf::new(pigeons * holes);
        for p in 0..pigeons {
            let c: Vec<Lit> = (0..holes).map(|h| x(p, h).positive()).collect();
            cnf.add_clause(&c);
        }
        for h in 0..holes {
            for p in 0..pigeons {
                for q in p + 1..pigeons {
                    cnf.add_clause(&[x(p, h).negative(), x(q, h).negative()]);
                }
            }
        }
        let mut s = Solver::from_cnf(&cnf);
        assert_eq!(s.solve(), SolveResult::Sat);
        assert!(cnf.is_satisfied_by(s.model()));
    }
}

#[test]
fn dimacs_round_trip_preserves_the_answer() {
    let mut rng = Rng(42);
    for _ in 0..200 {
        let vars = 1 + rng.below(10);
        let cnf = random_cnf(&mut rng, vars, vars * 4);
        let text = write_dimacs(&cnf);
        let back = parse_dimacs(&text).expect("writer output must parse");
        assert_eq!(back, cnf);
        let a = Solver::from_cnf(&cnf).solve();
        let b = Solver::from_cnf(&back).solve();
        assert_eq!(a, b);
    }
}

#[test]
fn larger_random_instances_are_consistent() {
    // Too big for brute force: check SAT models, and check UNSAT answers are
    // stable under clause reordering (a different search, same formula).
    let mut rng = Rng(7);
    for _ in 0..20 {
        let vars = 60;
        let cnf = random_cnf(&mut rng, vars, 255);
        let mut s = Solver::from_cnf(&cnf);
        let r = s.solve();
        if r == SolveResult::Sat {
            assert!(cnf.is_satisfied_by(s.model()));
        }
        let mut reversed = cnf.clone();
        reversed.clauses.reverse();
        assert_eq!(Solver::from_cnf(&reversed).solve(), r);
    }
}

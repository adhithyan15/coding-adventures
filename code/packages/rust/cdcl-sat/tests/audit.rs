//! # Audit suite: the VER09 §6.1 contracts, measured
//!
//! `cdcl-sat` is untrusted search; `drat-check` is the trusted kernel. The
//! labels "proved" downstream rest on four promises, and this file tests each
//! one against an oracle that shares no code with the solver:
//!
//! | Contract | Oracle |
//! |---|---|
//! | C1 Sat is sound | re-evaluate the model against every clause and assumption |
//! | C2 Unsat has a checkable proof | `drat_check::check` on the DIMACS text |
//! | C3 cores are genuine and certifiable | brute force + `drat_check` on core units |
//! | C4 Unknown is honest | compare with brute force once the budget is lifted |
//! | C5 small instances are complete | exhaustive enumeration of whole formula domains |
//! | C6 determinism | run twice, compare answers, stats and proof text |
//! | C9 DIMACS robustness | every malformed shape returns `Err` |
//!
//! "Never trust the solver's own claim about itself": nothing here asks the
//! solver whether its answer is right.

mod common;

use cdcl_sat::{
    parse_dimacs, parse_dimacs_with_limit, write_dimacs, Cnf, DimacsErrorKind, DratProof, Lit, ProofStep,
    SolveResult, Solver, Var,
};
use common::{pigeonhole, random_cnf, Rng};

// ---------------------------------------------------------------------------
// Oracles
// ---------------------------------------------------------------------------

/// Brute force over `n` variables using bit masks: clause `c` is satisfied by
/// the assignment `bits` iff `bits & pos != 0 || !bits & neg != 0`.
/// `fixed` literals must also hold. Variables beyond the formula's count are
/// free (they are only constrained by `fixed`).
fn brute(cnf: &Cnf, fixed: &[Lit]) -> bool {
    let n = fixed.iter().map(|l| l.var().index() + 1).chain([cnf.num_vars]).max().unwrap();
    assert!(n <= 24, "brute force is for small n");
    let masks: Vec<(u32, u32)> = cnf
        .clauses
        .iter()
        .map(|c| {
            let (mut pos, mut neg) = (0u32, 0u32);
            for l in c {
                if l.is_negated() {
                    neg |= 1 << l.var().index();
                } else {
                    pos |= 1 << l.var().index();
                }
            }
            (pos, neg)
        })
        .collect();
    let (mut fpos, mut fneg) = (0u32, 0u32);
    for l in fixed {
        if l.is_negated() {
            fneg |= 1 << l.var().index();
        } else {
            fpos |= 1 << l.var().index();
        }
    }
    (0u32..(1u32 << n)).any(|bits| {
        bits & fneg == 0 && !bits & fpos == 0 && masks.iter().all(|&(p, q)| bits & p != 0 || !bits & q != 0)
    })
}

fn text_of(proof: &DratProof) -> String {
    proof.to_drat_text()
}

/// The proof without any deletion step.
fn without_deletions(proof: &DratProof) -> String {
    let p = DratProof {
        steps: proof.steps.iter().filter(|s| matches!(s, ProofStep::Add(_))).cloned().collect(),
    };
    p.to_drat_text()
}

/// Ask the kernel; panic with the formula if it refuses.
fn certify(cnf: &Cnf, proof: &DratProof, label: &str) {
    let formula = write_dimacs(cnf);
    if let Err(e) = drat_check::check(&formula, &text_of(proof)) {
        panic!("{label}: drat-check rejected the solver's proof: {e}\nformula:\n{formula}\nproof:\n{}", text_of(proof));
    }
    // Deletions are an optimisation, never load-bearing for soundness.
    if let Err(e) = drat_check::check(&formula, &without_deletions(proof)) {
        panic!("{label}: proof without deletions rejected: {e}\nformula:\n{formula}");
    }
}

/// One fresh solve of `cnf` with proof logging, fully verified.
fn verify_formula(cnf: &Cnf, label: &str) -> SolveResult {
    let mut s = Solver::from_cnf_with_proof(cnf);
    let expected = brute(cnf, &[]);
    let r = s.solve();
    match r {
        SolveResult::Sat => {
            assert!(expected, "{label}: solver SAT but formula is UNSAT\n{}", write_dimacs(cnf));
            assert_eq!(s.model().len(), s.num_vars(), "{label}: model length");
            assert!(cnf.is_satisfied_by(s.model()), "{label}: wrong model\n{}", write_dimacs(cnf));
            assert!(s.failed_assumptions().is_empty());
        }
        SolveResult::Unsat => {
            assert!(!expected, "{label}: solver UNSAT but formula is SAT\n{}", write_dimacs(cnf));
            assert!(s.model().is_empty(), "{label}: model after Unsat");
            assert!(s.failed_assumptions().is_empty(), "{label}: core without assumptions");
            let p = s.proof().expect("logging on");
            assert!(p.derives_empty_clause(), "{label}: proof lacks the empty clause");
            certify(cnf, p, label);
        }
        SolveResult::Unknown => panic!("{label}: Unknown without a budget"),
    }
    r
}

// ---------------------------------------------------------------------------
// 1. Exhaustive formula domains (C5, C1, C2)
// ---------------------------------------------------------------------------

/// Every literal sequence of length <= `max_width` over `vars` variables,
/// raw: duplicates, tautologies and the empty clause included when
/// `raw` is set; otherwise only sorted, non-tautological, non-empty sets.
fn clause_universe(vars: usize, max_width: usize, raw: bool) -> Vec<Vec<Lit>> {
    let lits: Vec<Lit> = (0..vars).flat_map(|v| [Var::from_index(v).positive(), Var::from_index(v).negative()]).collect();
    let mut out: Vec<Vec<Lit>> = Vec::new();
    if raw {
        out.push(vec![]);
        if max_width >= 1 {
            for &a in &lits {
                out.push(vec![a]);
            }
        }
        if max_width >= 2 {
            for &a in &lits {
                for &b in &lits {
                    // Both orders of distinct literals are the same set; keep
                    // a <= b, but keep the duplicate (a, a) and tautology.
                    if a.code() <= b.code() {
                        out.push(vec![a, b]);
                    }
                }
            }
        }
        assert!(max_width <= 2, "raw universe only for width <= 2");
        return out;
    }
    // Sorted non-tautological sets by subset enumeration over literal codes.
    for mask in 1u32..(1 << lits.len()) {
        if mask.count_ones() as usize > max_width {
            continue;
        }
        let c: Vec<Lit> = (0..lits.len()).filter(|i| mask >> i & 1 == 1).map(|i| lits[i]).collect();
        if c.windows(2).any(|w| w[0].var() == w[1].var()) {
            continue; // tautology
        }
        out.push(c);
    }
    out
}

/// Visit every multiset of at most `max_clauses` clauses from `universe`.
fn for_each_multiset<F: FnMut(&[&Vec<Lit>])>(universe: &[Vec<Lit>], max_clauses: usize, f: &mut F) {
    fn rec<'a, F: FnMut(&[&'a Vec<Lit>])>(
        universe: &'a [Vec<Lit>],
        start: usize,
        left: usize,
        cur: &mut Vec<&'a Vec<Lit>>,
        f: &mut F,
    ) {
        f(cur);
        if left == 0 {
            return;
        }
        for i in start..universe.len() {
            cur.push(&universe[i]);
            rec(universe, i, left - 1, cur, f);
            cur.pop();
        }
    }
    rec(universe, 0, max_clauses, &mut Vec::new(), f);
}

fn cnf_from(vars: usize, clauses: &[&Vec<Lit>]) -> Cnf {
    let mut cnf = Cnf::new(vars);
    for c in clauses {
        cnf.add_clause(c);
    }
    cnf
}

fn run_domain(vars: usize, width: usize, raw: bool, max_clauses: usize, name: &str) -> (usize, usize, usize) {
    let universe = clause_universe(vars, width, raw);
    let (mut total, mut sat, mut unsat) = (0, 0, 0);
    let t = std::time::Instant::now();
    for_each_multiset(&universe, max_clauses, &mut |cs| {
        let cnf = cnf_from(vars, cs);
        total += 1;
        match verify_formula(&cnf, name) {
            SolveResult::Sat => sat += 1,
            _ => unsat += 1,
        }
    });
    eprintln!(
        "[audit] domain {name}: universe {} clauses, {total} formulas ({sat} SAT, {unsat} UNSAT) in {:?}",
        universe.len(),
        t.elapsed()
    );
    (total, sat, unsat)
}

#[test]
fn exhaustive_3_vars_3_clauses_width_2_raw() {
    // Raw: includes the empty clause, repeated literals (x x) and tautologies
    // (x ¬x). 3 vars: 1 + 6 + 21 = 28 clauses; multisets of <= 3 = C(31,3)=4495.. computed below.
    let (total, sat, unsat) = run_domain(3, 2, true, 3, "3v/3c/w2 raw");
    assert_eq!(total, 1 + 28 + 28 * 29 / 2 + 28 * 29 * 30 / 6);
    assert!(sat > 1000 && unsat > 500, "both answers exercised: {sat}/{unsat}");
}

#[test]
fn exhaustive_3_vars_4_clauses_width_3_sets() {
    // 26 non-empty non-tautological clauses of width <= 3; multisets of <= 4.
    let (total, sat, unsat) = run_domain(3, 3, false, 4, "3v/4c/w3");
    let u = 26usize;
    assert_eq!(total, 1 + u + u * (u + 1) / 2 + u * (u + 1) * (u + 2) / 6 + u * (u + 1) * (u + 2) * (u + 3) / 24);
    assert!(sat > 1000 && unsat > 1000, "both answers exercised: {sat}/{unsat}");
}

#[test]
fn exhaustive_4_vars_3_clauses_width_2_sets() {
    let (total, sat, unsat) = run_domain(4, 2, false, 3, "4v/3c/w2");
    let u = 8 + 24; // 8 units, 24 binary non-tautological sets
    assert_eq!(total, 1 + u + u * (u + 1) / 2 + u * (u + 1) * (u + 2) / 6);
    assert!(sat > 1000 && unsat > 100, "both answers exercised: {sat}/{unsat}");
}

/// Assumption API over a whole domain: every formula of <= 2 raw clauses of
/// width <= 2 over 3 variables, against every assumption set of <= 3
/// literals (contradictory pairs and repeats included). The assumptions may
/// also name a 4th variable that no clause mentions.
#[test]
fn exhaustive_assumption_domain() {
    let universe = clause_universe(3, 2, true);
    let lits: Vec<Lit> = (0..4).flat_map(|v| [Var::from_index(v).positive(), Var::from_index(v).negative()]).collect();
    let mut assumption_sets: Vec<Vec<Lit>> = vec![vec![]];
    for &a in &lits {
        assumption_sets.push(vec![a]);
        for &b in &lits {
            assumption_sets.push(vec![a, b]);
            for &c in &lits {
                assumption_sets.push(vec![a, b, c]);
            }
        }
    }
    let (mut solves, mut cores, mut sat) = (0usize, 0usize, 0usize);
    let t = std::time::Instant::now();
    for_each_multiset(&universe, 2, &mut |cs| {
        let cnf = cnf_from(3, cs);
        for a in &assumption_sets {
            solves += 1;
            let mut s = Solver::from_cnf_with_proof(&cnf);
            let expected = brute(&cnf, a);
            match s.solve_with_assumptions(a) {
                SolveResult::Sat => {
                    sat += 1;
                    assert!(expected, "SAT but formula+assumptions UNSAT: {a:?}\n{}", write_dimacs(&cnf));
                    assert!(cnf.is_satisfied_by(s.model()));
                    for l in a {
                        assert_eq!(s.model()[l.var().index()], !l.is_negated(), "assumption {l} not honoured");
                    }
                }
                SolveResult::Unsat => {
                    assert!(!expected, "UNSAT but formula+assumptions SAT: {a:?}\n{}", write_dimacs(&cnf));
                    check_core(&cnf, a, &s, &mut cores);
                }
                SolveResult::Unknown => panic!("Unknown without budget"),
            }
        }
    });
    eprintln!("[audit] assumption domain: {solves} solves ({sat} SAT, {cores} certified cores) in {:?}", t.elapsed());
    assert!(sat > 1000 && cores > 1000);
}

/// C3: the core is a sorted, duplicate-free subset of the assumptions;
/// formula AND core is UNSAT by brute force; and the kernel certifies the
/// core (formula + core units, solver proof, final `0`). When the formula is
/// UNSAT without assumptions the core must be empty and the proof stand alone.
fn check_core(cnf: &Cnf, assumptions: &[Lit], s: &Solver, cores: &mut usize) {
    let core = s.failed_assumptions();
    assert!(core.windows(2).all(|w| w[0] < w[1]), "core not sorted/deduped: {core:?}");
    assert!(core.iter().all(|c| assumptions.contains(c)), "core {core:?} not within assumptions {assumptions:?}");
    // Beyond brute-force size the kernel certification below is the oracle.
    if cnf.num_vars <= 24 {
    assert!(!brute(cnf, core), "core {core:?} is satisfiable with the formula\n{}", write_dimacs(cnf));
    }
    assert!(s.model().is_empty());
    let proof = s.proof().expect("logging on");
    if core.is_empty() {
        assert!(proof.derives_empty_clause());
        certify(cnf, proof, "empty core");
    } else {
        let mut with_core = cnf.clone();
        for &c in core {
            with_core.add_clause(&[c]);
        }
        let mut steps = proof.steps.clone();
        if !proof.derives_empty_clause() {
            steps.push(ProofStep::Add(vec![]));
        }
        let p = DratProof { steps };
        let formula = write_dimacs(&with_core);
        drat_check::check(&formula, &p.to_drat_text())
            .unwrap_or_else(|e| panic!("core {core:?} not certifiable: {e}\n{formula}\n{}", p.to_drat_text()));
    }
    *cores += 1;
}

// ---------------------------------------------------------------------------
// 2. Random 3-SAT at the phase transition (C1, C2, C5)
// ---------------------------------------------------------------------------

fn random_3sat(rng: &mut Rng, n: usize, m: usize) -> Cnf {
    let mut cnf = Cnf::new(n);
    for _ in 0..m {
        // Three distinct variables, random signs: classic uniform 3-SAT.
        let vs = loop {
            let vs = [rng.below(n), rng.below(n), rng.below(n)];
            if vs[0] != vs[1] && vs[1] != vs[2] && vs[0] != vs[2] {
                break vs;
            }
        };
        let c: Vec<Lit> = vs.iter().map(|&v| Var::from_index(v).lit(rng.next() & 1 == 1)).collect();
        cnf.add_clause(&c);
    }
    cnf
}

#[test]
fn seeded_random_3sat_near_the_phase_transition() {
    let t = std::time::Instant::now();
    let (mut sat, mut unsat, mut total) = (0, 0, 0);
    for n in 5..=20usize {
        let mut rng = Rng(0xC0DE_0000_0000_0000 ^ (n as u64 * 0x9E37_79B9));
        // Fewer instances at larger n: brute force is 2^n.
        let instances = if n <= 14 { 150 } else { 25 };
        for i in 0..instances {
            // Ratio sweep 3.8 .. 4.8 around 4.26.
            let ratio = 3.8 + (i % 11) as f64 * 0.1;
            let m = ((n as f64) * ratio).round() as usize;
            let cnf = random_3sat(&mut rng, n, m);
            total += 1;
            match verify_formula(&cnf, &format!("3sat n={n} i={i}")) {
                SolveResult::Sat => sat += 1,
                _ => unsat += 1,
            }
        }
    }
    eprintln!("[audit] random 3-SAT n=5..20: {total} formulas ({sat} SAT, {unsat} UNSAT) in {:?}", t.elapsed());
    assert!(sat > 300 && unsat > 300, "{sat}/{unsat}");
}

// ---------------------------------------------------------------------------
// 3. Incremental / assumption API on random formulas (C1-C3, C5)
// ---------------------------------------------------------------------------

#[test]
fn random_assumption_calls_on_one_solver_match_brute_force() {
    let mut rng = Rng(0xA55E_0000_1234_5678);
    let (mut sat, mut unsat, mut cores) = (0usize, 0usize, 0usize);
    for round in 0..400 {
        let vars = 3 + rng.below(10);
        let extra = rng.below(vars * 2);
        let cnf = random_3sat(&mut rng, vars, vars * 3 + extra);
        // One long-lived solver answers many assumption queries: learned
        // clauses persist between them.
        let mut s = Solver::from_cnf_with_proof(&cnf);
        let formula_sat = brute(&cnf, &[]);
        for q in 0..12 {
            let k = rng.below(5);
            let a: Vec<Lit> = (0..k).map(|_| rng.lit(vars + (q % 2))).collect(); // sometimes a fresh var
            let expected = brute(&cnf, &a);
            match s.solve_with_assumptions(&a) {
                SolveResult::Sat => {
                    sat += 1;
                    assert!(expected, "round {round} q {q}: SAT but UNSAT");
                    assert!(cnf.is_satisfied_by(s.model()));
                    assert!(a.iter().all(|l| s.model()[l.var().index()] != l.is_negated()));
                }
                SolveResult::Unsat => {
                    unsat += 1;
                    assert!(!expected, "round {round} q {q}: UNSAT but SAT");
                    check_core(&cnf, &a, &s, &mut cores);
                }
                SolveResult::Unknown => panic!("no budget"),
            }
        }
        // After many assumption calls the plain answer is still right, and an
        // UNSAT answer still carries a full checkable proof.
        match s.solve() {
            SolveResult::Sat => {
                assert!(formula_sat);
                assert!(cnf.is_satisfied_by(s.model()));
            }
            SolveResult::Unsat => {
                assert!(!formula_sat);
                certify(&cnf, s.proof().unwrap(), "after assumption calls");
            }
            SolveResult::Unknown => unreachable!(),
        }
    }
    eprintln!("[audit] assumption calls: {sat} SAT, {unsat} UNSAT, {cores} certified cores");
    assert!(sat > 500 && unsat > 500);
}

#[test]
fn incremental_batches_keep_proofs_checkable() {
    let mut rng = Rng(0x1AC3_0000_0000_0042);
    let mut proofs = 0;
    for round in 0..300 {
        let vars = 4 + rng.below(8);
        let mut s = Solver::new();
        assert!(s.enable_proof());
        s.reserve_vars(vars);
        let mut all = Cnf::new(vars);
        for batch in 0..8 {
            for _ in 0..(2 + rng.below(vars)) {
                let width = 1 + rng.below(3);
                let c: Vec<Lit> = (0..width).map(|_| rng.lit(vars)).collect();
                all.add_clause(&c);
                s.add_clause(&c);
            }
            let expected = brute(&all, &[]);
            match s.solve() {
                SolveResult::Sat => {
                    assert!(expected, "round {round} batch {batch}");
                    assert!(all.is_satisfied_by(s.model()));
                }
                SolveResult::Unsat => {
                    assert!(!expected);
                    certify(&all, s.proof().unwrap(), &format!("incremental round {round} batch {batch}"));
                    proofs += 1;
                    break;
                }
                SolveResult::Unknown => panic!("no budget"),
            }
        }
    }
    eprintln!("[audit] incremental proofs certified: {proofs}");
    assert!(proofs > 100);
}

// ---------------------------------------------------------------------------
// 4. Budgets (C4, C7) and determinism (C6)
// ---------------------------------------------------------------------------

#[test]
fn zero_conflict_budget_is_unknown_and_recovery_is_correct() {
    let mut rng = Rng(0xB0D6_0000_0000_0001);
    for round in 0..300 {
        let vars = 5 + rng.below(8);
        let extra = rng.below(vars);
        let cnf = random_3sat(&mut rng, vars, vars * 4 + extra);
        let mut s = Solver::from_cnf_with_proof(&cnf);
        let known_unsat = !s.is_ok();
        s.set_conflict_budget(Some(0));
        let r0 = s.solve();
        if known_unsat {
            assert_eq!(r0, SolveResult::Unsat);
        } else {
            assert_eq!(r0, SolveResult::Unknown, "round {round}: budget 0 must be Unknown");
            assert!(s.model().is_empty() && s.failed_assumptions().is_empty());
        }
        // Lift the budget: the answer must now be the true one and any proof
        // recorded across the Unknown call must still check.
        s.set_conflict_budget(None);
        let expected = brute(&cnf, &[]);
        match s.solve() {
            SolveResult::Sat => {
                assert!(expected);
                assert!(cnf.is_satisfied_by(s.model()));
            }
            SolveResult::Unsat => {
                assert!(!expected);
                certify(&cnf, s.proof().unwrap(), "after Unknown");
            }
            SolveResult::Unknown => panic!("Unknown without budget"),
        }
    }
}

#[test]
fn tiny_budgets_never_give_a_wrong_answer() {
    let mut rng = Rng(0xB0D6_0000_0000_0002);
    let (mut unknown, mut decided) = (0, 0);
    for round in 0..400 {
        let vars = 8 + rng.below(8);
        let cnf = random_3sat(&mut rng, vars, (vars as f64 * 4.4) as usize);
        let expected = brute(&cnf, &[]);
        for budget in [1u64, 2, 3, 5, 8] {
            let mut s = Solver::from_cnf_with_proof(&cnf);
            s.set_conflict_budget(Some(budget));
            // Resume repeatedly with fresh small budgets: Unknown, Unknown, ... answer.
            let mut calls = 0;
            loop {
                calls += 1;
                assert!(calls < 10_000, "no progress with budget {budget}");
                match s.solve() {
                    SolveResult::Unknown => {
                        unknown += 1;
                        assert!(s.model().is_empty());
                        s.set_conflict_budget(Some(budget));
                    }
                    SolveResult::Sat => {
                        decided += 1;
                        assert!(expected, "round {round} budget {budget}: wrong SAT");
                        assert!(cnf.is_satisfied_by(s.model()));
                        break;
                    }
                    SolveResult::Unsat => {
                        decided += 1;
                        assert!(!expected, "round {round} budget {budget}: wrong UNSAT");
                        certify(&cnf, s.proof().unwrap(), "tiny budget resume");
                        break;
                    }
                }
            }
        }
    }
    eprintln!("[audit] tiny budgets: {unknown} Unknown answers, {decided} decided");
    assert!(unknown > 100, "budgets must actually bite: {unknown}");
}

#[test]
fn conflict_budget_counts_down_across_calls() {
    // PHP(5,4) takes well over 3 conflicts.
    let cnf = pigeonhole(4);
    let mut s = Solver::from_cnf(&cnf);
    s.set_conflict_budget(Some(3));
    assert_eq!(s.solve(), SolveResult::Unknown);
    let c1 = s.stats().conflicts;
    assert!(c1 <= 3 + 1, "overran a 3-conflict budget: {c1}");
    // The budget is spent: later calls give Unknown immediately, doing no work.
    assert_eq!(s.solve(), SolveResult::Unknown);
    assert_eq!(s.stats().conflicts, c1, "an exhausted budget must stop the search at once");
    s.set_conflict_budget(None);
    assert_eq!(s.solve(), SolveResult::Unsat);
}

#[test]
fn work_budget_is_honest() {
    let mut rng = Rng(0x0B0D_6E70);
    let mut unknown = 0;
    for _ in 0..200 {
        let vars = 8 + rng.below(8);
        let cnf = random_3sat(&mut rng, vars, (vars as f64 * 4.4) as usize);
        let expected = brute(&cnf, &[]);
        let mut s = Solver::from_cnf_with_proof(&cnf);
        // Each Unknown throws the partial assignment away (restart), so a
        // budget smaller than the work to reach one conflict can never make
        // progress. Callers that resume must therefore grow the budget; this
        // one doubles it, which is how an anytime caller should behave.
        let mut budget = 1 + rng.below(8) as u64;
        s.set_work_budget(Some(budget));
        let mut guard = 0;
        loop {
            guard += 1;
            assert!(guard < 1000, "doubling the work budget must terminate");
            match s.solve() {
                SolveResult::Unknown => {
                    unknown += 1;
                    assert!(s.model().is_empty());
                    budget *= 2;
                    s.set_work_budget(Some(budget));
                }
                SolveResult::Sat => {
                    assert!(expected);
                    assert!(cnf.is_satisfied_by(s.model()));
                    break;
                }
                SolveResult::Unsat => {
                    assert!(!expected);
                    certify(&cnf, s.proof().unwrap(), "work budget resume");
                    break;
                }
            }
        }
    }
    assert!(unknown > 50, "work budget must bite: {unknown}");
}

#[test]
fn same_input_same_answer_stats_and_proof() {
    let mut rng = Rng(0xDE7E_0000_0000_0003);
    let mut corpus: Vec<Cnf> = (0..60).map(|i| random_3sat(&mut rng, 20 + i % 15, (4.3 * (20 + i % 15) as f64) as usize)).collect();
    corpus.push(pigeonhole(5));
    corpus.push(pigeonhole(6));
    for (i, cnf) in corpus.iter().enumerate() {
        let run = |budgeted: bool| {
            let mut s = Solver::from_cnf_with_proof(cnf);
            if budgeted {
                s.set_conflict_budget(Some(7));
            }
            let mut answers = vec![s.solve()];
            while answers.last() == Some(&SolveResult::Unknown) {
                s.set_conflict_budget(Some(7));
                answers.push(s.solve());
            }
            let model = s.model().to_vec();
            (answers, model, s.stats(), s.proof().unwrap().to_drat_text())
        };
        for budgeted in [false, true] {
            let (a, b) = (run(budgeted), run(budgeted));
            assert_eq!(a, b, "formula {i} budgeted={budgeted}: runs differ");
        }
        // Clause order is part of the input: a permuted formula may legitimately differ, but
        // must still be answered correctly (checked elsewhere).
    }
}

// ---------------------------------------------------------------------------
// 5. Hard families at small size: proof length and learning
// ---------------------------------------------------------------------------

#[test]
fn pigeonhole_up_to_6_is_certified_and_learns() {
    for n in 1..=6usize {
        let cnf = pigeonhole(n);
        let mut s = Solver::from_cnf_with_proof(&cnf);
        assert_eq!(s.solve(), SolveResult::Unsat, "PHP({},{n})", n + 1);
        let p = s.proof().unwrap();
        certify(&cnf, p, &format!("PHP({},{n})", n + 1));
        let adds = p.steps.iter().filter(|s| matches!(s, ProofStep::Add(_))).count();
        eprintln!(
            "[audit] PHP({},{n}): conflicts {}, learned {}, proof steps {}, adds {adds}",
            n + 1,
            s.stats().conflicts,
            s.stats().learned,
            p.steps.len()
        );
        if n >= 3 {
            assert!(s.stats().learned > 0, "PHP({},{n}) must need learning", n + 1);
        }
        // Every learned clause is logged, the empty clause ends the proof.
        assert!(adds as u64 > s.stats().learned);
        assert_eq!(p.steps.last(), Some(&ProofStep::Add(vec![])));
    }
}

#[test]
fn pigeonhole_with_a_free_hole_is_sat() {
    // n pigeons, n holes: SAT. Drop the last pigeon's rows.
    for n in 2..=7usize {
        let full = pigeonhole(n);
        let mut cnf = Cnf::new(full.num_vars);
        // pigeonhole(n) numbers pigeons 0..=n; remove every clause mentioning pigeon n.
        for c in &full.clauses {
            if c.iter().all(|l| l.var().index() / n < n) {
                cnf.add_clause(c);
            }
        }
        let mut s = Solver::from_cnf(&cnf);
        assert_eq!(s.solve(), SolveResult::Sat, "PHP-minus-one n={n}");
        assert!(cnf.is_satisfied_by(s.model()));
    }
}

/// Cycle of n variables, each adjacent pair XOR = 1 (x_i != x_{i+1}):
/// satisfiable iff n is even.
fn xor_cycle(n: usize) -> Cnf {
    let mut cnf = Cnf::new(n);
    for i in 0..n {
        let (a, b) = (Var::from_index(i), Var::from_index((i + 1) % n));
        cnf.add_clause(&[a.positive(), b.positive()]);
        cnf.add_clause(&[a.negative(), b.negative()]);
    }
    cnf
}

/// XOR of `k` variables via a Tseitin chain t_1 = x_1^x_2, t_2 = t_1^x_3, ...,
/// asserted to `parity`, over the same inputs twice with opposite parity
/// (so UNSAT), or once (SAT).
fn xor_chain_pair(k: usize, contradictory: bool) -> Cnf {
    let mut cnf = Cnf::new(k);
    let mut next = k;
    let mut chain = |cnf: &mut Cnf, parity: bool| {
        let mut acc = Var::from_index(0);
        for i in 1..k {
            let x = Var::from_index(i);
            let t = Var::from_index(next);
            next += 1;
            // t = acc XOR x
            for (a, b, c) in [(false, false, true), (false, true, false), (true, false, false), (true, true, true)] {
                // forbid assignments where t != a^b ... encode clause for each (acc=a, x=b, t=c) that is NOT allowed
                let _ = c;
                let allowed_t = a ^ b;
                // Forbid (acc=a, x=b, t=!allowed_t)
                cnf.add_clause(&[acc.lit(a), x.lit(b), t.lit(allowed_t)]);
            }
            acc = t;
        }
        cnf.add_clause(&[acc.lit(!parity)]);
    };
    chain(&mut cnf, false);
    if contradictory {
        chain(&mut cnf, true);
    }
    cnf
}

/// Tseitin formula on K_n: one boolean per edge, each vertex's incident edges
/// XOR to its charge; exactly one vertex is charged, so the total parity is
/// odd and the formula is UNSAT (every edge is counted at two vertices).
fn tseitin_complete(n: usize) -> Cnf {
    // Edge {i, j} (i < j) gets the index of its position in lexicographic order.
    let edge = |a: usize, b: usize| {
        let (i, j) = (a.min(b), a.max(b));
        i * n - i * (i + 1) / 2 + (j - i - 1)
    };
    let m = n * (n - 1) / 2;
    let mut cnf = Cnf::new(m);
    for v in 0..n {
        let vars: Vec<usize> = (0..n).filter(|&u| u != v).map(|u| edge(v, u)).collect();
        let charge = v == 0;
        // Forbid every assignment of incident edges whose parity != charge.
        for bits in 0u32..(1 << vars.len()) {
            if (bits.count_ones() % 2 == 1) != charge {
                let c: Vec<Lit> =
                    vars.iter().enumerate().map(|(i, &x)| Var::from_index(x).lit(bits >> i & 1 == 1)).collect();
                cnf.add_clause(&c);
            }
        }
    }
    cnf
}

#[test]
fn parity_and_xor_families() {
    for n in 3..=21usize {
        let cnf = xor_cycle(n);
        let mut s = Solver::from_cnf_with_proof(&cnf);
        if n % 2 == 1 {
            assert_eq!(s.solve(), SolveResult::Unsat, "xor cycle {n}");
            certify(&cnf, s.proof().unwrap(), &format!("xor cycle {n}"));
        } else {
            assert_eq!(s.solve(), SolveResult::Sat, "xor cycle {n}");
            assert!(cnf.is_satisfied_by(s.model()));
        }
    }
    for k in 2..=14usize {
        let bad = xor_chain_pair(k, true);
        let mut s = Solver::from_cnf_with_proof(&bad);
        assert_eq!(s.solve(), SolveResult::Unsat, "xor chain pair {k}");
        certify(&bad, s.proof().unwrap(), &format!("xor chain pair {k}"));
        let good = xor_chain_pair(k, false);
        let mut s = Solver::from_cnf(&good);
        assert_eq!(s.solve(), SolveResult::Sat, "single xor chain {k}");
        assert!(good.is_satisfied_by(s.model()));
    }
    for n in [3usize, 4, 5, 6] {
        let cnf = tseitin_complete(n);
        let mut s = Solver::from_cnf_with_proof(&cnf);
        assert_eq!(s.solve(), SolveResult::Unsat, "Tseitin K{n}");
        let p = s.proof().unwrap();
        eprintln!("[audit] Tseitin K{n}: {} vars, {} clauses, conflicts {}, proof steps {}", cnf.num_vars, cnf.clauses.len(), s.stats().conflicts, p.steps.len());
        certify(&cnf, p, &format!("Tseitin K{n}"));
    }
}

/// A bigger random UNSAT corpus (beyond brute force): every UNSAT answer is
/// trusted only because the kernel certifies it. Large enough that restarts
/// and clause deletion actually run.
#[test]
fn medium_instances_exercise_restarts_and_reduction_with_certified_proofs() {
    let mut rng = Rng(0x0ED1_0000_0000_0007);
    let (mut certified, mut max_restarts, mut max_deleted) = (0, 0, 0);
    let t = std::time::Instant::now();
    for i in 0..40 {
        let n = 70 + (i % 5) * 5;
        let cnf = random_3sat(&mut rng, n, (n as f64 * 4.4) as usize);
        let mut s = Solver::from_cnf_with_proof(&cnf);
        match s.solve() {
            SolveResult::Sat => assert!(cnf.is_satisfied_by(s.model())),
            SolveResult::Unsat => {
                certify(&cnf, s.proof().unwrap(), &format!("medium {i}"));
                certified += 1;
            }
            SolveResult::Unknown => panic!(),
        }
        max_restarts = max_restarts.max(s.stats().restarts);
        max_deleted = max_deleted.max(s.stats().deleted);
        let deletes = s.proof().unwrap().steps.iter().filter(|st| matches!(st, ProofStep::Delete(_))).count();
        assert_eq!(deletes as u64, s.stats().deleted, "every deletion is logged exactly once");
    }
    eprintln!("[audit] medium: certified {certified}, max restarts {max_restarts}, max deleted {max_deleted}, {:?}", t.elapsed());
    assert!(certified > 5);
    assert!(max_restarts > 0, "restarts never happened");
}

/// Past the learned-clause limit (2000) the solver reduces its database. The
/// proof must log each deletion and still pass the kernel; the model after a
/// reduction must still be right; and assumption cores found afterwards must
/// still be certifiable.
#[test]
fn clause_database_reduction_keeps_proofs_and_models_sound() {
    let mut rng = Rng(0x00DE_1E7E_0000_0001);
    let (mut unsat_deleting, mut sat_deleting) = (0, 0);
    for i in 0..4 {
        let n = 175 + i * 3;
        let cnf = random_3sat(&mut rng, n, (n as f64 * 4.4) as usize);
        let mut s = Solver::from_cnf_with_proof(&cnf);
        let r = s.solve();
        let deletes = s.proof().unwrap().steps.iter().filter(|st| matches!(st, ProofStep::Delete(_))).count();
        assert_eq!(deletes as u64, s.stats().deleted, "each deletion logged exactly once");
        for st in &s.proof().unwrap().steps {
            if let ProofStep::Delete(c) = st {
                assert!(c.len() >= 3, "binary/unit clauses are never deleted");
            }
        }
        match r {
            SolveResult::Sat => {
                assert!(cnf.is_satisfied_by(s.model()));
                if s.stats().deleted > 0 {
                    sat_deleting += 1;
                }
            }
            SolveResult::Unsat => {
                certify(&cnf, s.proof().unwrap(), &format!("reduction {i}"));
                if s.stats().deleted > 0 {
                    unsat_deleting += 1;
                }
            }
            SolveResult::Unknown => unreachable!(),
        }
        // The same solver, reused under assumptions after the reduction.
        let a = [Var::from_index(0).positive(), Var::from_index(1).negative()];
        let mut cores = 0;
        match s.solve_with_assumptions(&a) {
            SolveResult::Sat => {
                assert!(cnf.is_satisfied_by(s.model()));
                assert!(s.model()[0] && !s.model()[1]);
            }
            SolveResult::Unsat => check_core(&cnf, &a, &s, &mut cores),
            SolveResult::Unknown => unreachable!(),
        }
    }
    eprintln!("[audit] reduction: {unsat_deleting} UNSAT and {sat_deleting} SAT runs deleted clauses");
    assert!(unsat_deleting + sat_deleting >= 2, "reduce_db must actually run");
    assert!(unsat_deleting >= 1);
}

/// Beyond brute-force size, soundness rests on the oracles that do not need
/// enumeration: a model is re-evaluated, every UNSAT core is certified by the
/// kernel. A spurious Sat is impossible if models always verify; a spurious
/// Unsat is impossible if cores and proofs always certify.
#[test]
fn medium_assumption_fuzz_is_certified() {
    let mut rng = Rng(0x0FA2_0000_0000_0009);
    let (mut sat, mut cores, mut finals) = (0, 0, 0);
    for round in 0..60 {
        let n = 30 + rng.below(40);
        let ratio = 3.8 + (rng.below(9) as f64) * 0.1;
        let cnf = random_3sat(&mut rng, n, (n as f64 * ratio) as usize);
        let mut s = Solver::from_cnf_with_proof(&cnf);
        for q in 0..20 {
            let k = 1 + rng.below(6);
            let a: Vec<Lit> = (0..k).map(|_| rng.lit(n)).collect();
            match s.solve_with_assumptions(&a) {
                SolveResult::Sat => {
                    sat += 1;
                    assert!(cnf.is_satisfied_by(s.model()), "round {round} q {q}");
                    assert!(a.iter().all(|l| s.model()[l.var().index()] != l.is_negated()));
                }
                SolveResult::Unsat => check_core(&cnf, &a, &s, &mut cores),
                SolveResult::Unknown => unreachable!(),
            }
            if !s.is_ok() {
                break;
            }
        }
        match s.solve() {
            SolveResult::Sat => assert!(cnf.is_satisfied_by(s.model())),
            SolveResult::Unsat => {
                certify(&cnf, s.proof().unwrap(), &format!("medium fuzz {round}"));
                finals += 1;
            }
            SolveResult::Unknown => unreachable!(),
        }
    }
    eprintln!("[audit] medium assumption fuzz: {sat} SAT answers, {cores} certified cores, {finals} certified final refutations");
    assert!(sat > 100 && cores > 100 && finals > 5);
}

/// The solver is deterministic (C6), so its search on a fixed formula is a
/// fingerprint of its heuristics: VSIDS decay, clause activity, restarts,
/// phase saving and reduction all steer it. Correctness tests cannot see a
/// heuristic silently switched off (the answers stay right, only slower), so
/// this pins the exact search statistics of a few fixed instances. If a
/// deliberate heuristic change moves these numbers, update them here and
/// say why in the commit message; an unexplained change is a regression.
#[test]
fn search_statistics_fingerprint_pins_the_heuristics() {
    let mut observed = Vec::new();
    for n in [5usize, 6, 7] {
        let mut s = Solver::from_cnf(&pigeonhole(n));
        assert_eq!(s.solve(), SolveResult::Unsat);
        let st = s.stats();
        observed.push((format!("PHP({},{n})", n + 1), st.conflicts, st.restarts, st.deleted, st.learned));
    }
    let mut rng = Rng(0x00DE_1E7E_0000_0001);
    let n = 175;
    let cnf = random_3sat(&mut rng, n, (n as f64 * 4.4) as usize);
    let mut s = Solver::from_cnf(&cnf);
    let r = s.solve();
    let st = s.stats();
    observed.push((format!("random n=175 {r:?}"), st.conflicts, st.restarts, st.deleted, st.learned));
    for o in &observed {
        eprintln!("[audit] fingerprint {o:?}");
    }
    let expected: Vec<(String, u64, u64, u64, u64)> = FINGERPRINT
        .iter()
        .map(|&(name, c, r, d, l)| (name.to_string(), c, r, d, l))
        .collect();
    assert_eq!(observed, expected, "search heuristics changed; see the doc comment of this test");
}

/// (instance, conflicts, restarts, deleted, learned)
const FINGERPRINT: &[(&str, u64, u64, u64, u64)] = &[
    ("PHP(6,5)", 150, 1, 0, 149),
    ("PHP(7,6)", 788, 5, 0, 787),
    ("PHP(8,7)", 4719, 24, 3311, 4718),
    ("random n=175 Unsat", 4710, 24, 3310, 4709),
];

// ---------------------------------------------------------------------------
// 6. Malformed DIMACS and limits (C9)
// ---------------------------------------------------------------------------

#[test]
fn malformed_dimacs_is_always_an_error() {
    let bad: &[(&str, &str)] = &[
        ("", "missing header"),
        ("c only a comment\n", "missing header"),
        ("1 2 0\n", "clause before header"),
        ("p cnf 2 1\np cnf 2 1\n1 0\n", "duplicate header"),
        ("p cnf 2\n1 0\n", "short header"),
        ("p cnf 2 1 7\n1 0\n", "long header"),
        ("p dnf 2 1\n1 0\n", "wrong format word"),
        ("p cnf -2 1\n1 0\n", "negative var count"),
        ("p cnf 2 -1\n1 0\n", "negative clause count"),
        ("p cnf x 1\n1 0\n", "non-numeric var count"),
        ("p cnf 2 1.5\n1 0\n", "fractional clause count"),
        ("p cnf 2 1\n1 3 0\n", "var out of range"),
        ("p cnf 2 1\n-3 0\n", "negative var out of range"),
        ("p cnf 2 1\n1 2\n", "unterminated clause"),
        ("p cnf 2 2\n1 0\n", "too few clauses"),
        ("p cnf 2 1\n1 0\n2 0\n", "too many clauses"),
        ("p cnf 2 1\n1 x 0\n", "bad token"),
        ("p cnf 2 1\n1 2.0 0\n", "float literal"),
        ("p cnf 2 1\n1 99999999999999999999 0\n", "overflowing literal"),
        ("p cnf 2 1\n1 -9223372036854775808 0\n", "i64::MIN literal"),
        ("p cnf 2 0\n1 0\n", "clause with zero declared"),
        ("p cnf 0 1\n1 0\n", "literal with zero vars"),
        ("p cnf 4294967296 0\n", "var count beyond limit"),
    ];
    for (text, why) in bad {
        let r = std::panic::catch_unwind(|| parse_dimacs(text));
        match r {
            Ok(Err(_)) => {}
            Ok(Ok(c)) => panic!("{why}: accepted malformed input {text:?} as {c:?}"),
            Err(_) => panic!("{why}: reader panicked on {text:?}"),
        }
    }
}

#[test]
fn dimacs_edge_cases_that_must_parse() {
    // Empty formula, empty clause, comments anywhere, CRLF, tabs, spanning lines, SATLIB `%`.
    let ok: &[(&str, usize, usize)] = &[
        ("p cnf 0 0\n", 0, 0),
        ("p cnf 5 0\n", 5, 0),
        ("p cnf 0 1\n0\n", 0, 1),
        ("p cnf 1 2\n1 0\n0\n", 1, 2),
        ("c hi\np cnf 2 1\nc mid\n1\nc inside clause\n2 0\n", 2, 1),
        ("p cnf 2 1\r\n1 -2 0\r\n", 2, 1),
        ("p cnf 2 1\n\t1\t-2\t0\n", 2, 1),
        ("p cnf 2 1\n1 0\n%\n0\ngarbage\n", 2, 1),
        ("p cnf 2 2\n1 0 2 0\n", 2, 2),
        ("  p cnf 2 1  \n 1 0 \n", 2, 1),
        ("p cnf 3 1\n1 -1 1 0\n", 3, 1), // duplicates and tautology are legal clauses
    ];
    for (text, vars, clauses) in ok {
        let c = parse_dimacs(text).unwrap_or_else(|e| panic!("{text:?}: {e}"));
        assert_eq!((c.num_vars, c.clauses.len()), (*vars, *clauses), "{text:?}");
    }
}

/// The solver and the kernel read DIMACS with separate parsers. If they
/// disagreed about what a formula text means, the kernel could certify a
/// proof about a different formula than the solver solved. This feeds both
/// the same texts (the malformed corpus, the lenient shapes, and tokens like
/// `+1`, `-0`, `00`) and requires them to accept and reject identically.
#[test]
fn solver_and_kernel_read_dimacs_identically() {
    let texts = [
        "",
        "p cnf 2 1\n1 0\n",
        "1 2 0\n",
        "p cnf 2 1\np cnf 2 1\n1 0\n",
        "p cnf 2\n1 0\n",
        "p cnf 2 1\n1 3 0\n",
        "p cnf 2 1\n1 2\n",
        "p cnf 2 2\n1 0\n",
        "p cnf 2 1\n1 0\n2 0\n",
        "p cnf 2 1\n1 x 0\n",
        "p cnf 2 1\n+1 0\n",
        "p cnf 2 1\n1 -0\n",
        "p cnf 2 1\n1 00\n",
        "p cnf 2 1\n01 0\n",
        "p cnf 0 0\n",
        "p cnf 0 1\n0\n",
        "p cnf 2 1\n1 0\n%\n",
        "p cnf 2 1\r\n1 -2 0\r\n",
        "p cnf 2 1\n1 -1 0\n",
        "p cnf 2 1\ncnf stuff\n1 0\n",
        "c x\np cnf 2 1\nc y\n1\n2 0\n",
        "p cnf 2 1\n1 0 2\n",
        "p cnf 2 1\n1 0\nx 0\n",
        "p  cnf  2   1\n1 0\n",
        "p cnf 2 1\n1 99999999999999999999 0\n",
        "p cnf -2 1\n1 0\n",
        "p cnf 2 1 7\n1 0\n",
    ];
    let mut disagreements = Vec::new();
    for t in texts {
        let ours = parse_dimacs(t).is_ok();
        let theirs = !matches!(drat_check::check(t, "0\n"), Err(drat_check::Rejection::Parse(_)));
        if ours != theirs {
            disagreements.push((t, ours, theirs));
        }
    }
    assert!(
        disagreements.is_empty(),
        "solver/kernel DIMACS disagreement (text, solver accepts, kernel accepts): {disagreements:?}"
    );
}

#[test]
fn dimacs_limits() {
    let big = "p cnf 1048577 0\n";
    assert!(matches!(parse_dimacs(big).unwrap_err().kind, DimacsErrorKind::TooManyVars { .. }));
    assert!(parse_dimacs_with_limit(big, 2_000_000).is_ok());
    assert!(parse_dimacs_with_limit("p cnf 10 0\n", 9).is_err());
    assert!(parse_dimacs_with_limit("p cnf 10 0\n", 10).is_ok());
    // Limits above the encoding ceiling are clamped, never wrapping.
    assert!(parse_dimacs_with_limit("p cnf 4294967297 0\n", usize::MAX).is_err());
    // A header at exactly 2^31 vars parses (no clause allocation depends on it).
    let c = parse_dimacs_with_limit("p cnf 2147483648 1\n2147483648 0\n", usize::MAX).unwrap();
    assert_eq!(c.clauses[0][0].var().index(), (1usize << 31) - 1);
    // A hostile clause count in the header allocates nothing.
    assert!(parse_dimacs("p cnf 3 18446744073709551615\n1 0\n").is_err());
    // Very long single clause and many clauses parse linearly.
    let mut long = String::from("p cnf 5000 1\n");
    for v in 1..=5000 {
        long.push_str(&format!("{v} "));
    }
    long.push_str("0\n");
    assert_eq!(parse_dimacs(&long).unwrap().clauses[0].len(), 5000);
}

#[test]
fn dimacs_round_trip_solves_identically() {
    let mut rng = Rng(0xD1AA_C500_0000_0001);
    for _ in 0..200 {
        let vars = 2 + rng.below(8);
        let cnf = random_cnf(&mut rng, vars, vars * 4);
        let back = parse_dimacs(&write_dimacs(&cnf)).unwrap();
        assert_eq!(back, cnf);
    }
}

// ---------------------------------------------------------------------------
// 7. API robustness
// ---------------------------------------------------------------------------

#[test]
fn proof_enable_rules() {
    let mut s = Solver::new();
    s.reserve_vars(3); // creating variables is not touching the clause set
    assert!(s.enable_proof());
    assert!(s.enable_proof(), "idempotent before the first clause");
    s.add_clause(&[Var::from_index(0).positive()]);
    assert!(!s.enable_proof() || s.proof().is_some());
    let mut late = Solver::new();
    late.add_clause(&[Var::from_index(0).positive()]);
    assert!(!late.enable_proof(), "late enable must refuse");
    assert!(late.proof().is_none());
    assert!(late.take_proof().is_none());
}

#[test]
fn add_clause_after_unsat_and_degenerate_clauses() {
    let x = Var::from_index(0);
    let mut s = Solver::new();
    s.enable_proof();
    assert!(s.add_clause(&[x.positive(), x.negative()]), "tautology is accepted and ignored");
    assert!(s.add_clause(&[x.positive(), x.positive()]), "duplicate literal collapses to a unit");
    assert!(!s.add_clause(&[x.negative()]), "contradicting unit");
    assert!(!s.add_clause(&[x.positive()]), "adding after UNSAT is a no-op");
    assert!(!s.is_ok());
    assert_eq!(s.solve(), SolveResult::Unsat);
    assert_eq!(s.solve_with_assumptions(&[x.positive()]), SolveResult::Unsat);
    assert!(s.failed_assumptions().is_empty());
    let mut cnf = Cnf::new(1);
    cnf.add_clause(&[x.positive(), x.negative()]);
    cnf.add_clause(&[x.positive(), x.positive()]);
    cnf.add_clause(&[x.negative()]);
    certify(&cnf, s.proof().unwrap(), "degenerate clauses");
}

#[test]
fn sat_model_covers_variables_no_clause_mentions() {
    let mut s = Solver::new();
    s.reserve_vars(10);
    s.add_clause(&[Var::from_index(2).positive()]);
    assert_eq!(s.solve(), SolveResult::Sat);
    assert_eq!(s.model().len(), 10);
    assert!(s.model()[2]);
    // A variable created later is not in an older model.
    let v = s.new_var();
    assert_eq!(s.model_value(v), None);
    assert_eq!(s.solve(), SolveResult::Sat);
    assert!(s.model_value(v).is_some());
}

#[test]
fn solver_reusable_after_every_kind_of_answer() {
    let (a, b) = (Var::from_index(0), Var::from_index(1));
    let mut s = Solver::new();
    s.add_clause(&[a.positive(), b.positive()]);
    assert_eq!(s.solve_with_assumptions(&[a.negative(), b.negative()]), SolveResult::Unsat);
    assert_eq!(s.failed_assumptions().len(), 2);
    assert_eq!(s.solve(), SolveResult::Sat);
    assert!(s.failed_assumptions().is_empty(), "core must be cleared by the next call");
    s.set_conflict_budget(Some(0));
    assert_eq!(s.solve(), SolveResult::Unknown);
    s.set_conflict_budget(None);
    assert_eq!(s.solve_with_assumptions(&[a.negative()]), SolveResult::Sat);
    assert!(!s.model()[0] && s.model()[1]);
    s.add_clause(&[a.negative()]);
    s.add_clause(&[b.negative()]);
    assert_eq!(s.solve(), SolveResult::Unsat);
}

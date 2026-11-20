use std::{env, fs, path::PathBuf};

use cdcl_sat::{write_dimacs, Cnf, Lit, SolveResult, Solver, Var};

const OUTCOMES: [usize; 9] = [0, 1, 1, 2, 3, 0, 2, 3, 1];

fn add(cnf: &mut Cnf, row: &[Lit]) {
    cnf.add_clause(row);
}

fn formula(force_clean: bool) -> Cnf {
    // 0=pass, 1=crash, 2=timeout, 3=OOM; five triage, then four mutations.
    let mut cnf = Cnf::new(0);
    for (call, outcome) in OUTCOMES.iter().copied().enumerate() {
        let vars: Vec<Lit> = (0..4)
            .map(|kind| Var::from_index(call * 4 + kind).positive())
            .collect();
        add(&mut cnf, &vars);
        for left in 0..4 {
            for right in left + 1..4 {
                add(&mut cnf, &[!vars[left], !vars[right]]);
            }
        }
        for (kind, lit) in vars.iter().copied().enumerate() {
            add(&mut cnf, &[if kind == outcome { lit } else { !lit }]);
        }
    }

    let clean = Var::from_index(36).positive();
    // This model encodes only clean => every fixed return is pass. It does not
    // encode ledger sums, mutation energy, budget transitions, or progress.
    for call in 0..9 {
        add(&mut cnf, &[!clean, Var::from_index(call * 4).positive()]);
    }
    add(&mut cnf, &[if force_clean { clean } else { !clean }]);
    cnf
}

fn main() {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .expect("pass an output directory");
    fs::create_dir_all(&output).expect("create output directory");

    let feasible = formula(false);
    let feasible_dimacs = write_dimacs(&feasible);
    let mut sat_solver = Solver::new();
    for clause in &feasible.clauses {
        sat_solver.add_clause(clause);
    }
    assert_eq!(sat_solver.solve(), SolveResult::Sat);
    let sat_assignment = sat_solver.model();
    assert!(feasible.is_satisfied_by(&sat_assignment));
    fs::write(output.join("clean-false.cnf"), &feasible_dimacs)
        .expect("write satisfiable control CNF");

    let impossible = formula(true);
    let impossible_dimacs = write_dimacs(&impossible);
    let mut proof_solver = Solver::new();
    assert!(proof_solver.enable_proof());
    for clause in &impossible.clauses {
        proof_solver.add_clause(clause);
    }
    assert_eq!(proof_solver.solve(), SolveResult::Unsat);
    let proof = proof_solver
        .proof()
        .expect("proof logging enabled")
        .to_drat_text();
    let receipt = drat_check::check(&impossible_dimacs, &proof)
        .expect("independent DRAT checker accepts the unsatisfiable formula");
    fs::write(output.join("clean-true.cnf"), &impossible_dimacs)
        .expect("write forced-clean CNF");
    fs::write(output.join("clean-true.drat"), &proof).expect("write DRAT certificate");

    println!(
        "model=fixed-vector clean-implies-all-pass variables={} clauses={} proof_bytes={} lemmas={} rat_lemmas={} empty_clause_line={}",
        impossible.num_vars,
        impossible.clauses.len(),
        proof.len(),
        receipt.lemmas,
        receipt.rat_lemmas,
        receipt.empty_clause_line
    );
}

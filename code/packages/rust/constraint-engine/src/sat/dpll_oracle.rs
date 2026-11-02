//! # The old DPLL, kept as a test oracle
//!
//! Until VER09 this was `SatTactic`'s solver: the classic Davis–Putnam–
//! Logemann–Loveland procedure (1962) — unit propagation, pure-literal
//! elimination, and chronological backtracking on the first unassigned
//! variable, over string-named literals.
//!
//! VER09 replaced it with the `cdcl-sat` CDCL solver. It is not deleted,
//! because an independent implementation is worth more as a second opinion
//! than as nothing: it shares no code with `cdcl-sat`, so the adapter's tests
//! run both on the same random formulas and require the same answer. It is
//! compiled only under `cfg(test)`, so it can never be reached from
//! production code again.
//!
//! The code below is the pre-VER09 implementation, unchanged except for
//! visibility.

use std::collections::HashMap;

use super::Clause;

/// Solve `clauses`: `Some(assignment)` if SAT, `None` if UNSAT.
///
/// Panics if the budget runs out, which no test formula should cause.
pub(super) fn solve(clauses: &[Clause]) -> Option<HashMap<String, bool>> {
    let mut assignment = HashMap::new();
    let mut budget = 1_000_000;
    match dpll(clauses, &mut assignment, &mut budget) {
        DpllResult::Sat => Some(assignment),
        DpllResult::Unsat => None,
        DpllResult::Unknown(reason) => panic!("DPLL oracle gave up: {reason}"),
    }
}

#[derive(Debug)]
enum DpllResult {
    Sat,
    Unsat,
    /// The DPLL search budget was exhausted.  The formula may be satisfiable
    /// but the solver couldn't confirm it within the allowed node count.
    /// The caller should surface this as `SolverResult::Unknown`.
    Unknown(String),
}

/// Core DPLL procedure.
///
/// `clauses` is the current (possibly simplified) formula; `assignment`
/// accumulates committed assignments.  `budget` is a shared mutable counter
/// decremented on each recursive call; when it reaches zero the search is
/// abandoned and `DpllResult::Unknown` is returned to prevent exponential
/// blowup on adversarial formulae.
fn dpll(
    clauses: &[Clause],
    assignment: &mut HashMap<String, bool>,
    budget: &mut usize,
) -> DpllResult {
    // ── Budget guard ─────────────────────────────────────────────────────────
    //
    // DPLL is O(2^n) in the worst case.  For the small Boolean formulae that
    // arise from LANG23 refinement checking the budget is never reached in
    // practice.  For adversarially large or malformed programs, the budget
    // prevents a denial-of-service via an exhaustive search.
    if *budget == 0 {
        return DpllResult::Unknown("DPLL node budget exceeded".into());
    }
    *budget -= 1;

    // Step 1: unit propagation.
    let (mut clauses_owned, forced) = unit_propagate(clauses);
    for (var, val) in &forced {
        assignment.insert(var.clone(), *val);
    }

    // Check for contradictions (empty clause) or satisfaction (no clauses).
    if clauses_owned.iter().any(|c| c.is_empty()) {
        return DpllResult::Unsat;
    }
    if clauses_owned.is_empty() {
        return DpllResult::Sat;
    }

    // Step 2: pure literal elimination.
    let pure = find_pure_literals(&clauses_owned);
    for (var, val) in &pure {
        assignment.insert(var.clone(), *val);
        clauses_owned = simplify_clauses(&clauses_owned, var, *val);
    }

    if clauses_owned.is_empty() {
        return DpllResult::Sat;
    }
    if clauses_owned.iter().any(|c| c.is_empty()) {
        return DpllResult::Unsat;
    }

    // Step 3: choose an unassigned variable and branch.
    let branch_var = choose_variable(&clauses_owned, assignment);
    let branch_var = match branch_var {
        Some(v) => v,
        None => {
            // All variables assigned — check if satisfied.
            return DpllResult::Sat;
        }
    };

    // Try branch_var = true.
    {
        let mut a = assignment.clone();
        let simplified = simplify_clauses(&clauses_owned, &branch_var, true);
        a.insert(branch_var.clone(), true);
        match dpll(&simplified, &mut a, budget) {
            DpllResult::Sat => {
                *assignment = a;
                return DpllResult::Sat;
            }
            DpllResult::Unknown(r) => return DpllResult::Unknown(r),
            DpllResult::Unsat => {} // try false branch
        }
    }

    // Try branch_var = false.
    {
        let mut a = assignment.clone();
        let simplified = simplify_clauses(&clauses_owned, &branch_var, false);
        a.insert(branch_var.clone(), false);
        match dpll(&simplified, &mut a, budget) {
            DpllResult::Sat => {
                *assignment = a;
                return DpllResult::Sat;
            }
            DpllResult::Unknown(r) => return DpllResult::Unknown(r),
            DpllResult::Unsat => {} // backtrack — both branches failed
        }
    }

    DpllResult::Unsat
}

/// Unit propagation: repeatedly find unit clauses and propagate.
///
/// Returns (simplified_clauses, forced_assignments).
fn unit_propagate(clauses: &[Clause]) -> (Vec<Clause>, HashMap<String, bool>) {
    let mut clauses = clauses.to_vec();
    let mut forced: HashMap<String, bool> = HashMap::new();

    loop {
        // Find the first unit clause.
        let unit = clauses.iter().find(|c| c.len() == 1).map(|c| c[0].clone());
        let lit = match unit {
            Some(l) => l,
            None => break,
        };

        // Force this literal.
        forced.insert(lit.var.clone(), lit.positive);
        clauses = simplify_clauses(&clauses, &lit.var, lit.positive);

        if clauses.iter().any(|c| c.is_empty()) {
            break; // Contradiction found — let dpll handle it.
        }
    }

    (clauses, forced)
}

/// Pure-literal elimination: find variables that appear in only one polarity.
fn find_pure_literals(clauses: &[Clause]) -> HashMap<String, bool> {
    let mut pos: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut neg: std::collections::HashSet<String> = std::collections::HashSet::new();

    for clause in clauses {
        for lit in clause {
            if lit.positive {
                pos.insert(lit.var.clone());
            } else {
                neg.insert(lit.var.clone());
            }
        }
    }

    let mut pure = HashMap::new();
    for var in &pos {
        if !neg.contains(var) {
            pure.insert(var.clone(), true);
        }
    }
    for var in &neg {
        if !pos.contains(var) {
            pure.insert(var.clone(), false);
        }
    }
    pure
}

/// Simplify `clauses` by setting `var = val`:
/// - Remove any clause that is satisfied by this literal.
/// - Remove the negated literal from all remaining clauses.
fn simplify_clauses(clauses: &[Clause], var: &str, val: bool) -> Vec<Clause> {
    let mut out = Vec::new();
    for clause in clauses {
        // Does this clause contain the satisfied literal?
        if clause.iter().any(|l| l.var == var && l.positive == val) {
            continue; // Clause satisfied — drop it.
        }
        // Remove the falsified literal from the clause.
        let reduced: Clause = clause
            .iter()
            .filter(|l| !(l.var == var && l.positive != val))
            .cloned()
            .collect();
        out.push(reduced);
    }
    out
}

/// Pick the first unassigned variable from the clauses.
fn choose_variable(
    clauses: &[Clause],
    assignment: &HashMap<String, bool>,
) -> Option<String> {
    for clause in clauses {
        for lit in clause {
            if !assignment.contains_key(&lit.var) {
                return Some(lit.var.clone());
            }
        }
    }
    None
}

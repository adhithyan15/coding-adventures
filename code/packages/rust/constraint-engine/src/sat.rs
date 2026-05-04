//! # SAT tactic — boolean satisfiability solver.
//!
//! Implements a **DPLL-based SAT solver** (Davis-Putnam-Logemann-Loveland,
//! 1962) with unit propagation and pure-literal elimination.  For v1 this
//! is sufficient — the boolean predicates that arise from LANG23 refinement
//! checking are small (O(10) variables in practice).
//!
//! ## DPLL — a literate overview
//!
//! DPLL is the classic backtracking procedure for boolean satisfiability.
//! It operates on a formula in **Conjunctive Normal Form** (CNF) — a
//! conjunction of clauses where each clause is a disjunction of literals.
//!
//! The algorithm:
//!
//! 1. **Unit propagation.**  If a clause has only one literal left (`x` or
//!    `¬x`), that literal *must* be true.  Assign it and simplify the
//!    formula.  Repeat until no unit clauses remain.
//!
//! 2. **Pure-literal elimination.**  If a variable appears in *only* one
//!    polarity (always positive, or always negative) across all remaining
//!    clauses, assign the value that satisfies all its occurrences.  Those
//!    clauses all become satisfied and can be dropped.
//!
//! 3. **Branching.**  Pick an unassigned variable (heuristic: first
//!    unassigned), try it as `true`, recurse.  If that leads to a
//!    contradiction, try `false`, recurse.  If both fail, backtrack.
//!
//! DPLL is complete for propositional formulae — it always finds a model
//! if one exists or proves UNSAT.  CDCL (Conflict-Driven Clause Learning)
//! extends DPLL with non-chronological backtracking and clause learning;
//! that is a planned future enhancement for formulae with >100 variables.
//!
//! ## Input format
//!
//! The tactic accepts raw [`Predicate`] values (not pre-CNF'd).  It first
//! converts to CNF using `constraint-core`'s `to_cnf()` method, then
//! extracts clauses as sets of literals.
//!
//! ## Scope and limitations
//!
//! - Correct for all propositional formulae (and/or/not/implies/iff).
//! - Pure boolean variables only.  If a predicate mixes Booleans with
//!   integers, the LIA tactic handles it.
//! - No timeout — rely on caller (`constraint-vm`) for time limits.

use std::collections::HashMap;

use constraint_core::Predicate;

use crate::{Model, SolverResult, Value};

// ---------------------------------------------------------------------------
// Literal type
// ---------------------------------------------------------------------------

/// A literal is a variable (positive or negative).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Literal {
    var: String,
    positive: bool,
}

impl Literal {
    fn pos(var: impl Into<String>) -> Self {
        Literal { var: var.into(), positive: true }
    }
    fn neg(var: impl Into<String>) -> Self {
        Literal { var: var.into(), positive: false }
    }
    #[allow(dead_code)] // Reserved for future CDCL conflict-driven learning
    fn negated(&self) -> Self {
        Literal { var: self.var.clone(), positive: !self.positive }
    }
}

/// A clause is a disjunction of literals.
type Clause = Vec<Literal>;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// The boolean SAT tactic.
///
/// Solves conjunctions of boolean predicates using DPLL.
pub struct SatTactic;

impl SatTactic {
    /// Solve the conjunction of `assertions` over `bool_vars`.
    pub fn solve(assertions: &[Predicate], bool_vars: &[String]) -> SolverResult {
        // Fast paths.
        if assertions.is_empty() {
            let mut model = Model::new();
            for v in bool_vars {
                model.insert(v.clone(), Value::Bool(false));
            }
            return SolverResult::Sat(model);
        }

        // Convert every assertion to CNF and collect clauses.
        let mut clauses: Vec<Clause> = Vec::new();
        for p in assertions {
            let cnf = p.clone().to_cnf();
            match collect_clauses(&cnf) {
                Ok(new_clauses) => clauses.extend(new_clauses),
                Err(reason) => return SolverResult::Unknown(reason),
            }
        }

        // DPLL with an initial empty assignment.
        let mut assignment: HashMap<String, bool> = HashMap::new();
        match dpll(&clauses, &mut assignment) {
            DpllResult::Sat => {
                let mut model = Model::new();
                for v in bool_vars {
                    let val = *assignment.get(v).unwrap_or(&false);
                    model.insert(v.clone(), Value::Bool(val));
                }
                SolverResult::Sat(model)
            }
            DpllResult::Unsat => SolverResult::Unsat,
        }
    }
}

// ---------------------------------------------------------------------------
// CNF → clause extraction
// ---------------------------------------------------------------------------

/// Collect clauses from a CNF predicate.
///
/// CNF is an `And` of `Or`s of literals.  This function walks that
/// structure and extracts `Vec<Clause>`.  Returns `Err(reason)` if the
/// predicate contains sub-expressions that can't be reduced to literals
/// (e.g. nested integer arithmetic — those belong to the LIA tactic).
fn collect_clauses(p: &Predicate) -> Result<Vec<Clause>, String> {
    match p {
        // Single literal (no And, no Or wrapper).
        Predicate::Var(name) => Ok(vec![vec![Literal::pos(name.clone())]]),
        Predicate::Not(inner) => {
            match inner.as_ref() {
                Predicate::Var(name) => Ok(vec![vec![Literal::neg(name.clone())]]),
                _ => Err(format!(
                    "Not({inner:?}) is not a CNF literal — the formula may not be boolean-only"
                )),
            }
        }
        Predicate::Bool(true) => Ok(vec![]), // trivially satisfied, no clause needed
        Predicate::Bool(false) => Ok(vec![vec![]]), // empty clause = contradiction
        Predicate::And(parts) => {
            let mut out = Vec::new();
            for p in parts {
                out.extend(collect_clauses(p)?);
            }
            Ok(out)
        }
        Predicate::Or(parts) => {
            let mut clause: Clause = Vec::new();
            for p in parts {
                match p {
                    Predicate::Var(name) => clause.push(Literal::pos(name.clone())),
                    Predicate::Not(inner) => match inner.as_ref() {
                        Predicate::Var(name) => clause.push(Literal::neg(name.clone())),
                        other => {
                            return Err(format!(
                                "Not({other:?}) inside Or is not a CNF literal"
                            ));
                        }
                    },
                    Predicate::Bool(true) => return Ok(vec![]), // clause trivially true
                    Predicate::Bool(false) => { /* false literal — skip */ }
                    other => {
                        return Err(format!(
                            "non-literal {other:?} inside Or — formula not in CNF"
                        ));
                    }
                }
            }
            Ok(vec![clause])
        }
        other => Err(format!("predicate {other:?} is not boolean-CNF")),
    }
}

// ---------------------------------------------------------------------------
// DPLL
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum DpllResult {
    Sat,
    Unsat,
}

/// Core DPLL procedure.
///
/// `clauses` is the current (possibly simplified) formula; `assignment`
/// accumulates committed assignments.
fn dpll(clauses: &[Clause], assignment: &mut HashMap<String, bool>) -> DpllResult {
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
        if dpll(&simplified, &mut a) == DpllResult::Sat {
            *assignment = a;
            return DpllResult::Sat;
        }
    }

    // Try branch_var = false.
    {
        let mut a = assignment.clone();
        let simplified = simplify_clauses(&clauses_owned, &branch_var, false);
        a.insert(branch_var.clone(), false);
        if dpll(&simplified, &mut a) == DpllResult::Sat {
            *assignment = a;
            return DpllResult::Sat;
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use constraint_core::Predicate;

    use crate::eval_bool;
    use super::*;

    fn vb(name: &str) -> Predicate {
        Predicate::Var(name.into())
    }

    fn vars(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    // ---------- trivial ----------

    #[test]
    fn empty_is_sat() {
        let r = SatTactic::solve(&[], &vars(&["b"]));
        assert!(r.is_sat());
    }

    #[test]
    fn true_literal_is_sat() {
        let r = SatTactic::solve(&[Predicate::Bool(true)], &vars(&["b"]));
        assert!(r.is_sat());
    }

    #[test]
    fn false_literal_is_unsat() {
        let r = SatTactic::solve(&[Predicate::Bool(false)], &vars(&["b"]));
        assert!(r.is_unsat());
    }

    // ---------- single variable ----------

    #[test]
    fn single_var_positive_is_sat() {
        let r = SatTactic::solve(&[vb("a")], &vars(&["a"]));
        assert!(r.is_sat());
        assert_eq!(r.model().unwrap().get("a").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn single_var_negated_is_sat() {
        let r = SatTactic::solve(&[Predicate::Not(Box::new(vb("a")))], &vars(&["a"]));
        assert!(r.is_sat());
        assert_eq!(r.model().unwrap().get("a").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn var_and_negation_is_unsat() {
        let r = SatTactic::solve(
            &[vb("a"), Predicate::Not(Box::new(vb("a")))],
            &vars(&["a"]),
        );
        assert!(r.is_unsat());
    }

    // ---------- two variables ----------

    #[test]
    fn xor_is_sat() {
        // (a ∨ b) ∧ (¬a ∨ ¬b)  →  XOR
        let a = vb("a");
        let b = vb("b");
        let na = Predicate::Not(Box::new(vb("a")));
        let nb = Predicate::Not(Box::new(vb("b")));
        let r = SatTactic::solve(
            &[Predicate::or(vec![a, b]), Predicate::or(vec![na, nb])],
            &vars(&["a", "b"]),
        );
        assert!(r.is_sat());
        let m = r.model().unwrap();
        let a_val = m.get("a").unwrap().as_bool().unwrap();
        let b_val = m.get("b").unwrap().as_bool().unwrap();
        // Must satisfy both clauses.
        assert!(a_val || b_val);
        assert!(!a_val || !b_val);
    }

    #[test]
    fn implies_is_sat() {
        // a ⇒ b — satisfiable (e.g. a=false, b=anything)
        let r = SatTactic::solve(
            &[Predicate::Implies(Box::new(vb("a")), Box::new(vb("b")))],
            &vars(&["a", "b"]),
        );
        assert!(r.is_sat());
    }

    #[test]
    fn contradiction_three_vars() {
        // a ∧ b ∧ ¬(a ∨ b)
        let a = vb("a");
        let b = vb("b");
        let r = SatTactic::solve(
            &[
                a.clone(),
                b.clone(),
                Predicate::Not(Box::new(Predicate::or(vec![a, b]))),
            ],
            &vars(&["a", "b"]),
        );
        assert!(r.is_unsat());
    }

    // ---------- model correctness ----------

    #[test]
    fn model_satisfies_assertions() {
        // (a ∨ b) ∧ (a ∨ ¬b) — forces a=true
        let r = SatTactic::solve(
            &[
                Predicate::or(vec![vb("a"), vb("b")]),
                Predicate::or(vec![vb("a"), Predicate::Not(Box::new(vb("b")))]),
            ],
            &vars(&["a", "b"]),
        );
        assert!(r.is_sat());
        let m = r.model().unwrap();
        // Verify model satisfies both clauses.
        let a_val = m.get("a").unwrap().as_bool().unwrap();
        let b_val = m.get("b").unwrap().as_bool().unwrap();
        assert!(a_val || b_val);
        assert!(a_val || !b_val);
    }

    // ---------- literal extraction ----------

    #[test]
    fn collect_clauses_single_literal() {
        let clauses = collect_clauses(&vb("x")).unwrap();
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0], vec![Literal::pos("x")]);
    }

    #[test]
    fn collect_clauses_negated_literal() {
        let clauses = collect_clauses(&Predicate::Not(Box::new(vb("x")))).unwrap();
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0], vec![Literal::neg("x")]);
    }

    #[test]
    fn collect_clauses_and_of_lits() {
        let p = Predicate::and(vec![vb("a"), vb("b")]);
        let clauses = collect_clauses(&p).unwrap();
        assert_eq!(clauses.len(), 2);
    }

    #[test]
    fn collect_clauses_or_of_lits() {
        let p = Predicate::or(vec![vb("a"), Predicate::Not(Box::new(vb("b")))]);
        let clauses = collect_clauses(&p).unwrap();
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].len(), 2);
    }

    // ---------- eval_bool integration ----------

    #[test]
    fn eval_bool_through_sat_model() {
        let assertions = &[
            Predicate::or(vec![vb("p"), vb("q")]),
            Predicate::Not(Box::new(vb("p"))),
        ];
        let r = SatTactic::solve(assertions, &vars(&["p", "q"]));
        assert!(r.is_sat());
        let m = r.model().unwrap();
        // All assertions must hold under the returned model.
        for a in assertions {
            assert!(eval_bool(a, m), "model violated {a:?}");
        }
    }
}

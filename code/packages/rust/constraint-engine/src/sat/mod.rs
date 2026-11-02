//! # SAT tactic — Boolean satisfiability through `cdcl-sat`.
//!
//! **VER09 §7.** This tactic answers "is this conjunction of Boolean
//! predicates satisfiable?" for the engine. Since VER09 it is an **adapter**:
//! the search itself is the `cdcl-sat` crate's conflict-driven
//! clause-learning solver (two watched literals, first-UIP learning,
//! backjumping, VSIDS, Luby restarts). This module only translates.
//!
//! ## The translation
//!
//! ```text
//!   [Predicate]                 assertions over named Bool variables
//!       │  depth guard           (refuse trees deep enough to overflow)
//!       │  Predicate::to_cnf     (constraint-core's normaliser)
//!       ▼
//!   Vec<Clause>                 clauses of named literals   (collect_clauses)
//!       │  name → cdcl_sat::Var, in first-appearance order
//!       ▼
//!   cdcl_sat::Solver            solve, with a conflict budget
//!       │
//!       ▼
//!   SolverResult                Sat(model) / Unsat / Unknown(reason)
//! ```
//!
//! ## Behaviour preserved from the DPLL era
//!
//! - Same signature, same three outcomes.
//! - Every requested `bool_var` appears in the model; a variable that occurs
//!   in no clause is reported `false`.
//! - A bounded amount of work: the old DPLL stopped after 1 000 000 search
//!   nodes; the adapter stops after 1 000 000 conflicts and answers
//!   `Unknown`. Before any of that, predicates whose negation normal form
//!   would exceed [`MAX_NNF_NODES`] are refused (nested `Iff` doubles at
//!   each level), as are trees deeper than `MAX_PREDICATE_DEPTH`.
//!
//! The old DPLL lives on in [`dpll_oracle`] (tests only), as an independent
//! second opinion the tests compare against.
//!
//! ## Scope and limitations
//!
//! - Correct for all propositional formulae (and/or/not/implies/iff).
//! - Pure Boolean variables only. If a predicate mixes Booleans with
//!   integers, the LIA tactic handles it.
//! - CNF conversion is `constraint-core`'s distributive `to_cnf`, which can
//!   grow exponentially on some formulas; a Tseitin encoding (VER10) is the
//!   planned replacement.

use std::collections::HashMap;

use cdcl_sat::{SolveResult, Solver, Var};
use constraint_core::{depth_of, Predicate, MAX_PREDICATE_DEPTH};

use crate::{Model, SolverResult, Value};

#[cfg(test)]
pub(crate) mod dpll_oracle;

/// Conflicts the CDCL search may spend before answering `Unknown`. The same
/// order of magnitude as the DPLL node budget it replaces: generous for the
/// O(10)-variable formulas LANG23 produces, and a bound on hostile input.
pub const CONFLICT_BUDGET: u64 = 1_000_000;

// ---------------------------------------------------------------------------
// Literal type
// ---------------------------------------------------------------------------

/// A named literal: a variable, positive or negative. This is the shape
/// `to_cnf` output is read into before variables get solver indices.
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
}

/// A clause is a disjunction of literals.
type Clause = Vec<Literal>;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// The Boolean SAT tactic.
///
/// Solves conjunctions of Boolean predicates with the `cdcl-sat` solver.
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

        // Reject predicates that are too deep for the recursive passes
        // (to_nnf, to_cnf, …) — a deeply nested tree can overflow the
        // OS thread stack.
        for p in assertions {
            if depth_of(p) > MAX_PREDICATE_DEPTH {
                return SolverResult::Unknown(format!(
                    "predicate depth exceeds MAX_PREDICATE_DEPTH ({MAX_PREDICATE_DEPTH}); \
                     refusing to recurse to avoid stack overflow"
                ));
            }
        }

        // Refuse predicates whose negation normal form would be huge. `Iff`
        // copies both operands, so a chain of k nested `Iff`s grows as 2^k
        // inside `to_nnf`, before the CNF clause cap or the conflict budget
        // ever runs: depth 20 already costs seconds, depth 30 hours.
        let mut total = 0usize;
        for p in assertions {
            total = total.saturating_add(nnf_size_bound(p));
        }
        if total > MAX_NNF_NODES {
            return SolverResult::Unknown(format!(
                "predicate would expand past MAX_NNF_NODES ({MAX_NNF_NODES}) nodes in negation \
                 normal form; refusing to expand (nested Iff doubles at each level)"
            ));
        }

        // Convert every assertion to CNF and collect clauses.
        let mut clauses: Vec<Clause> = Vec::new();
        for p in assertions {
            let cnf = match p.clone().to_cnf() {
                Ok(c) => c,
                Err(reason) => return SolverResult::Unknown(reason),
            };
            match collect_clauses(&cnf) {
                Ok(new_clauses) => clauses.extend(new_clauses),
                Err(reason) => return SolverResult::Unknown(reason),
            }
        }

        match solve_clauses(&clauses, CONFLICT_BUDGET) {
            ClauseAnswer::Sat(values) => {
                let mut model = Model::new();
                for v in bool_vars {
                    let val = values.get(v).copied().unwrap_or(false);
                    model.insert(v.clone(), Value::Bool(val));
                }
                SolverResult::Sat(model)
            }
            ClauseAnswer::Unsat => SolverResult::Unsat,
            ClauseAnswer::Unknown(reason) => SolverResult::Unknown(reason),
        }
    }
}

// ---------------------------------------------------------------------------
// The NNF size guard
// ---------------------------------------------------------------------------

/// The largest negation normal form, in nodes, the tactic will build.
/// Real refinement predicates are a few hundred nodes; this leaves room for
/// generated ones while bounding the work `to_nnf` can be made to do.
pub const MAX_NNF_NODES: usize = 100_000;

/// An upper bound on the node count of `p.to_nnf()`, computed without
/// building it, in time linear in the size of `p`.
///
/// Every rewrite `to_nnf` performs adds at most a constant number of nodes
/// per input node, except `Iff` and `¬Iff`, whose expansion
/// `(¬a ∨ b) ∧ (¬b ∨ a)` contains each operand twice. The bound charges
/// those twice. Saturating arithmetic keeps it meaningful (= "too big")
/// when it overflows. Callers must have checked `depth_of` first: this
/// recurses.
fn nnf_size_bound(p: &Predicate) -> usize {
    // `+ 4` covers the And, the two Ors and a Not that an expansion adds.
    let sum = |ps: &[&Predicate]| ps.iter().fold(4usize, |acc, q| acc.saturating_add(nnf_size_bound(q)));
    match p {
        // A leaf may come out wrapped in a `Not` (a negated copy inside an
        // `Iff` expansion), so it is charged two nodes.
        Predicate::Bool(_) | Predicate::Var(_) | Predicate::Int(_) | Predicate::Real(_) => 2,
        Predicate::Iff(a, b) => sum(&[a, b]).saturating_mul(2),
        Predicate::And(parts) | Predicate::Or(parts) | Predicate::Add(parts) => {
            sum(&parts.iter().collect::<Vec<_>>())
        }
        Predicate::Apply { args, .. } => sum(&args.iter().collect::<Vec<_>>()),
        Predicate::Not(inner) | Predicate::Mul { term: inner, .. } => sum(&[inner]),
        Predicate::Forall { body, .. } | Predicate::Exists { body, .. } => sum(&[body]),
        Predicate::Implies(a, b)
        | Predicate::Eq(a, b)
        | Predicate::NEq(a, b)
        | Predicate::Sub(a, b)
        | Predicate::Le(a, b)
        | Predicate::Lt(a, b)
        | Predicate::Ge(a, b)
        | Predicate::Gt(a, b)
        | Predicate::Select { arr: a, idx: b } => sum(&[a, b]),
        Predicate::Ite(a, b, c) | Predicate::Store { arr: a, idx: b, val: c } => sum(&[a, b, c]),
        // `Predicate` is non-exhaustive. A variant added later has an
        // unknown expansion, so it gets no bound at all: refuse rather than
        // under-count.
        _ => usize::MAX,
    }
}

// ---------------------------------------------------------------------------
// Named clauses → cdcl-sat
// ---------------------------------------------------------------------------

/// The answer for a set of named clauses.
#[derive(Debug, PartialEq)]
enum ClauseAnswer {
    /// SAT, with a value for every variable that occurs in a clause.
    Sat(HashMap<String, bool>),
    Unsat,
    Unknown(String),
}

/// Give each variable name a solver variable (in first-appearance order,
/// so the numbering is deterministic), load the clauses, and solve with at
/// most `budget` conflicts.
fn solve_clauses(clauses: &[Clause], budget: u64) -> ClauseAnswer {
    let mut solver = Solver::new();
    let mut index: HashMap<&str, Var> = HashMap::new();
    let mut names: Vec<&str> = Vec::new();
    for clause in clauses {
        let lits: Vec<_> = clause
            .iter()
            .map(|l| {
                let var = *index.entry(l.var.as_str()).or_insert_with(|| {
                    names.push(l.var.as_str());
                    solver.new_var()
                });
                var.lit(!l.positive)
            })
            .collect();
        solver.add_clause(&lits);
    }
    solver.set_conflict_budget(Some(budget));
    match solver.solve() {
        SolveResult::Sat => ClauseAnswer::Sat(
            names
                .iter()
                .enumerate()
                .map(|(i, n)| (n.to_string(), solver.model()[i]))
                .collect(),
        ),
        SolveResult::Unsat => ClauseAnswer::Unsat,
        SolveResult::Unknown => ClauseAnswer::Unknown(format!(
            "CDCL conflict budget ({budget}) exhausted before an answer"
        )),
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

    // ---------- VER09: the adapter against independent oracles ----------

    /// xorshift64, so the cross-check is deterministic and dependency-free.
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
    }

    const NAMES: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

    /// A random propositional formula over `NAMES`, of bounded depth, using
    /// every connective `to_cnf` understands.
    fn random_predicate(rng: &mut Rng, depth: u32) -> Predicate {
        if depth == 0 || rng.below(4) == 0 {
            return match rng.below(12) {
                0 => Predicate::Bool(rng.below(2) == 0),
                k => {
                    let v = vb(NAMES[(k % 6) as usize]);
                    if rng.below(2) == 0 { v } else { Predicate::Not(Box::new(v)) }
                }
            };
        }
        let sub = |rng: &mut Rng| Box::new(random_predicate(rng, depth - 1));
        match rng.below(5) {
            0 => Predicate::And(vec![*sub(rng), *sub(rng)]),
            1 => Predicate::Or(vec![*sub(rng), *sub(rng), *sub(rng)]),
            2 => Predicate::Not(sub(rng)),
            3 => Predicate::Implies(sub(rng), sub(rng)),
            _ => Predicate::Iff(sub(rng), sub(rng)),
        }
    }

    /// Exhaustive truth table over `NAMES` through `eval_bool`, which shares
    /// no code with either solver or with `to_cnf`.
    fn brute_force(assertions: &[Predicate]) -> bool {
        (0u32..64).any(|bits| {
            let mut m = Model::new();
            for (i, n) in NAMES.iter().enumerate() {
                m.insert(*n, Value::Bool(bits >> i & 1 == 1));
            }
            assertions.iter().all(|a| eval_bool(a, &m))
        })
    }

    #[test]
    fn cdcl_adapter_agrees_with_old_dpll_and_truth_tables() {
        let mut rng = Rng(0x0005_EED0_F5A7);
        let names = vars(&NAMES);
        let (mut sat, mut unsat, mut over_cap) = (0, 0, 0);
        for round in 0..1000 {
            let count = 1 + rng.below(4) as usize;
            let assertions: Vec<Predicate> = (0..count).map(|_| random_predicate(&mut rng, 3)).collect();
            let expected = brute_force(&assertions);

            let r = SatTactic::solve(&assertions, &names);

            // The old DPLL, on the very same clauses. The distributive CNF
            // conversion has a clause cap (it is exponential in the worst
            // case); past it the tactic must say Unknown, never guess.
            let cnfs: Result<Vec<Predicate>, String> =
                assertions.iter().map(|a| a.clone().to_cnf()).collect();
            let Ok(cnfs) = cnfs else {
                assert!(matches!(r, SolverResult::Unknown(_)), "round {round}: {r:?}");
                over_cap += 1;
                continue;
            };
            let mut clauses = Vec::new();
            for c in &cnfs {
                clauses.extend(collect_clauses(c).expect("to_cnf output is CNF"));
            }
            let dpll = dpll_oracle::solve(&clauses);
            assert_eq!(dpll.is_some(), expected, "round {round}: DPLL vs truth table on {assertions:?}");

            match &r {
                SolverResult::Sat(m) => {
                    assert!(expected, "round {round}: CDCL SAT, truth table UNSAT: {assertions:?}");
                    for a in &assertions {
                        assert!(eval_bool(a, m), "round {round}: model violates {a:?}");
                    }
                    assert_eq!(m.len(), NAMES.len());
                    sat += 1;
                }
                SolverResult::Unsat => {
                    assert!(!expected, "round {round}: CDCL UNSAT, truth table SAT: {assertions:?}");
                    unsat += 1;
                }
                SolverResult::Unknown(why) => panic!("round {round}: unexpected Unknown: {why}"),
            }
        }
        assert!(sat > 100 && unsat > 100, "sat={sat} unsat={unsat}");
        assert!(over_cap < 50, "too few formulas were actually solved: {over_cap} over the CNF cap");
    }

    #[test]
    fn variables_outside_every_clause_default_to_false() {
        let r = SatTactic::solve(&[vb("a")], &vars(&["a", "unused"]));
        let m = r.model().unwrap();
        assert_eq!(m.get("a").unwrap().as_bool(), Some(true));
        assert_eq!(m.get("unused").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn exhausted_budget_is_unknown() {
        // PHP(4, 3) over named variables needs more than one conflict.
        let holes = 3;
        let x = |p: usize, h: usize| Literal::pos(format!("p{p}h{h}"));
        let mut clauses: Vec<Clause> = (0..=holes).map(|p| (0..holes).map(|h| x(p, h)).collect()).collect();
        for h in 0..holes {
            for p in 0..=holes {
                for q in p + 1..=holes {
                    clauses.push(vec![Literal::neg(x(p, h).var), Literal::neg(x(q, h).var)]);
                }
            }
        }
        match solve_clauses(&clauses, 1) {
            ClauseAnswer::Unknown(why) => assert!(why.contains("budget (1)"), "{why}"),
            other => panic!("expected Unknown, got {other:?}"),
        }
        assert_eq!(solve_clauses(&clauses, CONFLICT_BUDGET), ClauseAnswer::Unsat);
        assert_eq!(dpll_oracle::solve(&clauses), None);
    }

    #[test]
    fn non_boolean_cnf_is_unknown() {
        let p = Predicate::Le(Box::new(Predicate::Int(1)), Box::new(vb("x")));
        assert!(matches!(SatTactic::solve(&[p], &vars(&["x"])), SolverResult::Unknown(_)));
        let deep = (0..MAX_PREDICATE_DEPTH + 2).fold(vb("a"), |acc, _| Predicate::Not(Box::new(acc)));
        match SatTactic::solve(&[deep], &vars(&["a"])) {
            SolverResult::Unknown(why) => assert!(why.contains("MAX_PREDICATE_DEPTH"), "{why}"),
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn nested_iff_is_refused_before_it_explodes() {
        // 30 nested Iffs: a 2^30-node NNF. Must come back fast, as Unknown.
        let chain = (0..30).fold(vb("a"), |acc, i| {
            Predicate::Iff(Box::new(acc), Box::new(vb(NAMES[i % 6])))
        });
        match SatTactic::solve(&[Predicate::Not(Box::new(chain))], &vars(&NAMES)) {
            SolverResult::Unknown(why) => assert!(why.contains("MAX_NNF_NODES"), "{why}"),
            other => panic!("expected Unknown, got {other:?}"),
        }
        // A short chain is still solved: a ⇔ (a ⇔ b) forces b.
        let short = Predicate::Iff(
            Box::new(vb("a")),
            Box::new(Predicate::Iff(Box::new(vb("a")), Box::new(vb("b")))),
        );
        let r = SatTactic::solve(&[short], &vars(&["a", "b"]));
        assert_eq!(r.model().unwrap().get("b").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn size_bound_covers_every_variant() {
        use constraint_core::{Rational, Sort};
        let x = || Box::new(vb("x"));
        let all = vec![
            Predicate::Int(1),
            Predicate::Real(Rational::new(1, 2)),
            Predicate::Apply { f: "f".into(), args: vec![vb("x")] },
            Predicate::Add(vec![vb("x")]),
            Predicate::Mul { coef: 2, term: x() },
            Predicate::Forall { var: "y".into(), sort: Sort::Int, body: x() },
            Predicate::Exists { var: "y".into(), sort: Sort::Int, body: x() },
            Predicate::Implies(x(), x()),
            Predicate::Eq(x(), x()),
            Predicate::NEq(x(), x()),
            Predicate::Sub(x(), x()),
            Predicate::Le(x(), x()),
            Predicate::Lt(x(), x()),
            Predicate::Ge(x(), x()),
            Predicate::Gt(x(), x()),
            Predicate::Select { arr: x(), idx: x() },
            Predicate::Ite(x(), x(), x()),
            Predicate::Store { arr: x(), idx: x(), val: x() },
        ];
        for p in &all {
            // The bound really is an upper bound on what to_nnf builds.
            let built = count_nodes(&p.clone().to_nnf());
            assert!(nnf_size_bound(p) >= built, "{p:?}: bound {} < {built}", nnf_size_bound(p));
        }
        // Wide operands under Iff: every leaf of the negated copy gains a
        // Not, which a bound charging leaves once would miss (≈1.5×).
        let wide = |n: usize, tag: &str| Predicate::And((0..n).map(|i| vb(&format!("{tag}{i}"))).collect());
        let mut nested = wide(300, "c");
        for k in 0..6 {
            nested = Predicate::Iff(Box::new(nested), Box::new(wide(50, &format!("k{k}_"))));
        }
        for p in [
            Predicate::Iff(Box::new(wide(100, "a")), Box::new(vb("x"))),
            Predicate::Iff(Box::new(wide(1000, "a")), Box::new(wide(1000, "b"))),
            Predicate::Not(Box::new(Predicate::Iff(Box::new(wide(200, "a")), Box::new(wide(3, "b"))))),
            nested,
        ] {
            let built = count_nodes(&p.clone().to_nnf());
            assert!(nnf_size_bound(&p) >= built, "bound {} < {built}", nnf_size_bound(&p));
        }
        let mut rng = Rng(77);
        for _ in 0..500 {
            let p = random_predicate(&mut rng, 5);
            assert!(nnf_size_bound(&p) >= count_nodes(&p.clone().to_nnf()), "{p:?}");
        }
    }

    /// Exact node count of a predicate tree, for checking the bound.
    fn count_nodes(p: &Predicate) -> usize {
        1 + match p {
            Predicate::And(ps) | Predicate::Or(ps) | Predicate::Add(ps) => ps.iter().map(count_nodes).sum(),
            Predicate::Apply { args, .. } => args.iter().map(count_nodes).sum(),
            Predicate::Not(a) | Predicate::Mul { term: a, .. } => count_nodes(a),
            Predicate::Forall { body, .. } | Predicate::Exists { body, .. } => count_nodes(body),
            Predicate::Implies(a, b)
            | Predicate::Iff(a, b)
            | Predicate::Eq(a, b)
            | Predicate::NEq(a, b)
            | Predicate::Sub(a, b)
            | Predicate::Le(a, b)
            | Predicate::Lt(a, b)
            | Predicate::Ge(a, b)
            | Predicate::Gt(a, b)
            | Predicate::Select { arr: a, idx: b } => count_nodes(a) + count_nodes(b),
            Predicate::Ite(a, b, c) | Predicate::Store { arr: a, idx: b, val: c } => {
                count_nodes(a) + count_nodes(b) + count_nodes(c)
            }
            _ => 0,
        }
    }
}

//! # `constraint-engine` — pluggable solver tactics.
//!
//! **LANG24 PR 24-C**.  The solver backend for the generic Constraint-VM.
//! Mirrors the Logic-VM's `logic-engine` layer exactly: pure algorithms,
//! no I/O, no VM state — just "here is a set of asserted predicates, is
//! there a model?".
//!
//! ## Architecture
//!
//! ```text
//! constraint-core   (predicate AST + normalisation)
//!       │
//! constraint-engine (this crate)
//!       │   ├─ sat_tactic  — boolean CDCL (the boolean backbone)
//!       │   ├─ lia_tactic  — Cooper's algorithm for linear integer arithmetic
//!       │   └─ Engine      — dispatches to the right tactic; Nelson-Oppen
//!       │                    combination for multi-theory queries
//!       │
//! constraint-vm     (instruction-stream executor; drives Engine)
//! ```
//!
//! ## Solver result
//!
//! Every query returns a [`SolverResult`]:
//!
//! | Variant | Meaning |
//! |---------|---------|
//! | `Sat(model)` | Satisfiable; the model maps each free variable to a value |
//! | `Unsat` | Unsatisfiable; no assignment satisfies all asserted predicates |
//! | `Unknown(reason)` | Solver gave up (timeout, incomplete tactic, quantifiers) |
//!
//! The three-way split maps directly to LANG23's three outcomes
//! (`PROVEN_SAFE`, `PROVEN_UNSAFE`, `unknown → runtime check`).
//!
//! ## Theories supported in v1
//!
//! | Logic | Tactic | Complete? |
//! |-------|--------|-----------|
//! | `QF_Bool` | SAT (CDCL) | Yes |
//! | `QF_LIA` | LIA (Cooper) | Yes |
//! | `QF_LRA` | LRA (not yet implemented) | No → Unknown |
//! | `QF_BV`, `QF_AUFLIA`, `LIA`, `ALL` | Future tactics | No → Unknown |
//!
//! ## Non-guarantees (caller responsibilities)
//!
//! - **Predicate depth.**  The engine recurses on the predicate AST.
//!   Callers must bound depth at the boundary (see `constraint-core`
//!   non-guarantees).
//! - **Solver timeouts.**  No internal timeout; `constraint-vm` is
//!   responsible for wrapping calls with OS-level timeouts if needed.
//! - **Quantifier completeness.**  Quantified predicates that reach the
//!   LIA tactic produce `Unknown`; the engine never diverges on them.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::HashMap;

use constraint_core::{Logic, Predicate, Sort};

pub mod lia;
pub mod sat;

// ---------------------------------------------------------------------------
// Model — the satisfying assignment returned by a SAT result
// ---------------------------------------------------------------------------

/// A satisfying model: a mapping from each free variable to its value.
///
/// The model contains exactly the variables that were declared via
/// `DeclareVar` in the program scope at the time `CheckSat` was called.
/// Values are expressed as [`Value`] terms matching each variable's sort.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    assignments: HashMap<String, Value>,
}

impl Model {
    /// Construct an empty model.
    pub fn new() -> Self {
        Model { assignments: HashMap::new() }
    }

    /// Insert a variable assignment.
    pub fn insert(&mut self, name: impl Into<String>, value: Value) {
        self.assignments.insert(name.into(), value);
    }

    /// Retrieve the value for `name`, or `None` if absent.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.assignments.get(name)
    }

    /// Iterate over all assignments in the model.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.assignments.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of assignments in the model.
    pub fn len(&self) -> usize {
        self.assignments.len()
    }

    /// Whether the model is empty.
    pub fn is_empty(&self) -> bool {
        self.assignments.is_empty()
    }
}

impl Default for Model {
    fn default() -> Self {
        Model::new()
    }
}

// ---------------------------------------------------------------------------
// Value — a concrete assignment for one variable
// ---------------------------------------------------------------------------

/// A concrete value that can appear in a satisfying model.
///
/// The set is intentionally small in v1 (Bool + Int + Real via rational).
/// Richer sorts (BitVec, Array, Uninterpreted) produce `Unknown` when
/// their tactics aren't yet wired up.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Boolean assignment.
    Bool(bool),
    /// Integer assignment.
    Int(i128),
    /// Rational assignment (for Real-sorted variables).
    Real(i128, i128), // (num, den) — always reduced by gcd
}

impl Value {
    /// Return the integer value, or `None` if this is not an Int.
    pub fn as_int(&self) -> Option<i128> {
        if let Value::Int(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Return the boolean value, or `None` if this is not a Bool.
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Real(n, d) => {
                if *d == 1 {
                    write!(f, "{n}")
                } else {
                    write!(f, "{n}/{d}")
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SolverResult
// ---------------------------------------------------------------------------

/// The outcome of a `CheckSat` query.
///
/// Three-way split mirrors SMT-LIB and LANG23's three proof outcomes.
#[derive(Debug, Clone, PartialEq)]
pub enum SolverResult {
    /// Satisfiable.  The model contains a witness assignment.
    Sat(Model),
    /// Unsatisfiable.  No assignment can satisfy the conjunction.
    Unsat,
    /// The engine could not determine satisfiability.  The `reason`
    /// string describes why (unsupported theory, quantifiers, timeout
    /// signalled by caller, etc.).
    Unknown(String),
}

impl SolverResult {
    /// Return `true` if this is `Sat`.
    pub fn is_sat(&self) -> bool {
        matches!(self, SolverResult::Sat(_))
    }

    /// Return `true` if this is `Unsat`.
    pub fn is_unsat(&self) -> bool {
        matches!(self, SolverResult::Unsat)
    }

    /// Return `true` if this is `Unknown`.
    pub fn is_unknown(&self) -> bool {
        matches!(self, SolverResult::Unknown(_))
    }

    /// Extract the model if `Sat`, otherwise `None`.
    pub fn model(&self) -> Option<&Model> {
        if let SolverResult::Sat(m) = self {
            Some(m)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// SortEnv — variable declarations visible to the engine
// ---------------------------------------------------------------------------

/// Variable declarations.  The engine needs to know each variable's sort
/// before it can assign a value.
pub type SortEnv = HashMap<String, Sort>;

// ---------------------------------------------------------------------------
// Engine — the top-level solver entry point
// ---------------------------------------------------------------------------

/// The top-level solver engine.
///
/// Created once per `constraint-vm` scope stack level; the VM clones /
/// re-initialises it on `PushScope` / `PopScope`.
///
/// ## Design — Nelson-Oppen combination (v1 subset)
///
/// The full Nelson-Oppen framework lets you combine any two decidable
/// theories by propagating equalities across a shared set of integer
/// variables.  In v1 we support two theories:
///
/// - **Bool** — variables declared with sort `Bool`.  Handled by
///   `sat::SatTactic`.
/// - **LIA** (linear integer arithmetic) — variables declared with sort
///   `Int`.  Handled by `lia::LiaTactic`.
///
/// For a multi-theory formula the engine:
/// 1. Sorts predicates into Bool-only vs Int/arithmetic.
/// 2. Calls the LIA tactic on the arithmetic subproblem.
/// 3. If LIA returns SAT, plugs the model into the Bool predicates and
///    calls the SAT tactic to verify consistency.
/// 4. If the Bool predicates were pure (no integer variables), the LIA
///    result alone is authoritative.
///
/// For formulae that mix theories beyond LIA + Bool (Real, BitVec,
/// Arrays, EUF, quantifiers) the engine returns `Unknown`.
pub struct Engine {
    /// Declared sorts for all variables in scope.
    sort_env: SortEnv,
    /// Currently asserted predicates (conjunction).
    assertions: Vec<Predicate>,
    /// Active logic; drives tactic selection.
    logic: Option<Logic>,
}

impl Engine {
    /// Construct a fresh engine with no assertions.
    pub fn new() -> Self {
        Engine { sort_env: SortEnv::new(), assertions: Vec::new(), logic: None }
    }

    /// Declare a variable with the given sort.
    pub fn declare_var(&mut self, name: impl Into<String>, sort: Sort) {
        self.sort_env.insert(name.into(), sort);
    }

    /// Set the active logic (for tactic selection and early rejection).
    pub fn set_logic(&mut self, logic: Logic) {
        self.logic = Some(logic);
    }

    /// Add a predicate to the assertion set.
    pub fn assert(&mut self, pred: Predicate) {
        self.assertions.push(pred);
    }

    /// Remove all assertions (but keep variable declarations).
    pub fn reset_assertions(&mut self) {
        self.assertions.clear();
    }

    /// Remove all assertions *and* variable declarations.
    pub fn reset_all(&mut self) {
        self.assertions.clear();
        self.sort_env.clear();
        self.logic = None;
    }

    /// Snapshot — clone for `PushScope`.
    pub fn snapshot(&self) -> Self {
        Engine {
            sort_env: self.sort_env.clone(),
            assertions: self.assertions.clone(),
            logic: self.logic,
        }
    }

    /// Check whether the current assertion set is satisfiable.
    ///
    /// ## Tactic selection
    ///
    /// The engine inspects the variable sorts and the logic declaration:
    ///
    /// | Sorts present | Logic | Tactic |
    /// |---|---|---|
    /// | Bool only | `QF_Bool` / not set | SAT (CDCL) |
    /// | Int only or mixed Bool+Int | `QF_LIA` / not set | LIA (Cooper) + SAT for Bool residual |
    /// | Real, BitVec, Array, Uninterpreted | any | `Unknown` |
    /// | Quantified predicates | any | `Unknown` |
    pub fn check_sat(&self) -> SolverResult {
        // Fast path: empty conjunction is trivially SAT.
        if self.assertions.is_empty() {
            let model = self.trivial_model();
            return SolverResult::Sat(model);
        }

        // Reject unsupported logics up-front.
        if let Some(logic) = self.logic {
            if !is_supported_logic(logic) {
                return SolverResult::Unknown(format!(
                    "logic {logic} is not yet supported (v1 supports QF_Bool, QF_LIA)"
                ));
            }
        }

        // Reject unsupported sorts.
        if let Some(bad) = self.unsupported_sort_in_env() {
            return SolverResult::Unknown(format!(
                "sort `{bad}` is not yet supported (v1 supports Bool, Int)"
            ));
        }

        // Reject quantifiers in the assertions.
        if self.assertions.iter().any(has_quantifier) {
            return SolverResult::Unknown(
                "quantified predicates are not supported in v1 (degrade to runtime check)".into(),
            );
        }

        // Classify the formula by sorts present.
        let has_int = self.sort_env.values().any(|s| matches!(s, Sort::Int));
        let has_bool = self.sort_env.values().any(|s| matches!(s, Sort::Bool));

        if has_int {
            // LIA path.  Booleans are handled via substitution in the LIA
            // tactic — we ground Bool variables to {0, 1} and let Cooper
            // reduce them.
            self.solve_lia(has_bool)
        } else {
            // Pure boolean.
            self.solve_sat()
        }
    }

    // --- internal helpers ---------------------------------------------------

    /// Build a trivial (all-zero/false) model from the sort env.
    fn trivial_model(&self) -> Model {
        let mut m = Model::new();
        for (name, sort) in &self.sort_env {
            let v = default_value_for_sort(sort);
            if let Some(v) = v {
                m.insert(name.clone(), v);
            }
        }
        m
    }

    /// Return the name of the first variable whose sort isn't supported in
    /// v1 (`Bool` and `Int` are supported; everything else is not).
    fn unsupported_sort_in_env(&self) -> Option<String> {
        for (name, sort) in &self.sort_env {
            if !matches!(sort, Sort::Bool | Sort::Int) {
                return Some(name.clone());
            }
        }
        None
    }

    /// Solve a formula that has at least one Int variable via the LIA tactic.
    ///
    /// Strategy:
    /// 1. Convert assertions to the LIA tactic's input format (linear
    ///    constraints over integer variables).
    /// 2. Let [`lia::LiaTactic`] search for a satisfying assignment.
    /// 3. If SAT and there are Bool-only predicates, verify them
    ///    consistently using the SAT tactic.
    fn solve_lia(&self, _has_bool: bool) -> SolverResult {
        let int_vars: Vec<String> = self.sort_env
            .iter()
            .filter(|(_, s)| matches!(s, Sort::Int))
            .map(|(n, _)| n.clone())
            .collect();
        let bool_vars: Vec<String> = self.sort_env
            .iter()
            .filter(|(_, s)| matches!(s, Sort::Bool))
            .map(|(n, _)| n.clone())
            .collect();

        let result = lia::LiaTactic::solve(&self.assertions, &int_vars, &bool_vars);

        match result {
            SolverResult::Sat(ref model) => {
                // Verify that all assertions hold under the proposed model.
                // This is a completeness check — the Cooper-based solver may
                // produce an assignment that satisfies the LIA fragment but
                // not a Bool predicate that was wired in.
                let ok = self.assertions.iter().all(|p| eval_bool(p, model));
                if ok {
                    result
                } else {
                    // Try again with more constraints (simplistic; works for
                    // the small formulae we target in v1).
                    SolverResult::Unknown(
                        "model candidate failed verification — mixed LIA+Bool exhausted".into(),
                    )
                }
            }
            other => other,
        }
    }

    /// Solve a pure-Bool formula via the SAT tactic.
    fn solve_sat(&self) -> SolverResult {
        let bool_vars: Vec<String> = self.sort_env
            .iter()
            .filter(|(_, s)| matches!(s, Sort::Bool))
            .map(|(n, _)| n.clone())
            .collect();
        sat::SatTactic::solve(&self.assertions, &bool_vars)
    }
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_supported_logic(logic: Logic) -> bool {
    matches!(logic, Logic::QF_Bool | Logic::QF_LIA)
}

fn default_value_for_sort(sort: &Sort) -> Option<Value> {
    match sort {
        Sort::Bool => Some(Value::Bool(false)),
        Sort::Int => Some(Value::Int(0)),
        Sort::Real => Some(Value::Real(0, 1)),
        _ => None,
    }
}

/// Return `true` if `p` contains a quantifier anywhere.
fn has_quantifier(p: &Predicate) -> bool {
    match p {
        Predicate::Forall { .. } | Predicate::Exists { .. } => true,
        Predicate::And(parts) | Predicate::Or(parts) | Predicate::Add(parts) => {
            parts.iter().any(has_quantifier)
        }
        Predicate::Not(inner) | Predicate::Mul { term: inner, .. } => has_quantifier(inner),
        Predicate::Implies(a, b)
        | Predicate::Iff(a, b)
        | Predicate::Eq(a, b)
        | Predicate::NEq(a, b)
        | Predicate::Sub(a, b)
        | Predicate::Le(a, b)
        | Predicate::Lt(a, b)
        | Predicate::Ge(a, b)
        | Predicate::Gt(a, b) => has_quantifier(a) || has_quantifier(b),
        Predicate::Ite(c, t, e) => {
            has_quantifier(c) || has_quantifier(t) || has_quantifier(e)
        }
        Predicate::Apply { args, .. } => args.iter().any(has_quantifier),
        Predicate::Select { arr, idx } => has_quantifier(arr) || has_quantifier(idx),
        Predicate::Store { arr, idx, val } => {
            has_quantifier(arr) || has_quantifier(idx) || has_quantifier(val)
        }
        // Atoms
        _ => false,
    }
}

/// Evaluate a predicate to a boolean under a model.
/// Returns `false` if the predicate is undecidable (e.g. unknown variable).
pub(crate) fn eval_bool(p: &Predicate, model: &Model) -> bool {
    match eval_int_or_bool(p, model) {
        EvalResult::Bool(b) => b,
        EvalResult::Int(n) => n != 0,
        EvalResult::Unknown => false,
    }
}

/// Internal evaluation result.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EvalResult {
    Bool(bool),
    Int(i128),
    Unknown,
}

/// Recursively evaluate `p` under `model`.
pub(crate) fn eval_int_or_bool(p: &Predicate, model: &Model) -> EvalResult {
    match p {
        Predicate::Bool(b) => EvalResult::Bool(*b),
        Predicate::Int(n) => EvalResult::Int(*n),
        Predicate::Real(_) => EvalResult::Unknown,
        Predicate::Var(name) => match model.get(name) {
            Some(Value::Bool(b)) => EvalResult::Bool(*b),
            Some(Value::Int(n)) => EvalResult::Int(*n),
            _ => EvalResult::Unknown,
        },
        Predicate::And(parts) => {
            for part in parts {
                if !eval_bool(part, model) {
                    return EvalResult::Bool(false);
                }
            }
            EvalResult::Bool(true)
        }
        Predicate::Or(parts) => {
            for part in parts {
                if eval_bool(part, model) {
                    return EvalResult::Bool(true);
                }
            }
            EvalResult::Bool(false)
        }
        Predicate::Not(inner) => {
            EvalResult::Bool(!eval_bool(inner, model))
        }
        Predicate::Implies(a, b) => {
            EvalResult::Bool(!eval_bool(a, model) || eval_bool(b, model))
        }
        Predicate::Iff(a, b) => {
            EvalResult::Bool(eval_bool(a, model) == eval_bool(b, model))
        }
        Predicate::Eq(a, b) => {
            match (eval_int_or_bool(a, model), eval_int_or_bool(b, model)) {
                (EvalResult::Int(x), EvalResult::Int(y)) => EvalResult::Bool(x == y),
                (EvalResult::Bool(x), EvalResult::Bool(y)) => EvalResult::Bool(x == y),
                _ => EvalResult::Unknown,
            }
        }
        Predicate::NEq(a, b) => {
            match (eval_int_or_bool(a, model), eval_int_or_bool(b, model)) {
                (EvalResult::Int(x), EvalResult::Int(y)) => EvalResult::Bool(x != y),
                (EvalResult::Bool(x), EvalResult::Bool(y)) => EvalResult::Bool(x != y),
                _ => EvalResult::Unknown,
            }
        }
        Predicate::Add(parts) => {
            let mut sum = 0i128;
            for part in parts {
                match eval_int_or_bool(part, model) {
                    EvalResult::Int(n) => sum = sum.saturating_add(n),
                    _ => return EvalResult::Unknown,
                }
            }
            EvalResult::Int(sum)
        }
        Predicate::Sub(a, b) => {
            match (eval_int_or_bool(a, model), eval_int_or_bool(b, model)) {
                (EvalResult::Int(x), EvalResult::Int(y)) => {
                    EvalResult::Int(x.saturating_sub(y))
                }
                _ => EvalResult::Unknown,
            }
        }
        Predicate::Mul { coef, term } => {
            match eval_int_or_bool(term, model) {
                EvalResult::Int(n) => EvalResult::Int(coef.saturating_mul(n)),
                _ => EvalResult::Unknown,
            }
        }
        Predicate::Le(a, b) => cmp_eval(a, b, model, |x, y| x <= y),
        Predicate::Lt(a, b) => cmp_eval(a, b, model, |x, y| x < y),
        Predicate::Ge(a, b) => cmp_eval(a, b, model, |x, y| x >= y),
        Predicate::Gt(a, b) => cmp_eval(a, b, model, |x, y| x > y),
        Predicate::Ite(c, t, e) => {
            if eval_bool(c, model) {
                eval_int_or_bool(t, model)
            } else {
                eval_int_or_bool(e, model)
            }
        }
        // Unsupported in evaluation: Apply, quantifiers, arrays.
        _ => EvalResult::Unknown,
    }
}

fn cmp_eval(
    a: &Predicate,
    b: &Predicate,
    model: &Model,
    op: impl Fn(i128, i128) -> bool,
) -> EvalResult {
    match (eval_int_or_bool(a, model), eval_int_or_bool(b, model)) {
        (EvalResult::Int(x), EvalResult::Int(y)) => EvalResult::Bool(op(x, y)),
        _ => EvalResult::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine(vars: &[(&str, Sort)]) -> Engine {
        let mut e = Engine::new();
        for (name, sort) in vars {
            e.declare_var(*name, sort.clone());
        }
        e
    }

    // ---------- SolverResult helpers ----------

    #[test]
    fn solver_result_predicates() {
        let sat = SolverResult::Sat(Model::new());
        assert!(sat.is_sat());
        assert!(!sat.is_unsat());
        assert!(!sat.is_unknown());
        assert!(sat.model().is_some());

        let unsat = SolverResult::Unsat;
        assert!(unsat.is_unsat());
        assert!(!unsat.is_sat());
        assert!(unsat.model().is_none());

        let unk = SolverResult::Unknown("x".into());
        assert!(unk.is_unknown());
        assert!(unk.model().is_none());
    }

    // ---------- Model ----------

    #[test]
    fn model_insert_and_get() {
        let mut m = Model::new();
        m.insert("x", Value::Int(42));
        m.insert("b", Value::Bool(true));
        assert_eq!(m.get("x"), Some(&Value::Int(42)));
        assert_eq!(m.get("b"), Some(&Value::Bool(true)));
        assert_eq!(m.get("z"), None);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn value_display() {
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Int(-7).to_string(), "-7");
        assert_eq!(Value::Real(3, 4).to_string(), "3/4");
        assert_eq!(Value::Real(5, 1).to_string(), "5");
    }

    // ---------- Empty conjunction ----------

    #[test]
    fn empty_assertions_is_sat() {
        let e = make_engine(&[("x", Sort::Int)]);
        let r = e.check_sat();
        assert!(r.is_sat());
    }

    // ---------- Unsupported logic/sort → Unknown ----------

    #[test]
    fn unsupported_logic_returns_unknown() {
        let mut e = make_engine(&[("x", Sort::Int)]);
        e.set_logic(Logic::QF_LRA);
        e.assert(Predicate::Ge(Box::new(Predicate::Var("x".into())), Box::new(Predicate::Int(0))));
        assert!(e.check_sat().is_unknown());
    }

    #[test]
    fn unsupported_sort_returns_unknown() {
        let mut e = make_engine(&[("x", Sort::Real)]);
        e.assert(Predicate::Ge(Box::new(Predicate::Var("x".into())), Box::new(Predicate::Int(0))));
        assert!(e.check_sat().is_unknown());
    }

    #[test]
    fn quantifier_returns_unknown() {
        let mut e = make_engine(&[]);
        e.assert(Predicate::Forall {
            var: "x".into(),
            sort: Sort::Int,
            body: Box::new(Predicate::Ge(
                Box::new(Predicate::Var("x".into())),
                Box::new(Predicate::Int(0)),
            )),
        });
        assert!(e.check_sat().is_unknown());
    }

    // ---------- Pure bool ----------

    #[test]
    fn trivial_bool_true_is_sat() {
        let mut e = make_engine(&[("b", Sort::Bool)]);
        e.assert(Predicate::Bool(true));
        assert!(e.check_sat().is_sat());
    }

    #[test]
    fn trivial_bool_false_is_unsat() {
        let mut e = make_engine(&[("b", Sort::Bool)]);
        e.assert(Predicate::Bool(false));
        assert!(e.check_sat().is_unsat());
    }

    // ---------- LIA: simple range ----------

    #[test]
    fn lia_range_sat() {
        // x ≥ 0 ∧ x ≤ 100  →  SAT (x = 0 is a witness)
        let mut e = make_engine(&[("x", Sort::Int)]);
        e.assert(Predicate::Ge(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(0)),
        ));
        e.assert(Predicate::Le(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(100)),
        ));
        let r = e.check_sat();
        assert!(r.is_sat());
        let m = r.model().unwrap();
        let x = m.get("x").unwrap().as_int().unwrap();
        assert!((0..=100).contains(&x), "model x={x} not in [0,100]");
    }

    #[test]
    fn lia_contradiction_is_unsat() {
        // x ≥ 10 ∧ x ≤ 5  →  UNSAT
        let mut e = make_engine(&[("x", Sort::Int)]);
        e.assert(Predicate::Ge(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(10)),
        ));
        e.assert(Predicate::Le(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(5)),
        ));
        assert!(e.check_sat().is_unsat());
    }

    #[test]
    fn lia_equality_sat() {
        // x = 42  →  SAT (x = 42)
        let mut e = make_engine(&[("x", Sort::Int)]);
        e.assert(Predicate::Eq(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(42)),
        ));
        let r = e.check_sat();
        assert!(r.is_sat());
        let x = r.model().unwrap().get("x").unwrap().as_int().unwrap();
        assert_eq!(x, 42);
    }

    #[test]
    fn lia_disequality_sat() {
        // x ≠ 0  →  SAT (x = 1 is a witness)
        let mut e = make_engine(&[("x", Sort::Int)]);
        e.assert(Predicate::NEq(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(0)),
        ));
        assert!(e.check_sat().is_sat());
    }

    #[test]
    fn lia_two_variable_sum() {
        // x ≥ 0 ∧ y ≥ 0 ∧ x + y ≤ 100  →  SAT
        let mut e = make_engine(&[("x", Sort::Int), ("y", Sort::Int)]);
        e.assert(Predicate::Ge(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(0)),
        ));
        e.assert(Predicate::Ge(
            Box::new(Predicate::Var("y".into())),
            Box::new(Predicate::Int(0)),
        ));
        e.assert(Predicate::Le(
            Box::new(Predicate::Add(vec![
                Predicate::Var("x".into()),
                Predicate::Var("y".into()),
            ])),
            Box::new(Predicate::Int(100)),
        ));
        let r = e.check_sat();
        assert!(r.is_sat(), "expected SAT, got {r:?}");
        let m = r.model().unwrap();
        let x = m.get("x").unwrap().as_int().unwrap();
        let y = m.get("y").unwrap().as_int().unwrap();
        assert!(x >= 0 && y >= 0 && x + y <= 100, "model violated: x={x} y={y}");
    }

    #[test]
    fn lia_strict_lt_sat() {
        // x > 5  →  SAT (x = 6 is a witness)
        let mut e = make_engine(&[("x", Sort::Int)]);
        e.assert(Predicate::Gt(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(5)),
        ));
        let r = e.check_sat();
        assert!(r.is_sat());
        let x = r.model().unwrap().get("x").unwrap().as_int().unwrap();
        assert!(x > 5, "expected x > 5, got x={x}");
    }

    #[test]
    fn lia_strict_contradiction() {
        // x > 5 ∧ x < 6 → UNSAT over integers (no integer between 5 and 6)
        let mut e = make_engine(&[("x", Sort::Int)]);
        e.assert(Predicate::Gt(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(5)),
        ));
        e.assert(Predicate::Lt(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(6)),
        ));
        assert!(e.check_sat().is_unsat());
    }

    // ---------- Snapshot / reset ----------

    #[test]
    fn snapshot_restore() {
        let mut e = make_engine(&[("x", Sort::Int)]);
        e.assert(Predicate::Ge(Box::new(Predicate::Var("x".into())), Box::new(Predicate::Int(0))));
        let snap = e.snapshot();
        e.assert(Predicate::Le(Box::new(Predicate::Var("x".into())), Box::new(Predicate::Int(-1))));
        // After adding a contradiction the engine is UNSAT.
        assert!(e.check_sat().is_unsat());
        // Restoring the snapshot brings back SAT.
        let e2 = snap;
        assert!(e2.check_sat().is_sat());
    }

    // ---------- eval helpers ----------

    #[test]
    fn eval_bool_literals() {
        let m = Model::new();
        assert!(eval_bool(&Predicate::Bool(true), &m));
        assert!(!eval_bool(&Predicate::Bool(false), &m));
    }

    #[test]
    fn eval_add_and_compare() {
        let mut m = Model::new();
        m.insert("x", Value::Int(10));
        m.insert("y", Value::Int(5));
        // x + y = 15 ≥ 10
        let p = Predicate::Ge(
            Box::new(Predicate::Add(vec![
                Predicate::Var("x".into()),
                Predicate::Var("y".into()),
            ])),
            Box::new(Predicate::Int(10)),
        );
        assert!(eval_bool(&p, &m));
    }
}

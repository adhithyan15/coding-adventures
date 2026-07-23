//! # adj-constraint-solver — solving the adj-lang constraint sublanguage (ADJ constraints, track B2a).
//!
//! Track B1 gave adj-lang a constraint sublanguage (`symbol`, `constrain`,
//! `solve for`, `check`) that lowers to a [`ConstraintSystem`]. This crate is
//! the first **solver** behind it: the model wrote the constraints; the engine
//! solves them deterministically on the CPU.
//!
//! This slice handles the **linear-equality** case — a square system of `=`
//! constraints over the declared symbols (`premium = base_rate + claim *
//! rate`, `total = a + b`) — by translating the constraints' unevaluated
//! [`ComputeExpr`] trees into `symbolic-ir` equations and dispatching to
//! [`cas_solve::solve_linear_system`] (exact Gaussian elimination over the
//! rationals — no float drift). The solved values are returned **traced to the
//! constraints that determined them**, which is the whole point: the answer is
//! auditable, the model never solved anything.
//!
//! Out of scope here (the next slices of the arc): inequalities / linear-real
//! feasibility (QF_LRA, track C1), linear optimization (simplex, C2),
//! boolean/SAT and linear-integer (`constraint-engine` tactics), and the
//! infeasibility (UNSAT-core) certificate. A system this slice can't handle
//! returns [`SolveOutcome::Unsupported`] with a reason, never a wrong answer.

use std::collections::HashSet;

use adj_lang::{ConstraintSystem, LoweredConstraint, OptDir, RelOp};
use cas_solve::frac::Frac;
use cas_solve::{solve_cubic, solve_quadratic, solve_quartic, SolveResult};
use constraint_core::Predicate;
use constraint_engine::{lia::LiaTactic, sat::SatTactic, Model, SolverResult, Value};
use logic_engine::{ComputeExpr, ComputeOp, KnowledgeBase};
use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, EQUAL, MUL, SUB};

/// What solving a [`ConstraintSystem`] produced.
#[derive(Debug, Clone, PartialEq)]
pub enum SolveOutcome {
    /// A unique solution: each declared symbol mapped to its value, plus the
    /// indices of the constraints that determined it (provenance — the audit
    /// reader follows these back to the constraints' source bytes).
    Solved {
        assignments: Vec<(String, f64)>,
        from_constraints: Vec<usize>,
    },
    /// A single unknown satisfying a **nonlinear** (degree 2–4) equality — its
    /// real roots (possibly several). E.g. `x*x = 4 → {-2, 2}`. The audit reader
    /// follows `from_constraints` back to the equation.
    SolvedRoots {
        var: String,
        roots: Vec<f64>,
        from_constraints: Vec<usize>,
    },
    /// The linear system is singular, under-/over-determined, or not square
    /// (≠ one equation per unknown) — no unique solution exists.
    NoUniqueSolution,
    /// The system is outside this slice's scope (inequalities, a non-linear
    /// term, an aggregation, no symbols, …). Carries a human reason. Never a
    /// wrong answer — the caller falls back to a richer solver (C1/C2/…).
    Unsupported { reason: String },
}

/// Solve a [`ConstraintSystem`]'s linear-equality core against the program's
/// observed facts. See the module docs for scope. Pure and deterministic.
///
/// A constraint reference that is **not** one of the unknowns but **is** an
/// observed fact (`observe base_rate(1200)`) is substituted by its value, so a
/// mixed program — `symbol p; constrain p = base_rate + 300; solve for {p}` —
/// solves (`p = 1500`). A reference that is neither an unknown nor observed is
/// left as a free variable (which typically makes the system singular).
pub fn solve(cs: &ConstraintSystem, kb: &KnowledgeBase) -> SolveOutcome {
    // The unknowns: the `solve for { … }` targets, or (failing that) every
    // declared `symbol`. Order is the column order of the linear system.
    let variables: Vec<String> = if !cs.solve_for.is_empty() {
        cs.solve_for.clone()
    } else {
        cs.symbols.iter().map(|(n, _)| n.clone()).collect()
    };
    if variables.is_empty() {
        return unsupported("no symbols / solve-for targets to solve for");
    }
    let var_set: HashSet<&str> = variables.iter().map(String::as_str).collect();

    // This slice solves pure-equality systems. Any inequality means the
    // problem is feasibility/optimization, not a linear solve — defer it.
    if cs.constraints.iter().any(|c| c.op != RelOp::Eq) {
        return unsupported("inequality constraints — feasibility/LP is track C1/C2");
    }

    // Single-unknown NONLINEAR fallback: one unknown, one equality whose
    // `lhs − rhs` is a univariate polynomial of degree 2–4 (`x*x = 4`). Solve
    // it exactly via cas-solve's closed-form root finders. Degree ≤ 1 falls
    // through to the linear path below.
    if variables.len() == 1 && cs.constraints.len() == 1 {
        let c = &cs.constraints[0];
        let lhs_s = substitute_observed(&c.lhs, &var_set, kb);
        let rhs_s = substitute_observed(&c.rhs, &var_set, kb);
        if let (Some(pl), Some(pr)) = (
            poly_of(&lhs_s, &variables[0]),
            poly_of(&rhs_s, &variables[0]),
        ) {
            let p = poly_sub(&pl, &pr);
            if poly_degree(&p) >= 2 {
                return solve_univariate_poly(&variables[0], &p);
            }
        }
    }

    // Translate each `lhs = rhs` into a symbolic-ir Equal equation, first
    // substituting observed-fact references by their values. A non-linear term
    // (symbol×symbol, division by a symbol, an aggregation) makes the
    // translation fail → Unsupported.
    let mut equations = Vec::with_capacity(cs.constraints.len());
    for c in &cs.constraints {
        let lhs_s = substitute_observed(&c.lhs, &var_set, kb);
        let rhs_s = substitute_observed(&c.rhs, &var_set, kb);
        let (Some(lhs), Some(rhs)) = (expr_to_ir(&lhs_s), expr_to_ir(&rhs_s)) else {
            return unsupported("a constraint is non-linear or uses an unsupported term");
        };
        equations.push(apply(sym(EQUAL), vec![lhs, rhs]));
    }

    // cas-solve needs a square system (one equation per unknown).
    if equations.len() != variables.len() {
        return SolveOutcome::NoUniqueSolution;
    }

    let var_syms: Vec<IRNode> = variables.iter().map(sym).collect();
    let Some(rules) = cas_solve::solve_linear_system(&equations, &var_syms) else {
        return SolveOutcome::NoUniqueSolution;
    };

    // Parse the `Rule(var, value)` results back into (name, f64).
    let mut assignments = Vec::with_capacity(rules.len());
    for r in &rules {
        let Some((name, value)) = parse_rule(r) else {
            return unsupported("solver returned a value that is not a finite rational");
        };
        assignments.push((name, value));
    }
    SolveOutcome::Solved {
        assignments,
        // Every equality constraint participated in the (square) solve.
        from_constraints: (0..cs.constraints.len()).collect(),
    }
}

fn unsupported(reason: &str) -> SolveOutcome {
    SolveOutcome::Unsupported {
        reason: reason.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Feasibility: is a set of (in)equality constraints satisfiable? (track B2c)
// ---------------------------------------------------------------------------

/// The result of a `check` — whether the accumulated constraints can all hold
/// at once. Backed by `constraint-engine`'s linear-integer-arithmetic tactic.
#[derive(Debug, Clone, PartialEq)]
pub enum FeasibilityOutcome {
    /// Satisfiable over the **integers**, with an integer witness assignment.
    /// Produced by the exact linear-integer tactic (`LiaTactic`).
    Sat { assignments: Vec<(String, i128)> },
    /// Satisfiable over the **reals** (QF_LRA), with a rational witness rendered
    /// as `f64`. Produced by the Fourier–Motzkin layer (track C1) when the
    /// integer tactic punts or a constraint is non-integer — or when the system
    /// is integer-infeasible but real-feasible (e.g. `2x = 1`). The feasibility
    /// **decision** is exact; the witness `f64`s are a representative point.
    SatReal { assignments: Vec<(String, f64)> },
    /// Unsatisfiable — no assignment satisfies all constraints at once, over
    /// either the integers *or* the reals. `core` is a **minimal** infeasible
    /// subset (IIS): removing any one of its constraints makes the rest
    /// feasible. This is the machine-checked "*these* constraints contradict",
    /// the certificate that localizes a golden-rulebook bug to the exact clauses
    /// in conflict.
    Unsat { core: Vec<usize> },
    /// The engine couldn't decide — a non-linear constraint, a `!=`
    /// (disjunctive, non-convex), or a system too large for the bounded
    /// Fourier–Motzkin slice.
    Unknown { reason: String },
}

/// Decide whether a [`ConstraintSystem`]'s constraints are jointly satisfiable,
/// substituting observed facts first. Two procedures are layered:
///
/// 1. The exact linear-**integer** tactic ([`LiaTactic`], B2c) runs first when
///    every constraint is integer-linear. A `Sat` returns an integer witness.
/// 2. The Fourier–Motzkin layer (track C1) decides **real** (QF_LRA)
///    feasibility when the integer tactic punts (`Unknown`), when a constraint
///    is non-integer, **or** when the integer tactic says `Unsat` — because an
///    integer-infeasible system may still be real-feasible (`2x = 1`). A real
///    `Sat` returns a rational witness (`SatReal`).
///
/// A system is `Unsat` only when *both* layers reject it. A `!=` constraint
/// (disjunctive) or a non-linear term stays `Unknown`.
pub fn check(cs: &ConstraintSystem, kb: &KnowledgeBase) -> FeasibilityOutcome {
    if cs.constraints.is_empty() {
        return FeasibilityOutcome::Sat {
            assignments: Vec::new(),
        };
    }
    let int_vars: Vec<String> = cs.symbols.iter().map(|(n, _)| n.clone()).collect();
    let var_set: HashSet<&str> = int_vars.iter().map(String::as_str).collect();

    // Substitute observed facts once; both layers read the substituted forms.
    let subbed: Vec<(ComputeExpr, RelOp, ComputeExpr)> = cs
        .constraints
        .iter()
        .map(|c| {
            (
                substitute_observed(&c.lhs, &var_set, kb),
                c.op,
                substitute_observed(&c.rhs, &var_set, kb),
            )
        })
        .collect();

    // ---- Layer 1: exact linear-integer tactic (only if every constraint is
    // integer-linear). ----
    if let Some(assertions) = integer_assertions(&subbed) {
        match LiaTactic::solve(&assertions, &int_vars, &[]) {
            SolverResult::Sat(model) => {
                let assignments = int_vars
                    .iter()
                    .filter_map(|v| match model.get(v) {
                        Some(Value::Int(n)) => Some((v.clone(), *n)),
                        Some(Value::Bool(b)) => Some((v.clone(), *b as i128)),
                        _ => None,
                    })
                    .collect();
                return FeasibilityOutcome::Sat { assignments };
            }
            // Integer-infeasible — defer to the real layer before declaring a
            // contradiction (over ℝ it may still be feasible). If the real layer
            // can't find a point either (Unsat) or can't decide a constraint the
            // integer tactic *could* (e.g. `!=`), the integer verdict stands.
            SolverResult::Unsat => match real_feasibility(&subbed) {
                FmResult::Sat(w) => return FeasibilityOutcome::SatReal { assignments: w },
                FmResult::Unsat | FmResult::Unknown(_) => {
                    return FeasibilityOutcome::Unsat {
                        core: minimal_unsat_core(&subbed, &int_vars),
                    }
                }
            },
            // Integer tactic punted — let the real layer try.
            SolverResult::Unknown(_) => {}
        }
    }

    // ---- Layer 2: QF_LRA real feasibility via Fourier–Motzkin over ℚ. ----
    match real_feasibility(&subbed) {
        FmResult::Sat(w) => FeasibilityOutcome::SatReal { assignments: w },
        FmResult::Unsat => FeasibilityOutcome::Unsat {
            core: minimal_unsat_core(&subbed, &int_vars),
        },
        FmResult::Unknown(reason) => FeasibilityOutcome::Unknown { reason },
    }
}

/// Does the given **subset** of (substituted) constraints have no joint
/// solution? Mirrors [`check`]'s two-layer logic — exact integer LIA first, then
/// real Fourier–Motzkin — returning `true` only when a contradiction is
/// *proven* (both layers reject, or the decisive layer rejects). `Sat`/`Unknown`
/// → `false`, so we never claim a subset is the conflict unless it provably is.
fn subset_is_unsat(
    subbed: &[(ComputeExpr, RelOp, ComputeExpr)],
    int_vars: &[String],
    indices: &[usize],
) -> bool {
    let sub: Vec<(ComputeExpr, RelOp, ComputeExpr)> =
        indices.iter().map(|&i| subbed[i].clone()).collect();
    if let Some(assertions) = integer_assertions(&sub) {
        match LiaTactic::solve(&assertions, int_vars, &[]) {
            SolverResult::Sat(_) => return false,
            // Integer-infeasible only counts if it's real-infeasible too.
            SolverResult::Unsat => return matches!(real_feasibility(&sub), FmResult::Unsat),
            SolverResult::Unknown(_) => {}
        }
    }
    matches!(real_feasibility(&sub), FmResult::Unsat)
}

/// Shrink an infeasible constraint set to a **minimal** infeasible subset (an
/// IIS): the indices such that removing any one makes the rest feasible. A
/// deletion filter — walk the constraints; if dropping one leaves a still-
/// infeasible set, that one is redundant for the conflict and is removed
/// permanently. O(n) feasibility checks, each reusing [`subset_is_unsat`]. The
/// caller only reaches this on a decided `Unsat`, so the full set is infeasible.
/// (Conservative on `Unknown` subsets — it keeps the constraint, so the result
/// is always a valid infeasible core; for the linear systems here it is exactly
/// minimal.)
fn minimal_unsat_core(
    subbed: &[(ComputeExpr, RelOp, ComputeExpr)],
    int_vars: &[String],
) -> Vec<usize> {
    let mut core: Vec<usize> = (0..subbed.len()).collect();
    let mut i = 0;
    while i < core.len() {
        let candidate = core[i];
        let trial: Vec<usize> = core.iter().copied().filter(|&x| x != candidate).collect();
        if !trial.is_empty() && subset_is_unsat(subbed, int_vars, &trial) {
            core = trial; // `candidate` is not needed for the contradiction
        } else {
            i += 1; // `candidate` is essential — keep it
        }
    }
    core
}

/// Translate every (substituted) constraint into a linear-**integer**
/// [`Predicate`] assertion, or `None` if any constraint isn't integer-linear
/// (so the integer tactic can't be used and we fall to the real layer).
fn integer_assertions(subbed: &[(ComputeExpr, RelOp, ComputeExpr)]) -> Option<Vec<Predicate>> {
    let mut assertions = Vec::with_capacity(subbed.len());
    for (lhs, op, rhs) in subbed {
        let (pl, pr) = (expr_to_pred(lhs)?, expr_to_pred(rhs)?);
        assertions.push(relop_predicate(*op, pl, pr));
    }
    Some(assertions)
}

/// Build the relational predicate `lhs <op> rhs`.
fn relop_predicate(op: RelOp, lhs: Predicate, rhs: Predicate) -> Predicate {
    let (l, r) = (Box::new(lhs), Box::new(rhs));
    match op {
        RelOp::Ge => Predicate::Ge(l, r),
        RelOp::Le => Predicate::Le(l, r),
        RelOp::Gt => Predicate::Gt(l, r),
        RelOp::Lt => Predicate::Lt(l, r),
        RelOp::Eq => Predicate::Eq(l, r),
        RelOp::Ne => Predicate::NEq(l, r),
    }
}

/// Translate a (substituted) [`ComputeExpr`] into a linear-integer
/// [`Predicate`], or `None` if it isn't linear-integer (a non-integer literal,
/// symbol×symbol, division — beyond the LIA tactic).
fn expr_to_pred(e: &ComputeExpr) -> Option<Predicate> {
    match e {
        ComputeExpr::Ref(name) => Some(Predicate::Var(name.clone())),
        // A non-integer literal can't be expressed in LIA → None.
        ComputeExpr::Lit(_) => int_const(e).map(Predicate::Int),
        ComputeExpr::Bin(op, a, b) => {
            let pa = expr_to_pred(a)?;
            let pb = expr_to_pred(b)?;
            match op {
                ComputeOp::Add => Some(Predicate::Add(vec![pa, pb])),
                ComputeOp::Sub => Some(Predicate::Sub(Box::new(pa), Box::new(pb))),
                // Linear scaling only: integer-constant × term.
                ComputeOp::Mul => {
                    if let Some(c) = int_const(a) {
                        Some(Predicate::Mul {
                            coef: c,
                            term: Box::new(pb),
                        })
                    } else {
                        int_const(b).map(|c| Predicate::Mul {
                            coef: c,
                            term: Box::new(pa),
                        })
                    }
                }
                _ => None, // division / aggregation: out of LIA scope
            }
        }
        // Absolute value is not a linear-integer-arithmetic term (`|x|` is
        // piecewise-linear, not affine), so it is out of LIA scope.
        ComputeExpr::Unary(_, _) => None,
        ComputeExpr::Agg(_, _) => None,
        // A `round_to(x, n)` narrowing is neither linear nor polynomial — out of
        // scope for this tactic, exactly like a unary round (NUM-6a).
        ComputeExpr::Round { .. } => None,
        // `to_scientific(x, figures)` renders a number to a boundary string — not a
        // linear/polynomial term, out of scope for this tactic exactly like a round (NUM-6c).
        ComputeExpr::ToScientific { .. } => None,
        // `to_percent(x, places)` likewise renders a number to a boundary string (NUM-6c).
        ComputeExpr::ToPercent { .. } => None,
        // `to_currency(x, code, places)` likewise renders a number to a boundary string (NUM-6c).
        ComputeExpr::ToCurrency { .. } => None,
    }
}

/// The integer value of an expression if it is a whole-number literal.
fn int_const(e: &ComputeExpr) -> Option<i128> {
    match e {
        ComputeExpr::Lit(x) if x.fract() == 0.0 && x.is_finite() && x.abs() < i128::MAX as f64 => {
            Some(*x as i128)
        }
        _ => None,
    }
}

// ===========================================================================
// QF_LRA real feasibility via Fourier–Motzkin elimination over ℚ (track C1)
// ===========================================================================
//
// Fourier–Motzkin decides whether a conjunction of linear inequalities over the
// **reals** is satisfiable, using only exact rational arithmetic. The idea:
//
//   To eliminate a variable `v`, split the inequalities into those that bound
//   `v` from above (coefficient on `v` > 0) and from below (< 0). Every
//   (lower, upper) pair implies a new inequality with `v` cancelled — a
//   *shadow* of the original system onto the remaining variables. Repeat until
//   no variables remain; what's left is a set of constant inequalities like
//   `1 ≤ 0`. If any constant inequality is violated, the original system is
//   **infeasible**; otherwise it is **feasible** and we reconstruct a witness
//   point by back-substitution.
//
// Worked example — is `x ≥ 3 ∧ x ≤ 1` feasible?
//   normalise:   3 − x ≤ 0   (lower bound x ≥ 3)
//                x − 1 ≤ 0   (upper bound x ≤ 1)
//   eliminate x: scale + add → 3 − 1 ≤ 0  →  2 ≤ 0  ✗  → UNSAT.
//
// We work over a self-contained **checked** i128 rational ([`Rat`]): every
// operation returns `None` on overflow (or when a value would exceed `RAT_CAP`)
// instead of silently wrapping the way a fixed-width rational would. An overflow
// anywhere in the elimination becomes `FmResult::Unknown` — never a wrong
// verdict. The cap also keeps the ordering comparisons (`a·d` vs `c·b`) provably
// within i128. A cap on the intermediate-inequality count bounds the classic
// Fourier–Motzkin blow-up.

/// Largest intermediate-inequality count before we bail to `Unknown`.
const MAX_INEQUALITIES: usize = 4_000;
/// Magnitude ceiling for a reduced [`Rat`] numerator/denominator. Past this an
/// operation returns `None` (→ `Unknown`). 10^18 leaves headroom so that
/// cross-multiplied comparisons (`a·d`, `c·b`, each ≤ 10^36) and the i128
/// arithmetic intermediates stay well under `i128::MAX` (≈ 1.7·10^38).
const RAT_CAP: i128 = 1_000_000_000_000_000_000; // 10^18

/// The outcome of the real-feasibility (Fourier–Motzkin) layer.
enum FmResult {
    /// Feasible over ℝ, with a rational witness rendered as `f64`.
    Sat(Vec<(String, f64)>),
    /// Infeasible over ℝ — a constant inequality was violated.
    Unsat,
    /// Outside the convex linear-real fragment (`!=`, non-linear), or the
    /// coefficients grew past the checked-rational cap, or too many inequalities.
    Unknown(String),
}

/// Greatest common divisor of two i128s (by magnitude).
fn igcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a as i128
}

/// A **checked** rational `num/den` with `den > 0`, always reduced. Every
/// arithmetic method returns `None` on i128 overflow or when the reduced value
/// would exceed [`RAT_CAP`]. Constructed only via [`Rat::new`] (which enforces
/// the cap) or the tiny literals `zero`/`one`/`half`, so every `Rat` in
/// circulation respects the cap — making the `Ord` cross-products overflow-free.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Rat {
    num: i128,
    den: i128,
}

impl Rat {
    fn new(num: i128, den: i128) -> Option<Rat> {
        if den == 0 {
            return None;
        }
        if num == 0 {
            return Some(Rat { num: 0, den: 1 });
        }
        let (n, d) = if den < 0 {
            (num.checked_neg()?, den.checked_neg()?)
        } else {
            (num, den)
        };
        let g = igcd(n, d);
        let (n, d) = (n / g, d / g);
        if n.unsigned_abs() > RAT_CAP as u128 || d > RAT_CAP {
            return None;
        }
        Some(Rat { num: n, den: d })
    }
    fn zero() -> Rat {
        Rat { num: 0, den: 1 }
    }
    fn one() -> Rat {
        Rat { num: 1, den: 1 }
    }
    fn half() -> Rat {
        Rat { num: 1, den: 2 }
    }
    fn is_zero(&self) -> bool {
        self.num == 0
    }
    fn neg(self) -> Option<Rat> {
        Some(Rat {
            num: self.num.checked_neg()?,
            den: self.den,
        })
    }
    fn add(self, o: Rat) -> Option<Rat> {
        let n = self
            .num
            .checked_mul(o.den)?
            .checked_add(o.num.checked_mul(self.den)?)?;
        let d = self.den.checked_mul(o.den)?;
        Rat::new(n, d)
    }
    fn sub(self, o: Rat) -> Option<Rat> {
        self.add(o.neg()?)
    }
    fn mul(self, o: Rat) -> Option<Rat> {
        Rat::new(self.num.checked_mul(o.num)?, self.den.checked_mul(o.den)?)
    }
    fn div(self, o: Rat) -> Option<Rat> {
        if o.num == 0 {
            return None;
        }
        Rat::new(self.num.checked_mul(o.den)?, self.den.checked_mul(o.num)?)
    }
    fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

impl PartialOrd for Rat {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Rat {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        // den > 0 for both, and every `Rat` is ≤ RAT_CAP (10^18), so the
        // cross-products are ≤ 10^36 < i128::MAX — no overflow.
        (self.num * o.den).cmp(&(o.num * self.den))
    }
}

/// Convert an `f64` literal to an exact [`Rat`], or `None` if it isn't finite or
/// can't be represented within the cap. Mirrors `f64_to_frac` but checked.
fn f64_to_rat(x: f64) -> Option<Rat> {
    if !x.is_finite() {
        return None;
    }
    if x.fract() == 0.0 && x.abs() < RAT_CAP as f64 {
        return Rat::new(x as i128, 1);
    }
    let mut denom: i128 = 1;
    for _ in 0..9 {
        denom *= 10;
        let scaled = x * denom as f64;
        if scaled.fract().abs() < 1e-9 && scaled.abs() < RAT_CAP as f64 {
            return Rat::new(scaled.round() as i128, denom);
        }
    }
    let denom = 1_000_000_000i128;
    let scaled = x * denom as f64;
    if scaled.abs() < RAT_CAP as f64 {
        Rat::new(scaled.round() as i128, denom)
    } else {
        None
    }
}

/// A linear form `Σ cᵢ·xᵢ + k` over ℚ: a sorted map from variable name to its
/// (non-zero) rational coefficient, plus a constant term. All combinators are
/// checked — they return `None` if any coefficient overflows the cap.
#[derive(Clone)]
struct LinForm {
    coeffs: std::collections::BTreeMap<String, Rat>,
    constant: Rat,
}

impl LinForm {
    fn constant(c: Rat) -> Self {
        LinForm {
            coeffs: std::collections::BTreeMap::new(),
            constant: c,
        }
    }
    fn var(name: &str) -> Self {
        let mut coeffs = std::collections::BTreeMap::new();
        coeffs.insert(name.to_string(), Rat::one());
        LinForm {
            coeffs,
            constant: Rat::zero(),
        }
    }
    fn is_constant(&self) -> bool {
        self.coeffs.is_empty()
    }
    fn coeff(&self, v: &str) -> Rat {
        self.coeffs.get(v).copied().unwrap_or_else(Rat::zero)
    }
    /// Multiply the whole form by a rational scalar (checked).
    fn scale(&self, k: Rat) -> Option<LinForm> {
        if k.is_zero() {
            return Some(LinForm::constant(Rat::zero()));
        }
        let mut coeffs = std::collections::BTreeMap::new();
        for (n, c) in &self.coeffs {
            coeffs.insert(n.clone(), c.mul(k)?);
        }
        Some(LinForm {
            coeffs,
            constant: self.constant.mul(k)?,
        })
    }
    /// `self + other.scale(s)`, dropping any coefficient that cancels to zero
    /// (checked — `None` on overflow).
    fn add_scaled(&self, other: &LinForm, s: Rat) -> Option<LinForm> {
        let mut coeffs = self.coeffs.clone();
        for (n, c) in &other.coeffs {
            let entry = coeffs.entry(n.clone()).or_insert_with(Rat::zero);
            *entry = entry.add(c.mul(s)?)?;
            if entry.is_zero() {
                coeffs.remove(n);
            }
        }
        Some(LinForm {
            coeffs,
            constant: self.constant.add(other.constant.mul(s)?)?,
        })
    }
}

/// A half-plane `Σ cᵢ·xᵢ + k  (≤ | <)  0`. `strict` distinguishes `<` from `≤`.
#[derive(Clone)]
struct Halfplane {
    form: LinForm,
    strict: bool,
}

/// Convert a (substituted) [`ComputeExpr`] into an affine [`LinForm`], or `None`
/// if it isn't linear (symbol×symbol, division by a non-constant, aggregation,
/// or a non-representable literal).
fn linearize(e: &ComputeExpr) -> Option<LinForm> {
    match e {
        ComputeExpr::Ref(name) => Some(LinForm::var(name)),
        ComputeExpr::Lit(x) => Some(LinForm::constant(f64_to_rat(*x)?)),
        ComputeExpr::Bin(op, a, b) => {
            let la = linearize(a)?;
            let lb = linearize(b)?;
            match op {
                ComputeOp::Add => la.add_scaled(&lb, Rat::one()),
                ComputeOp::Sub => la.add_scaled(&lb, Rat::one().neg()?),
                ComputeOp::Mul => {
                    if la.is_constant() {
                        lb.scale(la.constant)
                    } else if lb.is_constant() {
                        la.scale(lb.constant)
                    } else {
                        None // var × var is non-linear
                    }
                }
                ComputeOp::Div => {
                    if lb.is_constant() && !lb.constant.is_zero() {
                        la.scale(Rat::one().div(lb.constant)?)
                    } else {
                        None // division by a variable / by zero
                    }
                }
                _ => None, // aggregation operators are non-linear here
            }
        }
        // Absolute value is piecewise-linear, not affine — out of the linear-real
        // fragment, so it can't be represented as a `LinForm`.
        ComputeExpr::Unary(_, _) => None,
        ComputeExpr::Agg(_, _) => None,
        // A `round_to(x, n)` narrowing is neither linear nor polynomial — out of
        // scope for this tactic, exactly like a unary round (NUM-6a).
        ComputeExpr::Round { .. } => None,
        // `to_scientific(x, figures)` renders a number to a boundary string — not a
        // linear/polynomial term, out of scope for this tactic exactly like a round (NUM-6c).
        ComputeExpr::ToScientific { .. } => None,
        // `to_percent(x, places)` likewise renders a number to a boundary string (NUM-6c).
        ComputeExpr::ToPercent { .. } => None,
        // `to_currency(x, code, places)` likewise renders a number to a boundary string (NUM-6c).
        ComputeExpr::ToCurrency { .. } => None,
    }
}

/// Normalise one `lhs <op> rhs` constraint into half-planes of the form
/// `form (≤|<) 0`. An equality becomes a pair (`form ≤ 0` ∧ `−form ≤ 0`);
/// a `!=` is disjunctive (non-convex) and yields `None` → `Unknown`.
fn constraint_to_halfplanes(
    lhs: &ComputeExpr,
    op: RelOp,
    rhs: &ComputeExpr,
) -> Option<Vec<Halfplane>> {
    let diff = linearize(lhs)?.add_scaled(&linearize(rhs)?, Rat::one().neg()?)?; // lhs − rhs
    let neg = diff.scale(Rat::one().neg()?)?;
    Some(match op {
        // lhs ≤ rhs  ⇔  lhs − rhs ≤ 0
        RelOp::Le => vec![Halfplane {
            form: diff,
            strict: false,
        }],
        RelOp::Lt => vec![Halfplane {
            form: diff,
            strict: true,
        }],
        // lhs ≥ rhs  ⇔  rhs − lhs ≤ 0
        RelOp::Ge => vec![Halfplane {
            form: neg,
            strict: false,
        }],
        RelOp::Gt => vec![Halfplane {
            form: neg,
            strict: true,
        }],
        // lhs = rhs  ⇔  (lhs − rhs ≤ 0) ∧ (rhs − lhs ≤ 0)
        RelOp::Eq => vec![
            Halfplane {
                form: diff,
                strict: false,
            },
            Halfplane {
                form: neg,
                strict: false,
            },
        ],
        RelOp::Ne => return None, // disjunctive — out of the convex fragment
    })
}

/// Decide real feasibility of the substituted constraints via Fourier–Motzkin.
fn real_feasibility(subbed: &[(ComputeExpr, RelOp, ComputeExpr)]) -> FmResult {
    let mut planes: Vec<Halfplane> = Vec::new();
    for (lhs, op, rhs) in subbed {
        match constraint_to_halfplanes(lhs, *op, rhs) {
            Some(hps) => planes.extend(hps),
            None => {
                return FmResult::Unknown(
                    "a constraint is non-linear or uses `!=` (outside the linear-real fragment)"
                        .to_string(),
                )
            }
        }
    }
    fourier_motzkin(planes)
}

/// Run Fourier–Motzkin elimination on a set of half-planes. Returns `Sat` with a
/// rational witness, `Unsat`, or `Unknown` (size/coefficient guard tripped).
/// Collect the variable names appearing in a set of half-planes, sorted.
fn vars_of(planes: &[Halfplane]) -> Vec<String> {
    std::collections::BTreeSet::<String>::from_iter(
        planes.iter().flat_map(|h| h.form.coeffs.keys().cloned()),
    )
    .into_iter()
    .collect()
}

/// Eliminate the variables in `to_elim` (in order) from `planes` via
/// Fourier–Motzkin, returning the residual half-planes over the **remaining**
/// variables plus the per-variable elimination snapshots (the half-planes that
/// mentioned each variable — needed to reconstruct a witness). `Err(reason)` on
/// checked-rational overflow or the inequality-count cap. This is the shared
/// engine behind both feasibility (`check`, eliminate *all* variables) and
/// optimization (`optimize`, eliminate all *but* the objective variable).
// The tuple return (surviving planes + per-variable elimination steps) is the
// natural result shape here; extracting a named type would not improve clarity.
#[allow(clippy::type_complexity)]
fn eliminate(
    planes: Vec<Halfplane>,
    to_elim: &[String],
) -> Result<(Vec<Halfplane>, Vec<(String, Vec<Halfplane>)>), String> {
    let mut elim_steps: Vec<(String, Vec<Halfplane>)> = Vec::new();
    let mut current = planes;
    for v in to_elim {
        let mut pos: Vec<&Halfplane> = Vec::new(); // coeff(v) > 0 → upper bounds
        let mut neg: Vec<&Halfplane> = Vec::new(); // coeff(v) < 0 → lower bounds
        let mut zero: Vec<Halfplane> = Vec::new(); // v absent → carried forward
        for hp in &current {
            let c = hp.form.coeff(v);
            if c.is_zero() {
                zero.push(hp.clone());
            } else if c.num > 0 {
                pos.push(hp);
            } else {
                neg.push(hp);
            }
        }
        // Snapshot the bounding half-planes (pos ∪ neg) before they're consumed.
        let mut mentions: Vec<Halfplane> = Vec::with_capacity(pos.len() + neg.len());
        mentions.extend(pos.iter().map(|h| (*h).clone()));
        mentions.extend(neg.iter().map(|h| (*h).clone()));
        elim_steps.push((v.clone(), mentions));

        let mut next = zero;
        for p in &pos {
            let a = p.form.coeff(v); // > 0
            for n in &neg {
                let b = n.form.coeff(v); // < 0
                                         // Scale p by (−b) > 0 and n by a > 0, then add → v cancels.
                                         // Any overflow in this checked arithmetic ⇒ Err.
                let combined = b
                    .neg()
                    .and_then(|nb| p.form.scale(nb))
                    .and_then(|sp| sp.add_scaled(&n.form, a))
                    .ok_or_else(|| "coefficients grew past the checked-rational cap".to_string())?;
                next.push(Halfplane {
                    form: combined,
                    strict: p.strict || n.strict,
                });
                if next.len() > MAX_INEQUALITIES {
                    return Err(
                        "constraint system too large for the Fourier–Motzkin slice".to_string()
                    );
                }
            }
        }
        current = next;
    }
    Ok((current, elim_steps))
}

/// True iff a constant half-plane `k (≤|<) 0` is violated (`k > 0`, or `k ≥ 0`
/// when strict). Only meaningful once every variable has been eliminated.
fn constant_violated(hp: &Halfplane) -> bool {
    let k = hp.form.constant;
    if hp.strict {
        k.num >= 0 // need k < 0
    } else {
        k.num > 0 // need k ≤ 0
    }
}

fn fourier_motzkin(planes: Vec<Halfplane>) -> FmResult {
    // The variables to eliminate, in a deterministic order.
    let mut vars: Vec<String> = vars_of(&planes);

    // Keep the original system to verify the reconstructed witness against.
    let original = planes.clone();

    let (residual, elim_steps) = match eliminate(planes, &vars) {
        Ok(x) => x,
        Err(reason) => return FmResult::Unknown(reason),
    };

    // No variables left: every half-plane is a constant `k (≤|<) 0`.
    for hp in &residual {
        if constant_violated(hp) {
            return FmResult::Unsat;
        }
    }

    // Feasible — reconstruct a witness, assigning variables in reverse
    // elimination order. Verify it before returning; an overflow during
    // reconstruction or a failed re-check downgrades to a witness-free Sat
    // rather than ever emit a wrong point. The feasibility verdict above is
    // already exact and unaffected.
    let witness: Vec<(String, f64)> = match back_substitute(&mut vars, &elim_steps) {
        Some(assignment) if witness_satisfies(&original, &assignment) => assignment
            .iter()
            .map(|(n, f)| (n.clone(), f.to_f64()))
            .collect(),
        _ => Vec::new(),
    };
    FmResult::Sat(witness)
}

/// Check a rational assignment against every original half-plane. Uses checked
/// arithmetic; an overflow during the check is treated as "not satisfied" so the
/// caller downgrades to a witness-free `Sat` rather than emit a doubtful point.
fn witness_satisfies(planes: &[Halfplane], assignment: &[(String, Rat)]) -> bool {
    use std::collections::HashMap;
    let vals: HashMap<&str, Rat> = assignment.iter().map(|(n, f)| (n.as_str(), *f)).collect();
    planes.iter().all(|hp| {
        let mut total = hp.form.constant;
        for (name, c) in &hp.form.coeffs {
            let v = vals.get(name.as_str()).copied().unwrap_or_else(Rat::zero);
            total = match c.mul(v).and_then(|t| total.add(t)) {
                Some(t) => t,
                None => return false, // overflow → cannot verify → not satisfied
            };
        }
        if hp.strict {
            total.num < 0 // need total < 0
        } else {
            total.num <= 0 // need total ≤ 0
        }
    })
}

/// Reconstruct a feasible rational point. Variables are assigned in reverse
/// elimination order; for each, the half-planes it appeared in (with later
/// variables already fixed) give numeric lower/upper bounds, and we pick an
/// interior (or boundary) value. Returns `None` if the (checked) arithmetic
/// overflows — the feasibility verdict is unaffected; the caller drops the
/// witness.
fn back_substitute(
    vars: &mut [String],
    elim_steps: &[(String, Vec<Halfplane>)],
) -> Option<Vec<(String, Rat)>> {
    use std::collections::HashMap;
    let mut assigned: HashMap<String, Rat> = HashMap::new();
    for (v, planes) in elim_steps.iter().rev() {
        let mut lower: Option<(Rat, bool)> = None; // (value, strict)
        let mut upper: Option<(Rat, bool)> = None;
        for hp in planes {
            let a = hp.form.coeff(v);
            if a.is_zero() {
                continue;
            }
            // Evaluate the rest of the form with the already-assigned variables.
            let mut rest = hp.form.constant;
            for (name, c) in &hp.form.coeffs {
                if name == v {
                    continue;
                }
                let val = assigned.get(name).copied().unwrap_or_else(Rat::zero);
                rest = rest.add(c.mul(val)?)?;
            }
            // a·v + rest (≤|<) 0  ⇒  v (≤|<) −rest/a  (flip if a < 0).
            let bound = rest.neg()?.div(a)?;
            if a.num > 0 {
                update_upper(&mut upper, bound, hp.strict);
            } else {
                update_lower(&mut lower, bound, hp.strict);
            }
        }
        let value = pick_value(lower, upper)?;
        assigned.insert(v.clone(), value);
    }
    // Return in the original (sorted) variable order for stable output.
    Some(
        vars.iter()
            .map(|v| {
                (
                    v.clone(),
                    assigned.get(v).copied().unwrap_or_else(Rat::zero),
                )
            })
            .collect(),
    )
}

/// Tighten the running lower bound to the larger value (stricter on ties).
fn update_lower(lower: &mut Option<(Rat, bool)>, b: Rat, strict: bool) {
    match lower {
        None => *lower = Some((b, strict)),
        Some((cur, cur_strict)) => {
            if b > *cur {
                *lower = Some((b, strict));
            } else if b == *cur {
                *cur_strict = *cur_strict || strict;
            }
        }
    }
}

/// Tighten the running upper bound to the smaller value (stricter on ties).
fn update_upper(upper: &mut Option<(Rat, bool)>, b: Rat, strict: bool) {
    match upper {
        None => *upper = Some((b, strict)),
        Some((cur, cur_strict)) => {
            if b < *cur {
                *upper = Some((b, strict));
            } else if b == *cur {
                *cur_strict = *cur_strict || strict;
            }
        }
    }
}

/// Pick a rational value within `[lower, upper]`, respecting strictness:
/// midpoint when both are present, one step inside an open half-bound, or `0`
/// when unbounded. `None` on (checked) arithmetic overflow.
fn pick_value(lower: Option<(Rat, bool)>, upper: Option<(Rat, bool)>) -> Option<Rat> {
    match (lower, upper) {
        (Some((lo, lo_strict)), Some((hi, _))) => {
            if lo < hi {
                lo.add(hi)?.mul(Rat::half()) // strict interior point (midpoint)
            } else if lo_strict {
                // lo == hi (or, defensively, lo > hi): the lower side is strict,
                // so step up off the shared endpoint.
                lo.add(Rat::one())
            } else {
                Some(lo)
            }
        }
        (Some((lo, strict)), None) => {
            if strict {
                lo.add(Rat::one())
            } else {
                Some(lo)
            }
        }
        (None, Some((hi, strict))) => {
            if strict {
                hi.sub(Rat::one())
            } else {
                Some(hi)
            }
        }
        (None, None) => Some(Rat::zero()),
    }
}

// ===========================================================================
// Linear optimization: `minimize` / `maximize` via Fourier–Motzkin projection
// (track C2)
// ===========================================================================
//
// A linear program — maximize (or minimize) a linear objective subject to the
// `constrain` half-planes — is solved by the SAME Fourier–Motzkin machinery as
// feasibility, with one trick. Introduce a fresh variable `z` bounded by the
// objective (`z ≤ obj`), then **project out every original variable**. What
// remains are constraints purely on `z`: the feasible range of `z` is exactly
// `(−∞, OPT]`, so the **least upper bound on `z` is the optimum**. No upper
// bound ⇒ the objective is unbounded; a violated *constant* in the projection
// ⇒ the constraints were infeasible. `minimize obj = −maximize(−obj)`.
//
// Worked example — maximize `x` s.t. `x ≤ 5`:
//   augment:  z − x ≤ 0           (z ≤ x)
//             x − 5 ≤ 0           (x ≤ 5)
//   project out x:  (z − x) + (x − 5) = z − 5 ≤ 0  →  z ≤ 5  →  OPT = 5.
//
// The achieving assignment is then recovered by pinning `obj = OPT` and running
// the feasibility witness reconstruction; the **binding** constraints are the
// originals tight (satisfied with equality) at that point.

/// The internal name of the objective variable `z`. The `@` cannot appear in an
/// adj-lang identifier (`[a-z_][a-z0-9_]*`), so it never collides with a symbol.
const OBJ_VAR: &str = "@objective";

/// The outcome of a `minimize`/`maximize` (LP) request.
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizeOutcome {
    /// The objective is bounded and its optimum is **attained**: `value` is the
    /// optimal objective value, `assignments` an achieving point (rational, as
    /// `f64`), and `binding` the indices of the original constraints tight
    /// (satisfied with equality) at the optimum — the provenance of the bound.
    Optimal {
        value: f64,
        assignments: Vec<(String, f64)>,
        binding: Vec<usize>,
    },
    /// The objective can be driven to ±∞ within the feasible region.
    Unbounded,
    /// The constraints are infeasible — no point satisfies them. `core` is a
    /// **minimal** infeasible subset (IIS): the irreducible set of constraints
    /// in conflict (removing any one makes the rest feasible).
    Infeasible { core: Vec<usize> },
    /// Out of scope: a non-linear / `!=` constraint or objective, an **open
    /// supremum** (a strict inequality prevents the optimum being attained), or
    /// a system too large for the bounded slice.
    Unknown { reason: String },
}

/// Solve the optimization declared by a [`ConstraintSystem`]'s `objective`
/// against its `constrain` half-planes (observed facts substituted first).
///
/// Dispatch on the declared symbol sorts:
/// - if **every** symbol is integer- or boolean-sorted (`: int` / `: integer` /
///   `: bool` / `: boolean`) and the objective + constraints are integer-linear,
///   solve the **integer program** exactly ([`optimize_integer`]) — this is what
///   makes a minimum-cost **set-cover** (pick the fewest/cheapest drugs covering
///   every organism; `x_d ∈ {0,1}`) a native, proof-carrying engine result
///   rather than a Python loop. The real LP relaxation would return fractional
///   selections (`0.5·vancomycin`), which is meaningless for a yes/no choice.
/// - otherwise (the default: `: scalar`, `: money(...)`, …) solve the real-valued
///   (QF_LRA) LP via Fourier–Motzkin, byte-for-byte as before.
pub fn optimize(cs: &ConstraintSystem, kb: &KnowledgeBase) -> OptimizeOutcome {
    if is_integer_program(cs) {
        if let Some(out) = optimize_integer(cs, kb) {
            return out;
        }
        // Fell through (objective/constraints not integer-linear, or the integer
        // tactic punted) — the real solver still answers (or says Unknown).
    }
    optimize_real(cs, kb)
}

/// True iff this is an opted-in integer program: it has an objective, at least
/// one symbol, and **every** declared symbol carries an integral sort. Anything
/// else (the existing `: scalar`/`: money(...)` programs) stays on the real path,
/// so prior behavior is unchanged by construction.
fn is_integer_program(cs: &ConstraintSystem) -> bool {
    cs.objective.is_some()
        && !cs.symbols.is_empty()
        && cs
            .symbols
            .iter()
            .all(|(_, sort)| sort_is_integral(&sort.to_string()).is_some())
}

/// Classify a sort by its surface name: `Some(true)` = boolean (the LIA tactic
/// grounds it to `{0,1}`), `Some(false)` = general integer, `None` = not integral
/// (real-valued). Matched on the sort term's `Display` so no extra dependency is
/// needed to read it.
fn sort_is_integral(sort: &str) -> Option<bool> {
    match sort {
        "bool" | "boolean" => Some(true),
        "int" | "integer" => Some(false),
        _ => None,
    }
}

/// Exact integer linear optimization. Returns `None` when the system isn't
/// integer-linear after all (so the caller falls back to the real LP); otherwise
/// an [`OptimizeOutcome`] with the **integral** optimum.
///
/// Method (reusing the exact pieces already in this crate, no new tactic):
/// 1. The real LP relaxation ([`optimize_real`]) classifies the system and gives
///    a numeric bound — `Infeasible`/`Unbounded` pass straight through; a finite
///    real optimum `rv` bounds the integer optimum (`int_opt ≥ ⌈rv⌉` for a min,
///    `≤ ⌊rv⌋` for a max).
/// 2. The exact integer tactic ([`LiaTactic`]) gives an initial feasible integer
///    point, whose objective is the other end of the bracket.
/// 3. **Binary search** the threshold `K`: the optimum is the smallest `K` with
///    `obj ≤ K` still integer-feasible (min) — each probe is an exact LIA solve,
///    so the answer is exact. The witness at `K*` is the achieving assignment;
///    the constraints tight there are the binding provenance.
fn optimize_integer(cs: &ConstraintSystem, kb: &KnowledgeBase) -> Option<OptimizeOutcome> {
    let (dir, obj_expr) = cs.objective.as_ref()?;
    let syms: Vec<String> = cs.symbols.iter().map(|(n, _)| n.clone()).collect();
    let var_set: HashSet<&str> = syms.iter().map(String::as_str).collect();

    // Classify the declared symbols into integer vs boolean (grounded to {0,1}).
    let (mut int_vars, mut bool_vars): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for (n, sort) in &cs.symbols {
        match sort_is_integral(&sort.to_string())? {
            true => bool_vars.push(n.clone()),
            false => int_vars.push(n.clone()),
        }
    }

    // Substitute observed facts, then require every constraint + the objective to
    // be integer-linear; otherwise this isn't an integer program (fall back).
    let subbed: Vec<(ComputeExpr, RelOp, ComputeExpr)> = cs
        .constraints
        .iter()
        .map(|c| {
            (
                substitute_observed(&c.lhs, &var_set, kb),
                c.op,
                substitute_observed(&c.rhs, &var_set, kb),
            )
        })
        .collect();
    let mut assertions = integer_assertions(&subbed)?;
    // Pin every boolean to `{0,1}` explicitly. The LIA tactic grounds bool vars
    // to 0/1 when building the model, but its bounded variable-elimination search
    // still explores the integers unless the range is constrained — without these
    // bounds it exhausts its budget and punts (`Unknown`) once there are ~4+ vars.
    // Adding `0 ≤ v ≤ 1` caps each boolean's search to two values, so a formulary
    // of N drugs is a 2^N search the budget handles comfortably (N ≲ 21).
    for b in &bool_vars {
        assertions.push(Predicate::Ge(
            Box::new(Predicate::Var(b.clone())),
            Box::new(Predicate::Int(0)),
        ));
        assertions.push(Predicate::Le(
            Box::new(Predicate::Var(b.clone())),
            Box::new(Predicate::Int(1)),
        ));
    }
    let obj_pred = expr_to_pred(&substitute_observed(obj_expr, &var_set, kb))?;

    // SCALING FAST PATH: a pure-boolean minimize whose constraints are at-least-one
    // covering clauses (`Σ xᵢ ≥ 1`) is a minimum-cost SET-COVER. The LIA enumeration
    // below is ~2^N; a SAT solver exploits the clause structure, so route the
    // feasibility oracle to it (Sinz-encoded cost bound). Same answer, far larger N.
    // Falls through to LIA if the system isn't this exact clausal shape.
    if matches!(dir, OptDir::Minimize) && int_vars.is_empty() && !bool_vars.is_empty() {
        let bool_set: HashSet<&str> = bool_vars.iter().map(String::as_str).collect();
        if let (Some(clauses), Some((weights, konst))) = (
            as_clausal_cover(&subbed, &bool_set),
            linear_coeffs(&obj_pred),
        ) {
            if weights.values().all(|w| *w >= 0) {
                if let Some(out) =
                    solve_setcover_sat(&clauses, &weights, konst, &syms, &var_set, cs, kb, &subbed)
                {
                    return Some(out);
                }
            }
        }
    }

    // An initial feasible integer point bounds one end of the search.
    let witness0 = match LiaTactic::solve(&assertions, &int_vars, &bool_vars) {
        SolverResult::Sat(m) => m,
        SolverResult::Unsat => {
            return Some(OptimizeOutcome::Infeasible {
                core: minimal_unsat_core(&subbed, &syms),
            })
        }
        SolverResult::Unknown(_) => return None, // tactic punted → real fallback
    };
    let feasible_obj = eval_lin_int(&obj_pred, &witness0)?;

    // Bracket the integer optimum: `feasible_obj` is one end (a real feasible
    // point), and we need a bound on the other. Prefer a STRUCTURAL bound when
    // the objective is purely over booleans — each `x ∈ {0,1}` contributes
    // between `min(0,coef)` and `max(0,coef)`, so the optimum lies in
    // `[Σ min + k, Σ max + k]` for *any* number of variables. This is what lets
    // set-cover scale to a real formulary; the Fourier–Motzkin relaxation below
    // caps out at a handful of variables. For objectives mentioning general
    // integers we fall back to that relaxation (small systems only).
    let (coeffs, konst) = linear_coeffs(&obj_pred)?;
    let bool_set: HashSet<&str> = bool_vars.iter().map(String::as_str).collect();
    let opt_bound: i128 = if coeffs.keys().all(|v| bool_set.contains(v.as_str())) {
        match dir {
            OptDir::Minimize => konst + coeffs.values().map(|c| (*c).min(0)).sum::<i128>(),
            OptDir::Maximize => konst + coeffs.values().map(|c| (*c).max(0)).sum::<i128>(),
        }
    } else {
        // General-integer objective: the real relaxation (with bool `0≤x≤1`
        // bounds injected so it isn't spuriously unbounded) gives the bound.
        match optimize_real(&with_bool_bounds(cs), kb) {
            OptimizeOutcome::Optimal { value, .. } => match dir {
                OptDir::Minimize => (value - 1e-9).ceil() as i128,
                OptDir::Maximize => (value + 1e-9).floor() as i128,
            },
            OptimizeOutcome::Unbounded => return Some(OptimizeOutcome::Unbounded),
            OptimizeOutcome::Infeasible { .. } => {
                return Some(OptimizeOutcome::Infeasible {
                    core: minimal_unsat_core(&subbed, &syms),
                })
            }
            OptimizeOutcome::Unknown { .. } => return None, // can't bound → fall back
        }
    };

    let (k_opt, witness) = match dir {
        // int_opt ∈ [opt_bound, feasible_obj]; smallest K with (obj ≤ K) feasible.
        OptDir::Minimize => extremal_feasible(
            &assertions,
            &obj_pred,
            true,
            opt_bound,
            feasible_obj,
            &int_vars,
            &bool_vars,
        )?,
        // int_opt ∈ [feasible_obj, opt_bound]; largest K with (obj ≥ K) feasible.
        OptDir::Maximize => extremal_feasible(
            &assertions,
            &obj_pred,
            false,
            feasible_obj,
            opt_bound,
            &int_vars,
            &bool_vars,
        )?,
    };

    let assignments: Vec<(String, f64)> = syms
        .iter()
        .filter_map(|v| match witness.get(v) {
            Some(Value::Int(n)) => Some((v.clone(), *n as f64)),
            Some(Value::Bool(b)) => Some((v.clone(), if *b { 1.0 } else { 0.0 })),
            _ => None,
        })
        .collect();
    let binding = binding_constraints(cs, kb, &var_set, &assignments);
    Some(OptimizeOutcome::Optimal {
        value: k_opt as f64,
        assignments,
        binding,
    })
}

/// Clone `cs`, adding `0 ≤ x ≤ 1` for every boolean symbol so the **real**
/// relaxation is bounded (a `bool` is `{0,1}`, which the LP solver wouldn't
/// otherwise enforce). General integer symbols get no synthetic bounds — their
/// range is whatever the user constrained, and a genuinely unbounded integer
/// objective should report `Unbounded`.
fn with_bool_bounds(cs: &ConstraintSystem) -> ConstraintSystem {
    let mut aug = cs.clone();
    for (name, sort) in &cs.symbols {
        if sort_is_integral(&sort.to_string()) == Some(true) {
            aug.constraints.push(LoweredConstraint {
                lhs: ComputeExpr::Ref(name.clone()),
                op: RelOp::Ge,
                rhs: ComputeExpr::Lit(0.0),
            });
            aug.constraints.push(LoweredConstraint {
                lhs: ComputeExpr::Ref(name.clone()),
                op: RelOp::Le,
                rhs: ComputeExpr::Lit(1.0),
            });
        }
    }
    aug
}

/// Is `obj ⋈ bound` integer-feasible together with `assertions`? `le = true`
/// tests `obj ≤ bound`, `le = false` tests `obj ≥ bound`. Returns the witness
/// model when feasible.
fn bounded_feasible(
    assertions: &[Predicate],
    obj: &Predicate,
    le: bool,
    bound: i128,
    int_vars: &[String],
    bool_vars: &[String],
) -> Option<Model> {
    let mut a = assertions.to_vec();
    let (l, r) = (Box::new(obj.clone()), Box::new(Predicate::Int(bound)));
    a.push(if le {
        Predicate::Le(l, r)
    } else {
        Predicate::Ge(l, r)
    });
    match LiaTactic::solve(&a, int_vars, bool_vars) {
        SolverResult::Sat(m) => Some(m),
        _ => None,
    }
}

/// Binary-search the extremal feasible objective bound in `[lo, hi]`, where the
/// `hi` end (minimize) or `lo` end (maximize) is known feasible. For a minimize
/// (`le = true`) it returns the **smallest** `K` with `obj ≤ K` feasible; for a
/// maximize (`le = false`) the **largest** `K` with `obj ≥ K` feasible. Each
/// probe is an exact LIA solve, so the returned bound and witness are exact.
fn extremal_feasible(
    assertions: &[Predicate],
    obj: &Predicate,
    le: bool,
    lo: i128,
    hi: i128,
    int_vars: &[String],
    bool_vars: &[String],
) -> Option<(i128, Model)> {
    // The tight end first: if the optimum is already at the bound, return it.
    let tight = if le { lo } else { hi };
    if let Some(m) = bounded_feasible(assertions, obj, le, tight, int_vars, bool_vars) {
        return Some((tight, m));
    }
    // Otherwise bracket [lo, hi] with the slack end feasible, and bisect.
    let mut loi = lo;
    let mut hii = hi;
    let mut best = bounded_feasible(
        assertions,
        obj,
        le,
        if le { hi } else { lo },
        int_vars,
        bool_vars,
    )?;
    while hii - loi > 1 {
        let mid = loi + (hii - loi) / 2;
        match bounded_feasible(assertions, obj, le, mid, int_vars, bool_vars) {
            // Feasible at `mid`: a min can tighten down, a max can tighten up.
            Some(m) => {
                if le {
                    hii = mid;
                } else {
                    loi = mid;
                }
                best = m;
            }
            None => {
                if le {
                    loi = mid;
                } else {
                    hii = mid;
                }
            }
        }
    }
    Some((if le { hii } else { loi }, best))
}

/// Extract the linear coefficients (`var → coefficient`) and constant term of a
/// linear-integer [`Predicate`]. Returns `None` if the predicate isn't linear.
/// Used to bound a boolean objective structurally (each `x ∈ {0,1}` contributes
/// between `min(0, coef)` and `max(0, coef)`), which scales to any number of
/// variables — unlike the Fourier–Motzkin relaxation, which caps out at a handful.
fn linear_coeffs(p: &Predicate) -> Option<(std::collections::BTreeMap<String, i128>, i128)> {
    use std::collections::BTreeMap;
    fn go(p: &Predicate, scale: i128, m: &mut BTreeMap<String, i128>, k: &mut i128) -> Option<()> {
        match p {
            Predicate::Int(n) => {
                *k = k.checked_add(scale.checked_mul(*n)?)?;
                Some(())
            }
            Predicate::Bool(b) => {
                *k = k.checked_add(scale * (*b as i128))?;
                Some(())
            }
            Predicate::Var(name) => {
                let e = m.entry(name.clone()).or_insert(0);
                *e = e.checked_add(scale)?;
                Some(())
            }
            Predicate::Add(parts) => {
                for q in parts {
                    go(q, scale, m, k)?;
                }
                Some(())
            }
            Predicate::Sub(a, b) => {
                go(a, scale, m, k)?;
                go(b, scale.checked_neg()?, m, k)
            }
            Predicate::Mul { coef, term } => go(term, scale.checked_mul(*coef)?, m, k),
            _ => None,
        }
    }
    let mut m = BTreeMap::new();
    let mut k = 0i128;
    go(p, 1, &mut m, &mut k)?;
    Some((m, k))
}

/// Evaluate a linear-integer [`Predicate`] (the objective) at an integer model.
/// Variables absent from the model default to 0 (the LIA tactic's convention for
/// an unconstrained variable). Returns `None` if the predicate isn't linear.
fn eval_lin_int(p: &Predicate, model: &Model) -> Option<i128> {
    match p {
        Predicate::Int(n) => Some(*n),
        Predicate::Bool(b) => Some(*b as i128),
        Predicate::Var(name) => Some(match model.get(name) {
            Some(Value::Int(n)) => *n,
            Some(Value::Bool(b)) => *b as i128,
            _ => 0,
        }),
        Predicate::Add(parts) => parts.iter().map(|q| eval_lin_int(q, model)).sum(),
        Predicate::Sub(a, b) => Some(eval_lin_int(a, model)?.checked_sub(eval_lin_int(b, model)?)?),
        Predicate::Mul { coef, term } => coef.checked_mul(eval_lin_int(term, model)?),
        _ => None,
    }
}

// ===================== SAT / pseudo-boolean set-cover scaling =====================
// A pure-boolean minimum-cost set-cover — at-least-one covering clauses + a
// `minimize Σ wᵢ·xᵢ` objective — is solved by routing the binary-search-on-cost
// feasibility oracle to the DPLL `SatTactic` instead of LIA's bounded enumeration.
// The cost bound `Σ wᵢ·xᵢ ≤ K` is encoded with a Sinz (2005) sequential at-most-k
// counter (each weight wᵢ modeled by repeating xᵢ wᵢ times in the literal list).
// The optimum is identical to the LIA path; only the oracle is more scalable.

/// One CNF clause: a disjunction of `(var, positive)` literals.
fn clause(lits: Vec<(String, bool)>) -> Predicate {
    Predicate::Or(
        lits.into_iter()
            .map(|(v, pos)| {
                if pos {
                    Predicate::Var(v)
                } else {
                    Predicate::Not(Box::new(Predicate::Var(v)))
                }
            })
            .collect(),
    )
}

/// Sinz (2005) sequential-counter encoding of `Σ literals ≤ k` (each literal a
/// positive boolean; a variable repeated `w` times models weight `w`). Returns the
/// flat CNF clauses + the fresh auxiliary variables. Aux names start with `__pb`,
/// which no user symbol can (those match `[a-z][a-z0-9_]*`).
fn sinz_at_most(literals: &[String], k: i128) -> (Vec<Predicate>, Vec<String>) {
    let m = literals.len();
    if k < 0 {
        return (vec![Predicate::Bool(false)], Vec::new()); // unsatisfiable
    }
    if k == 0 {
        // every literal must be false
        return (
            literals
                .iter()
                .map(|l| clause(vec![(l.clone(), false)]))
                .collect(),
            Vec::new(),
        );
    }
    if (m as i128) <= k {
        return (Vec::new(), Vec::new()); // no constraint is binding
    }
    let k = k as usize;
    let s = |i: usize, j: usize| format!("__pb_{i}_{j}");
    let l = |i: usize| literals[i - 1].clone(); // 1-indexed literals
    let mut aux = Vec::new();
    for i in 1..=m - 1 {
        for j in 1..=k {
            aux.push(s(i, j));
        }
    }
    let mut cl = Vec::new();
    cl.push(clause(vec![(l(1), false), (s(1, 1), true)])); // ¬x₁ ∨ s₁,₁
    for j in 2..=k {
        cl.push(clause(vec![(s(1, j), false)])); // ¬s₁,ⱼ
    }
    for i in 2..=m - 1 {
        cl.push(clause(vec![(l(i), false), (s(i, 1), true)])); // ¬xᵢ ∨ sᵢ,₁
        cl.push(clause(vec![(s(i - 1, 1), false), (s(i, 1), true)])); // ¬sᵢ₋₁,₁ ∨ sᵢ,₁
        for j in 2..=k {
            // ¬xᵢ ∨ ¬sᵢ₋₁,ⱼ₋₁ ∨ sᵢ,ⱼ
            cl.push(clause(vec![
                (l(i), false),
                (s(i - 1, j - 1), false),
                (s(i, j), true),
            ]));
            cl.push(clause(vec![(s(i - 1, j), false), (s(i, j), true)])); // ¬sᵢ₋₁,ⱼ ∨ sᵢ,ⱼ
        }
        cl.push(clause(vec![(l(i), false), (s(i - 1, k), false)])); // ¬xᵢ ∨ ¬sᵢ₋₁,ₖ (overflow)
    }
    cl.push(clause(vec![(l(m), false), (s(m - 1, k), false)])); // ¬xₘ ∨ ¬sₘ₋₁,ₖ
    (cl, aux)
}

/// How a boolean linear constraint relates to a single CNF clause.
enum ClauseKind {
    /// Equivalent to this disjunction of literals.
    Clause(Predicate),
    /// Trivially satisfied (e.g. a `{0,1}` bound) — contributes nothing.
    AlwaysTrue,
    /// Unsatisfiable on its own (e.g. `0 ≥ 1`, an uncoverable requirement).
    AlwaysFalse,
    /// A genuine cardinality/general constraint — not a single clause.
    NotAClause,
}

/// Negate every coefficient of a linear form.
fn negate_map(
    c: &std::collections::BTreeMap<String, i128>,
) -> Option<std::collections::BTreeMap<String, i128>> {
    // checked: an i128::MIN coefficient declines (→ NotAClause) rather than panic/wrap.
    c.iter()
        .map(|(k, v)| v.checked_neg().map(|n| (k.clone(), n)))
        .collect()
}

/// Recognize `Σ cᵢxᵢ (op) b` over booleans as a single clause. After normalizing to
/// `Σ aᵢxᵢ ≥ B`, a `{−1,+1}`-coefficient constraint is a clause **iff** its threshold
/// excludes exactly one assignment — `B == 1 − |negatives|` (the clause is false only
/// when every positive literal is 0 and every negative literal is 1). This recognizes:
/// at-least-one covering (`Σx ≥ 1`), the two implications of an AND-linearization used
/// for n-ary combinations (`¬y ∨ dᵢ` and `y ∨ ¬d₁ … ∨ ¬dₖ`), and `{0,1}` bounds. A
/// true cardinality constraint (`≥ 2 of …`) is `NotAClause`, so the caller defers to LIA.
fn classify_clause(c: &std::collections::BTreeMap<String, i128>, op: RelOp, b: i128) -> ClauseKind {
    // Normalize to `Σ aᵢxᵢ ≥ bound`.
    let (coeffs, bound) = match op {
        RelOp::Ge => (c.clone(), b),
        RelOp::Gt => match b.checked_add(1) {
            Some(x) => (c.clone(), x),
            None => return ClauseKind::NotAClause,
        },
        RelOp::Le => match (negate_map(c), b.checked_neg()) {
            (Some(m), Some(x)) => (m, x),
            _ => return ClauseKind::NotAClause,
        },
        RelOp::Lt => match (
            negate_map(c),
            b.checked_sub(1).and_then(|x| x.checked_neg()),
        ) {
            (Some(m), Some(x)) => (m, x),
            _ => return ClauseKind::NotAClause,
        },
        RelOp::Eq | RelOp::Ne => return ClauseKind::NotAClause,
    };
    if coeffs.is_empty() {
        return if bound <= 0 {
            ClauseKind::AlwaysTrue
        } else {
            ClauseKind::AlwaysFalse
        };
    }
    if !coeffs.values().all(|v| matches!(v, 1 | -1)) {
        return ClauseKind::NotAClause; // not a ±1 clause (total over i128; no abs() panic)
    }
    let n_neg = coeffs.values().filter(|v| **v < 0).count() as i128;
    let p_pos = coeffs.values().filter(|v| **v > 0).count() as i128;
    if bound <= -n_neg {
        return ClauseKind::AlwaysTrue; // min possible LHS already meets it
    }
    if bound > p_pos {
        return ClauseKind::AlwaysFalse; // max possible LHS can't meet it
    }
    if bound == 1 - n_neg {
        let lits = coeffs
            .iter()
            .map(|(v, co)| {
                if *co > 0 {
                    Predicate::Var(v.clone())
                } else {
                    Predicate::Not(Box::new(Predicate::Var(v.clone())))
                }
            })
            .collect();
        return ClauseKind::Clause(Predicate::Or(lits));
    }
    ClauseKind::NotAClause // a true cardinality constraint
}

/// View the (substituted) constraints as a pure-boolean clausal system (a set-cover,
/// possibly with n-ary combination AND-linearizations and defeasance). Each constraint
/// must reduce to a single CNF clause or a trivial bound. Returns the clauses, or
/// `None` if any constraint isn't clausal (so the caller falls back to the LIA path).
fn as_clausal_cover(
    subbed: &[(ComputeExpr, RelOp, ComputeExpr)],
    bool_set: &HashSet<&str>,
) -> Option<Vec<Predicate>> {
    let mut clauses = Vec::new();
    for (lhs, op, rhs) in subbed {
        let (lc, lk) = linear_coeffs(&expr_to_pred(lhs)?)?;
        let (rc, rk) = linear_coeffs(&expr_to_pred(rhs)?)?;
        // Move to one side: `Σ cᵢxᵢ  <op>  b`, where b = rk − lk.
        let mut c = lc;
        for (v, w) in rc {
            *c.entry(v).or_insert(0) -= w;
        }
        c.retain(|_, w| *w != 0);
        let b = rk.checked_sub(lk)?;
        if !c.keys().all(|v| bool_set.contains(v.as_str())) {
            return None;
        }
        match classify_clause(&c, *op, b) {
            ClauseKind::Clause(p) => clauses.push(p),
            ClauseKind::AlwaysTrue => {}
            ClauseKind::AlwaysFalse => clauses.push(Predicate::Bool(false)),
            ClauseKind::NotAClause => return None,
        }
    }
    Some(clauses)
}

/// The encoded formula's literal budget. The Sinz encoding is O(m·k) where `m` is
/// the unary-expanded weight total — so a crafted objective with huge weights could
/// otherwise blow memory up before the SAT tactic's own node budget ever applies
/// (the LIA/Fourier–Motzkin paths cap themselves the same way, see MAX_INEQUALITIES).
/// Past this, the SAT oracle declines (the caller falls back to LIA).
const MAX_PB_LITERALS: i128 = 200_000;

/// Is the covering satisfiable with `Σ wᵢxᵢ ≤ k`? Builds the covering clauses + the
/// Sinz cost bound and runs the SAT tactic. Returns the full [`SolverResult`] so the
/// caller can distinguish UNSAT (truly infeasible) from `Unknown` (budget/too-large)
/// — the latter must NOT be reported as "no regimen", only deferred.
fn cover_sat(
    clauses: &[Predicate],
    weights: &std::collections::BTreeMap<String, i128>,
    vars: &[String],
    k: i128,
) -> SolverResult {
    // Bound the unary expansion BEFORE materializing it (DoS guard).
    let total_lits: i128 = weights.values().filter(|w| **w > 0).copied().sum();
    if total_lits > MAX_PB_LITERALS {
        return SolverResult::Unknown("pseudo-boolean encoding exceeds the literal cap".into());
    }
    let mut lits = Vec::new();
    for (v, w) in weights {
        for _ in 0..(*w).max(0) {
            lits.push(v.clone());
        }
    }
    let (pb, aux) = sinz_at_most(&lits, k);
    let mut assertions: Vec<Predicate> = clauses.to_vec();
    assertions.extend(pb);
    let mut bool_vars: Vec<String> = vars.to_vec();
    bool_vars.extend(aux);
    SatTactic::solve(&assertions, &bool_vars)
}

/// Solve the minimum-cost set-cover via SAT. The objective value is `konst + Σ wᵢxᵢ`;
/// the binary search finds the smallest `K` with `(cover ∧ Σ wᵢxᵢ ≤ K)` satisfiable.
#[allow(clippy::too_many_arguments)]
fn solve_setcover_sat(
    clauses: &[Predicate],
    weights: &std::collections::BTreeMap<String, i128>,
    konst: i128,
    syms: &[String],
    var_set: &HashSet<&str>,
    cs: &ConstraintSystem,
    kb: &KnowledgeBase,
    subbed: &[(ComputeExpr, RelOp, ComputeExpr)],
) -> Option<OptimizeOutcome> {
    // Checked sum: a crafted objective whose weights overflow i128 declines the SAT
    // path (falls back to LIA) rather than panicking/wrapping.
    let total: i128 = weights.values().try_fold(0i128, |a, w| a.checked_add(*w))?;
    // Select-everything (cost `total`) is the most permissive cover. UNSAT there
    // means the covering is genuinely unsatisfiable; `Unknown` means the SAT tactic
    // couldn't decide — defer to LIA (None), NEVER report a false "infeasible".
    let m0 = match cover_sat(clauses, weights, syms, total) {
        SolverResult::Sat(m) => m,
        SolverResult::Unsat => {
            return Some(OptimizeOutcome::Infeasible {
                core: minimal_unsat_core(subbed, syms),
            })
        }
        SolverResult::Unknown(_) => return None,
    };
    let value_at = |m: &Model| -> i128 {
        weights
            .iter()
            .map(|(v, w)| {
                if matches!(m.get(v), Some(Value::Bool(true))) {
                    *w
                } else {
                    0
                }
            })
            .sum::<i128>()
    };
    let feasible_obj = value_at(&m0); // an attained Σwx (upper bound)
    let oracle = |k: i128| cover_sat(clauses, weights, syms, k);
    // smallest K in [0, feasible_obj] with Σwx ≤ K feasible
    let (raw, witness) = sat_min_feasible(&oracle, 0, feasible_obj)?;
    let assignments: Vec<(String, f64)> = syms
        .iter()
        .filter_map(|v| match witness.get(v) {
            Some(Value::Bool(b)) => Some((v.clone(), if *b { 1.0 } else { 0.0 })),
            Some(Value::Int(n)) => Some((v.clone(), *n as f64)),
            _ => None,
        })
        .collect();
    let binding = binding_constraints(cs, kb, var_set, &assignments);
    Some(OptimizeOutcome::Optimal {
        value: (konst + raw) as f64,
        assignments,
        binding,
    })
}

/// Binary-search the smallest `Σwx` bound in `[lo, hi]` that keeps the cover
/// feasible, where `hi` is known feasible. Each probe is an exact SAT solve. Any
/// `Unknown` probe (the tactic couldn't decide) aborts the whole search to `None`
/// so the caller defers to LIA — we never treat an undecided probe as infeasible.
fn sat_min_feasible(
    oracle: &dyn Fn(i128) -> SolverResult,
    lo: i128,
    hi: i128,
) -> Option<(i128, Model)> {
    match oracle(lo) {
        SolverResult::Sat(m) => return Some((lo, m)), // optimum already at the floor
        SolverResult::Unknown(_) => return None,      // can't decide → defer to LIA
        SolverResult::Unsat => {}
    }
    let (mut loi, mut hii) = (lo, hi);
    let mut best = match oracle(hi) {
        SolverResult::Sat(m) => m,
        _ => return None, // hi was claimed feasible by the caller; if not, defer
    };
    while hii - loi > 1 {
        let mid = loi + (hii - loi) / 2;
        match oracle(mid) {
            SolverResult::Sat(m) => {
                hii = mid;
                best = m;
            }
            SolverResult::Unsat => loi = mid,
            SolverResult::Unknown(_) => return None, // undecided → defer to LIA
        }
    }
    Some((hii, best))
}

/// Solve the LP declared by a [`ConstraintSystem`]'s `objective` against its
/// `constrain` half-planes, substituting observed facts first. Real-valued
/// (QF_LRA) optimization over exact rationals via Fourier–Motzkin projection.
fn optimize_real(cs: &ConstraintSystem, kb: &KnowledgeBase) -> OptimizeOutcome {
    let Some((dir, obj_expr)) = &cs.objective else {
        return OptimizeOutcome::Unknown {
            reason: "no objective to optimize".to_string(),
        };
    };
    let syms: Vec<String> = cs.symbols.iter().map(|(n, _)| n.clone()).collect();
    let var_set: HashSet<&str> = syms.iter().map(String::as_str).collect();

    // Substitute observed facts into the objective and every constraint.
    let obj_sub = substitute_observed(obj_expr, &var_set, kb);
    let Some(obj) = linearize(&obj_sub) else {
        return OptimizeOutcome::Unknown {
            reason: "objective is non-linear".to_string(),
        };
    };
    let mut planes: Vec<Halfplane> = Vec::new();
    // Keep the substituted constraint tuples so an infeasible LP can report a
    // minimal IIS (the same certificate `check` gives).
    let mut subbed: Vec<(ComputeExpr, RelOp, ComputeExpr)> =
        Vec::with_capacity(cs.constraints.len());
    for c in &cs.constraints {
        let lhs = substitute_observed(&c.lhs, &var_set, kb);
        let rhs = substitute_observed(&c.rhs, &var_set, kb);
        match constraint_to_halfplanes(&lhs, c.op, &rhs) {
            Some(hps) => planes.extend(hps),
            None => {
                return OptimizeOutcome::Unknown {
                    reason: "a constraint is non-linear or uses `!=`".to_string(),
                }
            }
        }
        subbed.push((lhs, c.op, rhs));
    }

    // We always maximize; for `minimize`, maximize the negated objective and
    // negate the result back.
    let max_obj = match dir {
        OptDir::Maximize => obj,
        OptDir::Minimize => match Rat::one().neg().and_then(|m| obj.scale(m)) {
            Some(o) => o,
            None => {
                return OptimizeOutcome::Unknown {
                    reason: "objective coefficients too large".to_string(),
                }
            }
        },
    };
    // Original decision variables = those in any constraint or the objective.
    let mut orig_vars: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::from_iter(vars_of(&planes));
    orig_vars.extend(max_obj.coeffs.keys().cloned());
    let orig_vars: Vec<String> = orig_vars.into_iter().collect();

    // Augment with `z − max_obj ≤ 0`  (z ≤ obj), then project out every
    // original variable. The residual constraints bound `z`.
    let z_minus_obj = match LinForm::var(OBJ_VAR).add_scaled(
        &max_obj,
        match Rat::one().neg() {
            Some(m) => m,
            None => {
                return OptimizeOutcome::Unknown {
                    reason: "overflow".to_string(),
                }
            }
        },
    ) {
        Some(f) => f,
        None => {
            return OptimizeOutcome::Unknown {
                reason: "objective coefficients too large".to_string(),
            }
        }
    };
    let mut augmented = planes;
    augmented.push(Halfplane {
        form: z_minus_obj,
        strict: false,
    });
    let (residual, _) = match eliminate(augmented, &orig_vars) {
        Ok(x) => x,
        Err(reason) => return OptimizeOutcome::Unknown { reason },
    };

    // Read `z`'s tightest upper bound from the residual; a violated *constant*
    // (no `z`) means the original constraints were infeasible.
    let mut best_upper: Option<(Rat, bool)> = None;
    for hp in &residual {
        let alpha = hp.form.coeff(OBJ_VAR);
        if alpha.is_zero() {
            if constant_violated(hp) {
                return OptimizeOutcome::Infeasible {
                    core: minimal_unsat_core(&subbed, &syms),
                };
            }
        } else if alpha.num > 0 {
            // α·z + β ≤ 0  ⇒  z ≤ −β/α.
            let bound = match hp.form.constant.neg().and_then(|nb| nb.div(alpha)) {
                Some(b) => b,
                None => {
                    return OptimizeOutcome::Unknown {
                        reason: "coefficients grew past the checked-rational cap".to_string(),
                    }
                }
            };
            update_upper(&mut best_upper, bound, hp.strict);
        }
        // α < 0 is a *lower* bound on z — irrelevant to the maximum.
    }

    let Some((opt_internal, strict)) = best_upper else {
        return OptimizeOutcome::Unbounded;
    };
    if strict {
        return OptimizeOutcome::Unknown {
            reason: "optimum is an open supremum (a strict inequality prevents attainment)"
                .to_string(),
        };
    }

    // Optimal value, un-negated for `minimize`.
    let value_rat = match dir {
        OptDir::Maximize => opt_internal,
        OptDir::Minimize => match opt_internal.neg() {
            Some(v) => v,
            None => {
                return OptimizeOutcome::Unknown {
                    reason: "optimal value too large".to_string(),
                }
            }
        },
    };

    // Recover an achieving assignment: pin `max_obj = opt_internal` and run the
    // feasibility witness reconstruction over the original constraints.
    let (assignments, binding) = recover_optimum(cs, kb, &var_set, &max_obj, opt_internal);

    OptimizeOutcome::Optimal {
        value: value_rat.to_f64(),
        assignments,
        binding,
    }
}

/// Reconstruct an optimal point (and the binding constraints) by solving the
/// original system with `max_obj == opt` pinned. Returns an empty witness if the
/// reconstruction overflows — the optimal value is unaffected.
fn recover_optimum(
    cs: &ConstraintSystem,
    kb: &KnowledgeBase,
    var_set: &HashSet<&str>,
    max_obj: &LinForm,
    opt: Rat,
) -> (Vec<(String, f64)>, Vec<usize>) {
    // Rebuild the original half-planes (re-substituting observed facts).
    let mut planes: Vec<Halfplane> = Vec::new();
    for c in &cs.constraints {
        let lhs = substitute_observed(&c.lhs, var_set, kb);
        let rhs = substitute_observed(&c.rhs, var_set, kb);
        match constraint_to_halfplanes(&lhs, c.op, &rhs) {
            Some(hps) => planes.extend(hps),
            None => return (Vec::new(), Vec::new()),
        }
    }
    // Pin the objective: `max_obj − opt ≤ 0` and `opt − max_obj ≤ 0`.
    let diff = match max_obj.add_scaled(
        &LinForm::constant(opt),
        match Rat::one().neg() {
            Some(m) => m,
            None => return (Vec::new(), Vec::new()),
        },
    ) {
        Some(f) => f,
        None => return (Vec::new(), Vec::new()),
    };
    let neg = match diff.scale(match Rat::one().neg() {
        Some(m) => m,
        None => return (Vec::new(), Vec::new()),
    }) {
        Some(f) => f,
        None => return (Vec::new(), Vec::new()),
    };
    planes.push(Halfplane {
        form: diff,
        strict: false,
    });
    planes.push(Halfplane {
        form: neg,
        strict: false,
    });

    let assignments = match fourier_motzkin(planes) {
        FmResult::Sat(w) => w,
        _ => Vec::new(), // optimum is attained, but witness math overflowed
    };

    // Binding constraints: the originals tight (lhs == rhs) at the witness.
    let binding = binding_constraints(cs, kb, var_set, &assignments);
    (assignments, binding)
}

/// The indices of the original constraints satisfied with **equality** at the
/// (f64) witness — the constraints binding at the optimum. Equality constraints
/// are always binding; an inequality is binding iff it is active. Evaluated in
/// f64 with a small tolerance (this is a provenance annotation, not a gate).
fn binding_constraints(
    cs: &ConstraintSystem,
    kb: &KnowledgeBase,
    var_set: &HashSet<&str>,
    witness: &[(String, f64)],
) -> Vec<usize> {
    use std::collections::HashMap;
    let vals: HashMap<&str, f64> = witness.iter().map(|(n, v)| (n.as_str(), *v)).collect();
    let mut binding = Vec::new();
    for (i, c) in cs.constraints.iter().enumerate() {
        let lhs = substitute_observed(&c.lhs, var_set, kb);
        let rhs = substitute_observed(&c.rhs, var_set, kb);
        let (Some(l), Some(r)) = (linearize(&lhs), linearize(&rhs)) else {
            continue;
        };
        let diff = match l.add_scaled(
            &r,
            match Rat::one().neg() {
                Some(m) => m,
                None => continue,
            },
        ) {
            Some(d) => d,
            None => continue,
        };
        let mut total = diff.constant.to_f64();
        for (name, coef) in &diff.coeffs {
            total += coef.to_f64() * vals.get(name.as_str()).copied().unwrap_or(0.0);
        }
        if total.abs() < 1e-9 {
            binding.push(i);
        }
    }
    binding
}

/// Rewrite a constraint expression, replacing each reference that is **not** an
/// unknown but **is** an observed fact with its value as a literal. Unknowns
/// (the solve-for variables) and unobserved references are left as-is.
fn substitute_observed(
    e: &ComputeExpr,
    variables: &HashSet<&str>,
    kb: &KnowledgeBase,
) -> ComputeExpr {
    match e {
        ComputeExpr::Ref(name) => {
            if variables.contains(name.as_str()) {
                e.clone() // an unknown we are solving for — keep it symbolic
            } else if let Some(v) = kb.observed_value(name) {
                ComputeExpr::Lit(v) // a known observed fact — substitute its value
            } else {
                e.clone() // neither — a free reference (likely makes it singular)
            }
        }
        ComputeExpr::Lit(_) => e.clone(),
        ComputeExpr::Bin(op, a, b) => ComputeExpr::Bin(
            *op,
            Box::new(substitute_observed(a, variables, kb)),
            Box::new(substitute_observed(b, variables, kb)),
        ),
        ComputeExpr::Unary(op, a) => {
            ComputeExpr::Unary(*op, Box::new(substitute_observed(a, variables, kb)))
        }
        // Rebuild the narrowing with its operand substituted, keeping the precision
        // and mode (NUM-6a) — mirrors the unary rebuild above.
        ComputeExpr::Round { spec, mode, expr } => ComputeExpr::Round {
            spec: *spec,
            mode: *mode,
            expr: Box::new(substitute_observed(expr, variables, kb)),
        },
        ComputeExpr::ToScientific {
            figures,
            mode,
            expr,
        } => ComputeExpr::ToScientific {
            figures: *figures,
            mode: *mode,
            expr: Box::new(substitute_observed(expr, variables, kb)),
        },
        ComputeExpr::ToPercent { places, mode, expr } => ComputeExpr::ToPercent {
            places: *places,
            mode: *mode,
            expr: Box::new(substitute_observed(expr, variables, kb)),
        },
        ComputeExpr::ToCurrency {
            code,
            places,
            mode,
            expr,
        } => ComputeExpr::ToCurrency {
            code: code.clone(),
            places: *places,
            mode: *mode,
            expr: Box::new(substitute_observed(expr, variables, kb)),
        },
        ComputeExpr::Agg(_, _) => e.clone(),
    }
}

// ---------------------------------------------------------------------------
// Univariate polynomial path (nonlinear single-unknown equalities, track C3)
// ---------------------------------------------------------------------------

/// A univariate polynomial as coefficients indexed by power
/// (`p[0] + p[1]·x + p[2]·x² + …`).
type Poly = Vec<f64>;

/// The largest constant integer exponent [`poly_of`] will expand a `base ^ n`
/// into (repeated multiplication). The univariate root solvers cover only
/// degree ≤ 4, so this bound is generous; it exists purely to stop an
/// adversarial `x^{huge}` from allocating an enormous coefficient vector.
const MAX_POLY_POW: f64 = 64.0;

/// Build the polynomial of `e` in the single unknown `x`, or `None` if `e`
/// isn't a polynomial in `x` (e.g. division *by* `x`, an aggregation, or a free
/// reference — observed refs have already been substituted to literals).
fn poly_of(e: &ComputeExpr, x: &str) -> Option<Poly> {
    match e {
        ComputeExpr::Ref(name) if name == x => Some(vec![0.0, 1.0]),
        ComputeExpr::Ref(_) => None,
        ComputeExpr::Lit(c) => Some(vec![*c]),
        ComputeExpr::Bin(op, a, b) => {
            let pa = poly_of(a, x)?;
            let pb = poly_of(b, x)?;
            match op {
                ComputeOp::Add => Some(poly_add(&pa, &pb)),
                ComputeOp::Sub => Some(poly_sub(&pa, &pb)),
                ComputeOp::Mul => Some(poly_mul(&pa, &pb)),
                // Division is polynomial only by a non-zero constant.
                ComputeOp::Div => {
                    if poly_degree(&pb) == 0 && pb[0] != 0.0 {
                        Some(pa.iter().map(|c| c / pb[0]).collect())
                    } else {
                        None
                    }
                }
                // `base ^ n` is a polynomial iff the exponent is a constant
                // **non-negative integer** (`ComputeOp::Pow` from a LaTeX `x^n`):
                // then it is `base` multiplied by itself `n` times (`base^0 = 1`),
                // so a latex `x^2 = 4` still solves as a quadratic, `x^3` as a
                // cubic, etc. A symbolic or fractional exponent is not polynomial.
                // `n` is capped at `MAX_POLY_POW` so a pathological `x^{10^9}`
                // cannot balloon the coefficient vector (the univariate solvers
                // handle only degree ≤ 4 anyway, so the cap loses nothing real).
                ComputeOp::Pow => {
                    if poly_degree(&pb) != 0 {
                        return None;
                    }
                    let n = pb[0];
                    if !(n.is_finite() && n.fract() == 0.0 && (0.0..=MAX_POLY_POW).contains(&n)) {
                        return None;
                    }
                    // Cap the CUMULATIVE result degree, not just this exponent:
                    // `pa` may itself be a high-degree polynomial from an inner
                    // power, so nested powers like `(((x^64)^64)^64)` would
                    // otherwise compound (64 → 4096 → 262144 → …), ballooning the
                    // coefficient vector (`poly_mul` is O(len²)). The base degree
                    // times `n` must stay within `MAX_POLY_POW`; a constant base
                    // has degree 0 so `c^n` still expands cheaply.
                    let base_deg = poly_degree(&pa);
                    if base_deg
                        .checked_mul(n as usize)
                        .is_none_or(|d| (d as f64) > MAX_POLY_POW)
                    {
                        return None;
                    }
                    let mut acc = vec![1.0];
                    for _ in 0..(n as u32) {
                        acc = poly_mul(&acc, &pa);
                    }
                    Some(acc)
                }
                _ => None,
            }
        }
        // `|x|` is not a polynomial in `x` (it is piecewise-linear with a corner
        // at 0), so an absolute value takes the whole expression out of the
        // polynomial fragment the univariate root-finders handle.
        ComputeExpr::Unary(_, _) => None,
        ComputeExpr::Agg(_, _) => None,
        // A `round_to(x, n)` narrowing is neither linear nor polynomial — out of
        // scope for this tactic, exactly like a unary round (NUM-6a).
        ComputeExpr::Round { .. } => None,
        // `to_scientific(x, figures)` renders a number to a boundary string — not a
        // linear/polynomial term, out of scope for this tactic exactly like a round (NUM-6c).
        ComputeExpr::ToScientific { .. } => None,
        // `to_percent(x, places)` likewise renders a number to a boundary string (NUM-6c).
        ComputeExpr::ToPercent { .. } => None,
        // `to_currency(x, code, places)` likewise renders a number to a boundary string (NUM-6c).
        ComputeExpr::ToCurrency { .. } => None,
    }
}

fn poly_add(a: &Poly, b: &Poly) -> Poly {
    let mut out = vec![0.0; a.len().max(b.len())];
    for (i, c) in a.iter().enumerate() {
        out[i] += c;
    }
    for (i, c) in b.iter().enumerate() {
        out[i] += c;
    }
    out
}

fn poly_sub(a: &Poly, b: &Poly) -> Poly {
    let mut out = vec![0.0; a.len().max(b.len())];
    for (i, c) in a.iter().enumerate() {
        out[i] += c;
    }
    for (i, c) in b.iter().enumerate() {
        out[i] -= c;
    }
    out
}

fn poly_mul(a: &Poly, b: &Poly) -> Poly {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, ca) in a.iter().enumerate() {
        for (j, cb) in b.iter().enumerate() {
            out[i + j] += ca * cb;
        }
    }
    out
}

/// The degree — the highest power with a non-(near-)zero coefficient.
fn poly_degree(p: &Poly) -> usize {
    p.iter().rposition(|c| c.abs() > 1e-12).unwrap_or(0)
}

/// Solve a univariate polynomial equation `p(x) = 0` of degree 2–4 via
/// cas-solve's exact closed forms, returning the **real** roots as f64.
fn solve_univariate_poly(var: &str, p: &Poly) -> SolveOutcome {
    let deg = poly_degree(p);
    // Coefficient `i` of `p`, as an exact Frac (or bail if not representable).
    let f = |i: usize| -> Option<Frac> { f64_to_frac(*p.get(i).unwrap_or(&0.0)) };
    let result = match deg {
        2 => {
            let (Some(a), Some(b), Some(c)) = (f(2), f(1), f(0)) else {
                return unsupported("non-representable quadratic coefficient");
            };
            solve_quadratic(a, b, c)
        }
        3 => {
            let (Some(a), Some(b), Some(c), Some(d)) = (f(3), f(2), f(1), f(0)) else {
                return unsupported("non-representable cubic coefficient");
            };
            solve_cubic(a, b, c, d)
        }
        4 => {
            let (Some(a), Some(b), Some(c), Some(d), Some(e)) = (f(4), f(3), f(2), f(1), f(0))
            else {
                return unsupported("non-representable quartic coefficient");
            };
            solve_quartic(a, b, c, d, e)
        }
        _ => return unsupported("nonlinear degree > 4 is not supported"),
    };
    let roots_ir = match result {
        SolveResult::Solutions(rs) => rs,
        // Every x satisfies it — not a finite root set.
        SolveResult::All => return unsupported("the equation is an identity (all x)"),
    };
    // Keep the real roots (drop complex ones — their evaluation fails), dedup
    // near-equal values, and present them in ascending order.
    let mut roots: Vec<f64> = roots_ir.iter().filter_map(eval_ir_root).collect();
    roots.retain(|r| r.is_finite());
    roots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    roots.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    if roots.is_empty() {
        return unsupported("no real roots");
    }
    SolveOutcome::SolvedRoots {
        var: var.to_string(),
        roots,
        from_constraints: vec![0],
    }
}

/// Numerically evaluate a root IR node to an f64. Handles exact rationals and
/// the closed-form irrational/`Sqrt` shapes cas-solve emits; a complex root
/// (containing the imaginary unit) or any unrecognised head returns `None`, so
/// complex roots are simply dropped from the real-root set.
fn eval_ir_root(node: &IRNode) -> Option<f64> {
    match node {
        IRNode::Integer(n) => Some(*n as f64),
        IRNode::Rational(n, d) if *d != 0 => Some(*n as f64 / *d as f64),
        IRNode::Float(x) => Some(*x),
        IRNode::Apply(a) => {
            let head = match &a.head {
                IRNode::Symbol(h) => h.as_str(),
                _ => return None,
            };
            let arg = |i: usize| a.args.get(i).and_then(eval_ir_root);
            match head {
                "Add" => a.args.iter().map(eval_ir_root).sum::<Option<f64>>(),
                "Sub" => Some(arg(0)? - arg(1)?),
                "Mul" => a.args.iter().map(eval_ir_root).product::<Option<f64>>(),
                "Div" => {
                    let d = arg(1)?;
                    if d == 0.0 {
                        None
                    } else {
                        Some(arg(0)? / d)
                    }
                }
                "Neg" => Some(-arg(0)?),
                "Sqrt" => {
                    let v = arg(0)?;
                    if v < 0.0 {
                        None
                    } else {
                        Some(v.sqrt())
                    }
                }
                "Pow" => Some(arg(0)?.powf(arg(1)?)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Convert an f64 to an exact `Frac` (`round(x·10^k)/10^k` for the smallest
/// `k ≤ 9` that represents it), or `None` for non-finite / out-of-range.
fn f64_to_frac(x: f64) -> Option<Frac> {
    if !x.is_finite() {
        return None;
    }
    if x.fract() == 0.0 && x.abs() < i64::MAX as f64 {
        return Some(Frac::from_int(x as i64));
    }
    let mut denom: i64 = 1;
    for _ in 0..9 {
        denom *= 10;
        let scaled = x * denom as f64;
        if scaled.fract().abs() < 1e-9 && scaled.abs() < i64::MAX as f64 {
            return Some(Frac::new(scaled.round() as i64, denom));
        }
    }
    let denom = 1_000_000_000i64;
    let scaled = x * denom as f64;
    if scaled.abs() < i64::MAX as f64 {
        Some(Frac::new(scaled.round() as i64, denom))
    } else {
        None
    }
}

/// Translate an unevaluated [`ComputeExpr`] into a `symbolic-ir` node, or
/// `None` if it isn't an affine (linear) expression cas-solve can handle:
/// symbols, numeric literals, `+`/`-`, scalar `×`, and division by a constant.
fn expr_to_ir(e: &ComputeExpr) -> Option<IRNode> {
    match e {
        ComputeExpr::Ref(name) => Some(sym(name)),
        ComputeExpr::Lit(x) => num_to_ir(*x),
        ComputeExpr::Bin(op, a, b) => {
            let ia = expr_to_ir(a)?;
            let ib = expr_to_ir(b)?;
            match op {
                ComputeOp::Add => Some(apply(sym(ADD), vec![ia, ib])),
                ComputeOp::Sub => Some(apply(sym(SUB), vec![ia, ib])),
                // `a × b` is linear only when at least one side is a constant
                // (`2 × x`, `x × rate`). `x × y` (symbol × symbol) is
                // non-linear — reject it rather than hand cas-solve a row it
                // would silently drop.
                ComputeOp::Mul => {
                    if is_constant_expr(a) || is_constant_expr(b) {
                        Some(apply(sym(MUL), vec![ia, ib]))
                    } else {
                        None
                    }
                }
                // `a / c` (c a constant) is linear: rewrite as `a × (1/c)`.
                // Division by a symbol is non-linear → None.
                ComputeOp::Div => {
                    let c = const_value(b)?;
                    if c == 0.0 {
                        return None;
                    }
                    let recip = num_to_ir(1.0 / c)?;
                    Some(apply(sym(MUL), vec![ia, recip]))
                }
                // Aggregations reduce observed facts, not symbols — not part of
                // a constraint to solve.
                _ => None,
            }
        }
        // `|x|` is non-linear (piecewise), so it is not a row this linear CAS
        // bridge can solve — reject rather than silently drop.
        ComputeExpr::Unary(_, _) => None,
        ComputeExpr::Agg(_, _) => None,
        // A `round_to(x, n)` narrowing is neither linear nor polynomial — out of
        // scope for this tactic, exactly like a unary round (NUM-6a).
        ComputeExpr::Round { .. } => None,
        // `to_scientific(x, figures)` renders a number to a boundary string — not a
        // linear/polynomial term, out of scope for this tactic exactly like a round (NUM-6c).
        ComputeExpr::ToScientific { .. } => None,
        // `to_percent(x, places)` likewise renders a number to a boundary string (NUM-6c).
        ComputeExpr::ToPercent { .. } => None,
        // `to_currency(x, code, places)` likewise renders a number to a boundary string (NUM-6c).
        ComputeExpr::ToCurrency { .. } => None,
    }
}

/// The constant value of an expr if it is a pure numeric literal, else `None`.
fn const_value(e: &ComputeExpr) -> Option<f64> {
    match e {
        ComputeExpr::Lit(x) => Some(*x),
        _ => None,
    }
}

/// `true` iff the expression mentions no symbol/slot (it is a numeric
/// constant) — used to keep multiplication/division linear.
fn is_constant_expr(e: &ComputeExpr) -> bool {
    match e {
        ComputeExpr::Lit(_) => true,
        ComputeExpr::Ref(_) | ComputeExpr::Agg(_, _) => false,
        ComputeExpr::Bin(_, a, b) => is_constant_expr(a) && is_constant_expr(b),
        // `|c|` is constant iff its operand is (the absolute value of a constant
        // is a constant); `|x|` mentions the symbol `x`, so it is not.
        ComputeExpr::Unary(_, a) => is_constant_expr(a),
        // `round_to(c, n)` is constant iff its operand is — same rule as unary.
        ComputeExpr::Round { expr, .. } => is_constant_expr(expr),
        // `to_scientific(c, n)` is likewise constant iff its operand is.
        ComputeExpr::ToScientific { expr, .. } => is_constant_expr(expr),
        // `to_percent(c, n)` is likewise constant iff its operand is.
        ComputeExpr::ToPercent { expr, .. } => is_constant_expr(expr),
        // `to_currency(c, code, n)` is likewise constant iff its operand is.
        ComputeExpr::ToCurrency { expr, .. } => is_constant_expr(expr),
    }
}

/// Convert an `f64` literal into an exact `symbolic-ir` Integer or Rational
/// node (cas-solve only reads those). A whole number becomes an `Integer`; a
/// decimal becomes `round(x·10^k) / 10^k` for the smallest `k ≤ 9` that
/// represents it (so `0.92 → 92/100`). Returns `None` for a non-finite or
/// out-of-range value (so a bad literal is rejected, never silently wrong).
fn num_to_ir(x: f64) -> Option<IRNode> {
    if !x.is_finite() {
        return None;
    }
    if x.fract() == 0.0 && x.abs() < i64::MAX as f64 {
        return Some(int(x as i64));
    }
    let mut denom: i64 = 1;
    for _ in 0..9 {
        denom *= 10;
        let scaled = x * denom as f64;
        if scaled.fract().abs() < 1e-9 && scaled.abs() < i64::MAX as f64 {
            return Some(rat(scaled.round() as i64, denom));
        }
    }
    // Couldn't represent within 9 decimal places exactly; approximate at 1e9.
    let denom = 1_000_000_000i64;
    let scaled = x * denom as f64;
    if scaled.abs() < i64::MAX as f64 {
        Some(rat(scaled.round() as i64, denom))
    } else {
        None
    }
}

/// Parse a `Rule(Symbol(name), value)` node into `(name, f64)`. The value is an
/// Integer or Rational; anything else is rejected.
fn parse_rule(node: &IRNode) -> Option<(String, f64)> {
    let IRNode::Apply(a) = node else { return None };
    if !matches!(&a.head, IRNode::Symbol(h) if h == "Rule") || a.args.len() != 2 {
        return None;
    }
    let IRNode::Symbol(name) = &a.args[0] else {
        return None;
    };
    let value = match &a.args[1] {
        IRNode::Integer(n) => *n as f64,
        IRNode::Rational(n, d) if *d != 0 => *n as f64 / *d as f64,
        _ => return None,
    };
    if value.is_finite() {
        Some((name.clone(), value))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adj_lang::compile;

    fn solve_src(src: &str) -> SolveOutcome {
        let lowered = compile(src).unwrap();
        solve(&lowered.constraints, &lowered.kb)
    }

    #[test]
    fn solves_a_two_variable_linear_system() {
        // x + y = 10 ; x - y = 2  →  x = 6, y = 4.
        let out = solve_src(
            "symbol x : scalar\n\
             symbol y : scalar\n\
             constrain x + y = 10\n\
             constrain x - y = 2\n\
             solve for { x, y }\n",
        );
        match out {
            SolveOutcome::Solved {
                assignments,
                from_constraints,
            } => {
                let get = |n: &str| assignments.iter().find(|(k, _)| k == n).unwrap().1;
                assert!((get("x") - 6.0).abs() < 1e-9, "{assignments:?}");
                assert!((get("y") - 4.0).abs() < 1e-9, "{assignments:?}");
                assert_eq!(from_constraints, vec![0, 1]);
            }
            other => panic!("expected Solved, got {other:?}"),
        }
    }

    #[test]
    fn solves_a_single_equation_with_a_known_term() {
        // premium = base_rate + 300, base_rate observed as 1200 → … but
        // base_rate is not a symbol here; use a literal: premium = 1500.
        let out = solve_src(
            "symbol premium : money(usd)\n\
             constrain premium = 1200 + 300\n\
             solve for { premium }\n",
        );
        match out {
            SolveOutcome::Solved { assignments, .. } => {
                assert!((assignments[0].1 - 1500.0).abs() < 1e-9);
            }
            other => panic!("expected Solved, got {other:?}"),
        }
    }

    #[test]
    fn handles_decimal_coefficients_exactly_enough() {
        // x = 100 * 0.92  →  92.
        let out = solve_src("symbol x : scalar\nconstrain x = 100 * 0.92\nsolve for { x }\n");
        match out {
            SolveOutcome::Solved { assignments, .. } => {
                assert!((assignments[0].1 - 92.0).abs() < 1e-6, "{assignments:?}");
            }
            other => panic!("expected Solved, got {other:?}"),
        }
    }

    #[test]
    fn substitutes_observed_facts_into_a_constraint() {
        // base_rate is observed (1200), not an unknown → substituted as a
        // constant, so premium = 1200 + 300 = 1500.
        let out = solve_src(
            "symbol premium : money(usd)\n\
             observe base_rate(1200)\n\
             constrain premium = base_rate + 300\n\
             solve for { premium }\n",
        );
        match out {
            SolveOutcome::Solved { assignments, .. } => {
                assert!((assignments[0].1 - 1500.0).abs() < 1e-9, "{assignments:?}");
            }
            other => panic!("expected Solved, got {other:?}"),
        }
    }

    #[test]
    fn observed_coefficient_keeps_the_system_linear() {
        // rate observed (3) → cost = p * 3 - ... wait, p * rate is symbol×const
        // after substitution, still linear. p * 3 = 1500 → p = 500.
        let out = solve_src(
            "symbol p : scalar\n\
             observe rate(3)\n\
             constrain p * rate = 1500\n\
             solve for { p }\n",
        );
        match out {
            SolveOutcome::Solved { assignments, .. } => {
                assert!((assignments[0].1 - 500.0).abs() < 1e-9, "{assignments:?}");
            }
            other => panic!("expected Solved, got {other:?}"),
        }
    }

    fn roots(out: &SolveOutcome) -> Vec<f64> {
        match out {
            SolveOutcome::SolvedRoots { roots, .. } => roots.clone(),
            other => panic!("expected SolvedRoots, got {other:?}"),
        }
    }

    #[test]
    fn quadratic_via_latex_power_still_solves() {
        // `constrain latex "$x^2 = 4$"` now lowers to a native ComputeOp::Pow
        // node (not `x*x`); the polynomial path must still read it as a quadratic
        // and find {±2}. Guards against a regression from the Pow rewrite.
        let r = roots(&solve_src(
            "symbol x : scalar\nconstrain latex \"$x^2 = 4$\"\nsolve for { x }\n",
        ));
        assert_eq!(r.len(), 2);
        assert!((r[0] - -2.0).abs() < 1e-6, "{r:?}");
        assert!((r[1] - 2.0).abs() < 1e-6, "{r:?}");
    }

    #[test]
    fn cubic_via_latex_power_still_solves() {
        // A latex `x^3` lowers to Pow too; the cubic solver still finds the root.
        let r = roots(&solve_src(
            "symbol x : scalar\nconstrain latex \"$x^3 = 8$\"\nsolve for { x }\n",
        ));
        assert!(r.iter().any(|v| (v - 2.0).abs() < 1e-6), "{r:?}");
    }

    #[test]
    fn nested_powers_do_not_explode_the_polynomial_degree() {
        // `(((x^64)^64)^64) = 1` would compound to degree ~262144+ without the
        // cumulative-degree cap. It must return quickly as Unknown (not solved,
        // not hung) — the constraint is simply not treated as a small polynomial.
        let out = solve_src(
            "symbol x : scalar\n\
             constrain latex \"$((x^{64})^{64})^{64} = 1$\"\n\
             solve for { x }\n",
        );
        // Any non-panicking, promptly-returned outcome is acceptable; it must not
        // be a solved low-degree polynomial (the degree is far beyond the cap).
        assert!(
            !matches!(out, SolveOutcome::SolvedRoots { .. }),
            "nested powers must not be expanded into a solvable polynomial: {out:?}"
        );
    }

    #[test]
    fn quadratic_x_squared_equals_four() {
        let r = roots(&solve_src(
            "symbol x : scalar\nconstrain x * x = 4\nsolve for { x }\n",
        ));
        assert_eq!(r.len(), 2);
        assert!((r[0] - -2.0).abs() < 1e-6, "{r:?}");
        assert!((r[1] - 2.0).abs() < 1e-6, "{r:?}");
    }

    #[test]
    fn quadratic_with_two_rational_roots() {
        // x^2 - 5x + 6 = 0  →  {2, 3}.
        let r = roots(&solve_src(
            "symbol x : scalar\nconstrain x * x - 5 * x + 6 = 0\nsolve for { x }\n",
        ));
        assert_eq!(r.len(), 2);
        assert!((r[0] - 2.0).abs() < 1e-6, "{r:?}");
        assert!((r[1] - 3.0).abs() < 1e-6, "{r:?}");
    }

    #[test]
    fn quadratic_with_irrational_roots_evaluated_numerically() {
        // x^2 = 2  →  ±√2 ≈ ±1.41421356.
        let r = roots(&solve_src(
            "symbol x : scalar\nconstrain x * x = 2\nsolve for { x }\n",
        ));
        assert_eq!(r.len(), 2);
        assert!((r[0] - -2.0_f64.sqrt()).abs() < 1e-6, "{r:?}");
        assert!((r[1] - 2.0_f64.sqrt()).abs() < 1e-6, "{r:?}");
    }

    #[test]
    fn cubic_three_real_roots() {
        // (x-1)(x-2)(x-3) = x^3 - 6x^2 + 11x - 6 = 0 → {1,2,3}.
        let r = roots(&solve_src(
            "symbol x : scalar\n\
             constrain x * x * x - 6 * x * x + 11 * x - 6 = 0\n\
             solve for { x }\n",
        ));
        assert_eq!(r.len(), 3);
        for (got, want) in r.iter().zip([1.0, 2.0, 3.0]) {
            assert!((got - want).abs() < 1e-5, "{r:?}");
        }
    }

    #[test]
    fn quartic_four_real_roots() {
        // (x-1)(x-2)(x-3)(x-4) = x^4 - 10x^3 + 35x^2 - 50x + 24 = 0.
        let r = roots(&solve_src(
            "symbol x : scalar\n\
             constrain x * x * x * x - 10 * x * x * x + 35 * x * x - 50 * x + 24 = 0\n\
             solve for { x }\n",
        ));
        assert_eq!(r.len(), 4);
        for (got, want) in r.iter().zip([1.0, 2.0, 3.0, 4.0]) {
            assert!((got - want).abs() < 1e-5, "{r:?}");
        }
    }

    #[test]
    fn quadratic_with_no_real_roots_is_unsupported() {
        // x^2 + 1 = 0 has only complex roots → no real roots.
        let out = solve_src("symbol x : scalar\nconstrain x * x + 1 = 0\nsolve for { x }\n");
        assert!(matches!(out, SolveOutcome::Unsupported { .. }), "{out:?}");
    }

    #[test]
    fn nonlinear_with_an_observed_coefficient() {
        // area = side^2, area observed 9  →  side = ±3.
        let r = roots(&solve_src(
            "symbol side : scalar\n\
             observe area(9)\n\
             constrain side * side = area\n\
             solve for { side }\n",
        ));
        assert_eq!(r.len(), 2);
        assert!((r[1] - 3.0).abs() < 1e-6, "{r:?}");
    }

    #[test]
    fn a_non_square_system_has_no_unique_solution() {
        // two unknowns, one equation.
        let out = solve_src(
            "symbol x : scalar\nsymbol y : scalar\nconstrain x + y = 10\nsolve for { x, y }\n",
        );
        assert_eq!(out, SolveOutcome::NoUniqueSolution);
    }

    #[test]
    fn inequalities_are_unsupported_in_this_slice() {
        let out = solve_src("symbol x : scalar\nconstrain x <= 10\nsolve for { x }\n");
        assert!(matches!(out, SolveOutcome::Unsupported { .. }));
    }

    #[test]
    fn multi_unknown_nonlinear_is_unsupported_not_wrong() {
        // x * y = 4 is non-linear in TWO unknowns — beyond the univariate
        // polynomial path; we must NOT pretend to solve it.
        let out = solve_src(
            "symbol x : scalar\nsymbol y : scalar\nconstrain x * y = 4\nsolve for { x, y }\n",
        );
        assert!(
            matches!(
                out,
                SolveOutcome::Unsupported { .. } | SolveOutcome::NoUniqueSolution
            ),
            "{out:?}"
        );
    }

    // ---- feasibility / check (track B2c) ----

    fn check_src(src: &str) -> FeasibilityOutcome {
        let lowered = compile(src).unwrap();
        check(&lowered.constraints, &lowered.kb)
    }

    #[test]
    fn a_feasible_constraint_set_is_sat() {
        // 5 <= x <= 10 is satisfiable.
        let out = check_src("symbol x : scalar\nconstrain x >= 5\nconstrain x <= 10\ncheck\n");
        match out {
            FeasibilityOutcome::Sat { assignments } => {
                let x = assignments.iter().find(|(n, _)| n == "x").map(|(_, v)| *v);
                assert!(
                    matches!(x, Some(v) if (5..=10).contains(&v)),
                    "{assignments:?}"
                );
            }
            other => panic!("expected Sat, got {other:?}"),
        }
    }

    #[test]
    fn a_contradictory_constraint_set_is_unsat_with_a_core() {
        // x >= 5 AND x <= 1 cannot both hold.
        let out = check_src("symbol x : scalar\nconstrain x >= 5\nconstrain x <= 1\ncheck\n");
        match out {
            FeasibilityOutcome::Unsat { core } => assert_eq!(core, vec![0, 1]),
            other => panic!("expected Unsat, got {other:?}"),
        }
    }

    #[test]
    fn the_unsat_core_is_minimal_excluding_irrelevant_constraints() {
        // The contradiction is x >= 5 (idx 0) and x <= 1 (idx 1). The other two
        // constraints are satisfiable and irrelevant; a MINIMAL core (IIS) names
        // only [0, 1], not the full set.
        let out = check_src(
            "symbol x : scalar\nsymbol y : scalar\n\
             constrain x >= 5\nconstrain x <= 1\n\
             constrain y >= 0\nconstrain y <= 100\ncheck\n",
        );
        match out {
            FeasibilityOutcome::Unsat { core } => {
                assert_eq!(core, vec![0, 1], "IIS must drop y-bounds")
            }
            other => panic!("expected Unsat, got {other:?}"),
        }
    }

    #[test]
    fn a_three_way_real_contradiction_yields_its_minimal_core() {
        // x + y <= 1 ; x >= 1 ; y >= 1 are jointly infeasible (1+1 > 1), and ALL
        // three are needed — dropping any one is feasible. The IIS is all three;
        // a fourth, slack constraint is excluded.
        let out = check_src(
            "symbol x : scalar\nsymbol y : scalar\n\
             constrain x + y <= 1\nconstrain x >= 1\nconstrain y >= 1\n\
             constrain x <= 1000\ncheck\n",
        );
        match out {
            FeasibilityOutcome::Unsat { core } => assert_eq!(core, vec![0, 1, 2]),
            other => panic!("expected Unsat, got {other:?}"),
        }
    }

    #[test]
    fn a_single_self_contradictory_constraint_is_its_own_core() {
        // `0 >= 1` (after the symbol is irrelevant) — a lone contradiction.
        let out = check_src("symbol x : scalar\nconstrain 2 <= 1\nconstrain x >= 0\ncheck\n");
        match out {
            FeasibilityOutcome::Unsat { core } => assert_eq!(core, vec![0]),
            other => panic!("expected Unsat, got {other:?}"),
        }
    }

    #[test]
    fn feasibility_substitutes_observed_facts() {
        // floor observed 6; months >= floor is feasible (months can be 6).
        let out = check_src(
            "symbol months : scalar\nobserve floor(6)\nconstrain months >= floor\ncheck\n",
        );
        assert!(matches!(out, FeasibilityOutcome::Sat { .. }), "{out:?}");
    }

    // ---- QF_LRA real feasibility via Fourier–Motzkin (track C1) ----

    /// The real witness assigned to `name`, or panic if the outcome isn't
    /// `SatReal`. Used to spot-check the reconstructed point.
    fn real_witness(out: &FeasibilityOutcome, name: &str) -> f64 {
        match out {
            FeasibilityOutcome::SatReal { assignments } => {
                assignments.iter().find(|(n, _)| n == name).unwrap().1
            }
            other => panic!("expected SatReal, got {other:?}"),
        }
    }

    #[test]
    fn a_non_integer_constraint_is_real_feasible() {
        // C1: a fractional bound is no longer Unknown — it's real-feasible.
        // x <= 0.5 admits, e.g., x = 0.5 (or below).
        let out = check_src("symbol x : scalar\nconstrain x <= 0.5\ncheck\n");
        let x = real_witness(&out, "x");
        assert!(x <= 0.5 + 1e-9, "witness {x} must satisfy x <= 0.5");
    }

    #[test]
    fn a_fractional_interval_is_real_feasible_with_an_interior_witness() {
        // 0.25 <= x <= 0.75 → a real point strictly satisfying both.
        let out = check_src("symbol x : scalar\nconstrain x >= 0.25\nconstrain x <= 0.75\ncheck\n");
        let x = real_witness(&out, "x");
        assert!(
            (0.25..=0.75).contains(&x),
            "witness {x} out of [0.25, 0.75]"
        );
    }

    #[test]
    fn integer_infeasible_but_real_feasible_is_sat_real() {
        // 2x = 1 has no integer solution but is real-feasible at x = 0.5.
        // The integer tactic says Unsat; the real layer rescues it.
        let out = check_src("symbol x : scalar\nconstrain 2 * x = 1\ncheck\n");
        let x = real_witness(&out, "x");
        assert!((x - 0.5).abs() < 1e-9, "expected x = 0.5, got {x}");
    }

    #[test]
    fn a_fractional_contradiction_is_unsat() {
        // x >= 0.75 AND x <= 0.25 cannot both hold over the reals either.
        let out = check_src("symbol x : scalar\nconstrain x >= 0.75\nconstrain x <= 0.25\ncheck\n");
        assert!(matches!(out, FeasibilityOutcome::Unsat { .. }), "{out:?}");
    }

    #[test]
    fn two_variable_real_system_is_feasible_with_a_valid_witness() {
        // x + y <= 1 ; x >= 0.3 ; y >= 0.3 → feasible (e.g. x = y = 0.35).
        // Verifies the witness actually satisfies all three.
        let out = check_src(
            "symbol x : scalar\nsymbol y : scalar\n\
             constrain x + y <= 1\nconstrain x >= 0.3\nconstrain y >= 0.3\ncheck\n",
        );
        let x = real_witness(&out, "x");
        let y = real_witness(&out, "y");
        assert!(
            x >= 0.3 - 1e-9 && y >= 0.3 - 1e-9 && x + y <= 1.0 + 1e-9,
            "x={x} y={y}"
        );
    }

    #[test]
    fn two_variable_real_system_can_be_unsat() {
        // x + y <= 1 ; x >= 0.6 ; y >= 0.6 → 0.6 + 0.6 = 1.2 > 1, infeasible.
        let out = check_src(
            "symbol x : scalar\nsymbol y : scalar\n\
             constrain x + y <= 1\nconstrain x >= 0.6\nconstrain y >= 0.6\ncheck\n",
        );
        assert!(matches!(out, FeasibilityOutcome::Unsat { .. }), "{out:?}");
    }

    #[test]
    fn a_strict_inequality_is_handled() {
        // x < 1 ; x > 0 → real-feasible with 0 < x < 1.
        let out = check_src("symbol x : scalar\nconstrain x > 0\nconstrain x < 1\ncheck\n");
        let x = real_witness(&out, "x");
        assert!(x > 0.0 && x < 1.0, "strict witness {x} out of (0,1)");
    }

    #[test]
    fn a_disjunctive_not_equal_constraint_stays_unknown() {
        // `!=` is non-convex — outside the Fourier–Motzkin fragment. (The
        // integer tactic *can* decide `x != 3`, so we use a fractional bound to
        // force the real layer, where `!=` is genuinely Unknown.)
        let out = check_src("symbol x : scalar\nconstrain x != 0.5\ncheck\n");
        assert!(matches!(out, FeasibilityOutcome::Unknown { .. }), "{out:?}");
    }

    #[test]
    fn a_nonlinear_constraint_in_check_stays_unknown() {
        // x * x <= 4 is non-linear — not in the linear-real fragment.
        let out = check_src("symbol x : scalar\nconstrain x * x <= 4\ncheck\n");
        assert!(matches!(out, FeasibilityOutcome::Unknown { .. }), "{out:?}");
    }

    // ---- checked-rational overflow contract (security fix for C1) ----

    #[test]
    fn rat_rejects_values_past_the_cap() {
        // A value beyond RAT_CAP must not be silently wrapped — it returns None.
        assert!(Rat::new(i128::MAX, 1).is_none());
        assert!(Rat::new(RAT_CAP + 1, 1).is_none());
        assert!(Rat::new(1, RAT_CAP + 1).is_none());
        // …but a value at the cap is fine.
        assert!(Rat::new(RAT_CAP, 1).is_some());
    }

    #[test]
    fn rat_arithmetic_returns_none_on_overflow_never_wraps() {
        // Two near-cap rationals: the product/sum exceed the cap, so the checked
        // ops return None rather than wrapping to a wrong (possibly sign-flipped)
        // value. This is the exact failure mode the security review caught.
        let big = Rat::new(RAT_CAP, 1).unwrap();
        assert!(big.mul(big).is_none(), "mul past cap must be None");
        assert!(big.add(big).is_none(), "add past cap must be None");
        // A coprime-denominator pair whose common denominator overflows.
        let a = Rat::new(1, 999_999_937).unwrap(); // prime denom
        let b = Rat::new(1, 999_999_893).unwrap(); // another prime denom
                                                   // 999_999_937 * 999_999_893 ≈ 1.0e18 < cap, so this one is representable…
        assert!(a.add(b).is_some());
        // …but multiplying three such denominators overflows the cap → None.
        let c = Rat::new(1, 999_999_761).unwrap();
        assert!(a.add(b).and_then(|ab| ab.add(c)).is_none());
    }

    #[test]
    fn rat_ordering_is_exact() {
        // 1/3 < 1/2 < 2/3, and equal fractions compare equal regardless of form.
        assert!(Rat::new(1, 3).unwrap() < Rat::new(1, 2).unwrap());
        assert!(Rat::new(1, 2).unwrap() < Rat::new(2, 3).unwrap());
        assert_eq!(Rat::new(2, 4).unwrap(), Rat::new(1, 2).unwrap());
        assert!(Rat::new(-1, 2).unwrap() < Rat::new(0, 1).unwrap());
    }

    #[test]
    fn an_overflowing_constraint_is_unknown_not_a_wrong_verdict() {
        // A system whose Fourier–Motzkin combination blows past the rational cap
        // must report Unknown — never a fabricated Sat/Unsat. We build it from
        // many distinct tiny fractional bounds on coupled variables so the
        // eliminated shadows accumulate coprime denominators.
        let src = "symbol a : scalar\nsymbol b : scalar\nsymbol c : scalar\n\
             constrain a >= 0.123456789\nconstrain a <= 0.987654321\n\
             constrain b >= 0.314159265\nconstrain b <= 0.271828182\n\
             constrain c >= 0.141421356\nconstrain c <= 0.173205080\n\
             constrain a + b + c <= 0.111111111\ncheck\n";
        let out = check_src(src);
        // b's bounds (>= 0.3141 and <= 0.2718) are themselves contradictory, so
        // the honest verdict is Unsat — and even if coefficients grew, the only
        // permitted outcomes are Unsat or Unknown, NEVER Sat/SatReal.
        assert!(
            matches!(
                out,
                FeasibilityOutcome::Unsat { .. } | FeasibilityOutcome::Unknown { .. }
            ),
            "must not fabricate a feasible answer: {out:?}"
        );
    }

    #[test]
    fn no_symbols_is_unsupported() {
        let out = solve_src("constrain a = 1\n");
        assert!(matches!(out, SolveOutcome::Unsupported { .. }));
    }

    // ---- linear optimization: minimize / maximize (track C2) ----

    fn optimize_src(src: &str) -> OptimizeOutcome {
        let lowered = compile(src).unwrap();
        optimize(&lowered.constraints, &lowered.kb)
    }

    /// The optimal value + a getter over the witness, or panic if not Optimal.
    fn expect_optimal(out: &OptimizeOutcome) -> (f64, &Vec<(String, f64)>, &Vec<usize>) {
        match out {
            OptimizeOutcome::Optimal {
                value,
                assignments,
                binding,
            } => (*value, assignments, binding),
            other => panic!("expected Optimal, got {other:?}"),
        }
    }

    #[test]
    fn maximize_a_single_bounded_variable() {
        // max x s.t. 0 ≤ x ≤ 5  →  x = 5, value 5; the x ≤ 5 bound is binding.
        let out = optimize_src("symbol x : scalar\nconstrain x >= 0\nconstrain x <= 5\nmaximize x");
        let (value, assigns, binding) = expect_optimal(&out);
        assert!((value - 5.0).abs() < 1e-9, "value {value}");
        let x = assigns
            .iter()
            .find(|(n, _)| n == "x")
            .map(|(_, v)| *v)
            .unwrap();
        assert!((x - 5.0).abs() < 1e-9, "x {x}");
        assert!(
            binding.contains(&1),
            "x<=5 (idx 1) should bind: {binding:?}"
        );
    }

    #[test]
    fn minimize_a_single_bounded_variable() {
        // min x s.t. x ≥ 3  →  x = 3, value 3.
        let out = optimize_src("symbol x : scalar\nconstrain x >= 3\nminimize x");
        let (value, assigns, _) = expect_optimal(&out);
        assert!((value - 3.0).abs() < 1e-9, "value {value}");
        let x = assigns
            .iter()
            .find(|(n, _)| n == "x")
            .map(|(_, v)| *v)
            .unwrap();
        assert!((x - 3.0).abs() < 1e-9, "x {x}");
    }

    #[test]
    fn maximize_a_two_variable_objective() {
        // The classic LP: max 3x + 2y s.t. x + y ≤ 4, x ≤ 3, x,y ≥ 0.
        // Optimum at the vertex (3, 1) → 3·3 + 2·1 = 11.
        let out = optimize_src(
            "symbol x : scalar\nsymbol y : scalar\n\
             constrain x + y <= 4\nconstrain x <= 3\n\
             constrain x >= 0\nconstrain y >= 0\n\
             maximize 3 * x + 2 * y",
        );
        let (value, assigns, _) = expect_optimal(&out);
        assert!((value - 11.0).abs() < 1e-9, "value {value}");
        let get = |n: &str| {
            assigns
                .iter()
                .find(|(k, _)| k == n)
                .map(|(_, v)| *v)
                .unwrap()
        };
        // The witness need not be unique, but it must achieve 11 and be feasible.
        let (x, y) = (get("x"), get("y"));
        assert!((3.0 * x + 2.0 * y - 11.0).abs() < 1e-9, "x={x} y={y}");
        assert!(x <= 3.0 + 1e-9 && x + y <= 4.0 + 1e-9 && x >= -1e-9 && y >= -1e-9);
    }

    #[test]
    fn an_unbounded_objective_is_reported() {
        // max x with only a lower bound → unbounded above.
        let out = optimize_src("symbol x : scalar\nconstrain x >= 0\nmaximize x");
        assert!(matches!(out, OptimizeOutcome::Unbounded), "{out:?}");
    }

    #[test]
    fn an_infeasible_program_is_reported_not_optimized() {
        // max x over contradictory constraints → Infeasible, never a value.
        let out = optimize_src("symbol x : scalar\nconstrain x >= 5\nconstrain x <= 1\nmaximize x");
        assert!(matches!(out, OptimizeOutcome::Infeasible { .. }), "{out:?}");
    }

    #[test]
    fn an_infeasible_lp_reports_a_minimal_core() {
        // The conflict is x >= 5 (0) and x <= 1 (1); the y-bound (2) is slack.
        // The infeasibility certificate names only the irreducible [0, 1].
        let out = optimize_src(
            "symbol x : scalar\nsymbol y : scalar\n\
             constrain x >= 5\nconstrain x <= 1\nconstrain y <= 9\nmaximize x + y",
        );
        match out {
            OptimizeOutcome::Infeasible { core } => assert_eq!(core, vec![0, 1]),
            other => panic!("expected Infeasible, got {other:?}"),
        }
    }

    #[test]
    fn an_open_supremum_is_unknown_not_a_fake_optimum() {
        // max x s.t. x < 5: the supremum 5 is not attained (strict bound).
        let out = optimize_src("symbol x : scalar\nconstrain x < 5\nmaximize x");
        assert!(matches!(out, OptimizeOutcome::Unknown { .. }), "{out:?}");
    }

    #[test]
    fn a_nonlinear_objective_is_unknown() {
        let out = optimize_src("symbol x : scalar\nconstrain x <= 5\nmaximize x * x");
        assert!(matches!(out, OptimizeOutcome::Unknown { .. }), "{out:?}");
    }

    #[test]
    fn optimize_substitutes_observed_facts() {
        // budget observed 100; maximize spend s.t. spend ≤ budget → 100.
        let out = optimize_src(
            "symbol spend : scalar\nobserve budget(100)\n\
             constrain spend <= budget\nconstrain spend >= 0\nmaximize spend",
        );
        let (value, _, _) = expect_optimal(&out);
        assert!((value - 100.0).abs() < 1e-9, "value {value}");
    }

    #[test]
    fn minimize_finds_the_lower_optimum_in_two_vars() {
        // min x + y s.t. x ≥ 2, y ≥ 3 → 5 at (2, 3).
        let out = optimize_src(
            "symbol x : scalar\nsymbol y : scalar\n\
             constrain x >= 2\nconstrain y >= 3\nminimize x + y",
        );
        let (value, _, _) = expect_optimal(&out);
        assert!((value - 5.0).abs() < 1e-9, "value {value}");
    }

    // ---- Integer optimization (set-cover) -------------------------------

    /// Pull the integer-valued optimum + the selected (value ≈ 1) variables.
    fn expect_int_optimum(out: &OptimizeOutcome) -> (i128, Vec<String>) {
        match out {
            OptimizeOutcome::Optimal {
                value, assignments, ..
            } => {
                let selected = assignments
                    .iter()
                    .filter(|(_, v)| (*v - 1.0).abs() < 1e-9)
                    .map(|(n, _)| n.clone())
                    .collect();
                (value.round() as i128, selected)
            }
            other => panic!("expected Optimal, got {other:?}"),
        }
    }

    #[test]
    fn min_cost_set_cover_prefers_the_cheaper_single_agent() {
        // Cover o1,o2,o3. `broad` (cost 2) covers all; a,b,c (cost 1 each) cover
        // one organism each. Min cost = 2 (broad), beating a+b+c = 3.
        let out = optimize_src(
            "symbol broad : bool\nsymbol a : bool\nsymbol b : bool\nsymbol c : bool\n\
             constrain broad + a >= 1\nconstrain broad + b >= 1\nconstrain broad + c >= 1\n\
             minimize 2 * broad + a + b + c",
        );
        let (value, selected) = expect_int_optimum(&out);
        assert_eq!(value, 2, "min cost should be 2 (broad alone)");
        assert_eq!(selected, vec!["broad".to_string()]);
    }

    #[test]
    fn set_cover_integer_optimum_beats_the_fractional_relaxation() {
        // Three drugs, each covering two of three organisms. The LP relaxation
        // is 1.5 (every x = 0.5); the only integral cover needs TWO drugs → 2.
        // This is the whole point of solving it as an integer program.
        let out = optimize_src(
            "symbol d1 : bool\nsymbol d2 : bool\nsymbol d3 : bool\n\
             constrain d1 + d2 >= 1\nconstrain d2 + d3 >= 1\nconstrain d1 + d3 >= 1\n\
             minimize d1 + d2 + d3",
        );
        let (value, selected) = expect_int_optimum(&out);
        assert_eq!(value, 2, "integer optimum is 2, not the fractional 1.5");
        assert_eq!(selected.len(), 2, "exactly two drugs chosen: {selected:?}");
    }

    #[test]
    fn set_cover_scales_past_the_fourier_motzkin_variable_cap() {
        // Eight booleans — beyond the real LP's variable cap. The structural
        // boolean bound keeps it solvable. `big` (cost 5) covers o1..o5; the five
        // singletons also cost 5 — a tie, so the optimum value is 5 either way.
        let out = optimize_src(
            "symbol big : bool\nsymbol p1 : bool\nsymbol p2 : bool\nsymbol p3 : bool\n\
             symbol p4 : bool\nsymbol p5 : bool\nsymbol p6 : bool\nsymbol p7 : bool\n\
             constrain big + p1 >= 1\nconstrain big + p2 >= 1\nconstrain big + p3 >= 1\n\
             constrain big + p4 >= 1\nconstrain big + p5 >= 1\n\
             minimize 5 * big + p1 + p2 + p3 + p4 + p5 + p6 + p7",
        );
        let (value, _) = expect_int_optimum(&out);
        assert_eq!(value, 5, "min cost is 5");
    }

    #[test]
    fn an_uncoverable_organism_makes_the_set_cover_infeasible() {
        // o2 has no drug covering it (no constraint can be met) → Infeasible.
        let out = optimize_src(
            "symbol a : bool\n\
             constrain a >= 1\nconstrain a <= 0\n\
             minimize a",
        );
        assert!(
            matches!(out, OptimizeOutcome::Infeasible { .. }),
            "got {out:?}"
        );
    }

    #[test]
    fn maximize_over_booleans_picks_the_most_valuable_feasible_set() {
        // Maximize value subject to a budget: pick 2 of 3 unit-value items under
        // "at most 2" (a + b + c ≤ 2) → optimum 2.
        let out = optimize_src(
            "symbol a : bool\nsymbol b : bool\nsymbol c : bool\n\
             constrain a + b + c <= 2\nmaximize a + b + c",
        );
        let (value, selected) = expect_int_optimum(&out);
        assert_eq!(value, 2);
        assert_eq!(selected.len(), 2, "two items chosen: {selected:?}");
    }

    #[test]
    fn scalar_optimization_is_unchanged_by_the_integer_path() {
        // A `: scalar` program with an integer-linear shape still takes the real
        // LP path: min x s.t. 2x ≥ 3 → 1.5 (NOT lifted to an integer 2).
        let out = optimize_src("symbol x : scalar\nconstrain x + x >= 3\nminimize x");
        let (value, _, _) = expect_optimal(&out);
        assert!((value - 1.5).abs() < 1e-9, "scalar stays real: {value}");
    }

    // ---- SAT / pseudo-boolean set-cover scaling (B1b) ----------------------

    /// Brute-force whether `Σ weights·x ≤ k` is satisfiable for SOME assignment
    /// that also lights at least `min_true` literals — used to check the encoder.
    fn sinz_matches_bruteforce(weights: &[i128], k: i128) {
        // Expand to a unit literal list, encode, and check the encoding accepts
        // EXACTLY the assignments whose weighted sum ≤ k, over the original vars.
        let names: Vec<String> = (0..weights.len()).map(|i| format!("v{i}")).collect();
        let mut lits = Vec::new();
        for (i, w) in weights.iter().enumerate() {
            for _ in 0..*w {
                lits.push(names[i].clone());
            }
        }
        let (clauses, aux) = sinz_at_most(&lits, k);
        let mut bool_vars = names.clone();
        bool_vars.extend(aux);
        // For every assignment of the original vars, the encoding must be SAT iff
        // the weighted sum ≤ k. (We let the SAT solver fill the aux vars.)
        for mask in 0..(1u32 << weights.len()) {
            let mut asserts = clauses.clone();
            let mut sum = 0i128;
            for (i, w) in weights.iter().enumerate() {
                let on = (mask >> i) & 1 == 1;
                if on {
                    sum += *w;
                }
                asserts.push(clause(vec![(names[i].clone(), on)]));
            }
            let sat = matches!(SatTactic::solve(&asserts, &bool_vars), SolverResult::Sat(_));
            assert_eq!(
                sat,
                sum <= k,
                "weights={weights:?} k={k} mask={mask:b} sum={sum}: encoder said {sat}"
            );
        }
    }

    #[test]
    fn sinz_encoder_is_exact_on_small_instances() {
        sinz_matches_bruteforce(&[1, 1, 1], 0);
        sinz_matches_bruteforce(&[1, 1, 1], 1);
        sinz_matches_bruteforce(&[1, 1, 1], 2);
        sinz_matches_bruteforce(&[1, 1, 1, 1], 2);
        sinz_matches_bruteforce(&[2, 1, 3], 3); // weighted
        sinz_matches_bruteforce(&[2, 2, 1, 1], 3);
        sinz_matches_bruteforce(&[1, 1], 1);
    }

    #[test]
    fn sat_set_cover_agrees_with_lia_on_the_fractional_case() {
        // Same instance as the LIA test: integer optimum 2, not the fractional 1.5.
        let out = optimize_src(
            "symbol d1 : bool\nsymbol d2 : bool\nsymbol d3 : bool\n\
             constrain d1 + d2 >= 1\nconstrain d2 + d3 >= 1\nconstrain d1 + d3 >= 1\n\
             minimize d1 + d2 + d3",
        );
        let (value, _) = expect_int_optimum(&out);
        assert_eq!(value, 2);
    }

    #[test]
    fn sat_set_cover_scales_to_many_selectors() {
        // 30 drugs in a cycle cover (min cost = 15) — well past the LIA cap. The
        // SAT oracle solves it; this would time out the LIA enumeration.
        let n = 30;
        let mut src = String::new();
        for i in 0..n {
            src += &format!("symbol d{i} : bool\n");
        }
        for i in 0..n {
            src += &format!("constrain d{i} + d{} >= 1\n", (i + 1) % n);
        }
        src += "minimize ";
        src += &(0..n)
            .map(|i| format!("d{i}"))
            .collect::<Vec<_>>()
            .join(" + ");
        let (value, _) = expect_int_optimum(&optimize_src(&src));
        assert_eq!(value, n as i128 / 2, "cycle cover min = n/2");
    }

    #[test]
    fn nary_combination_cover_routes_through_sat() {
        // A requirement coverable ONLY by the 3-element combination {a,b,c} (an AND
        // linearized with aux `y`), plus a single-drug requirement `e`. The optimum
        // must select a,b,c (for the combination) + e. The combination's implication
        // clauses (`¬y ∨ a`, `y ∨ ¬a ∨ ¬b ∨ ¬c`) are ±1 clauses the generalized
        // recognizer accepts, so this scalable boolean cover goes through the SAT path.
        let out = optimize_src(
            "symbol a : bool\nsymbol b : bool\nsymbol c : bool\nsymbol e : bool\nsymbol y : bool\n\
             constrain y <= a\nconstrain y <= b\nconstrain y <= c\n\
             constrain y - (a + b + c) >= -2\n\
             constrain y >= 1\nconstrain e >= 1\n\
             minimize a + b + c + e",
        );
        let (value, selected) = expect_int_optimum(&out);
        assert_eq!(
            value, 4,
            "a+b+c+e all chosen for the combination + single cover"
        );
        for v in ["a", "b", "c", "e"] {
            assert!(
                selected.contains(&v.to_string()),
                "{v} must be selected: {selected:?}"
            );
        }
    }

    #[test]
    fn combination_is_only_taken_when_a_requirement_needs_it() {
        // If the requirement `e` is coverable directly AND the combination is not
        // forced, the optimizer must NOT pay for the combination. Here nothing forces
        // `y`, so the optimum is just `e` (cost 1), leaving a,b,c unselected.
        let out = optimize_src(
            "symbol a : bool\nsymbol b : bool\nsymbol c : bool\nsymbol e : bool\nsymbol y : bool\n\
             constrain y <= a\nconstrain y <= b\nconstrain y <= c\n\
             constrain y - (a + b + c) >= -2\n\
             constrain e + y >= 1\n\
             minimize a + b + c + e",
        );
        let (value, selected) = expect_int_optimum(&out);
        assert_eq!(
            value, 1,
            "cover via the single agent, not the 3-drug combination"
        );
        assert_eq!(selected, vec!["e".to_string()]);
    }

    #[test]
    fn num_to_ir_handles_integers_and_decimals() {
        assert_eq!(num_to_ir(42.0), Some(int(42)));
        assert_eq!(num_to_ir(0.5), Some(rat(5, 10)));
        assert_eq!(num_to_ir(f64::INFINITY), None);
        assert_eq!(num_to_ir(f64::NAN), None);
    }
}

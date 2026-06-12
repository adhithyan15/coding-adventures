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

use adj_lang::{ConstraintSystem, RelOp};
use cas_solve::frac::Frac;
use cas_solve::{solve_cubic, solve_quadratic, solve_quartic, SolveResult};
use constraint_core::Predicate;
use constraint_engine::{lia::LiaTactic, SolverResult, Value};
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
    /// either the integers *or* the reals. `core` is a set of conflicting
    /// constraint indices (the full set for now; minimal-core extraction is
    /// future work).
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
    let all_core = || (0..cs.constraints.len()).collect::<Vec<_>>();

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
                    return FeasibilityOutcome::Unsat { core: all_core() }
                }
            },
            // Integer tactic punted — let the real layer try.
            SolverResult::Unknown(_) => {}
        }
    }

    // ---- Layer 2: QF_LRA real feasibility via Fourier–Motzkin over ℚ. ----
    match real_feasibility(&subbed) {
        FmResult::Sat(w) => FeasibilityOutcome::SatReal { assignments: w },
        FmResult::Unsat => FeasibilityOutcome::Unsat { core: all_core() },
        FmResult::Unknown(reason) => FeasibilityOutcome::Unknown { reason },
    }
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
                    } else if let Some(c) = int_const(b) {
                        Some(Predicate::Mul {
                            coef: c,
                            term: Box::new(pa),
                        })
                    } else {
                        None // var × var is non-linear
                    }
                }
                _ => None, // division / aggregation: out of LIA scope
            }
        }
        ComputeExpr::Agg(_, _) => None,
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
        ComputeExpr::Agg(_, _) => None,
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
fn fourier_motzkin(planes: Vec<Halfplane>) -> FmResult {
    // The variables to eliminate, in a deterministic order.
    let mut vars: Vec<String> = std::collections::BTreeSet::<String>::from_iter(
        planes.iter().flat_map(|h| h.form.coeffs.keys().cloned()),
    )
    .into_iter()
    .collect();

    // Keep the original system to verify the reconstructed witness against.
    let original = planes.clone();

    // Eliminate one variable at a time, recording for each the half-planes that
    // mentioned it (needed to reconstruct its value during back-substitution).
    let mut elim_steps: Vec<(String, Vec<Halfplane>)> = Vec::new();
    let mut current = planes;
    for v in &vars {
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
                                         // Any overflow in this checked arithmetic ⇒ Unknown.
                let combined = match b
                    .neg()
                    .and_then(|nb| p.form.scale(nb))
                    .and_then(|sp| sp.add_scaled(&n.form, a))
                {
                    Some(c) => c,
                    None => {
                        return FmResult::Unknown(
                            "coefficients grew past the checked-rational cap".to_string(),
                        )
                    }
                };
                next.push(Halfplane {
                    form: combined,
                    strict: p.strict || n.strict,
                });
                if next.len() > MAX_INEQUALITIES {
                    return FmResult::Unknown(
                        "constraint system too large for the Fourier–Motzkin slice".to_string(),
                    );
                }
            }
        }
        current = next;
    }

    // No variables left: every half-plane is a constant `k (≤|<) 0`.
    for hp in &current {
        let k = hp.form.constant;
        let violated = if hp.strict {
            k.num >= 0 // need k < 0
        } else {
            k.num > 0 // need k ≤ 0
        };
        if violated {
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
        ComputeExpr::Agg(_, _) => e.clone(),
    }
}

// ---------------------------------------------------------------------------
// Univariate polynomial path (nonlinear single-unknown equalities, track C3)
// ---------------------------------------------------------------------------

/// A univariate polynomial as coefficients indexed by power
/// (`p[0] + p[1]·x + p[2]·x² + …`).
type Poly = Vec<f64>;

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
                _ => None,
            }
        }
        ComputeExpr::Agg(_, _) => None,
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
        ComputeExpr::Agg(_, _) => None,
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

    #[test]
    fn num_to_ir_handles_integers_and_decimals() {
        assert_eq!(num_to_ir(42.0), Some(int(42)));
        assert_eq!(num_to_ir(0.5), Some(rat(5, 10)));
        assert_eq!(num_to_ir(f64::INFINITY), None);
        assert_eq!(num_to_ir(f64::NAN), None);
    }
}

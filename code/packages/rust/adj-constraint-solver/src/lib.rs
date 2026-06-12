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
    /// Satisfiable, with a witness assignment (each unknown → an integer value).
    Sat { assignments: Vec<(String, i128)> },
    /// Unsatisfiable — no assignment satisfies all constraints at once. `core`
    /// is a set of conflicting constraint indices (the full set for now;
    /// minimal-core extraction is future work).
    Unsat { core: Vec<usize> },
    /// The engine couldn't decide — a non-integer/non-linear constraint, or a
    /// theory the LIA tactic doesn't cover.
    Unknown { reason: String },
}

/// Decide whether a [`ConstraintSystem`]'s constraints are jointly satisfiable
/// over the integers, substituting observed facts first. Linear integer
/// (in)equalities only; a non-integer or non-linear constraint yields
/// `Unknown` (the richer real/LP tactics are tracks C1/C2).
pub fn check(cs: &ConstraintSystem, kb: &KnowledgeBase) -> FeasibilityOutcome {
    if cs.constraints.is_empty() {
        return FeasibilityOutcome::Sat {
            assignments: Vec::new(),
        };
    }
    let int_vars: Vec<String> = cs.symbols.iter().map(|(n, _)| n.clone()).collect();
    let var_set: HashSet<&str> = int_vars.iter().map(String::as_str).collect();

    let mut assertions = Vec::with_capacity(cs.constraints.len());
    for c in &cs.constraints {
        let lhs = substitute_observed(&c.lhs, &var_set, kb);
        let rhs = substitute_observed(&c.rhs, &var_set, kb);
        let (Some(pl), Some(pr)) = (expr_to_pred(&lhs), expr_to_pred(&rhs)) else {
            return FeasibilityOutcome::Unknown {
                reason: "a constraint is non-linear or not integer-valued".to_string(),
            };
        };
        assertions.push(relop_predicate(c.op, pl, pr));
    }

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
            FeasibilityOutcome::Sat { assignments }
        }
        SolverResult::Unsat => FeasibilityOutcome::Unsat {
            core: (0..cs.constraints.len()).collect(),
        },
        SolverResult::Unknown(reason) => FeasibilityOutcome::Unknown { reason },
    }
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

    #[test]
    fn a_non_integer_constraint_is_unknown() {
        // a fractional bound is outside the linear-integer tactic.
        let out = check_src("symbol x : scalar\nconstrain x <= 0.5\ncheck\n");
        assert!(matches!(out, FeasibilityOutcome::Unknown { .. }), "{out:?}");
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

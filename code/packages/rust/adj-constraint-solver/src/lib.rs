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

use adj_lang::{ConstraintSystem, RelOp};
use logic_engine::{ComputeExpr, ComputeOp};
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
    /// The linear system is singular, under-/over-determined, or not square
    /// (≠ one equation per unknown) — no unique solution exists.
    NoUniqueSolution,
    /// The system is outside this slice's scope (inequalities, a non-linear
    /// term, an aggregation, no symbols, …). Carries a human reason. Never a
    /// wrong answer — the caller falls back to a richer solver (C1/C2/…).
    Unsupported { reason: String },
}

/// Solve a [`ConstraintSystem`]'s linear-equality core. See the module docs for
/// scope. Pure and deterministic.
pub fn solve(cs: &ConstraintSystem) -> SolveOutcome {
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

    // This slice solves pure-equality systems. Any inequality means the
    // problem is feasibility/optimization, not a linear solve — defer it.
    if cs.constraints.iter().any(|c| c.op != RelOp::Eq) {
        return unsupported("inequality constraints — feasibility/LP is track C1/C2");
    }

    // Translate each `lhs = rhs` into a symbolic-ir Equal equation. A
    // non-linear term (symbol×symbol, division by a symbol, an aggregation)
    // makes the translation fail → Unsupported.
    let mut equations = Vec::with_capacity(cs.constraints.len());
    for c in &cs.constraints {
        let (Some(lhs), Some(rhs)) = (expr_to_ir(&c.lhs), expr_to_ir(&c.rhs)) else {
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
        solve(&compile(src).unwrap().constraints)
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
            SolveOutcome::Solved { assignments, from_constraints } => {
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
        let out = solve_src(
            "symbol x : scalar\nconstrain x = 100 * 0.92\nsolve for { x }\n",
        );
        match out {
            SolveOutcome::Solved { assignments, .. } => {
                assert!((assignments[0].1 - 92.0).abs() < 1e-6, "{assignments:?}");
            }
            other => panic!("expected Solved, got {other:?}"),
        }
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
        let out = solve_src(
            "symbol x : scalar\nconstrain x <= 10\nsolve for { x }\n",
        );
        assert!(matches!(out, SolveOutcome::Unsupported { .. }));
    }

    #[test]
    fn a_nonlinear_term_is_unsupported_not_wrong() {
        // x * x = 4 is non-linear — we must NOT pretend to solve it.
        let out = solve_src(
            "symbol x : scalar\nconstrain x * x = 4\nsolve for { x }\n",
        );
        assert!(matches!(out, SolveOutcome::Unsupported { .. }));
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

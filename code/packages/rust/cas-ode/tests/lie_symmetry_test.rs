//! Integration tests for the Track L2 Lie point-symmetry handler.
//!
//! Mirrors `code/packages/python/cas-ode/tests/test_lie_symmetry.py`.
//! All assertions exercise the public `solve_ode` dispatcher so the
//! production path a caller would touch is what we cover.
//!
//! Rust-cas-ode keeps `Integrate(...)` as a structural IR node (no
//! symbolic integrator at this layer), so the assertions check the
//! *shape* of the implicit form rather than a fully closed-form value.

use cas_ode::{solve_ode, ODE2};
use symbolic_ir::{
    apply, int, sym, IRNode, ADD, COS, D, DIV, EQUAL, EXP, INTEGRATE, LOG, MUL, NEG, POW, SIN, SUB,
};

fn x() -> IRNode {
    sym("x")
}
fn y() -> IRNode {
    sym("y")
}
fn yp() -> IRNode {
    apply(sym(D), vec![y(), x()])
}

fn add(a: IRNode, b: IRNode) -> IRNode {
    apply(sym(ADD), vec![a, b])
}
fn sub(a: IRNode, b: IRNode) -> IRNode {
    apply(sym(SUB), vec![a, b])
}
fn mul(a: IRNode, b: IRNode) -> IRNode {
    apply(sym(MUL), vec![a, b])
}
fn div(a: IRNode, b: IRNode) -> IRNode {
    apply(sym(DIV), vec![a, b])
}
fn pow(a: IRNode, b: IRNode) -> IRNode {
    apply(sym(POW), vec![a, b])
}
fn sin_(a: IRNode) -> IRNode {
    apply(sym(SIN), vec![a])
}
fn cos_(a: IRNode) -> IRNode {
    apply(sym(COS), vec![a])
}
fn exp_(a: IRNode) -> IRNode {
    apply(sym(EXP), vec![a])
}
fn neg_(a: IRNode) -> IRNode {
    apply(sym(NEG), vec![a])
}
fn log_(a: IRNode) -> IRNode {
    apply(sym(LOG), vec![a])
}

fn contains_head(node: &IRNode, head: &str) -> bool {
    matches!(node, IRNode::Apply(app) if app.head == sym(head))
        || matches!(node, IRNode::Apply(app) if app.args.iter().any(|a| contains_head(a, head)))
}

fn head_is(node: &IRNode, head: &str) -> bool {
    matches!(node, IRNode::Apply(app) if app.head == sym(head))
}

// -----------------------------------------------------------------------------
// Section A — End-to-end via solve_ode
// -----------------------------------------------------------------------------

#[test]
fn translation_y_y_prime_equals_sin_x_closes() {
    // y' = sin(x) — separable intercepts; result is `Equal(y, ...)`.
    let zero = sub(yp(), sin_(x()));
    let result = solve_ode(zero, y(), x()).expect("expected a closed-form");
    assert!(head_is(&result, EQUAL), "got {result}");
}

#[test]
fn translation_x_logistic_produces_implicit_autonomous_form() {
    // y' = y(1-y) — autonomous nonlinear.  The separable handler matches
    // first via the `is_const_wrt(rhs, x)` branch and emits the implicit
    // form `Integrate(Div(1, y(1-y)), y) = x + %c`.  If Lie were reached,
    // it would produce the equivalent form (LHS=x).  Either is a valid
    // implicit description of the logistic ODE.
    let rhs = mul(y(), sub(int(1), y()));
    let zero = sub(yp(), rhs);
    let result = solve_ode(zero, y(), x()).expect("expected an implicit form");
    assert!(head_is(&result, EQUAL), "got {result}");
    // Must contain an Integrate (purely structural in Rust).
    assert!(contains_head(&result, INTEGRATE));
}

#[test]
fn scaling_homogeneous_y_squared_plus_xy_over_x_squared_closes() {
    // y' = (y² + xy) / x²  —  scale-invariant under (x, y) → (λx, λy).
    // The homogeneous-type handler matches first; if not, Lie's k=1 path
    // does.  Either way we should get an Equal-headed implicit form
    // containing Log(x) (the canonical reduction signature).
    let num = add(pow(y(), int(2)), mul(x(), y()));
    let denom = pow(x(), int(2));
    let rhs = div(num, denom);
    let zero = sub(yp(), rhs);
    let result = solve_ode(zero, y(), x()).expect("expected an implicit form");
    assert!(head_is(&result, EQUAL), "got {result}");
    assert!(contains_head(&result, LOG));
}

#[test]
fn fall_through_sin_xy_returns_none() {
    // y' = sin(xy) — no recognised symmetry; solver returns None and the
    // public ODE2 handler returns the input unchanged.
    let zero = sub(yp(), sin_(mul(x(), y())));
    let result = solve_ode(zero, y(), x());
    assert!(result.is_none(), "expected None, got {result:?}");
}

// -----------------------------------------------------------------------------
// Section B — Regression: existing handlers still win the race
// -----------------------------------------------------------------------------

#[test]
fn regression_linear_y_prime_plus_y_equals_x_uses_integrating_factor() {
    // y' + y - x = 0  →  linear (integrating factor `e^x`).
    let zero = sub(add(yp(), y()), x());
    let result = solve_ode(zero, y(), x()).expect("linear must close");
    assert!(head_is(&result, EQUAL));
    // The integrating-factor shape: y = (... + %c) / mu, mu = Exp(Integrate(...)).
    let s = format!("{result}");
    assert!(s.contains("Exp"), "missing Exp in linear solution: {s}");
}

#[test]
fn regression_separable_y_prime_equals_x_y_uses_separable() {
    // y' = x·y  → separable;  implicit form Integrate(1/y, y) = Integrate(x, x) + %c.
    let rhs = mul(x(), y());
    let zero = sub(yp(), rhs);
    let result = solve_ode(zero, y(), x()).expect("separable must close");
    assert!(head_is(&result, EQUAL));
    let s = format!("{result}");
    assert!(s.contains("Integrate"));
}

#[test]
fn ode2_handler_routes_logistic() {
    // Exercise the dispatcher via the public ODE2 wrapper.
    let table = cas_ode::build_ode_handler_table();
    let handler = table.get(ODE2).expect("ODE2 handler exists");
    let rhs = mul(y(), sub(int(1), y()));
    let zero = sub(yp(), rhs);
    let result = handler(&apply(sym(ODE2), vec![zero, y(), x()]));
    assert!(head_is(&result, EQUAL), "got {result}");
}

// Silence unused-import warnings from convenience constructors held in
// reserve for future Lie-specific assertions.
#[test]
fn _touch_unused_helpers() {
    let _ = (cos_(int(0)), exp_(int(0)), neg_(int(1)), log_(int(1)));
}

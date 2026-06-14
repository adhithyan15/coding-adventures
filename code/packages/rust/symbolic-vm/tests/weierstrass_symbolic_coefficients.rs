//! Track G2 (Rust port) — symbolic-coefficient Weierstrass lift.
//!
//! Mirrors
//! `code/packages/python/symbolic-vm/tests/test_weierstrass_symbolic_coefficients.py`
//! shipped with Python G1 / PR #5361, and the TypeScript port in
//! `code/packages/typescript/symbolic-vm/tests/weierstrass-symbolic-coefficients.test.ts`.
//!
//! The numeric Phase-34 Weierstrass helper only fires when `a` and
//! `b` in `∫ c / (a + b·sin(α·x+β)) dx` are concrete rationals.
//! Track G2 generalises it: when the user has declared the sign of
//! the discriminant `a² − b²` via `Assume(...)`, the integrator emits
//! the corresponding closed form with symbolic `a, b`.
//!
//! The branch selection is driven by `vm.assumptions` lookups against
//! the compound-relation store added in the Rust cas-simplify Track
//! G2 first half.  These tests cover all four branches (`> 0`,
//! `< 0`, `= 0`, no assumption → unevaluated) plus the
//! linear-argument lifting that must still compose.
//!
//! Structural assertions rather than numeric ones — the result is a
//! tree in symbolic `a, b` that no numeric evaluation can collapse
//! cheaply.  We assert the kind of the outer head (`Atan`, `Log`,
//! `Integrate`) and that the recorded discriminant radicand appears
//! literally somewhere in the tree.

use symbolic_ir::{
    apply, int, sym, IRNode, ADD, ATAN, COS, DIV, EQUAL, GREATER, INTEGRATE, LESS, LOG, MUL, POW,
    SIN, SQRT, SUB,
};
use symbolic_vm::{SymbolicBackend, VM};

fn make_vm() -> VM {
    VM::new(Box::new(SymbolicBackend::new()))
}

fn x_sym() -> IRNode {
    sym("x")
}
fn a_sym() -> IRNode {
    sym("a")
}
fn b_sym() -> IRNode {
    sym("b")
}

fn sq(node: IRNode) -> IRNode {
    apply(sym(POW), vec![node, int(2)])
}

fn gt(lhs: IRNode, rhs: IRNode) -> IRNode {
    apply(sym(GREATER), vec![lhs, rhs])
}
fn lt(lhs: IRNode, rhs: IRNode) -> IRNode {
    apply(sym(LESS), vec![lhs, rhs])
}
fn eq(lhs: IRNode, rhs: IRNode) -> IRNode {
    apply(sym(EQUAL), vec![lhs, rhs])
}

fn assume(vm: &mut VM, rel: IRNode) {
    vm.eval(apply(sym("Assume"), vec![rel]));
}

fn integrate_expr(f: IRNode) -> IRNode {
    apply(sym(INTEGRATE), vec![f, x_sym()])
}

fn is_integrate(node: &IRNode) -> bool {
    matches!(node, IRNode::Apply(a) if a.head == sym(INTEGRATE))
}

fn contains_head(node: &IRNode, head: &IRNode) -> bool {
    match node {
        IRNode::Apply(a) => &a.head == head || a.args.iter().any(|arg| contains_head(arg, head)),
        _ => false,
    }
}

fn contains_subtree(node: &IRNode, target: &IRNode) -> bool {
    if node == target {
        return true;
    }
    match node {
        IRNode::Apply(a) => a.args.iter().any(|arg| contains_subtree(arg, target)),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// disc > 0  →  arctan branch
// ---------------------------------------------------------------------------

#[test]
fn symbolic_sin_arctan_branch() {
    let mut vm = make_vm();
    assume(&mut vm, gt(sq(a_sym()), sq(b_sym())));
    let denom = apply(
        sym(ADD),
        vec![
            a_sym(),
            apply(sym(MUL), vec![b_sym(), apply(sym(SIN), vec![x_sym()])]),
        ],
    );
    let result = vm.eval(integrate_expr(apply(sym(DIV), vec![int(1), denom])));
    assert!(!is_integrate(&result), "got {result}");
    assert!(contains_head(&result, &sym(ATAN)));
    let expected_sqrt = apply(sym(SQRT), vec![apply(sym(SUB), vec![sq(a_sym()), sq(b_sym())])]);
    assert!(contains_subtree(&result, &expected_sqrt), "result = {result}");
}

#[test]
fn symbolic_cos_arctan_branch() {
    let mut vm = make_vm();
    assume(&mut vm, gt(sq(a_sym()), sq(b_sym())));
    let denom = apply(
        sym(ADD),
        vec![
            a_sym(),
            apply(sym(MUL), vec![b_sym(), apply(sym(COS), vec![x_sym()])]),
        ],
    );
    let result = vm.eval(integrate_expr(apply(sym(DIV), vec![int(1), denom])));
    assert!(!is_integrate(&result), "got {result}");
    assert!(contains_head(&result, &sym(ATAN)));
    let expected_sqrt = apply(sym(SQRT), vec![apply(sym(SUB), vec![sq(a_sym()), sq(b_sym())])]);
    assert!(contains_subtree(&result, &expected_sqrt), "result = {result}");
}

// ---------------------------------------------------------------------------
// disc < 0  →  log branch
// ---------------------------------------------------------------------------

#[test]
fn symbolic_sin_log_branch() {
    let mut vm = make_vm();
    assume(&mut vm, lt(sq(a_sym()), sq(b_sym())));
    let denom = apply(
        sym(ADD),
        vec![
            a_sym(),
            apply(sym(MUL), vec![b_sym(), apply(sym(SIN), vec![x_sym()])]),
        ],
    );
    let result = vm.eval(integrate_expr(apply(sym(DIV), vec![int(1), denom])));
    assert!(!is_integrate(&result), "got {result}");
    assert!(contains_head(&result, &sym(LOG)));
    let expected_sqrt = apply(sym(SQRT), vec![apply(sym(SUB), vec![sq(b_sym()), sq(a_sym())])]);
    assert!(contains_subtree(&result, &expected_sqrt), "result = {result}");
}

// ---------------------------------------------------------------------------
// disc = 0  →  degenerate branch
// ---------------------------------------------------------------------------

#[test]
fn symbolic_sin_degenerate_branch() {
    let mut vm = make_vm();
    assume(&mut vm, eq(sq(a_sym()), sq(b_sym())));
    let denom = apply(
        sym(ADD),
        vec![
            a_sym(),
            apply(sym(MUL), vec![b_sym(), apply(sym(SIN), vec![x_sym()])]),
        ],
    );
    let result = vm.eval(integrate_expr(apply(sym(DIV), vec![int(1), denom])));
    assert!(!is_integrate(&result), "got {result}");
    // No outer arctan, no outer log — only `tan(...)` below an
    // arithmetic tree.
    assert!(!contains_head(&result, &sym(ATAN)));
    assert!(!contains_head(&result, &sym(LOG)));
}

// ---------------------------------------------------------------------------
// No assumption  →  unevaluated
// ---------------------------------------------------------------------------

#[test]
fn no_assumption_returns_unevaluated() {
    let mut vm = make_vm();
    let denom = apply(
        sym(ADD),
        vec![
            a_sym(),
            apply(sym(MUL), vec![b_sym(), apply(sym(SIN), vec![x_sym()])]),
        ],
    );
    let result = vm.eval(integrate_expr(apply(sym(DIV), vec![int(1), denom])));
    assert!(is_integrate(&result));
}

// ---------------------------------------------------------------------------
// Linear-argument lifting composes (Phase 38 + Track G2).
// ---------------------------------------------------------------------------

#[test]
fn symbolic_linear_argument_arctan() {
    let mut vm = make_vm();
    assume(&mut vm, gt(sq(a_sym()), sq(b_sym())));
    let inner = apply(sym(ADD), vec![apply(sym(MUL), vec![int(2), x_sym()]), int(1)]);
    let denom = apply(
        sym(ADD),
        vec![
            a_sym(),
            apply(sym(MUL), vec![b_sym(), apply(sym(SIN), vec![inner])]),
        ],
    );
    let result = vm.eval(integrate_expr(apply(sym(DIV), vec![int(1), denom])));
    assert!(!is_integrate(&result));
    assert!(contains_head(&result, &sym(ATAN)));
    let expected_sqrt = apply(sym(SQRT), vec![apply(sym(SUB), vec![sq(a_sym()), sq(b_sym())])]);
    assert!(contains_subtree(&result, &expected_sqrt));
}

// ---------------------------------------------------------------------------
// Numeric regression — the symbolic helper must not steal numeric cases.
// ---------------------------------------------------------------------------

#[test]
fn numeric_regression_arctan_still_works() {
    let mut vm = make_vm();
    let denom = apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![x_sym()])]);
    let result = vm.eval(integrate_expr(apply(sym(DIV), vec![int(1), denom])));
    assert!(!is_integrate(&result));
    assert!(contains_head(&result, &sym(ATAN)));
}

#[test]
fn numeric_regression_log_still_works() {
    let mut vm = make_vm();
    let denom = apply(
        sym(ADD),
        vec![int(1), apply(sym(MUL), vec![int(2), apply(sym(SIN), vec![x_sym()])])],
    );
    let result = vm.eval(integrate_expr(apply(sym(DIV), vec![int(1), denom])));
    assert!(!is_integrate(&result));
    assert!(contains_head(&result, &sym(LOG)));
}

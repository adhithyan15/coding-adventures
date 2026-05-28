//! Bivariate Hensel-lifting `Factor` integration tests (Track D2).
//!
//! Mirrors the Python reference tests in
//! `code/packages/python/symbolic-vm/tests/`.  Each acceptance case is
//! constructed structurally to avoid relying on the simplifier's term
//! ordering; we just verify that `Factor(...)` returns the expected
//! `Mul(g, h)` shape for factorable bivariate polynomials and the
//! original unevaluated `Factor(...)` (or the input itself) for inputs
//! Hensel can't factor.

use symbolic_ir::{apply, int, sym, IRNode, ADD, MUL, POW, SUB};
use symbolic_vm::{SymbolicBackend, VM};

fn symbolic() -> VM {
    VM::new(Box::new(SymbolicBackend::new()))
}

fn factor(inner: IRNode) -> IRNode {
    apply(sym("Factor"), vec![inner])
}

/// Build an IR node ``x^p · y^q`` (with elision of trivial factors).
fn mono(coef: i64, x_pow: i64, y_pow: i64) -> IRNode {
    let mut parts: Vec<IRNode> = Vec::new();
    if coef != 1 || (x_pow == 0 && y_pow == 0) {
        parts.push(int(coef));
    }
    if x_pow == 1 {
        parts.push(sym("x"));
    } else if x_pow > 1 {
        parts.push(apply(sym(POW), vec![sym("x"), int(x_pow)]));
    }
    if y_pow == 1 {
        parts.push(sym("y"));
    } else if y_pow > 1 {
        parts.push(apply(sym(POW), vec![sym("y"), int(y_pow)]));
    }
    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        apply(sym(MUL), parts)
    }
}

fn add(parts: Vec<IRNode>) -> IRNode {
    apply(sym(ADD), parts)
}

/// Is the result a Mul(...) of two non-trivial factor expressions?
fn is_mul_of_two_factors(node: &IRNode) -> bool {
    match node {
        IRNode::Apply(apply) => match &apply.head {
            IRNode::Symbol(s) if s == MUL => apply.args.len() >= 2,
            _ => false,
        },
        _ => false,
    }
}

#[test]
fn hensel_factor_x2_xy_minus_2y2_splits() {
    // x² + xy − 2y² → Mul(...) (Hensel splits as (x+2y)(x-y) or equivalent).
    let inner = add(vec![
        mono(1, 2, 0),
        mono(1, 1, 1),
        mono(-2, 0, 2),
    ]);
    let result = symbolic().eval(factor(inner));
    assert!(
        is_mul_of_two_factors(&result),
        "expected Mul(...) of two factors, got {:?}",
        result
    );
}

#[test]
fn hensel_factor_non_unit_leading_2x2_3xy_minus_2y2_splits() {
    // 2x² + 3xy − 2y² → Mul(...) (Hensel splits as (2x-y)(x+2y) or equivalent).
    let inner = add(vec![
        mono(2, 2, 0),
        mono(3, 1, 1),
        mono(-2, 0, 2),
    ]);
    let result = symbolic().eval(factor(inner));
    assert!(
        is_mul_of_two_factors(&result),
        "expected Mul(...) of two factors, got {:?}",
        result
    );
}

#[test]
fn hensel_factor_x3_minus_y3_splits() {
    // x³ − y³ → Mul((x − y), (x² + xy + y²))
    let inner = apply(sym(SUB), vec![mono(1, 3, 0), mono(1, 0, 3)]);
    let result = symbolic().eval(factor(inner));
    assert!(
        is_mul_of_two_factors(&result),
        "expected Mul(...) of two factors, got {:?}",
        result
    );
}

#[test]
fn hensel_factor_x2_plus_y2_plus_1_irreducible() {
    // x² + y² + 1 is irreducible over ℚ.  The factor handler should
    // return the input unevaluated (no Mul of two non-trivial factors).
    let inner = add(vec![mono(1, 2, 0), mono(1, 0, 2), int(1)]);
    let result = symbolic().eval(factor(inner.clone()));
    // Either we get the input back, or `Factor(...)` unevaluated.
    assert!(!is_mul_of_two_factors(&result),
        "expected no factorisation for irreducible input, got {:?}", result);
}

#[test]
fn hensel_factor_x2_minus_1_falls_through_to_univariate() {
    // Univariate x² − 1 should still be factored by the existing
    // univariate path — verify we get Mul((x+1), (x-1)) (or equivalent),
    // which the existing handler produces independently of Hensel.
    let inner = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)],
    );
    let result = symbolic().eval(factor(inner));
    // Expected: Mul(Add(1, x), Add(-1, x))
    let expected = apply(
        sym(MUL),
        vec![
            apply(sym(ADD), vec![int(1), sym("x")]),
            apply(sym(ADD), vec![int(-1), sym("x")]),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn hensel_factor_x_plus_y_is_already_irreducible() {
    // x + y has no non-trivial factorisation.  Should not invoke Hensel
    // (the univariate-of-one-variable check fails); Factor returns
    // unevaluated.
    let inner = add(vec![sym("x"), sym("y")]);
    let result = symbolic().eval(factor(inner.clone()));
    assert!(!is_mul_of_two_factors(&result));
}

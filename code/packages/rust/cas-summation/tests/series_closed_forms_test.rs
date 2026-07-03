//! Track I2 — end-to-end integration tests for the closed-form series
//! recogniser through `evaluate_sum`.
//!
//! Confirms the dispatcher routes ``hi = %inf`` patterns through the new
//! handler and that the finite-bound paths (Faulhaber, small-range
//! numeric, Gosper) stay untouched.

use cas_summation::{evaluate_sum, GAMMA_FUNC, SUM};
use symbolic_ir::{apply, int, sym, IRNode, ADD, COS, DIV, EXP, LOG, MUL, POW, SIN, SUB};

/// Identity-ish evaluator: fully recurses into Apply but does no numeric
/// folding.  Sufficient for verifying the dispatcher hands back the
/// recogniser's emitted IR shape.
fn ident(node: IRNode) -> IRNode {
    if let IRNode::Apply(a) = node {
        apply(
            a.head.clone(),
            a.args.iter().map(|x| ident(x.clone())).collect(),
        )
    } else {
        node
    }
}

fn k_sym() -> IRNode {
    sym("k")
}

fn inf() -> IRNode {
    sym("%inf")
}

fn binary(head: &str, a: IRNode, b: IRNode) -> IRNode {
    apply(sym(head), vec![a, b])
}

fn unary(head: &str, a: IRNode) -> IRNode {
    apply(sym(head), vec![a])
}

fn inv_k_pow(m: i64) -> IRNode {
    binary(DIV, int(1), binary(POW, k_sym(), int(m)))
}

fn alt_inv_k_pow(m: i64) -> IRNode {
    let neg_one_pow = binary(POW, int(-1), binary(SUB, k_sym(), int(1)));
    if m == 1 {
        binary(DIV, neg_one_pow, k_sym())
    } else {
        binary(DIV, neg_one_pow, binary(POW, k_sym(), int(m)))
    }
}

#[test]
fn evaluate_sum_zeta_6() {
    // Σ 1/k^6 = π^6/945
    let result = evaluate_sum(inv_k_pow(6), k_sym(), int(1), inf(), ident);
    assert_eq!(
        result,
        binary(DIV, binary(POW, sym("%pi"), int(6)), int(945))
    );
}

#[test]
fn evaluate_sum_eta_1_log_2() {
    // Σ (-1)^(k-1)/k = log(2)
    let result = evaluate_sum(alt_inv_k_pow(1), k_sym(), int(1), inf(), ident);
    assert_eq!(result, unary(LOG, int(2)));
}

#[test]
fn evaluate_sum_cos_series() {
    // Σ (-1)^k · x^(2k)/(2k)! = cos(x)
    let x = sym("x");
    let sign = binary(POW, int(-1), k_sym());
    let body = binary(
        DIV,
        binary(POW, x.clone(), binary(MUL, int(2), k_sym())),
        unary(
            GAMMA_FUNC,
            binary(ADD, binary(MUL, int(2), k_sym()), int(1)),
        ),
    );
    let summand = binary(MUL, sign, body);
    let result = evaluate_sum(summand, k_sym(), int(0), inf(), ident);
    assert_eq!(result, unary(COS, x));
}

#[test]
fn evaluate_sum_exp_series() {
    // Σ x^k/k! = exp(x)
    let x = sym("x");
    let summand = binary(
        DIV,
        binary(POW, x.clone(), k_sym()),
        unary(GAMMA_FUNC, binary(ADD, k_sym(), int(1))),
    );
    let result = evaluate_sum(summand, k_sym(), int(0), inf(), ident);
    assert_eq!(result, unary(EXP, x));
}

#[test]
fn unrecognised_sin_k_stays_unevaluated() {
    // Σ sin(k) at infinity — recogniser returns None; dispatcher emits SUM.
    let f = unary(SIN, k_sym());
    let result = evaluate_sum(f.clone(), k_sym(), int(1), inf(), ident);
    assert_eq!(
        result,
        apply(sym(SUM), vec![f, k_sym(), int(1), inf()])
    );
}

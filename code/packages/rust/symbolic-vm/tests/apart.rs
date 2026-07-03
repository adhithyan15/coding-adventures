//! Track B1 + B3 — Apart partial-fraction decomposition (Rust port).
//!
//! B1: simple roots (residue formula).
//! B3: repeated linear factors (Phase 48 — Taylor expansion + formal
//!     power-series division).
//!
//! Mirrors the TypeScript test cases (and the Python reference) listed in
//! ``code/specs/macsyma-finish-plan.md`` (Tracks B1 + B3) and verified
//! byte-for-byte against the Python output.

use symbolic_ir::{apply, int, rat, sym, ADD, DIV, MUL, NEG, POW, SUB};
use symbolic_vm::{SymbolicBackend, VM};

fn vm() -> VM {
    VM::new(Box::new(SymbolicBackend::new()))
}

fn apart(inner: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    apply(sym("Apart"), vec![inner, sym("x")])
}

fn apart_k(inner: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    apply(sym("Apart"), vec![inner, sym("k")])
}

#[test]
fn apart_one_over_x_squared_minus_one_acceptance() {
    let mut v = vm();
    // 1 / (x^2 - 1)
    let inner = apply(
        sym(DIV),
        vec![
            int(1),
            apply(sym(SUB), vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)]),
        ],
    );
    let result = v.eval(apart(inner));
    // Roots sort ascending: -1 first, then 1.  Matches Python's output:
    // Add(Div(-1/2, (1+x)), Div(1/2, (-1+x))).
    let expected = apply(
        sym(ADD),
        vec![
            apply(sym(DIV), vec![rat(-1, 2), apply(sym(ADD), vec![int(1), sym("x")])]),
            apply(sym(DIV), vec![rat(1, 2), apply(sym(ADD), vec![int(-1), sym("x")])]),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_three_distinct_simple_roots() {
    let mut v = vm();
    let f1 = apply(sym(SUB), vec![sym("x"), int(1)]);
    let f2 = apply(sym(SUB), vec![sym("x"), int(2)]);
    let f3 = apply(sym(SUB), vec![sym("x"), int(3)]);
    let inner = apply(
        sym(DIV),
        vec![int(1), apply(sym(MUL), vec![apply(sym(MUL), vec![f1, f2]), f3])],
    );
    let result = v.eval(apart(inner));
    // Residues: A_1 = 1/2, A_2 = -1, A_3 = 1/2.
    // ``A = -1`` renders as Neg(Div(1, ...)).
    let expected = apply(
        sym(ADD),
        vec![
            apply(
                sym(ADD),
                vec![
                    apply(sym(DIV), vec![rat(1, 2), apply(sym(ADD), vec![int(-1), sym("x")])]),
                    apply(
                        sym(NEG),
                        vec![apply(sym(DIV), vec![int(1), apply(sym(ADD), vec![int(-2), sym("x")])])],
                    ),
                ],
            ),
            apply(sym(DIV), vec![rat(1, 2), apply(sym(ADD), vec![int(-3), sym("x")])]),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_improper_fraction_x_cubed_over_x_squared_minus_one() {
    let mut v = vm();
    let inner = apply(
        sym(DIV),
        vec![
            apply(sym(POW), vec![sym("x"), int(3)]),
            apply(sym(SUB), vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)]),
        ],
    );
    let result = v.eval(apart(inner));
    // x^3/(x^2-1) = x + 1/(2(x-1)) + 1/(2(x+1)).
    // ``from_polynomial([0, 1], x)`` collapses to bare x.
    let expected = apply(
        sym(ADD),
        vec![
            sym("x"),
            apply(
                sym(ADD),
                vec![
                    apply(sym(DIV), vec![rat(1, 2), apply(sym(ADD), vec![int(1), sym("x")])]),
                    apply(sym(DIV), vec![rat(1, 2), apply(sym(ADD), vec![int(-1), sym("x")])]),
                ],
            ),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_irreducible_quadratic_returns_rational() {
    let mut v = vm();
    let inner = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)])],
    );
    let result = v.eval(apart(inner));
    let expected = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(1), apply(sym(POW), vec![sym("x"), int(2)])])],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_improper_irreducible_quadratic_splits_polynomial_part() {
    let mut v = vm();
    let x2 = apply(sym(POW), vec![sym("x"), int(2)]);
    let inner = apply(
        sym(DIV),
        vec![
            apply(sym(ADD), vec![x2.clone(), int(2)]),
            apply(sym(ADD), vec![x2, int(1)]),
        ],
    );
    let result = v.eval(apart(inner));
    let expected = apply(
        sym(ADD),
        vec![
            int(1),
            apply(
                sym(DIV),
                vec![int(1), apply(sym(ADD), vec![int(1), apply(sym(POW), vec![sym("x"), int(2)])])],
            ),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_already_polynomial_passes_through() {
    let mut v = vm();
    // Apart(x + 1, x) → x + 1
    let inner = apply(sym(ADD), vec![sym("x"), int(1)]);
    let result = v.eval(apart(inner));
    // ``to_rational(x+1, x)`` → num = [1, 1], den = [1]; the early-return
    // path emits ``from_polynomial(num, x)`` = Add(1, x).
    let expected = apply(sym(ADD), vec![int(1), sym("x")]);
    assert_eq!(result, expected);
}

// --- Track B3 (Phase 48 repeated linear factors) ---------------------------

#[test]
fn apart_k_sq_kp1_sq_acceptance() {
    let mut v = vm();
    // 1 / (k^2 * (k+1)^2)
    let k2 = apply(sym(POW), vec![sym("k"), int(2)]);
    let kp1 = apply(sym(ADD), vec![sym("k"), int(1)]);
    let kp1_sq = apply(sym(POW), vec![kp1, int(2)]);
    let inner = apply(sym(DIV), vec![int(1), apply(sym(MUL), vec![k2, kp1_sq])]);
    let result = v.eval(apart_k(inner));
    // Roots sorted ascending: -1 (mult 2), 0 (mult 2).
    // For r = -1: Q(x) = k^2, Q(-1+t) = 1 - 2t + t^2; φ_0 = 1, φ_1 = 2.
    //   A_{-1, 2} = 1,  A_{-1, 1} = 2.
    // For r = 0: Q(x) = (k+1)^2, Q(0+t) = 1 + 2t + t^2; φ_0 = 1, φ_1 = -2.
    //   A_{0, 2} = 1,  A_{0, 1} = -2.
    // Emit (left-associated): 2/(1+k), 1/(1+k)^2, -2/k, 1/k^2.
    let expected = apply(
        sym(ADD),
        vec![
            apply(
                sym(ADD),
                vec![
                    apply(
                        sym(ADD),
                        vec![
                            apply(sym(DIV), vec![int(2), apply(sym(ADD), vec![int(1), sym("k")])]),
                            apply(
                                sym(DIV),
                                vec![
                                    int(1),
                                    apply(
                                        sym(POW),
                                        vec![apply(sym(ADD), vec![int(1), sym("k")]), int(2)],
                                    ),
                                ],
                            ),
                        ],
                    ),
                    apply(sym(DIV), vec![int(-2), sym("k")]),
                ],
            ),
            apply(sym(DIV), vec![int(1), apply(sym(POW), vec![sym("k"), int(2)])]),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_triple_root_one_over_k_minus_one_cubed() {
    let mut v = vm();
    let inner = apply(
        sym(DIV),
        vec![
            int(1),
            apply(sym(POW), vec![apply(sym(SUB), vec![sym("k"), int(1)]), int(3)]),
        ],
    );
    let result = v.eval(apart_k(inner));
    // r = 1, m = 3, Q(x) = 1.  φ_0 = 1, φ_1 = φ_2 = 0.
    // Single term emitted: 1/(k-1)^3 in normalised form 1/(-1+k)^3.
    let expected = apply(
        sym(DIV),
        vec![
            int(1),
            apply(sym(POW), vec![apply(sym(ADD), vec![int(-1), sym("k")]), int(3)]),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_mixed_simple_plus_repeated() {
    let mut v = vm();
    // 1 / ((k-1)(k-2)^2)
    let f1 = apply(sym(SUB), vec![sym("k"), int(1)]);
    let f2_sq = apply(sym(POW), vec![apply(sym(SUB), vec![sym("k"), int(2)]), int(2)]);
    let inner = apply(sym(DIV), vec![int(1), apply(sym(MUL), vec![f1, f2_sq])]);
    let result = v.eval(apart_k(inner));
    // Roots ascending: 1 (simple) then 2 (mult 2).
    // For r = 1: Q(x) = (k-2)^2, Q(1) = 1.  φ_0 = 1. → A = 1.
    // For r = 2: Q(x) = (k-1); Q(2+t) = 1 + t.  φ_0 = 1, φ_1 = -1.
    //   A_{2, 2} = 1,  A_{2, 1} = -1.
    // Emit: 1/(-1+k), -1/(-2+k) [Neg(Div(1, …))], 1/(-2+k)^2.
    let expected = apply(
        sym(ADD),
        vec![
            apply(
                sym(ADD),
                vec![
                    apply(sym(DIV), vec![int(1), apply(sym(ADD), vec![int(-1), sym("k")])]),
                    apply(
                        sym(NEG),
                        vec![apply(sym(DIV), vec![int(1), apply(sym(ADD), vec![int(-2), sym("k")])])],
                    ),
                ],
            ),
            apply(
                sym(DIV),
                vec![
                    int(1),
                    apply(sym(POW), vec![apply(sym(ADD), vec![int(-2), sym("k")]), int(2)]),
                ],
            ),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_irreducible_plus_repeated_decomposes_poles_plus_residual() {
    let mut v = vm();
    // 1 / ((x^2 + 1) * (x - 1)^2)
    let quad = apply(sym(ADD), vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)]);
    let lin_sq = apply(sym(POW), vec![apply(sym(SUB), vec![sym("x"), int(1)]), int(2)]);
    let inner = apply(sym(DIV), vec![int(1), apply(sym(MUL), vec![quad, lin_sq])]);
    let result = v.eval(apart(inner.clone()));
    let expected = apply(
        sym(ADD),
        vec![
            apply(
                sym(ADD),
                vec![
                    apply(sym(DIV), vec![rat(-1, 2), apply(sym(ADD), vec![int(-1), sym("x")])]),
                    apply(
                        sym(DIV),
                        vec![
                            rat(1, 2),
                            apply(sym(POW), vec![apply(sym(ADD), vec![int(-1), sym("x")]), int(2)]),
                        ],
                    ),
                ],
            ),
            apply(
                sym(DIV),
                vec![
                    apply(sym(MUL), vec![rat(1, 2), sym("x")]),
                    apply(sym(ADD), vec![int(1), apply(sym(POW), vec![sym("x"), int(2)])]),
                ],
            ),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_single_repeated_root_one_over_x_minus_two_sq() {
    let mut v = vm();
    let inner = apply(
        sym(DIV),
        vec![
            int(1),
            apply(sym(POW), vec![apply(sym(SUB), vec![sym("x"), int(2)]), int(2)]),
        ],
    );
    let result = v.eval(apart(inner));
    // r = 2, m = 2, Q(x) = 1.  φ_0 = 1, φ_1 = 0.  Single term.
    let expected = apply(
        sym(DIV),
        vec![
            int(1),
            apply(sym(POW), vec![apply(sym(ADD), vec![int(-2), sym("x")]), int(2)]),
        ],
    );
    assert_eq!(result, expected);
}

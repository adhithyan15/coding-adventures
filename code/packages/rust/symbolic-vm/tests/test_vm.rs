//! Integration tests for the symbolic VM.
//!
//! These tests exercise the full eval loop — constructing IR expressions
//! directly and evaluating them under each backend.

use symbolic_ir::{
    apply, flt, int, rat, sym, ACOS, ACOSH, ADD, AND, ASIN, ASINH, ASSIGN, ATAN, ATANH, COS, COSH,
    COTH, CSCH, D, DEFINE, DIV, EXP, IF, INTEGRATE, LIST, LOG, MUL, NEG, NOT, OR, POW, SECH, SIN,
    SINH, SQRT, SUB, TAN, TANH,
};
use symbolic_vm::{StrictBackend, SymbolicBackend, VM};

// ---------------------------------------------------------------------------
// Helper: build a VM
// ---------------------------------------------------------------------------

fn strict() -> VM {
    VM::new(Box::new(StrictBackend::new()))
}

fn symbolic() -> VM {
    VM::new(Box::new(SymbolicBackend::new()))
}

/// Evaluate a single expression in a fresh symbolic VM.
fn eval(expr: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    symbolic().eval(expr)
}

fn d(f: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    symbolic().eval(apply(sym(D), vec![f, sym("x")]))
}

fn integrate(f: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    symbolic().eval(apply(sym(INTEGRATE), vec![f, sym("x")]))
}

fn assert_float_close(node: symbolic_ir::IRNode, expected: f64) {
    let symbolic_ir::IRNode::Float(actual) = node else {
        panic!("expected Float({expected:?}), got {node}");
    };
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected:?}, got {actual:?}"
    );
}

// ---------------------------------------------------------------------------
// Numeric literals pass through unchanged
// ---------------------------------------------------------------------------

#[test]
fn integer_literal_unchanged() {
    assert_eq!(symbolic().eval(int(42)), int(42));
}

#[test]
// 3.14 is arbitrary float test data, not an approximation of PI.
#[allow(clippy::approx_constant)]
fn float_literal_unchanged() {
    assert_eq!(symbolic().eval(flt(3.14)), flt(3.14));
}

#[test]
fn rational_literal_unchanged() {
    assert_eq!(symbolic().eval(rat(1, 2)), rat(1, 2));
}

// ---------------------------------------------------------------------------
// Arithmetic — strict backend (fully numeric)
// ---------------------------------------------------------------------------

#[test]
fn strict_add_integers() {
    assert_eq!(strict().eval(apply(sym(ADD), vec![int(2), int(3)])), int(5));
}

#[test]
fn strict_sub_integers() {
    assert_eq!(
        strict().eval(apply(sym(SUB), vec![int(10), int(3)])),
        int(7)
    );
}

#[test]
fn strict_mul_integers() {
    assert_eq!(
        strict().eval(apply(sym(MUL), vec![int(4), int(5)])),
        int(20)
    );
}

#[test]
fn strict_div_integers_exact() {
    // 6 / 3 = 2 (exact integer)
    assert_eq!(strict().eval(apply(sym(DIV), vec![int(6), int(3)])), int(2));
}

#[test]
fn strict_div_integers_rational() {
    // 1 / 3 = 1/3 (exact rational)
    assert_eq!(
        strict().eval(apply(sym(DIV), vec![int(1), int(3)])),
        rat(1, 3)
    );
}

#[test]
fn strict_pow_integers() {
    assert_eq!(
        strict().eval(apply(sym(POW), vec![int(2), int(10)])),
        int(1024)
    );
}

#[test]
fn strict_neg() {
    assert_eq!(strict().eval(apply(sym(NEG), vec![int(5)])), int(-5));
}

#[test]
fn strict_neg_rational() {
    assert_eq!(strict().eval(apply(sym(NEG), vec![rat(1, 3)])), rat(-1, 3));
}

#[test]
fn strict_nested_arithmetic() {
    // (2 + 3) * 4 = 20
    let inner = apply(sym(ADD), vec![int(2), int(3)]);
    let outer = apply(sym(MUL), vec![inner, int(4)]);
    assert_eq!(strict().eval(outer), int(20));
}

// ---------------------------------------------------------------------------
// Arithmetic — symbolic backend (identity folding)
// ---------------------------------------------------------------------------

#[test]
fn symbolic_add_fold() {
    // Add(2, 3) → 5
    assert_eq!(
        symbolic().eval(apply(sym(ADD), vec![int(2), int(3)])),
        int(5)
    );
}

#[test]
fn symbolic_add_identity_right() {
    // Add(x, 0) → x
    let expr = apply(sym(ADD), vec![sym("x"), int(0)]);
    assert_eq!(symbolic().eval(expr), sym("x"));
}

#[test]
fn symbolic_add_identity_left() {
    // Add(0, x) → x
    let expr = apply(sym(ADD), vec![int(0), sym("x")]);
    assert_eq!(symbolic().eval(expr), sym("x"));
}

#[test]
fn symbolic_mul_absorbing_zero() {
    // Mul(0, x) → 0
    assert_eq!(
        symbolic().eval(apply(sym(MUL), vec![int(0), sym("x")])),
        int(0)
    );
}

#[test]
fn symbolic_mul_identity() {
    // Mul(1, x) → x
    assert_eq!(
        symbolic().eval(apply(sym(MUL), vec![int(1), sym("x")])),
        sym("x")
    );
}

#[test]
fn symbolic_pow_zero_exponent() {
    // Pow(x, 0) → 1
    assert_eq!(
        symbolic().eval(apply(sym(POW), vec![sym("x"), int(0)])),
        int(1)
    );
}

#[test]
fn symbolic_pow_unit_exponent() {
    // Pow(x, 1) → x
    assert_eq!(
        symbolic().eval(apply(sym(POW), vec![sym("x"), int(1)])),
        sym("x")
    );
}

#[test]
fn symbolic_neg_double_negation() {
    // Neg(Neg(x)) → x
    let inner = apply(sym(NEG), vec![sym("x")]);
    let outer = apply(sym(NEG), vec![inner]);
    assert_eq!(symbolic().eval(outer), sym("x"));
}

#[test]
fn symbolic_unknown_head_passes_through() {
    // UnknownFunc(x) → UnknownFunc(x) unchanged in symbolic mode
    let expr = apply(sym("UnknownFunc"), vec![sym("x")]);
    assert_eq!(symbolic().eval(expr.clone()), expr);
}

#[test]
fn symbolic_factor_univariate_integer_polynomial() {
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(SUB),
            vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                apply(sym(ADD), vec![int(1), sym("x")]),
                apply(sym(ADD), vec![int(-1), sym("x")]),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_common_multivariate_term() {
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(SUB),
            vec![
                apply(
                    sym(MUL),
                    vec![apply(sym(POW), vec![sym("x"), int(2)]), sym("y")],
                ),
                sym("y"),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                sym("y"),
                apply(
                    sym(MUL),
                    vec![
                        apply(sym(ADD), vec![int(1), sym("x")]),
                        apply(sym(ADD), vec![int(-1), sym("x")]),
                    ],
                ),
            ],
        )
    );
}

// Integer content extraction: GCD of integer coefficients and/or common
// symbolic powers are pulled out before the specific pattern matchers.

#[test]
fn symbolic_factor_extracts_multivariate_integer_content_only() {
    // Factor(2*x + 4*y) → 2*(x + 2*y)
    //
    // Both terms share no symbolic factor, but their coefficients have GCD 2.
    // The residual (x + 2*y) is bivariate so it is not wrapped in Factor.
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(sym(MUL), vec![int(2), sym("x")]),
                apply(sym(MUL), vec![int(4), sym("y")]),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                int(2),
                apply(
                    sym(ADD),
                    vec![sym("x"), apply(sym(MUL), vec![int(2), sym("y")])],
                ),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_multivariate_integer_content_and_symbolic() {
    // Factor(2*x*y + 2*x*z) → 2*x*(y + z)
    //
    // All terms share integer GCD 2 and symbolic factor x; the residual
    // (y + z) spans two variables so stays unevaluated.
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(
                    sym(MUL),
                    vec![int(2), apply(sym(MUL), vec![sym("x"), sym("y")])],
                ),
                apply(
                    sym(MUL),
                    vec![int(2), apply(sym(MUL), vec![sym("x"), sym("z")])],
                ),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                apply(sym(MUL), vec![int(2), sym("x")]),
                apply(sym(ADD), vec![sym("y"), sym("z")]),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_multivariate_integer_content_with_recursive_factoring() {
    // Factor(2*x^2*y - 2*y) → 2*y*(x+1)*(x-1)
    //
    // GCD 2 and symbolic factor y are pulled out.  The residual x^2 - 1 is
    // univariate so it is wrapped in Factor and recursively factored to
    // (x+1)*(x-1).
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(
                    sym(MUL),
                    vec![
                        int(2),
                        apply(
                            sym(MUL),
                            vec![apply(sym(POW), vec![sym("x"), int(2)]), sym("y")],
                        ),
                    ],
                ),
                apply(sym(MUL), vec![int(-2), sym("y")]),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                apply(sym(MUL), vec![int(2), sym("y")]),
                apply(
                    sym(MUL),
                    vec![
                        apply(sym(ADD), vec![int(1), sym("x")]),
                        apply(sym(ADD), vec![int(-1), sym("x")]),
                    ],
                ),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_multivariate_integer_content() {
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(
                    sym(MUL),
                    vec![int(2), apply(sym(MUL), vec![sym("x"), sym("y")])],
                ),
                apply(
                    sym(MUL),
                    vec![int(2), apply(sym(MUL), vec![sym("x"), sym("z")])],
                ),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                apply(sym(MUL), vec![int(2), sym("x")]),
                apply(sym(ADD), vec![sym("y"), sym("z")]),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_negative_multivariate_integer_content() {
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(
                    sym(MUL),
                    vec![int(-2), apply(sym(MUL), vec![sym("x"), sym("y")])],
                ),
                apply(
                    sym(MUL),
                    vec![int(-2), apply(sym(MUL), vec![sym("x"), sym("z")])],
                ),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                apply(sym(MUL), vec![int(-2), sym("x")]),
                apply(sym(ADD), vec![sym("y"), sym("z")]),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_multivariate_perfect_square() {
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(
                    sym(ADD),
                    vec![
                        apply(sym(POW), vec![sym("x"), int(2)]),
                        apply(
                            sym(MUL),
                            vec![int(2), apply(sym(MUL), vec![sym("x"), sym("y")])],
                        ),
                    ],
                ),
                apply(sym(POW), vec![sym("y"), int(2)]),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(POW),
            vec![apply(sym(ADD), vec![sym("x"), sym("y")]), int(2)]
        )
    );
}

#[test]
fn symbolic_factor_extracts_multivariate_difference_of_squares() {
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(SUB),
            vec![
                apply(sym(POW), vec![sym("x"), int(2)]),
                apply(sym(POW), vec![sym("y"), int(2)]),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                apply(sym(SUB), vec![sym("x"), sym("y")]),
                apply(sym(ADD), vec![sym("x"), sym("y")]),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_multivariate_difference_of_cubes() {
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(SUB),
            vec![
                apply(sym(POW), vec![sym("x"), int(3)]),
                apply(sym(POW), vec![sym("y"), int(3)]),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                apply(sym(SUB), vec![sym("x"), sym("y")]),
                apply(
                    sym(ADD),
                    vec![
                        apply(
                            sym(ADD),
                            vec![
                                apply(sym(POW), vec![sym("x"), int(2)]),
                                apply(sym(MUL), vec![sym("x"), sym("y")]),
                            ],
                        ),
                        apply(sym(POW), vec![sym("y"), int(2)]),
                    ],
                ),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_multivariate_sum_of_cubes() {
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(sym(POW), vec![sym("x"), int(3)]),
                apply(sym(POW), vec![sym("y"), int(3)]),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                apply(sym(ADD), vec![sym("x"), sym("y")]),
                apply(
                    sym(ADD),
                    vec![
                        apply(
                            sym(ADD),
                            vec![
                                apply(sym(POW), vec![sym("x"), int(2)]),
                                apply(
                                    sym(MUL),
                                    vec![int(-1), apply(sym(MUL), vec![sym("x"), sym("y")]),],
                                ),
                            ],
                        ),
                        apply(sym(POW), vec![sym("y"), int(2)]),
                    ],
                ),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_grouped_multivariate_terms() {
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(
                    sym(ADD),
                    vec![
                        apply(sym(MUL), vec![sym("x"), sym("y")]),
                        apply(sym(MUL), vec![sym("x"), sym("z")]),
                    ],
                ),
                apply(sym(ADD), vec![sym("y"), sym("z")]),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                apply(sym(ADD), vec![sym("x"), int(1)]),
                apply(sym(ADD), vec![sym("y"), sym("z")]),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_grouped_multivariate_terms_with_signed_residuals() {
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(
                    sym(SUB),
                    vec![
                        apply(sym(MUL), vec![sym("x"), sym("y")]),
                        apply(sym(MUL), vec![sym("x"), sym("z")]),
                    ],
                ),
                apply(sym(SUB), vec![sym("y"), sym("z")]),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(MUL),
            vec![
                apply(sym(ADD), vec![sym("x"), int(1)]),
                apply(
                    sym(ADD),
                    vec![sym("y"), apply(sym(MUL), vec![int(-1), sym("z")]),],
                ),
            ],
        )
    );
}

#[test]
fn symbolic_factor_extracts_multivariate_perfect_cube_sum() {
    // Factor(x^3 + 3*x^2*y + 3*x*y^2 + y^3) → (x+y)^3
    //
    // The binomial cube expansion (a+b)^3 = a^3 + 3a^2b + 3ab^2 + b^3 is a
    // 4-term pattern handled by factor_multivariate_perfect_cube, distinct
    // from the 2-term cubic identity (a^3 ± b^3).
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(
                    sym(ADD),
                    vec![
                        apply(
                            sym(ADD),
                            vec![
                                apply(sym(POW), vec![sym("x"), int(3)]),
                                apply(
                                    sym(MUL),
                                    vec![
                                        int(3),
                                        apply(
                                            sym(MUL),
                                            vec![apply(sym(POW), vec![sym("x"), int(2)]), sym("y")],
                                        ),
                                    ],
                                ),
                            ],
                        ),
                        apply(
                            sym(MUL),
                            vec![
                                int(3),
                                apply(
                                    sym(MUL),
                                    vec![sym("x"), apply(sym(POW), vec![sym("y"), int(2)])],
                                ),
                            ],
                        ),
                    ],
                ),
                apply(sym(POW), vec![sym("y"), int(3)]),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(POW),
            vec![apply(sym(ADD), vec![sym("x"), sym("y")]), int(3)]
        )
    );
}

#[test]
fn symbolic_factor_extracts_multivariate_perfect_cube_difference() {
    // Factor(x^3 - 3*x^2*y + 3*x*y^2 - y^3) → (x-y)^3
    //
    // The difference expansion (a-b)^3 = a^3 - 3a^2b + 3ab^2 - b^3 has
    // a negative cross term (-3a^2b) and a negative cubic (-b^3), which
    // distinguishes it from the sum case.
    let expr = apply(
        sym("Factor"),
        vec![apply(
            sym(ADD),
            vec![
                apply(
                    sym(SUB),
                    vec![
                        apply(
                            sym(ADD),
                            vec![
                                apply(sym(POW), vec![sym("x"), int(3)]),
                                apply(
                                    sym(MUL),
                                    vec![
                                        int(3),
                                        apply(
                                            sym(MUL),
                                            vec![sym("x"), apply(sym(POW), vec![sym("y"), int(2)])],
                                        ),
                                    ],
                                ),
                            ],
                        ),
                        apply(
                            sym(MUL),
                            vec![
                                int(3),
                                apply(
                                    sym(MUL),
                                    vec![apply(sym(POW), vec![sym("x"), int(2)]), sym("y")],
                                ),
                            ],
                        ),
                    ],
                ),
                apply(
                    sym(MUL),
                    vec![int(-1), apply(sym(POW), vec![sym("y"), int(3)])],
                ),
            ],
        )],
    );

    assert_eq!(
        symbolic().eval(expr),
        apply(
            sym(POW),
            vec![apply(sym(SUB), vec![sym("x"), sym("y")]), int(3)]
        )
    );
}

// ---------------------------------------------------------------------------
// Symbol resolution
// ---------------------------------------------------------------------------

#[test]
fn symbolic_unbound_symbol_returns_self() {
    assert_eq!(symbolic().eval(sym("x")), sym("x"));
}

#[test]
fn symbolic_bound_symbol_resolves() {
    let mut vm = symbolic();
    vm.backend.bind("x", int(42));
    assert_eq!(vm.eval(sym("x")), int(42));
}

#[test]
fn symbolic_transitive_binding() {
    // a := b; b := 5  →  eval(a) == 5
    let mut vm = symbolic();
    vm.backend.bind("b", int(5));
    vm.backend.bind("a", sym("b"));
    assert_eq!(vm.eval(sym("a")), int(5));
}

#[test]
fn symbolic_self_loop_guard() {
    // x := x → x (no infinite recursion)
    let mut vm = symbolic();
    vm.backend.bind("x", sym("x"));
    assert_eq!(vm.eval(sym("x")), sym("x"));
}

// ---------------------------------------------------------------------------
// Elementary functions
// ---------------------------------------------------------------------------

#[test]
fn sin_zero() {
    assert_eq!(symbolic().eval(apply(sym(SIN), vec![int(0)])), int(0));
}

#[test]
fn cos_zero() {
    assert_eq!(symbolic().eval(apply(sym(COS), vec![int(0)])), int(1));
}

#[test]
fn exp_zero() {
    assert_eq!(symbolic().eval(apply(sym(EXP), vec![int(0)])), int(1));
}

#[test]
fn log_one() {
    assert_eq!(symbolic().eval(apply(sym(LOG), vec![int(1)])), int(0));
}

#[test]
fn sqrt_zero() {
    assert_eq!(symbolic().eval(apply(sym(SQRT), vec![int(0)])), int(0));
}

#[test]
fn sqrt_one() {
    assert_eq!(symbolic().eval(apply(sym(SQRT), vec![int(1)])), int(1));
}

#[test]
fn sin_symbolic_stays() {
    // Sin(x) stays when x is unbound (symbolic mode)
    let expr = apply(sym(SIN), vec![sym("x")]);
    assert_eq!(symbolic().eval(expr.clone()), expr);
}

#[test]
fn reciprocal_hyperbolic_numeric_identities() {
    let x = 1.25_f64;
    assert_float_close(
        symbolic().eval(apply(sym(COTH), vec![flt(x)])),
        x.cosh() / x.sinh(),
    );
    assert_float_close(
        symbolic().eval(apply(sym(SECH), vec![flt(x)])),
        1.0 / x.cosh(),
    );
    assert_float_close(
        symbolic().eval(apply(sym(CSCH), vec![flt(x)])),
        1.0 / x.sinh(),
    );
    assert_eq!(symbolic().eval(apply(sym(SECH), vec![int(0)])), int(1));
}

#[test]
fn reciprocal_hyperbolic_symbolic_stays_held() {
    let coth = apply(sym(COTH), vec![sym("x")]);
    let sech = apply(sym(SECH), vec![sym("x")]);
    let csch = apply(sym(CSCH), vec![sym("x")]);
    assert_eq!(symbolic().eval(coth.clone()), coth);
    assert_eq!(symbolic().eval(sech.clone()), sech);
    assert_eq!(symbolic().eval(csch.clone()), csch);
}

#[test]
#[should_panic(expected = "Coth undefined at zero")]
fn coth_zero_panics() {
    symbolic().eval(apply(sym(COTH), vec![int(0)]));
}

#[test]
#[should_panic(expected = "Csch undefined at zero")]
fn csch_zero_panics() {
    symbolic().eval(apply(sym(CSCH), vec![int(0)]));
}

// ---------------------------------------------------------------------------
// Symbolic derivative handler
// ---------------------------------------------------------------------------

#[test]
fn derivative_constants_and_variables() {
    assert_eq!(d(int(42)), int(0));
    assert_eq!(d(rat(1, 3)), int(0));
    assert_eq!(d(sym("y")), int(0));
    assert_eq!(d(sym("x")), int(1));
}

#[test]
fn derivative_arithmetic_rules() {
    assert_eq!(d(apply(sym(ADD), vec![sym("x"), int(3)])), int(1));
    assert_eq!(d(apply(sym(SUB), vec![sym("x"), int(3)])), int(1));
    assert_eq!(d(apply(sym(NEG), vec![sym("x")])), int(-1));

    assert_eq!(
        d(apply(sym(MUL), vec![sym("x"), sym("x")])),
        apply(sym(ADD), vec![sym("x"), sym("x")])
    );

    let denominator = apply(sym(ADD), vec![sym("x"), int(1)]);
    assert_eq!(
        d(apply(sym(DIV), vec![sym("x"), denominator.clone()])),
        apply(
            sym(DIV),
            vec![
                apply(sym(SUB), vec![denominator.clone(), sym("x")]),
                apply(sym(POW), vec![denominator, int(2)]),
            ],
        )
    );
}

#[test]
fn derivative_power_rules() {
    assert_eq!(
        d(apply(sym(POW), vec![sym("x"), int(3)])),
        apply(
            sym(MUL),
            vec![int(3), apply(sym(POW), vec![sym("x"), int(2)])]
        )
    );

    assert_eq!(
        d(apply(sym(POW), vec![sym("a"), sym("x")])),
        apply(
            sym(MUL),
            vec![
                apply(sym(POW), vec![sym("a"), sym("x")]),
                apply(sym(LOG), vec![sym("a")]),
            ],
        )
    );

    // Phase 30: exp(x·log(x)) now simplifies to x^x, so D(x^x, x) = x^x·(log(x)+1).
    assert_eq!(
        d(apply(sym(POW), vec![sym("x"), sym("x")])),
        apply(
            sym(MUL),
            vec![
                apply(sym(POW), vec![sym("x"), sym("x")]),
                apply(
                    sym(ADD),
                    vec![
                        apply(sym(LOG), vec![sym("x")]),
                        apply(
                            sym(MUL),
                            vec![sym("x"), apply(sym(DIV), vec![int(1), sym("x")])],
                        ),
                    ],
                ),
            ],
        )
    );
}

#[test]
fn derivative_elementary_chain_rules() {
    assert_eq!(
        d(apply(sym(SIN), vec![sym("x")])),
        apply(sym(COS), vec![sym("x")])
    );
    assert_eq!(
        d(apply(sym(COS), vec![sym("x")])),
        apply(sym(NEG), vec![apply(sym(SIN), vec![sym("x")])])
    );
    assert_eq!(
        d(apply(sym(TAN), vec![sym("x")])),
        apply(
            sym(DIV),
            vec![
                int(1),
                apply(sym(POW), vec![apply(sym(COS), vec![sym("x")]), int(2)]),
            ],
        )
    );
    assert_eq!(
        d(apply(sym(EXP), vec![sym("x")])),
        apply(sym(EXP), vec![sym("x")])
    );
    assert_eq!(
        d(apply(sym(LOG), vec![sym("x")])),
        apply(sym(DIV), vec![int(1), sym("x")])
    );
    assert_eq!(
        d(apply(sym(SQRT), vec![sym("x")])),
        apply(
            sym(DIV),
            vec![
                int(1),
                apply(sym(MUL), vec![int(2), apply(sym(SQRT), vec![sym("x")])]),
            ],
        )
    );
    assert_eq!(
        d(apply(sym(ASIN), vec![sym("x")])),
        apply(
            sym(DIV),
            vec![
                int(1),
                apply(
                    sym(SQRT),
                    vec![apply(
                        sym(SUB),
                        vec![int(1), apply(sym(POW), vec![sym("x"), int(2)])],
                    )],
                ),
            ],
        )
    );
    assert_eq!(
        d(apply(sym(ACOS), vec![sym("x")])),
        apply(
            sym(NEG),
            vec![apply(
                sym(DIV),
                vec![
                    int(1),
                    apply(
                        sym(SQRT),
                        vec![apply(
                            sym(SUB),
                            vec![int(1), apply(sym(POW), vec![sym("x"), int(2)])],
                        )],
                    ),
                ],
            )],
        )
    );

    assert_eq!(
        d(apply(
            sym(SIN),
            vec![apply(sym(MUL), vec![int(2), sym("x")])]
        )),
        apply(
            sym(MUL),
            vec![
                apply(sym(COS), vec![apply(sym(MUL), vec![int(2), sym("x")])]),
                int(2),
            ],
        )
    );
}

#[test]
fn derivative_hyperbolic_chain_rules() {
    assert_eq!(
        d(apply(sym(SINH), vec![sym("x")])),
        apply(sym(COSH), vec![sym("x")])
    );
    assert_eq!(
        d(apply(sym(COSH), vec![sym("x")])),
        apply(sym(SINH), vec![sym("x")])
    );
    assert_eq!(
        d(apply(sym(TANH), vec![sym("x")])),
        apply(
            sym(DIV),
            vec![
                int(1),
                apply(sym(POW), vec![apply(sym(COSH), vec![sym("x")]), int(2)]),
            ],
        )
    );
    assert_eq!(
        d(apply(sym(ASINH), vec![sym("x")])),
        apply(
            sym(DIV),
            vec![
                int(1),
                apply(
                    sym(SQRT),
                    vec![apply(
                        sym(ADD),
                        vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)],
                    )],
                ),
            ],
        )
    );
    assert_eq!(
        d(apply(sym(ACOSH), vec![sym("x")])),
        apply(
            sym(DIV),
            vec![
                int(1),
                apply(
                    sym(SQRT),
                    vec![apply(
                        sym(SUB),
                        vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)],
                    )],
                ),
            ],
        )
    );
    assert_eq!(
        d(apply(sym(ATANH), vec![sym("x")])),
        apply(
            sym(DIV),
            vec![
                int(1),
                apply(
                    sym(SUB),
                    vec![int(1), apply(sym(POW), vec![sym("x"), int(2)])]
                ),
            ],
        )
    );
}

#[test]
fn derivative_reciprocal_hyperbolic_chain_rules() {
    assert_eq!(
        d(apply(sym(COTH), vec![sym("x")])),
        apply(
            sym(NEG),
            vec![apply(
                sym(DIV),
                vec![
                    int(1),
                    apply(sym(POW), vec![apply(sym(SINH), vec![sym("x")]), int(2)]),
                ],
            )],
        )
    );
    assert_eq!(
        d(apply(sym(SECH), vec![sym("x")])),
        apply(
            sym(NEG),
            vec![apply(
                sym(DIV),
                vec![
                    apply(sym(SINH), vec![sym("x")]),
                    apply(sym(POW), vec![apply(sym(COSH), vec![sym("x")]), int(2)]),
                ],
            )],
        )
    );
    assert_eq!(
        d(apply(sym(CSCH), vec![sym("x")])),
        apply(
            sym(NEG),
            vec![apply(
                sym(DIV),
                vec![
                    apply(sym(COSH), vec![sym("x")]),
                    apply(sym(POW), vec![apply(sym(SINH), vec![sym("x")]), int(2)]),
                ],
            )],
        )
    );

    let inner = apply(
        sym(ADD),
        vec![apply(sym(MUL), vec![int(2), sym("x")]), int(1)],
    );
    assert_eq!(
        d(apply(sym(COTH), vec![inner.clone()])),
        apply(
            sym(NEG),
            vec![apply(
                sym(DIV),
                vec![
                    int(2),
                    apply(sym(POW), vec![apply(sym(SINH), vec![inner]), int(2)]),
                ],
            )],
        )
    );

    let triple_x = apply(sym(MUL), vec![int(3), sym("x")]);
    assert_eq!(
        d(apply(sym(SECH), vec![triple_x.clone()])),
        apply(
            sym(NEG),
            vec![apply(
                sym(DIV),
                vec![
                    apply(
                        sym(MUL),
                        vec![int(3), apply(sym(SINH), vec![triple_x.clone()])]
                    ),
                    apply(sym(POW), vec![apply(sym(COSH), vec![triple_x]), int(2)]),
                ],
            )],
        )
    );

    let half_x = apply(sym(DIV), vec![sym("x"), int(2)]);
    assert_eq!(
        d(apply(sym(CSCH), vec![half_x.clone()])),
        apply(
            sym(NEG),
            vec![apply(
                sym(DIV),
                vec![
                    apply(
                        sym(MUL),
                        vec![rat(1, 2), apply(sym(COSH), vec![half_x.clone()])],
                    ),
                    apply(sym(POW), vec![apply(sym(SINH), vec![half_x]), int(2)]),
                ],
            )],
        )
    );
}

#[test]
fn derivative_unknown_head_stays_unevaluated() {
    let f = apply(sym("F"), vec![sym("x")]);
    let expr = apply(sym(D), vec![f, sym("x")]);
    assert_eq!(symbolic().eval(expr.clone()), expr);
}

// ---------------------------------------------------------------------------
// Symbolic integration handler
// ---------------------------------------------------------------------------

#[test]
fn integrate_is_symbolic_backend_only() {
    assert!(symbolic().backend.handler_for(INTEGRATE).is_some());
    assert!(strict().backend.handler_for(INTEGRATE).is_none());
}

#[test]
fn integrate_constants_and_variables() {
    assert_eq!(integrate(int(5)), apply(sym(MUL), vec![int(5), sym("x")]));
    assert_eq!(
        integrate(sym("y")),
        apply(sym(MUL), vec![sym("y"), sym("x")])
    );
    assert_eq!(
        integrate(sym("x")),
        apply(
            sym(MUL),
            vec![rat(1, 2), apply(sym(POW), vec![sym("x"), int(2)])],
        )
    );
    assert_eq!(integrate(int(0)), int(0));
}

#[test]
fn integrate_power_rules() {
    assert_eq!(
        integrate(apply(sym(POW), vec![sym("x"), int(2)])),
        apply(
            sym(MUL),
            vec![rat(1, 3), apply(sym(POW), vec![sym("x"), int(3)])],
        )
    );
    assert_eq!(
        integrate(apply(sym(POW), vec![sym("x"), rat(1, 2)])),
        apply(
            sym(MUL),
            vec![rat(2, 3), apply(sym(POW), vec![sym("x"), rat(3, 2)])],
        )
    );
    assert_eq!(
        integrate(apply(sym(POW), vec![sym("x"), int(-1)])),
        apply(sym(LOG), vec![sym("x")])
    );
    assert_eq!(
        integrate(apply(sym(DIV), vec![int(3), sym("x")])),
        apply(sym(MUL), vec![int(3), apply(sym(LOG), vec![sym("x")])])
    );
}

#[test]
fn integrate_linearity_and_constant_factor() {
    let half_x_squared = apply(
        sym(MUL),
        vec![rat(1, 2), apply(sym(POW), vec![sym("x"), int(2)])],
    );

    assert_eq!(
        integrate(apply(sym(ADD), vec![sym("x"), int(3)])),
        apply(
            sym(ADD),
            vec![
                half_x_squared.clone(),
                apply(sym(MUL), vec![int(3), sym("x")])
            ],
        )
    );
    assert_eq!(
        integrate(apply(sym(SUB), vec![sym("x"), int(1)])),
        apply(sym(SUB), vec![half_x_squared.clone(), sym("x")])
    );
    assert_eq!(
        integrate(apply(sym(NEG), vec![sym("x")])),
        apply(sym(NEG), vec![half_x_squared.clone()])
    );
    assert_eq!(
        integrate(apply(
            sym(MUL),
            vec![sym("y"), apply(sym(POW), vec![sym("x"), int(2)])]
        )),
        apply(
            sym(MUL),
            vec![
                sym("y"),
                apply(
                    sym(MUL),
                    vec![rat(1, 3), apply(sym(POW), vec![sym("x"), int(3)])],
                ),
            ],
        )
    );
}

#[test]
fn integrate_elementary_direct_forms() {
    assert_eq!(
        integrate(apply(sym(SIN), vec![sym("x")])),
        apply(sym(NEG), vec![apply(sym(COS), vec![sym("x")])])
    );
    assert_eq!(
        integrate(apply(sym(COS), vec![sym("x")])),
        apply(sym(SIN), vec![sym("x")])
    );
    assert_eq!(
        integrate(apply(sym(EXP), vec![sym("x")])),
        apply(sym(EXP), vec![sym("x")])
    );
    assert_eq!(
        integrate(apply(sym(LOG), vec![sym("x")])),
        apply(
            sym(SUB),
            vec![
                apply(sym(MUL), vec![sym("x"), apply(sym(LOG), vec![sym("x")])]),
                sym("x"),
            ],
        )
    );
    assert_eq!(
        integrate(apply(sym(SQRT), vec![sym("x")])),
        apply(
            sym(MUL),
            vec![rat(2, 3), apply(sym(POW), vec![sym("x"), rat(3, 2)])],
        )
    );
}

#[test]
fn integrate_constant_base_power() {
    assert_eq!(
        integrate(apply(sym(POW), vec![sym("a"), sym("x")])),
        apply(
            sym(DIV),
            vec![
                apply(sym(POW), vec![sym("a"), sym("x")]),
                apply(sym(LOG), vec![sym("a")]),
            ],
        )
    );
}

#[test]
fn integrate_phase23_erf_erfi_quadratic_exponentials() {
    let xsq = apply(sym(POW), vec![sym("x"), int(2)]);

    let erf_out = integrate(apply(sym(EXP), vec![apply(sym(NEG), vec![xsq.clone()])]));
    assert!(!is_unevaluated_integrate(&erf_out));
    assert!(contains_head(&erf_out, "Erf"));

    let erfi_out = integrate(apply(sym(EXP), vec![xsq.clone()]));
    assert!(!is_unevaluated_integrate(&erfi_out));
    assert!(contains_head(&erfi_out, "Erfi"));

    let scaled = integrate(apply(sym(EXP), vec![apply(sym(MUL), vec![int(-4), xsq])]));
    assert!(!is_unevaluated_integrate(&scaled));
    assert!(contains_head(&scaled, "Erf"));
}

#[test]
fn integrate_elliptic_first_kind_forms() {
    let theta = sym("theta");
    let k = sym("k");
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(SQRT),
                vec![apply(
                    sym(SUB),
                    vec![
                        int(1),
                        apply(
                            sym(MUL),
                            vec![
                                apply(sym(POW), vec![k.clone(), int(2)]),
                                apply(sym(POW), vec![apply(sym(SIN), vec![theta.clone()]), int(2)]),
                            ],
                        ),
                    ],
                )],
            ),
        ],
    );

    assert_eq!(
        symbolic().eval(apply(
            sym(INTEGRATE),
            vec![integrand.clone(), theta.clone()]
        )),
        apply(sym("EllipticF"), vec![theta.clone(), k.clone()])
    );
    assert_eq!(
        symbolic().eval(apply(
            sym(INTEGRATE),
            vec![
                integrand,
                theta,
                int(0),
                apply(sym(DIV), vec![sym("%pi"), int(2)]),
            ],
        )),
        apply(sym("EllipticK"), vec![k])
    );
}

#[test]
fn integrate_unknown_dependent_form_stays_unevaluated() {
    let f = apply(sym("F"), vec![sym("x")]);
    let expr = apply(sym(INTEGRATE), vec![f, sym("x")]);
    assert_eq!(symbolic().eval(expr.clone()), expr);
}

// ---------------------------------------------------------------------------
// Phase 26 — log-power integration via IBP reduction
// ---------------------------------------------------------------------------

/// Numerically evaluate a simple IR tree by substituting ``x = xval``.
fn eval_at(node: &symbolic_ir::IRNode, xval: f64) -> f64 {
    match node {
        symbolic_ir::IRNode::Integer(n) => *n as f64,
        symbolic_ir::IRNode::Rational(n, d) => *n as f64 / *d as f64,
        symbolic_ir::IRNode::Float(f) => *f,
        symbolic_ir::IRNode::Symbol(s) if s == "x" => xval,
        symbolic_ir::IRNode::Apply(a) => {
            let h = match &a.head {
                symbolic_ir::IRNode::Symbol(s) => s.as_str(),
                _ => panic!("eval_at: non-symbol head"),
            };
            let args = &a.args;
            match (h, args.as_slice()) {
                ("Add", [l, r]) => eval_at(l, xval) + eval_at(r, xval),
                ("Sub", [l, r]) => eval_at(l, xval) - eval_at(r, xval),
                ("Mul", [l, r]) => eval_at(l, xval) * eval_at(r, xval),
                ("Div", [l, r]) => eval_at(l, xval) / eval_at(r, xval),
                ("Neg", [v]) => -eval_at(v, xval),
                ("Pow", [b, e]) => eval_at(b, xval).powf(eval_at(e, xval)),
                ("Log", [v]) => eval_at(v, xval).ln(),
                ("Sqrt", [v]) => eval_at(v, xval).sqrt(),
                ("Sin", [v]) => eval_at(v, xval).sin(),
                ("Cos", [v]) => eval_at(v, xval).cos(),
                ("Asin", [v]) => eval_at(v, xval).asin(),
                ("Acos", [v]) => eval_at(v, xval).acos(),
                ("Atan", [v]) => eval_at(v, xval).atan(),
                ("Sinh", [v]) => eval_at(v, xval).sinh(),
                ("Cosh", [v]) => eval_at(v, xval).cosh(),
                ("Tanh", [v]) => eval_at(v, xval).tanh(),
                ("Coth", [v]) => {
                    let u = eval_at(v, xval);
                    u.cosh() / u.sinh()
                }
                ("Sech", [v]) => 1.0 / eval_at(v, xval).cosh(),
                ("Csch", [v]) => 1.0 / eval_at(v, xval).sinh(),
                _ => panic!("eval_at: unsupported head {h}"),
            }
        }
        _ => panic!("eval_at: unsupported node"),
    }
}

fn trapezoid(f: impl Fn(f64) -> f64, a: f64, b: f64, n: usize) -> f64 {
    let h = (b - a) / n as f64;
    let mut total = 0.5 * (f(a) + f(b));
    for i in 1..n {
        total += f(a + i as f64 * h);
    }
    total * h
}

fn contains_head(node: &symbolic_ir::IRNode, name: &str) -> bool {
    match node {
        symbolic_ir::IRNode::Apply(a) => {
            if let symbolic_ir::IRNode::Symbol(h) = &a.head {
                if h == name {
                    return true;
                }
            }
            a.args.iter().any(|arg| contains_head(arg, name))
        }
        _ => false,
    }
}

#[test]
fn phase26_log_power_2_is_closed() {
    // ∫ log(x)^2 dx must return a closed form containing Log.
    let result = integrate(apply(sym(POW), vec![apply(sym(LOG), vec![sym("x")]), int(2)]));
    let original = apply(
        sym(INTEGRATE),
        vec![
            apply(sym(POW), vec![apply(sym(LOG), vec![sym("x")]), int(2)]),
            sym("x"),
        ],
    );
    assert_ne!(result, original, "expected closed form, got unevaluated Integrate");
    assert!(contains_head(&result, "Log"), "expected Log in result");
}

#[test]
fn phase26_x_log2_x_numeric() {
    // ∫₁^2 x · log(x)^2 dx  vs trapezoidal ground truth.
    let integrand = apply(
        sym(MUL),
        vec![
            sym("x"),
            apply(sym(POW), vec![apply(sym(LOG), vec![sym("x")]), int(2)]),
        ],
    );
    let antideriv = integrate(integrand);
    let diff = eval_at(&antideriv, 2.0) - eval_at(&antideriv, 1.0);
    let numerical = trapezoid(|t| t * t.ln().powi(2), 1.0, 2.0, 10_000);
    assert!((diff - numerical).abs() < 1e-5, "diff={diff:.8}, numerical={numerical:.8}");
}

// ---------------------------------------------------------------------------
// Phase 27 — trig-of-log integration via u = log(x) substitution
// ---------------------------------------------------------------------------

#[test]
fn phase27_sin_log_x_is_closed() {
    // ∫ sin(log(x)) dx must return a closed form with SIN and COS.
    let integrand = apply(sym(SIN), vec![apply(sym(LOG), vec![sym("x")])]);
    let result = integrate(integrand.clone());
    let original = apply(sym(INTEGRATE), vec![integrand, sym("x")]);
    assert_ne!(result, original, "expected closed form");
    assert!(contains_head(&result, "Sin"), "expected Sin in result");
    assert!(contains_head(&result, "Cos"), "expected Cos in result");
}

#[test]
fn phase27_sin_log_x_numeric() {
    // ∫₁^3 sin(log(x)) dx  vs trapezoidal.
    let integrand = apply(sym(SIN), vec![apply(sym(LOG), vec![sym("x")])]);
    let antideriv = integrate(integrand);
    let diff = eval_at(&antideriv, 3.0) - eval_at(&antideriv, 1.0);
    let numerical = trapezoid(|t| t.ln().sin(), 1.0, 3.0, 10_000);
    assert!((diff - numerical).abs() < 1e-5, "diff={diff:.8}, numerical={numerical:.8}");
}

#[test]
fn phase27_cos_log_x_numeric() {
    // ∫₁^3 cos(log(x)) dx  vs trapezoidal.
    let integrand = apply(sym(COS), vec![apply(sym(LOG), vec![sym("x")])]);
    let antideriv = integrate(integrand);
    let diff = eval_at(&antideriv, 3.0) - eval_at(&antideriv, 1.0);
    let numerical = trapezoid(|t| t.ln().cos(), 1.0, 3.0, 10_000);
    assert!((diff - numerical).abs() < 1e-5, "diff={diff:.8}, numerical={numerical:.8}");
}

#[test]
fn phase27_x_sin_log_x_numeric() {
    // ∫₁^2 x · sin(log(x)) dx  vs trapezoidal.
    let integrand = apply(
        sym(MUL),
        vec![sym("x"), apply(sym(SIN), vec![apply(sym(LOG), vec![sym("x")])])],
    );
    let antideriv = integrate(integrand);
    let diff = eval_at(&antideriv, 2.0) - eval_at(&antideriv, 1.0);
    let numerical = trapezoid(|t| t * t.ln().sin(), 1.0, 2.0, 10_000);
    assert!((diff - numerical).abs() < 1e-5, "diff={diff:.8}, numerical={numerical:.8}");
}

#[test]
fn phase27_regression_sin_x_unchanged() {
    // ∫ sin(x) dx = -cos(x) — must not be broken by Phase 27.
    assert_eq!(
        integrate(apply(sym(SIN), vec![sym("x")])),
        apply(sym(NEG), vec![apply(sym(COS), vec![sym("x")])])
    );
}

#[test]
fn phase27_regression_cos_x_unchanged() {
    // ∫ cos(x) dx = sin(x).
    assert_eq!(
        integrate(apply(sym(COS), vec![sym("x")])),
        apply(sym(SIN), vec![sym("x")])
    );
}

// ---------------------------------------------------------------------------
// Phase 28 — General IBP: ∫ P(x)·log(Q(x)) dx and ∫ P(x)·atan(Q(x)) dx
// ---------------------------------------------------------------------------
//
// Helper: build log(x^2 + 1) as an IR node.
fn log_x2p1() -> symbolic_ir::IRNode {
    apply(
        sym(LOG),
        vec![apply(
            sym(ADD),
            vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)],
        )],
    )
}

// Helper: build atan(x^2) as an IR node.
fn atan_x2() -> symbolic_ir::IRNode {
    apply(sym(ATAN), vec![apply(sym(POW), vec![sym("x"), int(2)])])
}

#[test]
fn phase28_log_x2p1_is_closed() {
    // ∫ log(x²+1) dx must produce a closed form containing Log and Atan.
    let result = integrate(log_x2p1());
    let original = apply(sym(INTEGRATE), vec![log_x2p1(), sym("x")]);
    assert_ne!(result, original, "expected closed form, got unevaluated Integrate");
    assert!(contains_head(&result, "Log"), "expected Log in result: {result:?}");
    assert!(contains_head(&result, "Atan"), "expected Atan in result: {result:?}");
}

#[test]
fn phase28_log_x2p1_numeric() {
    // ∫₀¹ log(x²+1) dx  ≈ 0.26260649...  (trapezoidal ground truth)
    let antideriv = integrate(log_x2p1());
    let diff = eval_at(&antideriv, 1.0) - eval_at(&antideriv, 0.0);
    let numerical = trapezoid(|t| (t * t + 1.0_f64).ln(), 0.0, 1.0, 10_000);
    assert!((diff - numerical).abs() < 1e-5, "diff={diff:.8}, numerical={numerical:.8}");
}

#[test]
fn phase28_x_log_x2p1_is_closed() {
    // ∫ x · log(x²+1) dx must produce a closed form containing Log.
    let integrand = apply(sym(MUL), vec![sym("x"), log_x2p1()]);
    let result = integrate(integrand.clone());
    let original = apply(sym(INTEGRATE), vec![integrand, sym("x")]);
    assert_ne!(result, original, "expected closed form, got unevaluated Integrate");
    assert!(contains_head(&result, "Log"), "expected Log in result: {result:?}");
}

#[test]
fn phase28_x_log_x2p1_numeric() {
    // ∫₀² x · log(x²+1) dx  vs trapezoidal.
    let integrand = apply(sym(MUL), vec![sym("x"), log_x2p1()]);
    let antideriv = integrate(integrand);
    let diff = eval_at(&antideriv, 2.0) - eval_at(&antideriv, 0.0);
    let numerical = trapezoid(|t| t * (t * t + 1.0_f64).ln(), 0.0, 2.0, 10_000);
    assert!((diff - numerical).abs() < 1e-5, "diff={diff:.8}, numerical={numerical:.8}");
}

#[test]
fn phase28_x2_log_x2p1_numeric() {
    // ∫₀² x² · log(x²+1) dx  vs trapezoidal.
    let x2 = apply(sym(POW), vec![sym("x"), int(2)]);
    let integrand = apply(sym(MUL), vec![x2, log_x2p1()]);
    let antideriv = integrate(integrand);
    let diff = eval_at(&antideriv, 2.0) - eval_at(&antideriv, 0.0);
    let numerical = trapezoid(|t| t * t * (t * t + 1.0_f64).ln(), 0.0, 2.0, 10_000);
    assert!((diff - numerical).abs() < 1e-5, "diff={diff:.8}, numerical={numerical:.8}");
}

#[test]
fn phase28_atan_x2_fallthrough() {
    // ∫ atan(x²) dx — residual 2x²/(1+x⁴) requires irrational partial fractions;
    // Phase 28 must leave it unevaluated.
    let result = integrate(atan_x2());
    let original = apply(sym(INTEGRATE), vec![atan_x2(), sym("x")]);
    assert_eq!(result, original, "expected unevaluated Integrate, got: {result:?}");
}

#[test]
fn phase28_x_atan_x2_is_closed() {
    // ∫ x · atan(x²) dx must produce a closed form containing Atan and Log.
    let integrand = apply(sym(MUL), vec![sym("x"), atan_x2()]);
    let result = integrate(integrand.clone());
    let original = apply(sym(INTEGRATE), vec![integrand, sym("x")]);
    assert_ne!(result, original, "expected closed form, got unevaluated Integrate");
    assert!(contains_head(&result, "Atan"), "expected Atan in result: {result:?}");
    assert!(contains_head(&result, "Log"), "expected Log in result: {result:?}");
}

#[test]
fn phase28_x_atan_x2_numeric() {
    // ∫₀² x · atan(x²) dx  vs trapezoidal.
    let integrand = apply(sym(MUL), vec![sym("x"), atan_x2()]);
    let antideriv = integrate(integrand);
    let diff = eval_at(&antideriv, 2.0) - eval_at(&antideriv, 0.0);
    let numerical = trapezoid(|t| t * (t * t).atan(), 0.0, 2.0, 10_000);
    assert!((diff - numerical).abs() < 1e-5, "diff={diff:.8}, numerical={numerical:.8}");
}

#[test]
fn phase28_regression_log_x_still_phase3() {
    // ∫ log(x) dx = x·log(x) − x — Phase 3 must still handle this.
    let result = integrate(apply(sym(LOG), vec![sym("x")]));
    assert_eq!(
        result,
        apply(
            sym(SUB),
            vec![
                apply(sym(MUL), vec![sym("x"), apply(sym(LOG), vec![sym("x")])]),
                sym("x"),
            ]
        )
    );
}

#[test]
fn phase11_atan_x_is_closed() {
    // ∫ atan(x) dx = x*atan(x) - 1/2*log(1+x^2).
    let result = integrate(apply(sym(ATAN), vec![sym("x")]));
    assert!(
        !is_unevaluated_integrate(&result),
        "Phase 11 should close ∫ atan(x) dx; got {result:?}"
    );
    assert!(contains_head(&result, ATAN), "Expected Atan in {result:?}");
    assert!(contains_head(&result, LOG), "Expected Log in {result:?}");
}

#[test]
fn phase11_shifted_atan_derivative_matches() {
    // The shifted case exercises the shifted-quadratic residual closure:
    // D = 1 + (x+1)^2.
    let integrand = apply(sym(ATAN), vec![apply(sym(ADD), vec![sym("x"), int(1)])]);
    let phi = integrate(integrand);
    assert!(
        !is_unevaluated_integrate(&phi),
        "Phase 11 should close ∫ atan(x+1) dx; got {phi:?}"
    );
    for &x_val in &[-0.5_f64, 0.0, 0.5, 1.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = (x_val + 1.0).atan();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase11_x_atan_x_derivative_matches() {
    let integrand = apply(sym(MUL), vec![sym("x"), apply(sym(ATAN), vec![sym("x")])]);
    let phi = integrate(integrand);
    assert!(
        !is_unevaluated_integrate(&phi),
        "Phase 11 should close ∫ x*atan(x) dx; got {phi:?}"
    );
    for &x_val in &[-1.0_f64, -0.25, 0.25, 1.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = x_val * x_val.atan();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase12_asin_x_derivative_matches() {
    let phi = integrate(apply(sym(ASIN), vec![sym("x")]));
    assert!(!is_unevaluated_integrate(&phi), "Phase 12 should close ∫ asin(x) dx; got {phi:?}");
    for &x_val in &[0.0_f64, 0.25, 0.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = x_val.asin();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase12_x_asin_x_derivative_matches() {
    let integrand = apply(sym(MUL), vec![sym("x"), apply(sym(ASIN), vec![sym("x")])]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi), "Phase 12 should close ∫ x*asin(x) dx; got {phi:?}");
    for &x_val in &[0.0_f64, 0.25, 0.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = x_val * x_val.asin();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase12_x_acos_x_derivative_matches_and_keeps_asin_residual() {
    let integrand = apply(sym(MUL), vec![sym("x"), apply(sym(ACOS), vec![sym("x")])]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi), "Phase 12 should close ∫ x*acos(x) dx; got {phi:?}");
    assert!(contains_head(&phi, ASIN), "expected Asin residual in {phi:?}");
    for &x_val in &[0.0_f64, 0.25, 0.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = x_val * x_val.acos();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase12_shifted_linear_asin_derivative_matches() {
    let arg = apply(sym(ADD), vec![apply(sym(MUL), vec![int(2), sym("x")]), int(1)]);
    let phi = integrate(apply(sym(ASIN), vec![arg]));
    assert!(!is_unevaluated_integrate(&phi), "Phase 12 should close ∫ asin(2x+1) dx; got {phi:?}");
    for &x_val in &[-0.4_f64, -0.25, -0.1] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = (2.0 * x_val + 1.0).asin();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase12_nonlinear_asin_falls_through() {
    let integrand = apply(sym(ASIN), vec![apply(sym(POW), vec![sym("x"), int(2)])]);
    let result = integrate(integrand.clone());
    let original = apply(sym(INTEGRATE), vec![integrand, sym("x")]);
    assert_eq!(result, original);
}

#[test]
fn phase13_x_sinh_x_derivative_matches() {
    let integrand = apply(sym(MUL), vec![sym("x"), apply(sym(SINH), vec![sym("x")])]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi), "Phase 13 should close x*sinh(x); got {phi:?}");
    assert!(contains_head(&phi, COSH), "expected Cosh term in {phi:?}");
    for &x_val in &[-0.4_f64, 0.2, 0.8] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = x_val * x_val.sinh();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase13_x_cosh_x_derivative_matches() {
    let integrand = apply(sym(MUL), vec![sym("x"), apply(sym(COSH), vec![sym("x")])]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi), "Phase 13 should close x*cosh(x); got {phi:?}");
    assert!(contains_head(&phi, SINH), "expected Sinh term in {phi:?}");
    for &x_val in &[-0.4_f64, 0.2, 0.8] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = x_val * x_val.cosh();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase13_asinh_x_derivative_matches() {
    let phi = integrate(apply(sym(ASINH), vec![sym("x")]));
    assert!(!is_unevaluated_integrate(&phi), "Phase 13 should close asinh(x); got {phi:?}");
    assert!(contains_head(&phi, SQRT), "expected Sqrt term in {phi:?}");
    for &x_val in &[-0.4_f64, 0.2, 0.8] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = x_val.asinh();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase13_x_acosh_x_derivative_matches() {
    let integrand = apply(sym(MUL), vec![sym("x"), apply(sym(ACOSH), vec![sym("x")])]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi), "Phase 13 should close x*acosh(x); got {phi:?}");
    assert!(contains_head(&phi, SQRT), "expected Sqrt term in {phi:?}");
    for &x_val in &[1.3_f64, 1.7, 2.2] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = x_val * x_val.acosh();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase13_tanh_linear_derivative_matches() {
    let arg = apply(sym(ADD), vec![apply(sym(MUL), vec![int(2), sym("x")]), int(1)]);
    let phi = integrate(apply(sym(TANH), vec![arg]));
    assert!(!is_unevaluated_integrate(&phi), "Phase 13 should close tanh(2x+1); got {phi:?}");
    assert!(contains_head(&phi, LOG), "expected Log term in {phi:?}");
    for &x_val in &[-0.4_f64, 0.1, 0.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = (2.0 * x_val + 1.0).tanh();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase13_atanh_linear_derivative_matches() {
    let arg = apply(sym(MUL), vec![rat(1, 2), sym("x")]);
    let phi = integrate(apply(sym(ATANH), vec![arg]));
    assert!(!is_unevaluated_integrate(&phi), "Phase 13 should close atanh(x/2); got {phi:?}");
    assert!(contains_head(&phi, LOG), "expected Log term in {phi:?}");
    for &x_val in &[-0.5_f64, 0.2, 0.7] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = (x_val / 2.0).atanh();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase13_x_tanh_x_falls_through() {
    let integrand = apply(sym(MUL), vec![sym("x"), apply(sym(TANH), vec![sym("x")])]);
    let result = integrate(integrand);
    assert!(is_unevaluated_integrate(&result), "x*tanh(x) should remain deferred; got {result:?}");
}

#[test]
fn phase15_coth_linear_derivative_matches() {
    let arg = apply(sym(ADD), vec![apply(sym(MUL), vec![int(2), sym("x")]), int(1)]);
    let phi = integrate(apply(sym(COTH), vec![arg]));
    assert!(!is_unevaluated_integrate(&phi), "Phase 15 should close coth(2x+1); got {phi:?}");
    assert!(contains_head(&phi, LOG), "expected Log term in {phi:?}");
    assert!(contains_head(&phi, SINH), "expected Sinh term in {phi:?}");
    for &x_val in &[0.2_f64, 0.6, 1.0] {
        let u = 2.0 * x_val + 1.0;
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = u.cosh() / u.sinh();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase15_sech_linear_derivative_matches() {
    let arg = apply(sym(MUL), vec![int(3), sym("x")]);
    let phi = integrate(apply(sym(SECH), vec![arg]));
    assert!(!is_unevaluated_integrate(&phi), "Phase 15 should close sech(3x); got {phi:?}");
    assert!(contains_head(&phi, ATAN), "expected Atan term in {phi:?}");
    assert!(contains_head(&phi, SINH), "expected Sinh term in {phi:?}");
    for &x_val in &[-0.4_f64, 0.2, 0.8] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (3.0 * x_val).cosh();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase15_csch_linear_derivative_matches() {
    let arg = apply(sym(MUL), vec![rat(1, 2), sym("x")]);
    let phi = integrate(apply(sym(CSCH), vec![arg]));
    assert!(!is_unevaluated_integrate(&phi), "Phase 15 should close csch(x/2); got {phi:?}");
    assert!(contains_head(&phi, LOG), "expected Log term in {phi:?}");
    assert!(contains_head(&phi, TANH), "expected Tanh term in {phi:?}");
    for &x_val in &[0.6_f64, 1.2, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (x_val / 2.0).sinh();
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase15_x_coth_x_falls_through() {
    let integrand = apply(sym(MUL), vec![sym("x"), apply(sym(COTH), vec![sym("x")])]);
    let result = integrate(integrand);
    assert!(is_unevaluated_integrate(&result), "x*coth(x) should remain deferred; got {result:?}");
}

#[test]
fn phase16_sech_squared_linear_derivative_matches() {
    let arg = apply(sym(MUL), vec![int(3), sym("x")]);
    let integrand = apply(sym(POW), vec![apply(sym(SECH), vec![arg]), int(2)]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi), "Phase 16 should close sech^2(3x); got {phi:?}");
    assert!(contains_head(&phi, TANH), "expected Tanh term in {phi:?}");
    for &x_val in &[-0.4_f64, 0.2, 0.8] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (3.0 * x_val).cosh().powi(2);
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase16_sech_cubed_derivative_matches() {
    let integrand = apply(sym(POW), vec![apply(sym(SECH), vec![sym("x")]), int(3)]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi), "Phase 16 should close sech^3(x); got {phi:?}");
    assert!(contains_head(&phi, ATAN), "expected Atan term in {phi:?}");
    for &x_val in &[-0.4_f64, 0.2, 0.8] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / x_val.cosh().powi(3);
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase16_csch_squared_linear_derivative_matches() {
    let arg = apply(sym(MUL), vec![rat(1, 2), sym("x")]);
    let integrand = apply(sym(POW), vec![apply(sym(CSCH), vec![arg]), int(2)]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi), "Phase 16 should close csch^2(x/2); got {phi:?}");
    assert!(contains_head(&phi, COTH), "expected Coth term in {phi:?}");
    for &x_val in &[0.6_f64, 1.2, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (x_val / 2.0).sinh().powi(2);
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase16_csch_cubed_derivative_matches() {
    let integrand = apply(sym(POW), vec![apply(sym(CSCH), vec![sym("x")]), int(3)]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi), "Phase 16 should close csch^3(x); got {phi:?}");
    assert!(contains_head(&phi, LOG), "expected Log term in {phi:?}");
    for &x_val in &[0.6_f64, 1.2, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / x_val.sinh().powi(3);
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase16_coth_powers_derivative_match() {
    for power in [2_i64, 3] {
        let integrand = apply(sym(POW), vec![apply(sym(COTH), vec![sym("x")]), int(power)]);
        let phi = integrate(integrand);
        assert!(!is_unevaluated_integrate(&phi), "Phase 16 should close coth^{power}(x); got {phi:?}");
        for &x_val in &[0.6_f64, 1.2, 2.0] {
            let coth = x_val.cosh() / x_val.sinh();
            let got = phase34_numerical_derivative(&phi, x_val);
            let expected = coth.powi(power as i32);
            assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
        }
    }
}

#[test]
fn phase16_nonlinear_sech_power_falls_through() {
    let nonlinear_arg = apply(sym(POW), vec![sym("x"), int(2)]);
    let integrand = apply(sym(POW), vec![apply(sym(SECH), vec![nonlinear_arg]), int(2)]);
    let result = integrate(integrand);
    assert!(is_unevaluated_integrate(&result), "sech^2(x^2) should remain deferred; got {result:?}");
}

#[test]
fn phase17_tanh_powers_derivative_match() {
    for power in [2_i64, 3, 4] {
        let integrand = apply(sym(POW), vec![apply(sym(TANH), vec![sym("x")]), int(power)]);
        let phi = integrate(integrand);
        assert!(!is_unevaluated_integrate(&phi), "Phase 17 should close tanh^{power}(x); got {phi:?}");
        for &x_val in &[-0.4_f64, 0.2, 0.8] {
            let got = phase34_numerical_derivative(&phi, x_val);
            let expected = x_val.tanh().powi(power as i32);
            assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
        }
    }
}

#[test]
fn phase17_tanh_squared_linear_derivative_matches() {
    let arg = apply(sym(ADD), vec![apply(sym(MUL), vec![int(2), sym("x")]), int(1)]);
    let integrand = apply(sym(POW), vec![apply(sym(TANH), vec![arg]), int(2)]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi), "Phase 17 should close tanh^2(2x+1); got {phi:?}");
    assert!(contains_head(&phi, TANH), "expected Tanh term in {phi:?}");
    for &x_val in &[-0.4_f64, 0.1, 0.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = (2.0 * x_val + 1.0).tanh().powi(2);
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase17_nonlinear_tanh_power_falls_through() {
    let nonlinear_arg = apply(sym(POW), vec![sym("x"), int(2)]);
    let integrand = apply(sym(POW), vec![apply(sym(TANH), vec![nonlinear_arg]), int(2)]);
    let result = integrate(integrand);
    assert!(is_unevaluated_integrate(&result), "tanh^2(x^2) should remain deferred; got {result:?}");
}

// ---------------------------------------------------------------------------
// Phase 29: Abs and Sqrt algebraic rules
// ---------------------------------------------------------------------------

#[test]
fn phase29_abs_numeric_fold() {
    // abs(-3) = 3, abs(3) = 3
    assert_eq!(eval(apply(sym("Abs"), vec![int(-3)])), int(3));
    assert_eq!(eval(apply(sym("Abs"), vec![int(5)])), int(5));
}

#[test]
fn phase29_abs_idempotent() {
    // abs(abs(x)) = abs(x)
    let x = sym("x");
    let abs_x = apply(sym("Abs"), vec![x.clone()]);
    assert_eq!(eval(apply(sym("Abs"), vec![abs_x.clone()])), abs_x);
}

#[test]
fn phase29_abs_strips_neg() {
    // abs(-x) = abs(x)
    let x = sym("x");
    assert_eq!(
        eval(apply(sym("Abs"), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym("Abs"), vec![x])
    );
}

#[test]
fn phase29_abs_even_power() {
    // abs(x^2) = x^2,  abs(x^4) = x^4
    let x = sym("x");
    let x2 = apply(sym(POW), vec![x.clone(), int(2)]);
    let x4 = apply(sym(POW), vec![x.clone(), int(4)]);
    assert_eq!(eval(apply(sym("Abs"), vec![x2.clone()])), x2);
    assert_eq!(eval(apply(sym("Abs"), vec![x4.clone()])), x4);
}

#[test]
fn phase29_sqrt_perfect_squares() {
    // sqrt(0)=0, sqrt(1)=1, sqrt(4)=2, sqrt(9)=3, sqrt(16)=4
    let sqrt = |n: i64| eval(apply(sym(SQRT), vec![int(n)]));
    assert_eq!(sqrt(0), int(0));
    assert_eq!(sqrt(1), int(1));
    assert_eq!(sqrt(4), int(2));
    assert_eq!(sqrt(9), int(3));
    assert_eq!(sqrt(16), int(4));
    assert_eq!(sqrt(25), int(5));
}

#[test]
fn phase29_sqrt_x2_is_abs_x() {
    // sqrt(x^2) = |x|  — k=1, odd
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(SQRT), vec![apply(sym(POW), vec![x.clone(), int(2)])])),
        apply(sym("Abs"), vec![x])
    );
}

#[test]
fn phase29_sqrt_x4_is_x2() {
    // sqrt(x^4) = x^2  — k=2, even
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(SQRT), vec![apply(sym(POW), vec![x.clone(), int(4)])])),
        apply(sym(POW), vec![x, int(2)])
    );
}

#[test]
fn phase29_sqrt_x6_is_abs_x3() {
    // sqrt(x^6) = |x^3|  — k=3, odd
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(SQRT), vec![apply(sym(POW), vec![x.clone(), int(6)])])),
        apply(sym("Abs"), vec![apply(sym(POW), vec![x, int(3)])])
    );
}

#[test]
fn phase29_sqrt_x8_is_x4() {
    // sqrt(x^8) = x^4  — k=4, even
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(SQRT), vec![apply(sym(POW), vec![x.clone(), int(8)])])),
        apply(sym(POW), vec![x, int(4)])
    );
}

// ---------------------------------------------------------------------------
// Phase 30: Log and Exp cancellation rules
// ---------------------------------------------------------------------------

#[test]
fn phase30_log_exp_cancels() {
    // log(exp(x)) = x
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(LOG), vec![apply(sym(EXP), vec![x.clone()])])),
        x
    );
}

#[test]
fn phase30_exp_log_cancels() {
    // exp(log(x)) = x
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(EXP), vec![apply(sym(LOG), vec![x.clone()])])),
        x
    );
}

#[test]
fn phase30_exp_n_log_x_is_pow() {
    // exp(2*log(x)) = x^2
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(EXP), vec![apply(
            sym(MUL),
            vec![int(2), apply(sym(LOG), vec![x.clone()])],
        )])),
        apply(sym(POW), vec![x.clone(), int(2)])
    );
}

#[test]
fn phase30_exp_log_x_n_commuted_is_pow() {
    // exp(log(x)*3) = x^3  (commuted Mul)
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(EXP), vec![apply(
            sym(MUL),
            vec![apply(sym(LOG), vec![x.clone()]), int(3)],
        )])),
        apply(sym(POW), vec![x.clone(), int(3)])
    );
}

// ---------------------------------------------------------------------------
// Phase 31: Trig/hyperbolic symmetry and arc-cancellation
// ---------------------------------------------------------------------------

#[test]
fn phase31_sin_odd_symmetry() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(SIN), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(NEG), vec![apply(sym(SIN), vec![x])])
    );
}

#[test]
fn phase31_sin_arc_cancel() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(SIN), vec![apply(sym(ASIN), vec![x.clone()])])),
        x
    );
}

#[test]
fn phase31_cos_even_symmetry() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(COS), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(COS), vec![x])
    );
}

#[test]
fn phase31_cos_arc_cancel() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(COS), vec![apply(sym(ACOS), vec![x.clone()])])),
        x
    );
}

#[test]
fn phase31_tan_odd_symmetry() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(TAN), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(NEG), vec![apply(sym(TAN), vec![x])])
    );
}

#[test]
fn phase31_tan_arc_cancel() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(TAN), vec![apply(sym(ATAN), vec![x.clone()])])),
        x
    );
}

#[test]
fn phase31_sinh_odd_symmetry() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(SINH), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(NEG), vec![apply(sym(SINH), vec![x])])
    );
}

#[test]
fn phase31_sinh_arc_cancel() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(SINH), vec![apply(sym(ASINH), vec![x.clone()])])),
        x
    );
}

#[test]
fn phase31_cosh_even_symmetry() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(COS), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(COS), vec![x])
    );
}

#[test]
fn phase31_cosh_arc_cancel() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(COSH), vec![apply(sym(ACOSH), vec![x.clone()])])),
        x
    );
}

#[test]
fn phase31_tanh_odd_symmetry() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(TANH), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(NEG), vec![apply(sym(TANH), vec![x])])
    );
}

#[test]
fn phase31_tanh_arc_cancel() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(TANH), vec![apply(sym(ATANH), vec![x.clone()])])),
        x
    );
}

// ---------------------------------------------------------------------------
// Phase 32: Inverse trig/hyperbolic odd symmetry + acos reflection
// ---------------------------------------------------------------------------

#[test]
fn phase32_asin_odd() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(ASIN), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(NEG), vec![apply(sym(ASIN), vec![x])])
    );
}

#[test]
fn phase32_acos_reflection() {
    // acos(-x) = %pi - acos(x)
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(ACOS), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(SUB), vec![
            sym("%pi"),
            apply(sym(ACOS), vec![x]),
        ])
    );
}

#[test]
fn phase32_atan_odd() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(ATAN), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(NEG), vec![apply(sym(ATAN), vec![x])])
    );
}

#[test]
fn phase32_asinh_odd() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(ASINH), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(NEG), vec![apply(sym(ASINH), vec![x])])
    );
}

#[test]
fn phase32_atanh_odd() {
    let x = sym("x");
    assert_eq!(
        eval(apply(sym(ATANH), vec![apply(sym(NEG), vec![x.clone()])])),
        apply(sym(NEG), vec![apply(sym(ATANH), vec![x])])
    );
}

// ---------------------------------------------------------------------------
// Phase 33: Trig exact values at rational multiples of π
// ---------------------------------------------------------------------------

fn pi() -> symbolic_ir::IRNode { sym("%pi") }

#[test]
fn phase33_sin_pi_is_zero() {
    assert_eq!(eval(apply(sym(SIN), vec![pi()])), int(0));
}

#[test]
fn phase33_sin_pi_over_6_is_half() {
    assert_eq!(
        eval(apply(sym(SIN), vec![apply(sym(DIV), vec![pi(), int(6)])])),
        symbolic_ir::IRNode::Rational(1, 2)
    );
}

#[test]
fn phase33_sin_pi_over_4_is_sqrt2_over_2() {
    let result = eval(apply(sym(SIN), vec![apply(sym(DIV), vec![pi(), int(4)])]));
    // Div(Sqrt(2), 2)
    assert_eq!(result, apply(sym(DIV), vec![
        apply(sym(SQRT), vec![int(2)]),
        int(2),
    ]));
}

#[test]
fn phase33_sin_pi_over_2_is_one() {
    assert_eq!(
        eval(apply(sym(SIN), vec![apply(sym(DIV), vec![pi(), int(2)])])),
        int(1)
    );
}

#[test]
fn phase33_sin_2pi_is_zero() {
    assert_eq!(
        eval(apply(sym(SIN), vec![apply(sym(MUL), vec![int(2), pi()])])),
        int(0)
    );
}

#[test]
fn phase33_sin_3pi_over_2_is_neg_one() {
    // 3π/2: sin = -1
    let arg = apply(sym(DIV), vec![apply(sym(MUL), vec![int(3), pi()]), int(2)]);
    assert_eq!(eval(apply(sym(SIN), vec![arg])), int(-1));
}

#[test]
fn phase33_cos_pi_is_neg_one() {
    assert_eq!(eval(apply(sym(COS), vec![pi()])), int(-1));
}

#[test]
fn phase33_cos_pi_over_2_is_zero() {
    assert_eq!(
        eval(apply(sym(COS), vec![apply(sym(DIV), vec![pi(), int(2)])])),
        int(0)
    );
}

#[test]
fn phase33_cos_pi_over_3_is_half() {
    assert_eq!(
        eval(apply(sym(COS), vec![apply(sym(DIV), vec![pi(), int(3)])])),
        symbolic_ir::IRNode::Rational(1, 2)
    );
}

#[test]
fn phase33_cos_2pi_is_one() {
    assert_eq!(
        eval(apply(sym(COS), vec![apply(sym(MUL), vec![int(2), pi()])])),
        int(1)
    );
}

#[test]
fn phase33_tan_pi_is_zero() {
    assert_eq!(eval(apply(sym(TAN), vec![pi()])), int(0));
}

#[test]
fn phase33_tan_pi_over_4_is_one() {
    assert_eq!(
        eval(apply(sym(TAN), vec![apply(sym(DIV), vec![pi(), int(4)])])),
        int(1)
    );
}

#[test]
fn phase33_tan_pi_over_3_is_sqrt3() {
    assert_eq!(
        eval(apply(sym(TAN), vec![apply(sym(DIV), vec![pi(), int(3)])])),
        apply(sym(SQRT), vec![int(3)])
    );
}

#[test]
fn phase33_tan_3pi_over_4_is_neg_one() {
    let arg = apply(sym(DIV), vec![apply(sym(MUL), vec![int(3), pi()]), int(4)]);
    assert_eq!(eval(apply(sym(TAN), vec![arg])), int(-1));
}

#[test]
fn phase33_tan_pi_over_2_stays_unevaluated() {
    // tan(π/2) is undefined — must remain unevaluated.
    let arg = apply(sym(DIV), vec![pi(), int(2)]);
    let result = eval(apply(sym(TAN), vec![arg.clone()]));
    // Result should still be Tan(π/2), not a numeric value.
    match &result {
        symbolic_ir::IRNode::Apply(ap) => {
            assert_eq!(ap.head, sym(TAN), "head should be Tan");
        }
        _ => panic!("expected unevaluated Tan, got: {result:?}"),
    }
}

#[test]
fn phase33_sin_neg_pi_over_6_is_neg_half() {
    // sin(-π/6) = -1/2 via odd symmetry through table lookup
    let arg = apply(sym(NEG), vec![apply(sym(DIV), vec![pi(), int(6)])]);
    assert_eq!(eval(apply(sym(SIN), vec![arg])), symbolic_ir::IRNode::Rational(-1, 2));
}

#[test]
fn phase33_cos_neg_pi_over_3_is_half() {
    // cos(-π/3) = 1/2 via even-symmetry modular reduction
    let arg = apply(sym(NEG), vec![apply(sym(DIV), vec![pi(), int(3)])]);
    assert_eq!(
        eval(apply(sym(COS), vec![arg])),
        symbolic_ir::IRNode::Rational(1, 2)
    );
}

#[test]
fn phase33_regression_numeric_sin_cos_tan() {
    // Existing numeric fold must still work after Phase 33.
    assert_eq!(eval(apply(sym(SIN), vec![int(0)])), int(0));
    assert_eq!(eval(apply(sym(COS), vec![int(0)])), int(1));
    assert_eq!(eval(apply(sym(TAN), vec![int(0)])), int(0));
}

// ---------------------------------------------------------------------------
// Logic
// ---------------------------------------------------------------------------

#[test]
fn and_true_true() {
    let expr = apply(sym(AND), vec![sym("True"), sym("True")]);
    assert_eq!(symbolic().eval(expr), sym("True"));
}

#[test]
fn and_short_circuit_false() {
    // And(False, x) → False regardless of x
    let expr = apply(sym(AND), vec![sym("False"), sym("x")]);
    assert_eq!(symbolic().eval(expr), sym("False"));
}

#[test]
fn or_true_short_circuits() {
    let expr = apply(sym(OR), vec![sym("True"), sym("x")]);
    assert_eq!(symbolic().eval(expr), sym("True"));
}

#[test]
fn not_true() {
    assert_eq!(
        symbolic().eval(apply(sym(NOT), vec![sym("True")])),
        sym("False")
    );
}

#[test]
fn not_false() {
    assert_eq!(
        symbolic().eval(apply(sym(NOT), vec![sym("False")])),
        sym("True")
    );
}

// ---------------------------------------------------------------------------
// If — held head
// ---------------------------------------------------------------------------

#[test]
fn if_true_branch() {
    // If(True, 1, 2) → 1
    let expr = apply(sym(IF), vec![sym("True"), int(1), int(2)]);
    assert_eq!(symbolic().eval(expr), int(1));
}

#[test]
fn if_false_branch() {
    // If(False, 1, 2) → 2
    let expr = apply(sym(IF), vec![sym("False"), int(1), int(2)]);
    assert_eq!(symbolic().eval(expr), int(2));
}

#[test]
fn if_false_no_else() {
    // If(False, 1) → False
    let expr = apply(sym(IF), vec![sym("False"), int(1)]);
    assert_eq!(symbolic().eval(expr), sym("False"));
}

// ---------------------------------------------------------------------------
// Assign
// ---------------------------------------------------------------------------

#[test]
fn assign_binds_and_returns_value() {
    let mut vm = symbolic();
    let expr = apply(sym(ASSIGN), vec![sym("x"), int(42)]);
    let result = vm.eval(expr);
    assert_eq!(result, int(42));
    assert_eq!(vm.backend.lookup("x"), Some(int(42)));
}

// ---------------------------------------------------------------------------
// Define / user-defined functions
// ---------------------------------------------------------------------------

#[test]
fn define_and_call_function() {
    // f(x) := x * 2; f(5) → 10
    let mut vm = symbolic();

    // Define(f, List(x), Mul(x, 2))
    let def = apply(
        sym(DEFINE),
        vec![
            sym("f"),
            apply(sym(LIST), vec![sym("x")]),
            apply(sym(MUL), vec![sym("x"), int(2)]),
        ],
    );
    vm.eval(def);

    // Call f(5)
    let call = apply(sym("f"), vec![int(5)]);
    assert_eq!(vm.eval(call), int(10));
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

#[test]
fn list_passthrough() {
    let expr = apply(sym(LIST), vec![int(1), int(2), int(3)]);
    assert_eq!(symbolic().eval(expr.clone()), expr);
}

// ---------------------------------------------------------------------------
// eval_program
// ---------------------------------------------------------------------------

#[test]
fn eval_program_returns_last_value() {
    let mut vm = symbolic();
    let result = vm.eval_program(vec![int(1), int(2), int(3)]);
    assert_eq!(result, Some(int(3)));
}

#[test]
fn eval_program_empty_returns_none() {
    let mut vm = symbolic();
    assert_eq!(vm.eval_program(vec![]), None);
}

#[test]
fn eval_program_bindings_persist() {
    let mut vm = symbolic();
    // x := 5; x + 1
    let assign = apply(sym(ASSIGN), vec![sym("x"), int(5)]);
    let expr = apply(sym(ADD), vec![sym("x"), int(1)]);
    let result = vm.eval_program(vec![assign, expr]);
    assert_eq!(result, Some(int(6)));
}

// ---------------------------------------------------------------------------
// Exact rational arithmetic
// ---------------------------------------------------------------------------

#[test]
fn rational_add_exact() {
    // 1/2 + 1/3 = 5/6
    let expr = apply(sym(ADD), vec![rat(1, 2), rat(1, 3)]);
    assert_eq!(strict().eval(expr), rat(5, 6));
}

#[test]
fn rational_mul_exact() {
    // 2/3 * 3/4 = 1/2
    let expr = apply(sym(MUL), vec![rat(2, 3), rat(3, 4)]);
    assert_eq!(strict().eval(expr), rat(1, 2));
}

#[test]
fn integer_div_exact_rational() {
    // 1 / 4 = 1/4
    assert_eq!(
        strict().eval(apply(sym(DIV), vec![int(1), int(4)])),
        rat(1, 4)
    );
}

// ---------------------------------------------------------------------------
// Strict backend panics on unknowns
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "undefined symbol")]
fn strict_panics_on_unbound_symbol() {
    strict().eval(sym("unbound_x_yz"));
}

#[test]
#[should_panic(expected = "no handler for head")]
fn strict_panics_on_unknown_head() {
    strict().eval(apply(sym("UnknownFunc"), vec![int(1)]));
}

#[test]
#[should_panic(expected = "no handler for head")]
fn strict_panics_on_derivative_head() {
    strict().eval(apply(sym(D), vec![int(1), int(2)]));
}

#[test]
#[should_panic(expected = "undefined symbol")]
fn strict_panics_on_symbolic_add() {
    // In strict mode, evaluating args first means `x` triggers on_unresolved
    // before Add even sees the arguments.
    strict().eval(apply(sym(ADD), vec![sym("x"), int(1)]));
}

// ---------------------------------------------------------------------------
// Phase 34: Weierstrass substitution for ∫ 1/(a + b·sin/cos x) dx
// Mirrors the Python `test_phase34_weierstrass.py` and the TS
// `phase34-weierstrass.test.ts` suites — numerical-derivative verification
// and discriminant fallthrough checks.
// ---------------------------------------------------------------------------

/// Structural substitution: replace every occurrence of `var_name` in `node`
/// with `value`.  Returns a fresh tree.
fn phase34_subst(node: &symbolic_ir::IRNode, var_name: &str, value: &symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    if let symbolic_ir::IRNode::Symbol(name) = node {
        if name == var_name {
            return value.clone();
        }
    }
    if let symbolic_ir::IRNode::Apply(apply) = node {
        let new_args: Vec<symbolic_ir::IRNode> = apply
            .args
            .iter()
            .map(|a| phase34_subst(a, var_name, value))
            .collect();
        return symbolic_ir::IRNode::Apply(Box::new(symbolic_ir::IRApply {
            head: apply.head.clone(),
            args: new_args,
        }));
    }
    node.clone()
}

/// Evaluate `expr` after substituting x ← `x_val` through a fresh symbolic VM
/// — so the full handler suite (including Tan, Sqrt, Atan) folds the tree.
/// Returns NaN when the result is not numeric.
fn phase34_eval_at(expr: &symbolic_ir::IRNode, x_val: f64) -> f64 {
    let substituted = phase34_subst(expr, "x", &flt(x_val));
    let folded = symbolic().eval(substituted);
    match folded {
        symbolic_ir::IRNode::Float(v) => v,
        symbolic_ir::IRNode::Integer(n) => n as f64,
        symbolic_ir::IRNode::Rational(n, d) => n as f64 / d as f64,
        _ => f64::NAN,
    }
}

/// Central-difference derivative of `expr` w.r.t. x at `x_val`.
fn phase34_numerical_derivative(expr: &symbolic_ir::IRNode, x_val: f64) -> f64 {
    let h = 1e-5;
    (phase34_eval_at(expr, x_val + h) - phase34_eval_at(expr, x_val - h)) / (2.0 * h)
}

fn is_unevaluated_integrate(node: &symbolic_ir::IRNode) -> bool {
    matches!(
        node,
        symbolic_ir::IRNode::Apply(apply)
            if apply.head == symbolic_ir::IRNode::Symbol(INTEGRATE.to_string())
    )
}

#[test]
fn phase34_sin_two_plus_sin_closes_with_atan() {
    // ∫ 1/(2 + sin x) dx — must close with an Atan in the body.
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![sym("x")])])],
    );
    let result = integrate(integrand);
    assert!(
        !is_unevaluated_integrate(&result),
        "Phase 34 should close ∫ 1/(2+sin x) dx; got {result:?}"
    );
    assert!(
        contains_head(&result, ATAN),
        "Expected Atan in closed form; got {result:?}"
    );
}

#[test]
fn phase34_sin_two_plus_sin_derivative_matches() {
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![sym("x")])])],
    );
    let phi = integrate(integrand);
    for &x_val in &[-2.5_f64, -1.0, -0.3, 0.0, 0.3, 1.0, 2.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (2.0 + x_val.sin());
        assert!(
            (got - expected).abs() < 1e-4,
            "At x={x_val}: derivative={got}, expected={expected}"
        );
    }
}

#[test]
fn phase34_sin_perfect_square_discriminant() {
    // ∫ 1/(5 + 3·sin x) dx — disc=16 (perfect square) → Sqrt-free result.
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(ADD),
                vec![int(5), apply(sym(MUL), vec![int(3), apply(sym(SIN), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(
        !contains_head(&phi, SQRT),
        "Perfect-square disc should fold; got {phi:?}"
    );
    for &x_val in &[-1.0_f64, -0.2, 0.0, 0.7, 1.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (5.0 + 3.0 * x_val.sin());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase34_sin_with_numerator_coefficient() {
    // ∫ 3/(2 + sin x) dx — coefficient 3 must scale the closed form.
    let integrand = apply(
        sym(DIV),
        vec![int(3), apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![sym("x")])])],
    );
    let phi = integrate(integrand);
    for &x_val in &[-1.0_f64, -0.2, 0.0, 0.7, 1.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 3.0 / (2.0 + x_val.sin());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase34_sin_with_rational_coefficients() {
    // ∫ 1/((3/2) + (1/2)·sin x) dx — rational a, b with disc = 2.
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(ADD),
                vec![
                    rat(3, 2),
                    apply(sym(MUL), vec![rat(1, 2), apply(sym(SIN), vec![sym("x")])]),
                ],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-1.5_f64, -0.4, 0.0, 0.4, 1.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (1.5 + 0.5 * x_val.sin());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase34_cos_two_plus_cos_closes_and_matches() {
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(COS), vec![sym("x")])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-1.5_f64, -0.4, 0.0, 0.4, 1.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (2.0 + x_val.cos());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase34_cos_five_plus_three_cos_sqrt_free() {
    // disc=16, ratio=(5-3)/(5+3)=1/4 — both perfect squares → no Sqrt.
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(ADD),
                vec![int(5), apply(sym(MUL), vec![int(3), apply(sym(COS), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(
        !contains_head(&phi, SQRT),
        "Expected Sqrt-free form; got {phi:?}"
    );
    for &x_val in &[-1.5_f64, -0.4, 0.0, 0.4, 1.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (5.0 + 3.0 * x_val.cos());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase34_cos_negative_a_arctan_branch_closes() {
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(-2), apply(sym(COS), vec![sym("x")])])],
    );
    let phi = integrate(integrand);
    assert!(
        !is_unevaluated_integrate(&phi),
        "Phase 34 should close 1/(-2 + cos x); got {phi:?}"
    );
    assert!(contains_head(&phi, ATAN), "Expected Atan in {phi:?}");
    for &x_val in &[-1.5_f64, -0.4, 0.0, 0.4, 1.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (-2.0 + x_val.cos());
        assert!(
            (got - expected).abs() < 1e-4,
            "At x={x_val}: derivative={got}, expected={expected}"
        );
    }
}

#[test]
fn phase34_sin_operand_order_swapped() {
    // ∫ 1/(sin x + 2) dx — constant on the right.  Must still close.
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![apply(sym(SIN), vec![sym("x")]), int(2)])],
    );
    let result = integrate(integrand);
    assert!(
        !is_unevaluated_integrate(&result),
        "Swapped form should still close; got {result:?}"
    );
}

#[test]
fn phase36_a_less_than_b_sin_now_closes() {
    // ∫ 1/(1 + 2·sin x) dx — Phase 36 closes the log form that Phase 34 deferred.
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(ADD),
                vec![int(1), apply(sym(MUL), vec![int(2), apply(sym(SIN), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    // 1 + 2 sin x zero at sin x = -1/2.  Sample inside (-π/4, π/4).
    for &x_val in &[-0.7_f64, -0.2, 0.0, 0.2, 0.7] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (1.0 + 2.0 * x_val.sin());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase35_a_equals_b_now_closes() {
    // ∫ 1/(1 + sin x) dx — Phase 35 closes the degenerate `a² = b²` case
    // that Phase 34 previously left unevaluated. Closed form: -2/(tan(x/2)+1).
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(1), apply(sym(SIN), vec![sym("x")])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-2.0_f64, -1.0, -0.3, 0.3, 1.0, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (1.0 + x_val.sin());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase35_one_minus_sin_closes() {
    // ∫ 1/(2 − 2·sin x) dx — sin, b = −a.  Closed form: 2/(2·(1−tan(x/2))).
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(SUB),
                vec![int(2), apply(sym(MUL), vec![int(2), apply(sym(SIN), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-2.0_f64, -1.0, -0.3, 0.3, 1.0, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (2.0 - 2.0 * x_val.sin());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase35_one_plus_cos_closes() {
    // ∫ 1/(1 + cos x) dx — cos, b = a.  Closed form: tan(x/2).
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(1), apply(sym(COS), vec![sym("x")])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-2.0_f64, -1.0, -0.3, 0.3, 1.0, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (1.0 + x_val.cos());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase35_one_minus_cos_closes() {
    // ∫ 1/(1 − cos x) dx — cos, b = −a.  Closed form: −cot(x/2).
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(SUB), vec![int(1), apply(sym(COS), vec![sym("x")])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    // Sample on (0, π) — avoid x = 0, 2π where 1 − cos x = 0.
    for &x_val in &[0.5_f64, 1.0, 1.5, 2.0, 2.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (1.0 - x_val.cos());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase35_with_numerator_coefficient() {
    // ∫ 5/(2 + 2·sin x) dx — coefficient c=5 scales the closed form.
    let integrand = apply(
        sym(DIV),
        vec![
            int(5),
            apply(
                sym(ADD),
                vec![int(2), apply(sym(MUL), vec![int(2), apply(sym(SIN), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-2.0_f64, -1.0, -0.3, 0.3, 1.0, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 5.0 / (2.0 + 2.0 * x_val.sin());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase35_rational_coefficients() {
    // ∫ 1/((3/2) + (3/2)·cos x) dx — rational a = b.
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(ADD),
                vec![rat(3, 2), apply(sym(MUL), vec![rat(3, 2), apply(sym(COS), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-2.0_f64, -1.0, -0.3, 0.3, 1.0, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (1.5 + 1.5 * x_val.cos());
        assert!((got - expected).abs() < 1e-3);
    }
}

// The "non-bare argument falls through" deferral previously asserted here has
// been promoted to a Phase 38 success test below — Phase 38 closes such
// integrals via the linear-substitution dispatcher.

#[test]
fn phase34_fallthrough_symbolic_coefficient() {
    // ∫ 1/(a + sin x) dx — can't decide disc sign without assumptions.
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![sym("a"), apply(sym(SIN), vec![sym("x")])])],
    );
    let result = integrate(integrand);
    assert!(is_unevaluated_integrate(&result));
}

#[test]
fn phase34_regression_pure_sin_unchanged() {
    // ∫ sin(x) dx = −cos(x) must continue to work.
    let result = integrate(apply(sym(SIN), vec![sym("x")]));
    let cos_x = apply(sym(COS), vec![sym("x")]);
    let neg_cos = apply(sym(NEG), vec![cos_x.clone()]);
    let mul_neg = apply(sym(MUL), vec![int(-1), cos_x]);
    assert!(result == neg_cos || result == mul_neg, "got {result:?}");
}

#[test]
fn phase34_regression_one_over_cos_now_closes() {
    // ∫ 1/cos(x) dx — a = 0 cosine log branch.
    let integrand = apply(sym(DIV), vec![int(1), apply(sym(COS), vec![sym("x")])]);
    let result = integrate(integrand);
    assert!(!is_unevaluated_integrate(&result));
    assert!(contains_head(&result, LOG), "Expected Log in {result:?}");
    for &x_val in &[-1.0_f64, -0.4, 0.0, 0.4, 1.0] {
        let got = phase34_numerical_derivative(&result, x_val);
        let expected = 1.0 / x_val.cos();
        assert!((got - expected).abs() < 1e-3, "x={x_val}: got={got}, expected={expected}");
    }
}

// ---------------------------------------------------------------------------
// Phase 36 — Weierstrass log form for a² < b² (cos + edge cases)
// ---------------------------------------------------------------------------

#[test]
fn phase36_one_over_sin_now_closes() {
    // ∫ 1/sin(x) dx = log|tan(x/2)|.
    let integrand = apply(sym(DIV), vec![int(1), apply(sym(SIN), vec![sym("x")])]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    assert!(contains_head(&phi, LOG), "Expected Log in {phi:?}");
    for &x_val in &[0.4_f64, 0.8, 1.2, 1.6, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / x_val.sin();
        assert!((got - expected).abs() < 1e-3, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase36_a_less_than_b_cos_now_closes() {
    // ∫ 1/(1 + 2·cos x) dx — cos branch with b > |a|, b² > a².
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(ADD),
                vec![int(1), apply(sym(MUL), vec![int(2), apply(sym(COS), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    // 1+2 cos x zero at x=±2π/3.  Sample inside (-π/2, π/2).
    for &x_val in &[-1.2_f64, -0.5, 0.0, 0.5, 1.2] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (1.0 + 2.0 * x_val.cos());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase36_negative_a_sin() {
    // ∫ 1/(−1 + 2·sin x) dx — sin branch with a < 0.
    let neg_one_plus_two_sin = apply(
        sym(ADD),
        vec![int(-1), apply(sym(MUL), vec![int(2), apply(sym(SIN), vec![sym("x")])])],
    );
    let integrand = apply(sym(DIV), vec![int(1), neg_one_plus_two_sin]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[1.0_f64, 1.5, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (-1.0 + 2.0 * x_val.sin());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase36_with_numerator_coefficient() {
    // ∫ 3/(1 + 2·sin x) dx — c=3 scaling.
    let integrand = apply(
        sym(DIV),
        vec![
            int(3),
            apply(
                sym(ADD),
                vec![int(1), apply(sym(MUL), vec![int(2), apply(sym(SIN), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    for &x_val in &[-0.7_f64, -0.2, 0.0, 0.2, 0.7] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 3.0 / (1.0 + 2.0 * x_val.sin());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase36_perfect_square_discriminant() {
    // ∫ 1/(3 + 5·sin x) dx — disc=−16, perfect-square magnitude.
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(ADD),
                vec![int(3), apply(sym(MUL), vec![int(5), apply(sym(SIN), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(!contains_head(&phi, SQRT), "Perfect-square |disc|=16 should fold");
    for &x_val in &[-0.3_f64, 0.0, 0.3, 0.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (3.0 + 5.0 * x_val.sin());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase37_cos_negative_b_now_closes() {
    // ∫ 1/(1 − 2·cos x) dx — Phase 37 (cos b<−|a|) now closes via the
    // same log formula as Phase 36; Abs wrapping handles the sign flip.
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(SUB),
                vec![int(1), apply(sym(MUL), vec![int(2), apply(sym(COS), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    // 1 − 2 cos x zeros at cos x = 1/2 (x = ±π/3 ≈ ±1.047).
    // Sample on (π/3, π).
    for &x_val in &[1.2_f64, 1.6, 2.0, 2.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (1.0 - 2.0 * x_val.cos());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase37_cos_negative_b_with_negative_a() {
    // ∫ 1/(−1 − 3·cos x) dx — both a and b negative.
    let neg_one_minus_three_cos = apply(
        sym(SUB),
        vec![int(-1), apply(sym(MUL), vec![int(3), apply(sym(COS), vec![sym("x")])])],
    );
    let integrand = apply(sym(DIV), vec![int(1), neg_one_minus_three_cos]);
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[0.5_f64, 1.0, 1.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (-1.0 - 3.0 * x_val.cos());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase37_cos_negative_b_with_numerator_coefficient() {
    // ∫ 5/(1 − 2·cos x) dx — numerator c=5 scales.
    let integrand = apply(
        sym(DIV),
        vec![
            int(5),
            apply(
                sym(SUB),
                vec![int(1), apply(sym(MUL), vec![int(2), apply(sym(COS), vec![sym("x")])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    for &x_val in &[1.2_f64, 1.6, 2.0, 2.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 5.0 / (1.0 - 2.0 * x_val.cos());
        assert!((got - expected).abs() < 1e-3);
    }
}

// ---------------------------------------------------------------------------
// Phase 38: linear-argument substitution lifts Weierstrass to ``trig(α·x + β)``.
//
// With ``u = α·x + β`` (``du = α·dx``), every Phase 34/35/36/37 closed form
// applies unchanged with ``tan((α·x+β)/2)`` in place of ``tan(x/2)`` and the
// outer coefficient scaled by ``1/α``.  Each case is verified by central
// differencing the closed form against the original integrand at sample
// points away from the integrand's singularities.
// ---------------------------------------------------------------------------

#[test]
fn phase38_sin_two_x_closes() {
    // ∫ 1/(2 + sin 2x) dx — promoted from Phase 34 deferral.
    let two_x = apply(sym(MUL), vec![int(2), sym("x")]);
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![two_x])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-1.0_f64, -0.3, 0.0, 0.3, 1.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (2.0 + (2.0 * x_val).sin());
        assert!((got - expected).abs() < 1e-4, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase38_scaled_csc_branch_closes() {
    // ∫ 3/(2·sin(2x+1)) dx — c, b, and alpha scaling.
    let arg = apply(sym(ADD), vec![apply(sym(MUL), vec![int(2), sym("x")]), int(1)]);
    let denominator = apply(sym(MUL), vec![int(2), apply(sym(SIN), vec![arg])]);
    let phi = integrate(apply(sym(DIV), vec![int(3), denominator]));
    assert!(!is_unevaluated_integrate(&phi));
    assert!(contains_head(&phi, LOG), "Expected Log in {phi:?}");
    for &x_val in &[0.0_f64, 0.2, 0.5, 0.8] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 3.0 / (2.0 * (2.0 * x_val + 1.0).sin());
        assert!((got - expected).abs() < 1e-3, "x={x_val}: got={got}, expected={expected}");
    }
}

#[test]
fn phase38_cos_three_x() {
    // ∫ 1/(2 + cos 3x) dx — α = 3, β = 0.
    let three_x = apply(sym(MUL), vec![int(3), sym("x")]);
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(COS), vec![three_x])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-0.5_f64, -0.2, 0.0, 0.2, 0.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (2.0 + (3.0 * x_val).cos());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase38_sin_x_plus_constant_phase() {
    // ∫ 1/(2 + sin(x + 1)) dx — pure phase shift α = 1, β = 1.
    let x_plus_one = apply(sym(ADD), vec![sym("x"), int(1)]);
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![x_plus_one])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-2.0_f64, -1.0, 0.0, 1.0, 2.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (2.0 + (x_val + 1.0).sin());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase38_sin_two_x_plus_phase() {
    // ∫ 1/(2 + sin(2x + 1)) dx — full α = 2, β = 1.
    let two_x_plus_one = apply(
        sym(ADD),
        vec![apply(sym(MUL), vec![int(2), sym("x")]), int(1)],
    );
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![two_x_plus_one])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-0.8_f64, -0.3, 0.0, 0.3, 0.8] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (2.0 + (2.0 * x_val + 1.0).sin());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase38_rational_alpha() {
    // ∫ 1/(2 + sin(x/2)) dx — rational α = 1/2.
    let half_x = apply(sym(MUL), vec![rat(1, 2), sym("x")]);
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![half_x])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-2.0_f64, -1.0, 0.5, 1.5] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (2.0 + (0.5 * x_val).sin());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase38_negative_alpha() {
    // ∫ 1/(2 + sin(−2x)) dx — α = −2, sign-flipped.
    let neg_two_x = apply(sym(MUL), vec![int(-2), sym("x")]);
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![neg_two_x])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-1.0_f64, -0.3, 0.0, 0.3, 1.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (2.0 + (-2.0 * x_val).sin());
        assert!((got - expected).abs() < 1e-4);
    }
}

#[test]
fn phase38_degenerate_branch_under_substitution() {
    // ∫ 1/(1 + cos 2x) dx — Phase 35 degenerate path with α = 2.
    // 1 + cos 2x = 2·cos²(x); antiderivative is (1/2)·tan(x).
    let two_x = apply(sym(MUL), vec![int(2), sym("x")]);
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(1), apply(sym(COS), vec![two_x])])],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    for &x_val in &[-1.0_f64, -0.3, 0.3, 1.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (1.0 + (2.0 * x_val).cos());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase38_log_form_under_substitution() {
    // ∫ 1/(1 + 2·sin 2x) dx — Phase 36 log path with α = 2.
    let two_x = apply(sym(MUL), vec![int(2), sym("x")]);
    let integrand = apply(
        sym(DIV),
        vec![
            int(1),
            apply(
                sym(ADD),
                vec![int(1), apply(sym(MUL), vec![int(2), apply(sym(SIN), vec![two_x])])],
            ),
        ],
    );
    let phi = integrate(integrand);
    assert!(!is_unevaluated_integrate(&phi));
    // 1 + 2·sin 2x zeros at sin 2x = −1/2 (2x ≈ ±π/6 ≈ ±0.52).  Sample away.
    for &x_val in &[-1.0_f64, -0.5, 0.0, 0.5, 1.0] {
        let got = phase34_numerical_derivative(&phi, x_val);
        let expected = 1.0 / (1.0 + 2.0 * (2.0 * x_val).sin());
        assert!((got - expected).abs() < 1e-3);
    }
}

#[test]
fn phase38_fallthrough_nonlinear_argument() {
    // ∫ 1/(2 + sin(x²)) dx — argument is x², not linear in x.  Deferred.
    let x_sq = apply(sym(MUL), vec![sym("x"), sym("x")]);
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![x_sq])])],
    );
    let result = integrate(integrand);
    assert!(is_unevaluated_integrate(&result));
}

#[test]
fn phase38_fallthrough_symbolic_alpha() {
    // ∫ 1/(2 + sin(α·x)) dx — symbolic α can't be tested for ≠ 0.  Deferred.
    let alpha_x = apply(sym(MUL), vec![sym("alpha"), sym("x")]);
    let integrand = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![int(2), apply(sym(SIN), vec![alpha_x])])],
    );
    let result = integrate(integrand);
    assert!(is_unevaluated_integrate(&result));
}

// ---------------------------------------------------------------------------
// Phase 47 (Rust port): Nested-Add flattening.
//
// Ports the Python Phase 47 fix.  When either binary Add operand is itself
// an Add(...) apply, the handler flattens the tree, sums numeric literals
// once, and rebuilds a left-associated chain.  Makes Add canonical for
// any consumer comparing trees structurally.
// ---------------------------------------------------------------------------

#[test]
fn phase47_nested_add_flattens() {
    let mut vm = VM::new(Box::new(SymbolicBackend::new()));
    let k = sym("k");
    let nested = apply(sym(ADD), vec![
        apply(sym(ADD), vec![k.clone(), int(1)]),
        int(1),
    ]);
    let out = vm.eval(nested);
    let expected = apply(sym(ADD), vec![k, int(2)]);
    assert_eq!(out, expected);
}

#[test]
fn phase47_triply_nested_add() {
    let mut vm = VM::new(Box::new(SymbolicBackend::new()));
    let k = sym("k");
    let nested = apply(sym(ADD), vec![
        apply(sym(ADD), vec![
            apply(sym(ADD), vec![k.clone(), int(1)]),
            int(1),
        ]),
        int(1),
    ]);
    let out = vm.eval(nested);
    let expected = apply(sym(ADD), vec![k, int(3)]);
    assert_eq!(out, expected);
}

#[test]
fn phase47_add_constants_fold() {
    let mut vm = VM::new(Box::new(SymbolicBackend::new()));
    let k = sym("k");
    let nested = apply(sym(ADD), vec![
        apply(sym(ADD), vec![k.clone(), int(2)]),
        int(3),
    ]);
    let out = vm.eval(nested);
    let expected = apply(sym(ADD), vec![k, int(5)]);
    assert_eq!(out, expected);
}

#[test]
fn phase47_constants_cancel_to_bare_symbol() {
    let mut vm = VM::new(Box::new(SymbolicBackend::new()));
    let k = sym("k");
    let nested = apply(sym(ADD), vec![
        apply(sym(ADD), vec![k.clone(), int(1)]),
        int(-1),
    ]);
    let out = vm.eval(nested);
    // Constants sum to zero → drop the literal → bare k.
    assert_eq!(out, k);
}

#[test]
fn phase47_non_nested_add_untouched() {
    let mut vm = VM::new(Box::new(SymbolicBackend::new()));
    let k = sym("k");
    let flat = apply(sym(ADD), vec![k.clone(), int(1)]);
    let out = vm.eval(flat);
    let expected = apply(sym(ADD), vec![k, int(1)]);
    assert_eq!(out, expected);
}

#[test]
fn phase47_add_zero_still_simplifies() {
    let mut vm = VM::new(Box::new(SymbolicBackend::new()));
    let x = sym("x");
    let out = vm.eval(apply(sym(ADD), vec![int(0), x.clone()]));
    assert_eq!(out, x);
}

use cas_summation::{
    evaluate_product, evaluate_product_expr, evaluate_sum, faulhaber_ir, geometric_sum_ir,
    poly_sum_ir, rational_value, try_special_infinite, Rational, GAMMA_FUNC, PRODUCT, SUM,
};
use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, DIV, EXP, LOG, MUL, NEG, POW, SUB};

fn eval(node: IRNode) -> IRNode {
    match node {
        IRNode::Apply(apply_node) => {
            let head = apply_node.head.clone();
            let args: Vec<IRNode> = apply_node.args.into_iter().map(eval).collect();
            let name = match &head {
                IRNode::Symbol(name) => name.as_str(),
                _ => return apply(head, args),
            };
            let out = match name {
                ADD => fold_numeric(&args, Rational::new(0, 1), |a, b| a + b),
                MUL => fold_numeric(&args, Rational::new(1, 1), |a, b| a * b),
                SUB if args.len() == 2 => numeric_binary(&args[0], &args[1], |a, b| a - b),
                DIV if args.len() == 2 => numeric_binary(&args[0], &args[1], |a, b| a / b),
                POW if args.len() == 2 => numeric_pow(&args[0], &args[1]),
                NEG if args.len() == 1 => rational_value(&args[0]).map(|v| (-v).to_ir()),
                _ => None,
            };
            out.unwrap_or_else(|| apply(head, args))
        }
        other => other,
    }
}

fn fold_numeric(
    args: &[IRNode],
    init: Rational,
    op: impl Fn(Rational, Rational) -> Rational,
) -> Option<IRNode> {
    let mut acc = init;
    for arg in args {
        acc = op(acc, rational_value(arg)?);
    }
    Some(acc.to_ir())
}

fn numeric_binary(
    a: &IRNode,
    b: &IRNode,
    op: impl FnOnce(Rational, Rational) -> Rational,
) -> Option<IRNode> {
    Some(op(rational_value(a)?, rational_value(b)?).to_ir())
}

fn numeric_pow(a: &IRNode, b: &IRNode) -> Option<IRNode> {
    let base = rational_value(a)?;
    let IRNode::Integer(exp) = b else {
        return None;
    };
    if *exp < 0 {
        return None;
    }
    let mut out = Rational::new(1, 1);
    for _ in 0..*exp {
        out = out * base;
    }
    Some(out.to_ir())
}

#[test]
fn evaluates_constant_and_geometric_sums() {
    let k = sym("k");
    assert_eq!(
        evaluate_sum(int(5), k.clone(), int(1), int(10), eval),
        int(50)
    );

    let geo = apply(sym(POW), vec![rat(1, 2), k.clone()]);
    assert_eq!(evaluate_sum(geo, k, int(0), sym("%inf"), eval), int(2));
}

#[test]
fn geometric_sum_builder_matches_finite_and_infinite_values() {
    let finite = geometric_sum_ir(int(1), int(3), int(0), Some(int(3)), false);
    assert_eq!(eval(finite), int(40));

    let infinite = geometric_sum_ir(int(1), rat(1, 4), int(2), None, true);
    assert_eq!(eval(infinite), rat(1, 12));
}

#[test]
fn faulhaber_and_power_sum_cover_degrees_zero_through_five() {
    let expected = [4, 10, 30, 100, 354, 1300];
    for (m, expected) in expected.into_iter().enumerate() {
        let node = faulhaber_ir(m as i64, int(4)).unwrap();
        assert_eq!(eval(node), int(expected));
    }
    assert!(faulhaber_ir(6, int(4)).is_none());

    assert_eq!(
        eval(poly_sum_ir(2, Rational::new(1, 1), 1, int(4)).unwrap()),
        int(30)
    );
    assert_eq!(
        eval(poly_sum_ir(0, Rational::new(1, 1), 0, int(4)).unwrap()),
        int(5)
    );
    assert_eq!(
        evaluate_sum(
            apply(sym(MUL), vec![int(3), sym("k")]),
            sym("k"),
            int(1),
            int(4),
            eval
        ),
        int(30)
    );
}

#[test]
fn recognises_classic_infinite_series() {
    let k = sym("k");
    let x = sym("x");
    let basel = apply(
        sym(DIV),
        vec![int(1), apply(sym(POW), vec![k.clone(), int(2)])],
    );
    let result = try_special_infinite(&basel, &k, &int(1)).unwrap();
    assert_eq!(
        result,
        apply(
            sym(DIV),
            vec![apply(sym(POW), vec![sym("%pi"), int(2)]), int(6)]
        )
    );

    let gamma_kp1 = apply(
        sym(GAMMA_FUNC),
        vec![apply(sym(ADD), vec![k.clone(), int(1)])],
    );
    let inv_fact = apply(sym(DIV), vec![int(1), gamma_kp1.clone()]);
    assert_eq!(
        try_special_infinite(&inv_fact, &k, &int(0)),
        Some(sym("%e"))
    );

    let exp_series = apply(
        sym(DIV),
        vec![apply(sym(POW), vec![x.clone(), k.clone()]), gamma_kp1],
    );
    assert_eq!(
        try_special_infinite(&exp_series, &k, &int(0)),
        Some(apply(sym(EXP), vec![x]))
    );
}

#[test]
fn evaluates_products_and_fallbacks() {
    let k = sym("k");
    let n = sym("n");
    let factorial = evaluate_product(k.clone(), k.clone(), int(1), n.clone(), eval);
    assert!(matches!(factorial, IRNode::Apply(node) if node.head == sym(GAMMA_FUNC)));

    assert_eq!(
        evaluate_product(int(2), k.clone(), int(0), int(4), eval),
        int(32)
    );
    assert_eq!(
        evaluate_product(
            apply(sym(MUL), vec![int(2), k.clone()]),
            k.clone(),
            int(1),
            n,
            eval
        ),
        apply(
            sym(MUL),
            vec![
                apply(sym(POW), vec![int(2), sym("n")]),
                apply(
                    sym(GAMMA_FUNC),
                    vec![apply(sym(ADD), vec![sym("n"), int(1)])]
                ),
            ],
        )
    );

    let fallback = evaluate_product(
        apply(sym(POW), vec![k.clone(), int(3)]),
        k.clone(),
        int(1),
        sym("n"),
        eval,
    );
    assert!(matches!(fallback, IRNode::Apply(node) if node.head == sym(PRODUCT)));

    assert!(evaluate_product_expr(k.clone(), k, int(0), sym("n")).is_none());
}

#[test]
fn unknown_sum_falls_back_to_sum_node() {
    let k = sym("k");
    let f = apply(sym("Sin"), vec![k.clone()]);
    let out = evaluate_sum(f, k, int(1), sym("n"), eval);
    assert!(matches!(out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 39: Telescoping sums.
//
// ∑_{k=lo}^{hi} [g(k+1) − g(k)] = g(hi+1) − g(lo)
// ∑_{k=lo}^{hi} [g(k) − g(k+1)] = g(lo) − g(hi+1)
//
// Detection is purely structural: substitute k → k+1 in one half of the SUB
// shape and compare to the other half after eval normalisation.
// ---------------------------------------------------------------------------

#[test]
fn phase39_standard_telescope_concrete_bounds() {
    // ∑_{k=1}^{4} [(k+1)² − k²] = 5² − 1² = 24.
    let k = sym("k");
    let k_plus_one_sq = apply(
        sym(POW),
        vec![apply(sym(ADD), vec![k.clone(), int(1)]), int(2)],
    );
    let k_sq = apply(sym(POW), vec![k.clone(), int(2)]);
    let f = apply(sym(SUB), vec![k_plus_one_sq, k_sq]);
    assert_eq!(evaluate_sum(f, k, int(1), int(4), eval), int(24));
}

#[test]
fn phase39_antisymmetric_telescope() {
    // ∑_{k=1}^{3} [k² − (k+1)²] = 1² − 4² = −15.
    let k = sym("k");
    let k_sq = apply(sym(POW), vec![k.clone(), int(2)]);
    let k_plus_one_sq = apply(
        sym(POW),
        vec![apply(sym(ADD), vec![k.clone(), int(1)]), int(2)],
    );
    let f = apply(sym(SUB), vec![k_sq, k_plus_one_sq]);
    assert_eq!(evaluate_sum(f, k, int(1), int(3), eval), int(-15));
}

#[test]
fn phase39_linear_g_counts_terms() {
    // ∑_{k=1}^{10} [(k+1) − k] = g(11) − g(1) = 11 − 1 = 10.
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![apply(sym(ADD), vec![k.clone(), int(1)]), k.clone()],
    );
    assert_eq!(evaluate_sum(f, k, int(1), int(10), eval), int(10));
}

#[test]
fn phase39_constant_offset_in_g() {
    // ∑_{k=1}^{5} [(k + 6) − (k + 5)] = g(6) − g(1) = 11 − 6 = 5.
    let k = sym("k");
    let g_at_k_plus_1 = apply(
        sym(ADD),
        vec![apply(sym(ADD), vec![k.clone(), int(1)]), int(5)],
    );
    let g_at_k = apply(sym(ADD), vec![k.clone(), int(5)]);
    let f = apply(sym(SUB), vec![g_at_k_plus_1, g_at_k]);
    assert_eq!(evaluate_sum(f, k, int(1), int(5), eval), int(5));
}

#[test]
fn phase39_non_telescoping_falls_through() {
    // ∑_{k=1}^{3} [k² − k] = (1−1)+(4−2)+(9−3) = 8 (numeric fallback).
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![k.clone(), int(2)]), k.clone()],
    );
    assert_eq!(evaluate_sum(f, k, int(1), int(3), eval), int(8));
}

#[test]
fn phase39_constant_difference_routes_through_constant_rule() {
    // ∑_{k=1}^{10} [5 − 3] = ∑ 2 = 20 (step 1 fires first; telescope never runs).
    let k = sym("k");
    let f = apply(sym(SUB), vec![int(5), int(3)]);
    assert_eq!(evaluate_sum(f, k, int(1), int(10), eval), int(20));
}

#[test]
fn phase39_symbolic_upper_bound_non_unevaluated() {
    let k = sym("k");
    let n = sym("n");
    let f = apply(
        sym(SUB),
        vec![apply(sym(ADD), vec![k.clone(), int(1)]), k.clone()],
    );
    let out = evaluate_sum(f, k, int(1), n, eval);
    // Must not stay as a SUM(...) node.
    assert!(
        !matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)),
        "expected non-unevaluated form, got {out:?}"
    );
}

#[test]
fn phase39_infinite_upper_falls_through() {
    // g(k) = k grows at infinity; Phase 41 guard refuses.
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![apply(sym(ADD), vec![k.clone(), int(1)]), k.clone()],
    );
    let out = evaluate_sum(f, k, int(0), sym("%inf"), eval);
    assert!(matches!(out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 41+42: limit-aware infinite telescope.
//
// When `hi = %inf` AND `g(k)` provably vanishes at infinity, the dispatcher
// emits −g(lo) (standard orientation) or g(lo) (antisymmetric).
// ---------------------------------------------------------------------------

#[test]
fn phase41_antisymmetric_one_over_k_minus_one_over_kp1() {
    // ∑_{k=1}^∞ [1/k − 1/(k+1)] = 1 (Phase 41 antisymmetric).
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![
            apply(sym(DIV), vec![int(1), k.clone()]),
            apply(
                sym(DIV),
                vec![int(1), apply(sym(ADD), vec![k.clone(), int(1)])],
            ),
        ],
    );
    assert_eq!(evaluate_sum(f, k, int(1), sym("%inf"), eval), int(1));
}

#[test]
fn phase41_standard_orientation_kp1_minus_k() {
    // ∑_{k=1}^∞ [1/(k+1) − 1/k] = −1.
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![
            apply(
                sym(DIV),
                vec![int(1), apply(sym(ADD), vec![k.clone(), int(1)])],
            ),
            apply(sym(DIV), vec![int(1), k.clone()]),
        ],
    );
    assert_eq!(evaluate_sum(f, k, int(1), sym("%inf"), eval), int(-1));
}

#[test]
fn phase41_higher_starting_index() {
    // ∑_{k=2}^∞ [1/k − 1/(k+1)] = 1/2.
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![
            apply(sym(DIV), vec![int(1), k.clone()]),
            apply(
                sym(DIV),
                vec![int(1), apply(sym(ADD), vec![k.clone(), int(1)])],
            ),
        ],
    );
    assert_eq!(evaluate_sum(f, k, int(2), sym("%inf"), eval), rat(1, 2));
}

#[test]
fn phase41_quadratic_denominator() {
    // ∑_{k=1}^∞ [1/k² − 1/(k+1)²] = 1.
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![
            apply(
                sym(DIV),
                vec![int(1), apply(sym(POW), vec![k.clone(), int(2)])],
            ),
            apply(
                sym(DIV),
                vec![
                    int(1),
                    apply(
                        sym(POW),
                        vec![apply(sym(ADD), vec![k.clone(), int(1)]), int(2)],
                    ),
                ],
            ),
        ],
    );
    assert_eq!(evaluate_sum(f, k, int(1), sym("%inf"), eval), int(1));
}

#[test]
fn phase42_proper_rational_k_over_k_squared_plus_1() {
    // ∑_{k=1}^∞ [k/(k²+1) − (k+1)/((k+1)²+1)] = g(1) = 1/2.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let g_k = apply(
        sym(DIV),
        vec![
            k.clone(),
            apply(
                sym(ADD),
                vec![apply(sym(POW), vec![k.clone(), int(2)]), int(1)],
            ),
        ],
    );
    let g_kp1 = apply(
        sym(DIV),
        vec![
            kp1.clone(),
            apply(
                sym(ADD),
                vec![apply(sym(POW), vec![kp1.clone(), int(2)]), int(1)],
            ),
        ],
    );
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    assert_eq!(evaluate_sum(f, k, int(1), sym("%inf"), eval), rat(1, 2));
}

#[test]
fn phase42_improper_rational_falls_through() {
    // g(k) = k/(k+1) has equal degrees; limit is 1, not 0.  Refuse.
    let k = sym("k");
    let g_k = apply(
        sym(DIV),
        vec![k.clone(), apply(sym(ADD), vec![k.clone(), int(1)])],
    );
    let g_kp1 = apply(
        sym(DIV),
        vec![
            apply(sym(ADD), vec![k.clone(), int(1)]),
            apply(sym(ADD), vec![k.clone(), int(2)]),
        ],
    );
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase42_transcendental_numerator_falls_through() {
    // sin(k)/k² is non-polynomial; conservative refuse.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let f = apply(
        sym(SUB),
        vec![
            apply(
                sym(DIV),
                vec![sin_k, apply(sym(POW), vec![k.clone(), int(2)])],
            ),
            apply(
                sym(DIV),
                vec![sin_kp1, apply(sym(POW), vec![kp1, int(2)])],
            ),
        ],
    );
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 43: transcendental vanishing-at-infinity.
//
// Extends Phase 41/42 to accept Pow(b, h(k)) with |b| > 1 and h positive-
// degree with positive leading coefficient.  Sign-aware guard refuses
// `2^(-k)` and similar (these vanish, not diverge).
// ---------------------------------------------------------------------------

#[test]
fn phase43_pow_2_diverges_closes() {
    // ∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = 1.
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![
            apply(sym(DIV), vec![int(1), apply(sym(POW), vec![int(2), k.clone()])]),
            apply(
                sym(DIV),
                vec![
                    int(1),
                    apply(sym(POW), vec![int(2), apply(sym(ADD), vec![k.clone(), int(1)])]),
                ],
            ),
        ],
    );
    assert_eq!(evaluate_sum(f, k, int(0), sym("%inf"), eval), int(1));
}

#[test]
fn phase43_pow_3_higher_start() {
    // ∑_{k=1}^∞ [1/3^k − 1/3^(k+1)] = 1/3.
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![
            apply(sym(DIV), vec![int(1), apply(sym(POW), vec![int(3), k.clone()])]),
            apply(
                sym(DIV),
                vec![
                    int(1),
                    apply(sym(POW), vec![int(3), apply(sym(ADD), vec![k.clone(), int(1)])]),
                ],
            ),
        ],
    );
    assert_eq!(evaluate_sum(f, k, int(1), sym("%inf"), eval), rat(1, 3));
}

#[test]
fn phase43_base_half_falls_through() {
    // Pow(1/2, k) → 0, not ∞; Phase 43 refuses.
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![
            apply(sym(DIV), vec![int(1), apply(sym(POW), vec![rat(1, 2), k.clone()])]),
            apply(
                sym(DIV),
                vec![
                    int(1),
                    apply(sym(POW), vec![rat(1, 2), apply(sym(ADD), vec![k.clone(), int(1)])]),
                ],
            ),
        ],
    );
    let out = evaluate_sum(f, k, int(0), sym("%inf"), eval);
    assert!(matches!(out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase43_mul_polynomial_times_exponential() {
    // ∑_{k=1}^∞ [1/(k·2^k) − 1/((k+1)·2^(k+1))] = g(1) = 1/2.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let f = apply(
        sym(SUB),
        vec![
            apply(
                sym(DIV),
                vec![
                    int(1),
                    apply(sym(MUL), vec![k.clone(), apply(sym(POW), vec![int(2), k.clone()])]),
                ],
            ),
            apply(
                sym(DIV),
                vec![
                    int(1),
                    apply(sym(MUL), vec![kp1.clone(), apply(sym(POW), vec![int(2), kp1.clone()])]),
                ],
            ),
        ],
    );
    assert_eq!(evaluate_sum(f, k, int(1), sym("%inf"), eval), rat(1, 2));
}

#[test]
fn phase43_pow_negative_exponent_polynomial_refuses() {
    // Pow(2, Mul(-1, k)) = 2^(-k) → 0, not ∞.  Sign-aware guard must refuse.
    let k = sym("k");
    let neg_k = apply(sym(MUL), vec![int(-1), k.clone()]);
    let neg_kp1 = apply(sym(MUL), vec![int(-1), apply(sym(ADD), vec![k.clone(), int(1)])]);
    let f = apply(
        sym(SUB),
        vec![
            apply(sym(DIV), vec![int(1), apply(sym(POW), vec![int(2), neg_k])]),
            apply(sym(DIV), vec![int(1), apply(sym(POW), vec![int(2), neg_kp1])]),
        ],
    );
    let out = evaluate_sum(f, k, int(0), sym("%inf"), eval);
    assert!(matches!(out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase43_pow_neg_wrapper_refuses() {
    // Pow(2, Neg(k)) — alternate IR for 2^(-k); same refusal.
    let k = sym("k");
    let f = apply(
        sym(SUB),
        vec![
            apply(
                sym(DIV),
                vec![int(1), apply(sym(POW), vec![int(2), apply(sym(NEG), vec![k.clone()])])],
            ),
            apply(
                sym(DIV),
                vec![
                    int(1),
                    apply(
                        sym(POW),
                        vec![int(2), apply(sym(NEG), vec![apply(sym(ADD), vec![k.clone(), int(1)])])],
                    ),
                ],
            ),
        ],
    );
    let out = evaluate_sum(f, k, int(0), sym("%inf"), eval);
    assert!(matches!(out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 44: Log divergence in vanishing-at-infinity recogniser.
// ---------------------------------------------------------------------------

fn substitute_kp1(node: &IRNode, k: &IRNode, kp1: &IRNode) -> IRNode {
    if node == k {
        return kp1.clone();
    }
    if let IRNode::Apply(apply_node) = node {
        let new_args: Vec<IRNode> = apply_node
            .args
            .iter()
            .map(|a| substitute_kp1(a, k, kp1))
            .collect();
        return apply(apply_node.head.clone(), new_args);
    }
    node.clone()
}

#[test]
fn phase44_log_of_polynomial_recognised() {
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let g_k = apply(
        sym(DIV),
        vec![int(1), apply(sym(LOG), vec![kp1.clone()])],
    );
    let g_kp1 = substitute_kp1(&g_k, &k, &kp1);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase44_log_of_exp_recognised() {
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let g_k = apply(
        sym(DIV),
        vec![
            int(1),
            apply(sym(LOG), vec![apply(sym(POW), vec![int(2), k.clone()])]),
        ],
    );
    let g_kp1 = substitute_kp1(&g_k, &k, &kp1);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase44_log_of_pow_negative_base_refuses() {
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let g_k = apply(
        sym(DIV),
        vec![
            int(1),
            apply(sym(LOG), vec![apply(sym(POW), vec![int(-2), k.clone()])]),
        ],
    );
    let g_kp1 = substitute_kp1(&g_k, &k, &kp1);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase44_log_of_negative_polynomial_refuses() {
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let neg_k = apply(sym(MUL), vec![int(-1), k.clone()]);
    let g_k = apply(sym(DIV), vec![int(1), apply(sym(LOG), vec![neg_k])]);
    let g_kp1 = substitute_kp1(&g_k, &k, &kp1);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

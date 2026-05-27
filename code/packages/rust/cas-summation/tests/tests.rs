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
fn phase42_transcendental_numerator_closes_via_phase49() {
    // Phase 49 closes this: |sin(k)| ≤ 1 + k² → ∞.  Antisymmetric
    // telescope reduces to g(1) = sin(1)/1² = sin(1).
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
    // Must NOT be the unevaluated Sum form.
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
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

// ---------------------------------------------------------------------------
// Phase 40+46 (Rust port): Add-with-negation telescope normaliser.
//
// Ports the Python helpers `_extract_negation` and
// `_normalise_add_neg_to_sub` so a user-written summand in
// `Add(g(k+1), Neg(g(k)))` or `Add(g(k+1), Div(-c, d))` form is
// rewritten to the canonical `Sub` shape before the Phase 39 / 41
// telescope detectors run.
//
// On the Python side this also feeds the symbolic-vm Apart-retry path,
// but `cas-summation` (Rust) doesn't depend on an Apart implementation
// — the value here is purely letting the telescope detector match
// more shapes the user might write directly.
// ---------------------------------------------------------------------------

#[test]
fn phase46_add_neg_standard_orientation() {
    // ∑_{k=1}^∞ [1/(k+1) + Neg(1/k)] = -1.
    let k = sym("k");
    let f = apply(
        sym(ADD),
        vec![
            apply(sym(DIV), vec![int(1), apply(sym(ADD), vec![k.clone(), int(1)])]),
            apply(sym(NEG), vec![apply(sym(DIV), vec![int(1), k.clone()])]),
        ],
    );
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert_eq!(out, int(-1));
}

#[test]
fn phase46_add_neg_order_swapped() {
    // ∑_{k=1}^∞ [Neg(1/k) + 1/(k+1)] = -1.
    let k = sym("k");
    let f = apply(
        sym(ADD),
        vec![
            apply(sym(NEG), vec![apply(sym(DIV), vec![int(1), k.clone()])]),
            apply(sym(DIV), vec![int(1), apply(sym(ADD), vec![k.clone(), int(1)])]),
        ],
    );
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert_eq!(out, int(-1));
}

#[test]
fn phase46_div_with_negative_numerator_antisymmetric() {
    // ∑_{k=1}^∞ [1/k + (-1)/(k+1)] = 1 (numerator-folded Neg).
    let k = sym("k");
    let f = apply(
        sym(ADD),
        vec![
            apply(sym(DIV), vec![int(1), k.clone()]),
            apply(sym(DIV), vec![int(-1), apply(sym(ADD), vec![k.clone(), int(1)])]),
        ],
    );
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert_eq!(out, int(1));
}

#[test]
fn phase46_div_with_negative_numerator_non_unit_constant() {
    // ∑_{k=1}^∞ [Div(-5, k+1) + Div(5, k)] = 5 — the constant-numerator case.
    let k = sym("k");
    let f = apply(
        sym(ADD),
        vec![
            apply(sym(DIV), vec![int(-5), apply(sym(ADD), vec![k.clone(), int(1)])]),
            apply(sym(DIV), vec![int(5), k.clone()]),
        ],
    );
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert_eq!(out, int(5));
}

#[test]
fn phase46_div_with_rational_negative_numerator() {
    // ∑_{k=1}^∞ [Div(1/2, k) + Div(-1/2, k+1)] = 1/2.
    // Exercises the IRNode::Rational arm of extract_negation.
    let k = sym("k");
    let f = apply(
        sym(ADD),
        vec![
            apply(sym(DIV), vec![rat(1, 2), k.clone()]),
            apply(sym(DIV), vec![rat(-1, 2), apply(sym(ADD), vec![k.clone(), int(1)])]),
        ],
    );
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert_eq!(out, rat(1, 2));
}

#[test]
fn phase46_both_negative_left_untouched() {
    // Add(Neg(a), Neg(b)) has no telescope structure — stays unevaluated.
    let k = sym("k");
    let f = apply(
        sym(ADD),
        vec![
            apply(sym(NEG), vec![apply(sym(DIV), vec![int(1), k.clone()])]),
            apply(sym(NEG), vec![apply(sym(DIV), vec![int(1), apply(sym(ADD), vec![k.clone(), int(1)])])]),
        ],
    );
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 49 (Rust port): Bounded × vanishing recogniser.
//
// Extends g_vanishes_at_infinity to accept Div(bounded, diverging)
// shapes where the numerator is uniformly bounded (Sin/Cos, closed
// under Mul/Add/Neg, constants in k).  Closes telescopes like
// ∑ [sin(k)/k² − sin(k+1)/(k+1)²] = sin(1).
// ---------------------------------------------------------------------------

#[test]
fn phase49_sin_over_k_squared_closes() {
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![sin_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![sin_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase49_cos_over_k_cube_closes() {
    let k = sym("k");
    let cos_k = apply(sym("Cos"), vec![k.clone()]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let cos_kp1 = apply(sym("Cos"), vec![kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![cos_k, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![cos_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase49_sin_cos_product_over_diverging() {
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let cos_k = apply(sym("Cos"), vec![k.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, cos_k]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let cos_kp1 = apply(sym("Cos"), vec![kp1.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, cos_kp1]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 50 (Rust port): Log/polynomial growth-rate recogniser.
//
// Phase 49 refused log(k)/k² (log isn't bounded).  Phase 50 closes it
// via the squeeze argument: log(h) grows slower than any diverging
// denominator, so log/poly → 0.
// ---------------------------------------------------------------------------

#[test]
fn phase50_log_over_k_squared_closes() {
    let k = sym("k");
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![log_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![log_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase50_log_of_polynomial_argument_closes() {
    let k = sym("k");
    let k_sq_plus_1 = apply(sym(ADD), vec![apply(sym(POW), vec![k.clone(), int(2)]), int(1)]);
    let log_term = apply(sym(LOG), vec![k_sq_plus_1]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let kp1_sq_plus_1 = apply(sym(ADD), vec![apply(sym(POW), vec![kp1.clone(), int(2)]), int(1)]);
    let log_kp1 = apply(sym(LOG), vec![kp1_sq_plus_1]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![log_term, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![log_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase50_log_of_negative_argument_refused() {
    // log of negative arg isn't real for odd k — must stay unevaluated.
    let k = sym("k");
    let neg_k = apply(sym(MUL), vec![int(-1), k.clone()]);
    let log_neg_k = apply(sym(LOG), vec![neg_k]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let neg_kp1 = apply(sym(MUL), vec![int(-1), kp1.clone()]);
    let log_neg_kp1 = apply(sym(LOG), vec![neg_kp1]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![log_neg_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![log_neg_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 51 (Rust port): Sqrt/polynomial growth-rate.
// ---------------------------------------------------------------------------

#[test]
fn phase51_sqrt_k_over_k_squared_closes() {
    let k = sym("k");
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![sqrt_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![sqrt_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase51_sqrt_k_cubed_over_k_squared_closes() {
    let k = sym("k");
    let k_cubed = apply(sym(POW), vec![k.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k_cubed]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![apply(sym(POW), vec![kp1.clone(), int(3)])]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![sqrt_k3, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![sqrt_kp1_3, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase51_sqrt_of_negative_polynomial_refused() {
    let k = sym("k");
    let neg_k = apply(sym(MUL), vec![int(-1), k.clone()]);
    let sqrt_neg_k = apply(sym("Sqrt"), vec![neg_k]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_neg_kp1 = apply(sym("Sqrt"), vec![apply(sym(MUL), vec![int(-1), kp1.clone()])]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![sqrt_neg_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![sqrt_neg_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 52 (Rust port): Bounded × polynomial numerator.
// ---------------------------------------------------------------------------
// The numerator is Mul(bounded_factor, polynomial_in_k).  Phase 52 catches
// shapes like sin(k)·k/k³ that Phase 49 misses (the whole Mul isn't bounded)
// and Phase 42 refuses (sin is not polynomial).

#[test]
fn phase52_sin_k_times_k_over_k_cubed_closes() {
    // Numerator = sin(k)·k: bounded × polynomial deg 1.
    // Denominator = k³: polynomial deg 3.  3 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, kp1.clone()]);
    let den_k = apply(sym(POW), vec![k.clone(), int(3)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, den_k]),
        apply(sym(DIV), vec![num_kp1, den_kp1]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase52_k_times_cos_k_over_k_squared_closes() {
    // Numerator = k·cos(k): polynomial deg 1 × bounded.  Factor order irrelevant.
    // Denominator = k²: polynomial deg 2.  2 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let cos_k = apply(sym("Cos"), vec![k.clone()]);
    let cos_kp1 = apply(sym("Cos"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![k.clone(), cos_k]);
    let num_kp1 = apply(sym(MUL), vec![kp1.clone(), cos_kp1]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, den_k]),
        apply(sym(DIV), vec![num_kp1, den_kp1]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase52_sin_k_times_k_squared_over_k_cubed_closes() {
    // Numerator = sin(k)·k²: bounded × polynomial deg 2.
    // Denominator = k³: polynomial deg 3.  3 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, apply(sym(POW), vec![k.clone(), int(2)])]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, apply(sym(POW), vec![kp1.clone(), int(2)])]);
    let den_k = apply(sym(POW), vec![k.clone(), int(3)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, den_k]),
        apply(sym(DIV), vec![num_kp1, den_kp1]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase52_sin_k_times_k_squared_over_k_squared_stays() {
    // Numerator = sin(k)·k²: bounded × polynomial deg 2.
    // Denominator = k²: polynomial deg 2.  2 > 2 is false → stays unevaluated.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, apply(sym(POW), vec![k.clone(), int(2)])]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, apply(sym(POW), vec![kp1.clone(), int(2)])]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, den_k]),
        apply(sym(DIV), vec![num_kp1, den_kp1]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase52_regression_k_over_k_squared_still_closes_via_phase42() {
    // Numerator = k: pure polynomial deg 1.  No bounded factor → Phase 52 skips.
    // Phase 42 closes it: deg 1 < deg 2.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![k.clone(), apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![kp1.clone(), apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 53 (Rust port): Sqrt × polynomial numerator.
//
// The numerator is Mul(Sqrt(P(k)), polynomial_in_k).  Phase 53 catches
// shapes like sqrt(k)·k/k³ (eff deg x2 = 1+2 = 3, den deg = 3, 2*3 = 6 > 3)
// that fall through all earlier phases.
//
// Uses ×2 integer arithmetic: effective_x2 = deg(P) + 2*deg(Q).
// Closes when 2*den_deg > effective_x2.
// ---------------------------------------------------------------------------

#[test]
fn phase53_sqrt_k_times_k_over_k_cubed_closes() {
    // Numerator = Sqrt(k)·k: eff deg x2 = 1 + 2*1 = 3.  Denominator = k³: deg 3.
    // 2*3 = 6 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let num_k = apply(sym(MUL), vec![apply(sym("Sqrt"), vec![k.clone()]), k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![apply(sym("Sqrt"), vec![kp1.clone()]), kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase53_sqrt_k_squared_times_k_over_k_cubed_closes() {
    // Numerator = Sqrt(k²)·k: eff deg x2 = 2 + 2*1 = 4.  Denominator = k³: deg 3.
    // 2*3 = 6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k_sq = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_sq = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![apply(sym("Sqrt"), vec![k_sq]), k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![apply(sym("Sqrt"), vec![kp1_sq]), kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase53_sqrt_k_times_k_squared_over_k_cubed_closes() {
    // Numerator = Sqrt(k)·k²: eff deg x2 = 1 + 2*2 = 5.  Denominator = k³: deg 3.
    // 2*3 = 6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k_sq = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_sq = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![apply(sym("Sqrt"), vec![k.clone()]), k_sq]);
    let num_kp1 = apply(sym(MUL), vec![apply(sym("Sqrt"), vec![kp1.clone()]), kp1_sq]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase53_sqrt_k_times_k_squared_over_k_squared_stays() {
    // Numerator = Sqrt(k)·k²: eff deg x2 = 1 + 2*2 = 5.  Denominator = k²: deg 2.
    // 2*2 = 4 NOT > 5 → stays unevaluated.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k_sq = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_sq = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![apply(sym("Sqrt"), vec![k.clone()]), k_sq.clone()]);
    let num_kp1 = apply(sym(MUL), vec![apply(sym("Sqrt"), vec![kp1.clone()]), kp1_sq.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k_sq]),
        apply(sym(DIV), vec![num_kp1, kp1_sq]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase53_regression_sqrt_k_over_k_squared_still_closes_via_phase51() {
    // Plain Sqrt(k)/k² — Phase 53 requires Mul; Phase 51 handles this.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![apply(sym("Sqrt"), vec![k.clone()]), apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![apply(sym("Sqrt"), vec![kp1.clone()]), apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 54 — Log×polynomial numerator (Rust port).
// ---------------------------------------------------------------------------
// log(h(k))·P(k)/Q(k) vanishes when deg(Q) > deg(P) (strictly).
// log grows sub-polynomially so its effective growth degree equals deg(P).
// ---------------------------------------------------------------------------

#[test]
fn phase54_log_k_times_k_over_k_cubed_closes() {
    // log(k)·k / k³: poly_deg=1, den_deg=3.  3 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k, k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase54_log_k_times_k_squared_over_k_cubed_closes() {
    // log(k)·k² / k³: poly_deg=2, den_deg=3.  3 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let k_sq = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_sq = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![log_k, k_sq]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1, kp1_sq]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase54_log_k_times_k_over_k_squared_closes() {
    // log(k)·k / k²: poly_deg=1, den_deg=2.  2 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k, k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase54_log_k_times_k_squared_over_k_squared_stays() {
    // log(k)·k² / k² = log(k) → diverges.  Equal degrees must be refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let k_sq = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_sq = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![log_k, k_sq.clone()]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1, kp1_sq.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k_sq]),
        apply(sym(DIV), vec![num_kp1, kp1_sq]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase54_regression_log_k_over_k_cubed_still_closes_via_phase50() {
    // Plain Log(k)/k³ — Phase 54 requires Mul; Phase 50 handles this.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![apply(sym("Log"), vec![k.clone()]), apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![apply(sym("Log"), vec![kp1.clone()]), apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 55 — Bounded×Log(diverging) numerator (Rust port).
// ---------------------------------------------------------------------------
// sin(k)·log(k)/Q(k) vanishes at infinity when Q(k) diverges.
// The numerator grows sub-polynomially (bounded × log); any polynomial
// or faster-growing denominator dominates.
// ---------------------------------------------------------------------------

#[test]
fn phase55_sin_k_times_log_k_over_k_squared_closes() {
    // sin(k)·log(k) / k²: bounded×log / poly-2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase55_cos_k_times_log_k_over_k_closes() {
    // cos(k)·log(k) / k: bounded×log / poly-1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let cos_k = apply(sym("Cos"), vec![k.clone()]);
    let cos_kp1 = apply(sym("Cos"), vec![kp1.clone()]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![cos_k, log_k]);
    let num_kp1 = apply(sym(MUL), vec![cos_kp1, log_kp1]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k.clone()]),
        apply(sym(DIV), vec![num_kp1, kp1]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase55_sin_cos_times_log_over_k_cubed_closes() {
    // sin(k)·cos(k)·log(k) / k³: two bounded factors × log / poly-3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let cos_k = apply(sym("Cos"), vec![k.clone()]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let cos_kp1 = apply(sym("Cos"), vec![kp1.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, cos_k, log_k]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, cos_kp1, log_kp1]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase55_sin_times_log_k_squared_over_k_cubed_closes() {
    // sin(k)·log(k²) / k³: log of k² diverges (positive-degree poly inner).
    // After k→k+1 substitution: log((k+1)²) matches structurally. → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k_sq = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_sq = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_ksq = apply(sym("Log"), vec![k_sq]);
    let log_kp1_sq = apply(sym("Log"), vec![kp1_sq]);
    let num_k = apply(sym(MUL), vec![sin_k, log_ksq]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1_sq]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase55_bounded_times_log_constant_denominator_stays() {
    // sin(k)·log(k) / 1: constant denominator does not diverge.
    // h_diverges_at_infinity(1) = false → Phase 55 refuses.
    // No other phase closes this. Must stay unevaluated.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, int(1)]),
        apply(sym(DIV), vec![num_kp1, int(1)]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 56 (Rust port): bounded × Sqrt(diverging) numerator.
// ---------------------------------------------------------------------------

#[test]
fn phase56_sin_times_sqrt_k_over_k_squared_closes() {
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, sqrt_k]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, sqrt_kp1]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase56_sin_times_sqrt_k_cubed_over_exponential_closes() {
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![apply(sym(POW), vec![k.clone(), int(3)])]);
    let num_k = apply(sym(MUL), vec![sin_k, sqrt_k3]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![apply(sym(POW), vec![kp1.clone(), int(3)])]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, sqrt_kp1_3]);
    // Denominator is 2^k — exponential, dominates polynomial of any degree.
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![int(2), k.clone()])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![int(2), kp1])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase56_sin_times_sqrt_k_cubed_over_k_refused() {
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![apply(sym(POW), vec![k.clone(), int(3)])]);
    let num_k = apply(sym(MUL), vec![sin_k, sqrt_k3]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![apply(sym(POW), vec![kp1.clone(), int(3)])]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, sqrt_kp1_3]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k.clone()]),
        apply(sym(DIV), vec![num_kp1, kp1]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    // 3/2 > 1 → does not vanish.
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// Phase 57 (Rust port): bounded × Log(diverging) × Sqrt(positive-poly) numerator.
// ---------------------------------------------------------------------------

#[test]
fn phase57_sin_log_k_sqrt_k_over_k_squared_closes() {
    // sin(k)·log(k)·sqrt(k)/k²: effective growth k^½·log(k), dominated by
    // k² (2*den_deg=4 > sqrt_inner_deg=1, the ×2 value of half-degree ½).
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k, sqrt_k]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1, sqrt_kp1]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase57_sin_log_k_sqrt_k_cubed_over_exponential_closes() {
    // sin(k)·log(k)·sqrt(k³) / 2^k: exponential denominator dominates
    // any sub-polynomial growth automatically.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![apply(sym(POW), vec![k.clone(), int(3)])]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k, sqrt_k3]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![apply(sym(POW), vec![kp1.clone(), int(3)])]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1, sqrt_kp1_3]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![int(2), k.clone()])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![int(2), kp1])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase57_sin_log_k_sqrt_k_cubed_over_k_refused() {
    // sin(k)·log(k)·sqrt(k³)/k: sqrt inner deg (×2) = 3, 2*den_deg=2 ≤ 3
    // → does NOT vanish.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![apply(sym(POW), vec![k.clone(), int(3)])]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k, sqrt_k3]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![apply(sym(POW), vec![kp1.clone(), int(3)])]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1, sqrt_kp1_3]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k.clone()]),
        apply(sym(DIV), vec![num_kp1, kp1]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    // 3/2 > 1 → does not vanish.
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// Phase 58 (Rust port): bounded × Log(diverging) × polynomial numerator.
// ---------------------------------------------------------------------------

#[test]
fn phase58_sin_log_k_times_k_over_k_cubed_closes() {
    // sin(k)·log(k)·k / k³: poly_deg=1, den_deg=3, 3>1 → vanishes.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k, k.clone()]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase58_sin_log_k_times_k_sq_over_exponential_closes() {
    // sin(k)·log(k)·k² / 2^k: exponential denominator dominates.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k, k2]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![int(2), k.clone()])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![int(2), kp1])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase58_sin_log_k_times_k_sq_over_k_sq_refused() {
    // sin(k)·log(k)·k² / k²: equal degrees → log(k)·C diverges, refused.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let log_k = apply(sym("Log"), vec![k.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k, k2.clone()]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_kp1 = apply(sym("Log"), vec![kp1.clone()]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1, kp1_2.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k2]),
        apply(sym(DIV), vec![num_kp1, kp1_2]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 59: Mul(bounded, Sqrt(positive-poly), polynomial) ─────────────────
// Effective_x2 = sqrt_inner_deg + 2·poly_deg.  Vanishes when
// 2·den_deg > effective_x2 or non-polynomial diverging denominator.
// Closes the gap between Phase 53 (Sqrt×poly, refuses bounded) and
// Phase 56 (bounded×Sqrt, refuses poly).

#[test]
fn phase59_sin_sqrt_k_times_k_over_k_cubed_closes() {
    // sin(k)·√k·k / k³: x2=1+2=3; 2·3=6 > 3 → closes.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, sqrt_k, k.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, sqrt_kp1, kp1.clone()]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k3]),
        apply(sym(DIV), vec![num_kp1, kp1_3]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase59_sin_sqrt_k_times_k_sq_over_exponential_closes() {
    // sin(k)·√k·k² / 2^k: non-polynomial diverging denominator.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![sin_k, sqrt_k, k2]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, sqrt_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![int(2), k.clone()])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![int(2), kp1])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase59_sin_sqrt_k_sq_times_k_over_k_sq_refused() {
    // sin(k)·√(k²)·k / k²: x2=2+2=4; 2·2=4 not > 4 → equal, refused.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k2.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, sqrt_k2, k.clone()]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, sqrt_kp1_2, kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k2]),
        apply(sym(DIV), vec![num_kp1, kp1_2]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 60: Mul(bounded, Log(diverging), Sqrt(positive-poly), polynomial) ──
// effective_x2 = sqrt_inner_deg + 2·poly_deg.  Vanishes when
// 2·den_deg > effective_x2 or non-polynomial diverging denominator.
// Closes the gap left by Phase 57 (bounded×Log×Sqrt, refuses polynomial
// factors).

#[test]
fn phase60_sin_log_sqrt_k_times_k_over_k_cubed_closes() {
    // sin(k)·log(k)·√k·k / k³: x2=1+2=3; 2·3=6 > 3 → closes.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k, sqrt_k, k.clone()]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1, sqrt_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(3)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(3)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase60_sin_log_sqrt_k_times_k_sq_over_exponential_closes() {
    // sin(k)·log(k)·√k·k² / 2^k: non-polynomial diverging denominator.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k, sqrt_k, k2]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1, sqrt_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![int(2), k.clone()])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![int(2), kp1])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase60_sin_log_sqrt_k_sq_times_k_over_k_sq_refused() {
    // sin(k)·log(k)·√(k²)·k / k²: x2=2+2=4; 2·2=4 not > 4 → equal, refused.
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k2.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, log_k, sqrt_k2, k.clone()]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, log_kp1, sqrt_kp1_2, kp1.clone()]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k2]),
        apply(sym(DIV), vec![num_kp1, kp1_2]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 61: Mul(Sqrt(P1), Sqrt(P2), polynomial..., bounded...) ──────────────
// effective_x2 = deg(P1) + deg(P2) + 2·poly_deg.
// Vanishes when 2·den_deg > effective_x2 or non-polynomial diverging denom.
// Closes the gap where all prior Sqrt phases require exactly one Sqrt.

#[test]
fn phase61_sqrt_k_times_sqrt_k3_over_k3_closes() {
    // √k · √(k³) / k³: x2=1+3=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k, sqrt_k3]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1, sqrt_kp1_3]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k3]),
        apply(sym(DIV), vec![num_kp1, kp1_3]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase61_sin_sqrt_k_times_sqrt_k_over_k_sq_closes() {
    // sin(k) · √k · √k / k²: x2=1+1=2; 2·2=4 > 2 → closes (bounded + two Sqrt).
    let k = sym("k");
    let sin_k = apply(sym("Sin"), vec![k.clone()]);
    let sqrt_k1 = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k.clone()]);
    let num_k = apply(sym(MUL), vec![sin_k, sqrt_k1, sqrt_k2]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sin_kp1 = apply(sym("Sin"), vec![kp1.clone()]);
    let sqrt_kp1_1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sin_kp1, sqrt_kp1_1, sqrt_kp1_2]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, apply(sym(POW), vec![k.clone(), int(2)])]),
        apply(sym(DIV), vec![num_kp1, apply(sym(POW), vec![kp1, int(2)])]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase61_sqrt_k2_times_sqrt_k2_over_k2_refused() {
    // √(k²) · √(k²) / k²: x2=2+2=4; 2·2=4 not > 4 → equal, refused.
    let k = sym("k");
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let sqrt_k2_1 = apply(sym("Sqrt"), vec![k2.clone()]);
    let sqrt_k2_2 = apply(sym("Sqrt"), vec![k2.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k2_1, sqrt_k2_2]);
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_kp1_2_1 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let sqrt_kp1_2_2 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_2_1, sqrt_kp1_2_2]);
    let f = apply(sym(SUB), vec![
        apply(sym(DIV), vec![num_k, k2]),
        apply(sym(DIV), vec![num_kp1, kp1_2]),
    ]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 62: Mul(Log(diverging), Log(diverging), polynomial..., bounded...) ─
// effective_x2 = 2 * poly_deg.  log²(k) grows sub-polynomially (o(k^ε) for
// any ε > 0).  Vanishes when 2·den_deg > effective_x2 or non-polynomial
// diverging denominator.  Sqrt factors are refused.

#[test]
fn phase62_log_k_sq_over_k_sq_closes() {
    // log(k)·log(k) / k²: poly_deg=0, effective_x2=0; 2·2=4 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1.clone(), log_kp1]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase62_log_k_sq_times_k_over_k_cubed_closes() {
    // log(k)²·k / k³: poly_deg=1, effective_x2=2; 2·3=6 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k.clone(), log_k, k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1.clone(), log_kp1, kp1.clone()]);
    let den_k = apply(sym(POW), vec![k.clone(), int(3)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase62_log_k_sq_times_k_sq_over_k_sq_refused() {
    // log(k)²·k² / k²: poly_deg=2, effective_x2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![log_k.clone(), log_k, k2.clone()]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1.clone(), log_kp1, kp1_2.clone()]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 63: Mul(Sqrt(P1), Sqrt(P2), Log(diverging), polynomial..., bounded...) ─
// effective_x2 = deg(P1) + deg(P2) + 2*poly_deg.  Log is sub-polynomial.

#[test]
fn phase63_sqrt_k_sq_log_k_over_k_sq_closes() {
    // √k · √k · log(k) / k²: effective_x2=1+1=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k1 = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let sqrt_kp1_1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k1, sqrt_k2, log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_1, sqrt_kp1_2, log_kp1]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase63_sqrt_k3_sqrt_k_log_k_over_k3_closes() {
    // √(k³) · √k · log(k) / k³: effective_x2=3+1=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k3, sqrt_k, log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_3, sqrt_kp1, log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase63_sqrt_k2_sq_log_k_over_k2_refused() {
    // √(k²) · √(k²) · log(k) / k²: effective_x2=2+2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k2_1 = apply(sym("Sqrt"), vec![k2.clone()]);
    let sqrt_k2_2 = apply(sym("Sqrt"), vec![k2.clone()]);
    let sqrt_kp1_2_1 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let sqrt_kp1_2_2 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k2_1, sqrt_k2_2, log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_2_1, sqrt_kp1_2_2, log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 64: Mul(Log(diverging), Log(diverging), Sqrt(P), polynomial..., bounded...) ─

#[test]
fn phase64_log_k_sq_sqrt_k_over_k_sq_closes() {
    // log(k)²·√k / k²: effective_x2=1; 2·2=4 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k1 = apply(sym(LOG), vec![k.clone()]);
    let log_k2 = apply(sym(LOG), vec![k.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let log_kp1_1 = apply(sym(LOG), vec![kp1.clone()]);
    let log_kp1_2 = apply(sym(LOG), vec![kp1.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k1, log_k2, sqrt_k]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1_1, log_kp1_2, sqrt_kp1]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase64_log_k_sq_sqrt_k3_over_k3_closes() {
    // log(k)²·√(k³) / k³: effective_x2=3; 2·3=6 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let log_k1 = apply(sym(LOG), vec![k.clone()]);
    let log_k2 = apply(sym(LOG), vec![k.clone()]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let log_kp1_1 = apply(sym(LOG), vec![kp1.clone()]);
    let log_kp1_2 = apply(sym(LOG), vec![kp1.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let num_k = apply(sym(MUL), vec![log_k1, log_k2, sqrt_k3]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1_1, log_kp1_2, sqrt_kp1_3]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase64_log_k_sq_sqrt_k2_over_k_refused() {
    // log(k)²·√(k²) / k: effective_x2=2; 2·1=2 not > 2 → refused (equal).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let log_k1 = apply(sym(LOG), vec![k.clone()]);
    let log_k2 = apply(sym(LOG), vec![k.clone()]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k2]);
    let log_kp1_1 = apply(sym(LOG), vec![kp1.clone()]);
    let log_kp1_2 = apply(sym(LOG), vec![kp1.clone()]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1_2]);
    let num_k = apply(sym(MUL), vec![log_k1, log_k2, sqrt_k2]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1_1, log_kp1_2, sqrt_kp1_2]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 65: Two Sqrts × Two Logs × polynomial ──────────────────────────────

#[test]
fn phase65_sqrt_k_sq_log_k_sq_over_k_sq_closes() {
    // √k·√k·log(k)²/k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k1 = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k.clone()]);
    let log_k1 = apply(sym(LOG), vec![k.clone()]);
    let log_k2 = apply(sym(LOG), vec![k.clone()]);
    let sqrt_kp1_1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_kp1_1 = apply(sym(LOG), vec![kp1.clone()]);
    let log_kp1_2 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k1, sqrt_k2, log_k1, log_k2]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_1, sqrt_kp1_2, log_kp1_1, log_kp1_2]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase65_sqrt_k3_sqrt_k_log_k_sq_over_k3_closes() {
    // √(k³)·√k·log(k)²/k³: effective_x2=3+1=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k1 = apply(sym(LOG), vec![k.clone()]);
    let log_k2 = apply(sym(LOG), vec![k.clone()]);
    let log_kp1_1 = apply(sym(LOG), vec![kp1.clone()]);
    let log_kp1_2 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k3, sqrt_k, log_k1, log_k2]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_3, sqrt_kp1, log_kp1_1, log_kp1_2]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase65_sqrt_k2_sq_log_k_sq_over_k2_refused() {
    // √(k²)·√(k²)·log(k)²/k²: effective_x2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k2_1 = apply(sym("Sqrt"), vec![k2.clone()]);
    let sqrt_k2_2 = apply(sym("Sqrt"), vec![k2.clone()]);
    let sqrt_kp1_2_1 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let sqrt_kp1_2_2 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let log_k1 = apply(sym(LOG), vec![k.clone()]);
    let log_k2 = apply(sym(LOG), vec![k.clone()]);
    let log_kp1_1 = apply(sym(LOG), vec![kp1.clone()]);
    let log_kp1_2 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k2_1, sqrt_k2_2, log_k1, log_k2]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_2_1, sqrt_kp1_2_2, log_kp1_1, log_kp1_2]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 66: Three Sqrts × polynomial ───────────────────────────────────────

#[test]
fn phase66_sqrt_k_cubed_over_k_sq_closes() {
    // √k·√k·√k/k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k1 = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1_1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k1, sqrt_k2, sqrt_k3]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_1, sqrt_kp1_2, sqrt_kp1_3]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase66_sqrt_k3_sqrt_k_sqrt_k_over_k3_closes() {
    // √(k³)·√k·√k/k³: effective_x2=3+1+1=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let sqrt_k1 = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1_1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k3, sqrt_k1, sqrt_k2]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_3, sqrt_kp1_1, sqrt_kp1_2]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase66_sqrt_k2_cubed_over_k_sq_refused() {
    // √(k²)·√(k²)·√(k²)/k²: effective_x2=2+2+2=6; 2·2=4 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k2_1 = apply(sym("Sqrt"), vec![k2.clone()]);
    let sqrt_k2_2 = apply(sym("Sqrt"), vec![k2.clone()]);
    let sqrt_k2_3 = apply(sym("Sqrt"), vec![k2.clone()]);
    let sqrt_kp1_2_1 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let sqrt_kp1_2_2 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let sqrt_kp1_2_3 = apply(sym("Sqrt"), vec![kp1_2.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k2_1, sqrt_k2_2, sqrt_k2_3]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_2_1, sqrt_kp1_2_2, sqrt_kp1_2_3]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 67: Three Logs × polynomial ────────────────────────────────────────

#[test]
fn phase67_log_k_cubed_over_k_closes() {
    // log(k)³ / k: poly_deg=0, effective_x2=0; 2·1=2 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase67_log_k_cubed_poly_over_k_sq_closes() {
    // log(k)³·k / k²: poly_deg=1, effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k.clone(), log_k.clone(), log_k, k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1.clone(), log_kp1.clone(), log_kp1, kp1.clone()]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase67_log_k_cubed_over_k_refused_when_equal() {
    // log(k)³·k / k: poly_deg=1, effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k.clone(), log_k.clone(), log_k, k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1.clone(), log_kp1.clone(), log_kp1, kp1.clone()]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 68 — Three-Sqrt × Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase68_sqrt_k_cubed_log_k_over_k2_closes() {
    // √k·√k·√k·log(k) / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k.clone(), sqrt_k.clone(), sqrt_k, log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1, log_kp1]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase68_three_sqrt_higher_log_k_over_k3_closes() {
    // √(k³)·√k·√k·log(k) / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let sqrt_k1 = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let sqrt_kp1_1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k3, sqrt_k1, sqrt_k2, log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_3, sqrt_kp1_1, sqrt_kp1_2, log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase68_three_sqrt_log_over_k_refused() {
    // √k·√k·√k·log(k) / k: effective_x2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k.clone(), sqrt_k.clone(), sqrt_k, log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1, log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 69 — One-Sqrt × Three-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase69_sqrt_k_log3_k_over_k2_closes() {
    // √k·log(k)·log(k)·log(k) / k²: effective_x2=1; 2·2=4 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k, log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase69_sqrt_k3_log3_k_over_k3_closes() {
    // √(k³)·log(k)³ / k³: effective_x2=3; 2·3=6 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k3, log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_3, log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase69_sqrt_k_log3_k_over_const_refused() {
    // √k·log(k)³ / 1: effective_x2=1; 2·0=0 not > 1 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k, log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, int(1)]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, int(1)]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// Phase 70 — Three-Sqrt × Two-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase70_sqrt_k_cubed_log2_k_over_k2_closes() {
    // √k·√k·√k·log(k)²/k²: effective_x2=1+1+1=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k.clone(), sqrt_k.clone(), sqrt_k, log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1, log_kp1.clone(), log_kp1]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase70_sqrt_k3_sqrt_k_sqrt_k_log2_k_over_k3_closes() {
    // √(k³)·√k·√k·log(k)²/k³: effective_x2=3+1+1=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k3, sqrt_k.clone(), sqrt_k, log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_3, sqrt_kp1.clone(), sqrt_kp1, log_kp1.clone(), log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase70_sqrt_k_cubed_log2_k_over_k_refused() {
    // √k·√k·√k·log(k)²/k: effective_x2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k.clone(), sqrt_k.clone(), sqrt_k, log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1, log_kp1.clone(), log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// Phase 71 — Two-Sqrt × Three-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase71_sqrt_k_sq_log3_k_over_k2_closes() {
    // √k·√k·log(k)³/k²: effective_x2=1+1=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k.clone(), sqrt_k, log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1.clone(), sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase71_sqrt_k3_sqrt_k_log3_k_over_k3_closes() {
    // √(k³)·√k·log(k)³/k³: effective_x2=3+1=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k3, sqrt_k, log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_3, sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase71_sqrt_k_sq_log3_k_over_k_refused() {
    // √k·√k·log(k)³/k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k.clone(), sqrt_k, log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1.clone(), sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// Phase 72 — Three-Sqrt × Three-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase72_sqrt_k_cubed_log3_k_over_k2_closes() {
    // √k·√k·√k·log(k)³/k²: effective_x2=1+1+1=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k.clone(), sqrt_k.clone(), sqrt_k, log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let den_k = apply(sym(POW), vec![k.clone(), int(2)]);
    let den_kp1 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let g_k = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase72_sqrt_k3_sqrt_k_sq_log3_k_over_k3_closes() {
    // √(k³)·√k·√k·log(k)³/k³: effective_x2=3+1+1=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k3, sqrt_k.clone(), sqrt_k, log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_3, sqrt_kp1.clone(), sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase72_sqrt_k_cubed_log3_k_over_k_refused() {
    // √k·√k·√k·log(k)³/k: effective_x2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k.clone(), sqrt_k.clone(), sqrt_k, log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 73: Four-Log × polynomial numerator ──────────────────────────────
// g(k) = log(k)^4 * poly(k) / den(k); effective_x2 = 2·poly_deg.
// Closes when 2·den_deg > effective_x2 (or denom is non-polynomial diverging).

#[test]
fn phase73_log4_k_over_k_closes() {
    // log(k)^4 / k: effective_x2=0; 2·1=2 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k.clone(), log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase73_log4_k_times_k_over_k2_closes() {
    // log(k)^4·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k.clone(), log_k.clone(), log_k.clone(), log_k, k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, kp1.clone()]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase73_log4_k_times_k_over_k_refused() {
    // log(k)^4·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![log_k.clone(), log_k.clone(), log_k.clone(), log_k, k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, kp1.clone()]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 74: One-Sqrt × Four-Log × polynomial numerator ─────────────────────
// g(k) = √P(k) · log(k)^4 · poly(k) / den(k); effective_x2 = d + 2·poly_deg.
// Closes when 2·den_deg > effective_x2 (or denom is non-polynomial diverging).

#[test]
fn phase74_sqrt_k_log4_k_over_k_closes() {
    // √k·log(k)^4 / k: effective_x2=1; 2·1=2 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k, log_k.clone(), log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k74 = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp174 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k74, g_kp174]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase74_sqrt_k3_log4_k_over_k2_closes() {
    // √(k³)·log(k)^4 / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k3, log_k.clone(), log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_3, log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k74b = apply(sym(DIV), vec![num_k, k2]);
    let g_kp174b = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k74b, g_kp174b]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase74_sqrt_k_log4_k_times_k_over_k_refused() {
    // √k·log(k)^4·k / k: effective_x2=1+2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k, log_k.clone(), log_k.clone(), log_k.clone(), log_k, k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, kp1.clone()]);
    let g_k74r = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp174r = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k74r, g_kp174r]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 75: Two-Sqrt × Four-Log × polynomial numerator ─────────────────────
// g(k) = √P1(k) · √P2(k) · log(k)^4 · poly(k) / den(k).
// effective_x2 = d1 + d2 + 2·poly_deg.
// Closes when 2·den_deg > effective_x2 (or denom is non-polynomial diverging).

#[test]
fn phase75_two_sqrt_k_log4_k_over_k2_closes() {
    // √k·√k·log(k)^4 / k²: effective_x2=1+1=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![sqrt_k.clone(), sqrt_k, log_k.clone(), log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1.clone(), sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k75 = apply(sym(DIV), vec![num_k, k2]);
    let g_kp175 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k75, g_kp175]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase75_two_sqrt_k3_log4_k_over_k2_refused() {
    // √(k³)·√(k³)·log(k)^4 / k²: effective_x2=3+3=6; 2·2=4 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![sqrt_k3.clone(), sqrt_k3, log_k.clone(), log_k.clone(), log_k.clone(), log_k]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1_3.clone(), sqrt_kp1_3, log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1]);
    let g_k75b = apply(sym(DIV), vec![num_k, k2]);
    let g_kp175b = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k75b, g_kp175b]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase75_two_sqrt_k_log4_k_times_k_over_k2_refused() {
    // √k·√k·log(k)^4·k / k²: effective_x2=1+1+2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![sqrt_k.clone(), sqrt_k, log_k.clone(), log_k.clone(), log_k.clone(), log_k, k.clone()]);
    let num_kp1 = apply(sym(MUL), vec![sqrt_kp1.clone(), sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, kp1.clone()]);
    let g_k75r = apply(sym(DIV), vec![num_k, k2]);
    let g_kp175r = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k75r, g_kp175r]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 76: Three-Sqrt × Four-Log × polynomial numerator ──────────────────

#[test]
fn phase76_three_sqrt_k_log4_k_over_k2_closes() {
    // √k·√k·√k·log(k)^4 / k²: effective_x2=1+1+1=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k76 = apply(sym(DIV), vec![num_k, k2]);
    let g_kp176 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k76, g_kp176]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase76_three_sqrt_k3_log4_k_over_k2_refused() {
    // √(k³)·√(k³)·√(k³)·log(k)^4 / k²: effective_x2=3+3+3=9; 2·2=4 not > 9 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3.clone()]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k3.clone(), sqrt_k3.clone(), sqrt_k3,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_3.clone(), sqrt_kp1_3.clone(), sqrt_kp1_3,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k76b = apply(sym(DIV), vec![num_k, k2]);
    let g_kp176b = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k76b, g_kp176b]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase76_three_sqrt_k_log4_k_times_k_over_k2_refused() {
    // √k·√k·√k·log(k)^4·k / k²: effective_x2=1+1+1+2=5; 2·2=4 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k76r = apply(sym(DIV), vec![num_k, k2]);
    let g_kp176r = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k76r, g_kp176r]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 77: Five-Log × polynomial numerator ────────────────────────────────

#[test]
fn phase77_log5_k_over_k2_closes() {
    // log(k)^5 / k²: effective_x2=0; 2·2=4 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f77a = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f77a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase77_log5_k_times_k2_over_k3_closes() {
    // log(k)^5 · k² / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k2,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1_2,
    ]);
    let g_k77b = apply(sym(DIV), vec![num_k, k3]);
    let g_kp177b = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f77b = apply(sym(SUB), vec![g_k77b, g_kp177b]);
    let out = evaluate_sum(f77b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase77_log5_k_times_k3_over_k3_refused() {
    // log(k)^5 · k³ / k³: effective_x2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k3.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1_3.clone(),
    ]);
    let g_k77r = apply(sym(DIV), vec![num_k, k3]);
    let g_kp177r = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f77r = apply(sym(SUB), vec![g_k77r, g_kp177r]);
    let out = evaluate_sum(f77r, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 78: One-Sqrt × Five-Log × polynomial numerator ─────────────────────

#[test]
fn phase78_sqrt_k_log5_k_over_k2_closes() {
    // √k · log(k)^5 / k²: effective_x2=1; 2·2=4 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k, log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f78a = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f78a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase78_sqrt_k3_log5_k_over_k_refused() {
    // √(k³) · log(k)^5 / k: effective_x2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k3, log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_3, log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k78b = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp178b = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f78b = apply(sym(SUB), vec![g_k78b, g_kp178b]);
    let out = evaluate_sum(f78b, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase78_sqrt_k_log5_k_times_k_over_k_refused() {
    // √k · log(k)^5 · k / k: effective_x2=1+2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k, log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1, log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k78r = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp178r = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f78r = apply(sym(SUB), vec![g_k78r, g_kp178r]);
    let out = evaluate_sum(f78r, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 79: Two-Sqrt × Five-Log × polynomial numerator ─────────────────────

#[test]
fn phase79_two_sqrt_k_log5_k_over_k2_closes() {
    // √k·√k·log(k)^5 / k²: effective_x2=1+1=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k79a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp179a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k79a, g_kp179a]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase79_two_sqrt_k3_log5_k_over_k_refused() {
    // √(k³)·√(k³)·log(k)^5 / k: effective_x2=3+3=6; 2·1=2 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k3.clone(), sqrt_k3,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_3.clone(), sqrt_kp1_3,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k79b = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp179b = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k79b, g_kp179b]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase79_two_sqrt_k_log5_k_times_k_over_k2_refused() {
    // √k·√k·log(k)^5·k / k²: effective_x2=1+1+2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k79c = apply(sym(DIV), vec![num_k, k2]);
    let g_kp179c = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k79c, g_kp179c]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 80: Three-Sqrt × Five-Log × polynomial numerator ───────────────────

#[test]
fn phase80_three_sqrt_k_log5_k_over_k2_closes() {
    // √k·√k·√k·log(k)^5 / k²: effective_x2=1+1+1=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k80a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp180a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k80a, g_kp180a]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase80_three_sqrt_k3_log5_k_over_k_refused() {
    // √(k³)·√(k³)·√(k³)·log(k)^5 / k: effective_x2=3+3+3=9; 2·1=2 not > 9 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k3.clone(), sqrt_k3.clone(), sqrt_k3,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_3.clone(), sqrt_kp1_3.clone(), sqrt_kp1_3,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k80b = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp180b = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k80b, g_kp180b]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase80_three_sqrt_k_log5_k_times_k_over_k2_refused() {
    // √k·√k·√k·log(k)^5·k / k²: effective_x2=1+1+1+2=5; 2·2=4 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k80c = apply(sym(DIV), vec![num_k, k2]);
    let g_kp180c = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k80c, g_kp180c]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 81: Four-Sqrt × Five-Log × polynomial numerator ────────────────────

#[test]
fn phase81_four_sqrt_k_log5_k_over_k3_closes() {
    // √k·√k·√k·√k·log(k)^5 / k³: effective_x2=1+1+1+1=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k81a = apply(sym(DIV), vec![num_k, k3]);
    let g_kp181a = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k81a, g_kp181a]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase81_four_sqrt_k3_log5_k_over_k_refused() {
    // √(k³)·√(k³)·√(k³)·√(k³)·log(k)^5 / k: effective_x2=3+3+3+3=12; 2·1=2 not > 12 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k3.clone(), sqrt_k3.clone(), sqrt_k3.clone(), sqrt_k3,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_3.clone(), sqrt_kp1_3.clone(), sqrt_kp1_3.clone(), sqrt_kp1_3,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k81b = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp181b = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k81b, g_kp181b]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase81_four_sqrt_k_log5_k_times_k_over_k3_refused() {
    // √k·√k·√k·√k·log(k)^5·k / k³: effective_x2=1+1+1+1+2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k81c = apply(sym(DIV), vec![num_k, k3]);
    let g_kp181c = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k81c, g_kp181c]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 82: Five-Sqrt × Five-Log × polynomial numerator ────────────────────

#[test]
fn phase82_five_sqrt_k_log5_k_over_k3_closes() {
    // √k·√k·√k·√k·√k·log(k)^5 / k³: effective_x2=1+1+1+1+1=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k82a = apply(sym(DIV), vec![num_k, k3]);
    let g_kp182a = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k82a, g_kp182a]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase82_five_sqrt_k3_log5_k_over_k_refused() {
    // √(k³)·√(k³)·√(k³)·√(k³)·√(k³)·log(k)^5 / k: effective_x2=3+3+3+3+3=15; 2·1=2 not > 15 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k3.clone(), sqrt_k3.clone(), sqrt_k3.clone(), sqrt_k3.clone(), sqrt_k3,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_3.clone(), sqrt_kp1_3.clone(), sqrt_kp1_3.clone(), sqrt_kp1_3.clone(), sqrt_kp1_3,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k82b = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp182b = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k82b, g_kp182b]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase82_five_sqrt_k_log5_k_times_k_over_k4_closes() {
    // √k·√k·√k·√k·√k·log(k)^5·k / k⁴: effective_x2=1+1+1+1+1+2=7; 2·4=8 > 7 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k4 = apply(sym(POW), vec![k.clone(), int(4)]);
    let kp1_4 = apply(sym(POW), vec![kp1.clone(), int(4)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k82c = apply(sym(DIV), vec![num_k, k4]);
    let g_kp182c = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k82c, g_kp182c]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 83: Six-Log × polynomial numerator ──────────────────────────────────

#[test]
fn phase83_log6_k_over_k2_closes() {
    // log(k)^6 / k²: effective_x2=0; 2·2=4 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f83a = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f83a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase83_log6_k_times_k2_over_k3_closes() {
    // log(k)^6 · k² / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k2,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1_2,
    ]);
    let g_k83b = apply(sym(DIV), vec![num_k, k3]);
    let g_kp183b = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f83b = apply(sym(SUB), vec![g_k83b, g_kp183b]);
    let out = evaluate_sum(f83b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase83_log6_k_times_k3_over_k3_refused() {
    // log(k)^6 · k³ / k³: effective_x2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k3.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1_3.clone(),
    ]);
    let g_k83r = apply(sym(DIV), vec![num_k, k3]);
    let g_kp183r = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f83r = apply(sym(SUB), vec![g_k83r, g_kp183r]);
    let out = evaluate_sum(f83r, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 84: One-Sqrt × Six-Log × polynomial ─────────────────────────────

#[test]
fn phase84_sqrt_k_log6_k_over_k2_closes() {
    // √k·log(k)^6 / k²: effective_x2=1; 2·2=4 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase84_sqrt_k_log6_k_times_k_over_k2_closes() {
    // √k·log(k)^6·k / k²: effective_x2=1+2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase84_sqrt_k_log6_k_times_k_over_k_refused() {
    // √k·log(k)^6·k / k: effective_x2=3; 2·1=2 not > 3 → refused (diverges).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase89_log7_k_over_k2_closes() {
    // log(k)^7 / k²: effective_x2=0; 2·2=4 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase89_log7_k_times_k2_over_k3_closes() {
    // log(k)^7·k² / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k2,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1_2,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase89_log7_k_times_k3_over_k3_refused() {
    // log(k)^7·k³ / k³: effective_x2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k3.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1_3.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase90_sqrt_k_log7_k_over_k2_closes() {
    // √k·log(k)^7 / k²: effective_x2=1; 2·2=4 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase90_sqrt_k_log7_k_times_k_over_k2_closes() {
    // √k·log(k)^7·k / k²: effective_x2=1+2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase90_sqrt_k_log7_k_times_k_over_k_refused() {
    // √k·log(k)^7·k / k: effective_x2=3; 2·1=2 not > 3 → refused (diverges).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase91_sqrt_k_sqrt_k_log7_k_over_k2_closes() {
    // √k·√k·log(k)^7 / k²: effective_x2=1+1=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase91_sqrt_k3_sqrt_k3_log7_k_over_k4_closes() {
    // √(k³)·√(k³)·log(k)^7 / k⁴: effective_x2=3+3=6; 2·4=8 > 6 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k4 = apply(sym(POW), vec![k.clone(), int(4)]);
    let kp1_4 = apply(sym(POW), vec![kp1.clone(), int(4)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k3.clone(), sqrt_k3,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_3.clone(), sqrt_kp1_3,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase91_sqrt_k_sqrt_k_log7_k_times_k_over_k2_refused() {
    // √k·√k·log(k)^7·k / k²: effective_x2=1+1+2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase92_sqrt_k_sqrt_k_sqrt_k_log7_k_over_k2_closes() {
    // √k·√k·√k·log(k)^7 / k²: effective_x2=1+1+1=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase92_sqrt_k3_sqrt_k3_sqrt_k3_log7_k_over_k5_closes() {
    // √(k³)·√(k³)·√(k³)·log(k)^7 / k⁵: effective_x2=3+3+3=9; 2·5=10 > 9 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let sqrt_k3 = apply(sym("Sqrt"), vec![k3]);
    let sqrt_kp1_3 = apply(sym("Sqrt"), vec![kp1_3]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k5 = apply(sym(POW), vec![k.clone(), int(5)]);
    let kp1_5 = apply(sym(POW), vec![kp1.clone(), int(5)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k3.clone(), sqrt_k3.clone(), sqrt_k3,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_3.clone(), sqrt_kp1_3.clone(), sqrt_kp1_3,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k5]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_5]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase92_sqrt_k_sqrt_k_sqrt_k_log7_k_times_k_over_k2_refused() {
    // √k·√k·√k·log(k)^7·k / k²: effective_x2=1+1+1+2=5; 2·2=4 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase93_sqrt_k_x4_log7_k_over_k3_closes() {
    // √k×4·log(k)^7 / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase93_sqrt_k2_x4_log7_k_over_k5_closes() {
    // √(k²)×4·log(k)^7 / k⁵: effective_x2=8; 2·5=10 > 8 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k2]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1_2]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k5 = apply(sym(POW), vec![k.clone(), int(5)]);
    let kp1_5 = apply(sym(POW), vec![kp1.clone(), int(5)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k5]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_5]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase93_sqrt_k_x4_log7_k_times_k_over_k3_refused() {
    // √k×4·log(k)^7·k / k³: effective_x2=4+2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 94: Five-Sqrt × Seven-Log × polynomial numerator ──────────────────

#[test]
fn phase94_sqrt_k_x5_log7_k_over_k4_closes() {
    // √k×5·log(k)^7 / k⁴: effective_x2=5; 2·4=8 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k4 = apply(sym(POW), vec![k.clone(), int(4)]);
    let kp1_4 = apply(sym(POW), vec![kp1.clone(), int(4)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase94_sqrt_k2_x5_log7_k_over_k6_closes() {
    // √(k²)×5·log(k)^7 / k⁶: effective_x2=10; 2·6=12 > 10 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k2]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1_2]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k6 = apply(sym(POW), vec![k.clone(), int(6)]);
    let kp1_6 = apply(sym(POW), vec![kp1.clone(), int(6)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k6]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_6]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase94_sqrt_k_x5_log7_k_times_k_over_k3_refused() {
    // √k×5·log(k)^7·k / k³: effective_x2=5+2=7; 2·3=6 not > 7 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 95: Eight-Log × polynomial numerator (zero Sqrt) ──────────────────

#[test]
fn phase95_log8_k_over_k2_closes() {
    // log(k)^8 / k²: effective_x2=0; 2·2=4 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase95_log8_k_times_k_over_k3_closes() {
    // log(k)^8·k / k³: effective_x2=2; 2·3=6 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase95_log8_k_times_k_over_k_refused() {
    // log(k)^8·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 96: One-Sqrt × Eight-Log × polynomial numerator ───────────────────

#[test]
fn phase96_sqrt_k_log8_k_over_k2_closes() {
    // √k·log(k)^8 / k²: effective_x2=1; 2·2=4 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase96_sqrt_k2_log8_k_over_k3_closes() {
    // √(k²)·log(k)^8 / k³: effective_x2=2; 2·3=6 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k2]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1_2]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k2,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_2,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase96_sqrt_k_log8_k_times_k_over_k_refused() {
    // √k·log(k)^8·k / k: effective_x2=1+2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 97: Two-Sqrt × Eight-Log × polynomial numerator ───────────────────

#[test]
fn phase97_sqrt_k_x2_log8_k_over_k2_closes() {
    // √k×2·log(k)^8 / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase97_sqrt_k2_x2_log8_k_over_k3_closes() {
    // √(k²)×2·log(k)^8 / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k2]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1_2]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k2.clone(), sqrt_k2,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_2.clone(), sqrt_kp1_2,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase97_sqrt_k_x2_log8_k_times_k_over_k_refused() {
    // √k×2·log(k)^8·k / k: effective_x2=2+2=4; 2·1=2 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 98: Three-Sqrt × Eight-Log × polynomial numerator ──────────────────

#[test]
fn phase98_sqrt_k_x3_log8_k_over_k2_closes() {
    // √k×3·log(k)^8 / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase98_sqrt_k2_x3_log8_k_over_k4_closes() {
    // √(k²)×3·log(k)^8 / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k2]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1_2]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k4 = apply(sym(POW), vec![k.clone(), int(4)]);
    let kp1_4 = apply(sym(POW), vec![kp1.clone(), int(4)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase98_sqrt_k_x3_log8_k_times_k_over_k_refused() {
    // √k×3·log(k)^8·k / k: effective_x2=3+2=5; 2·1=2 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 99: Four-Sqrt × Eight-Log × polynomial numerator ───────────────────

#[test]
fn phase99_sqrt_k_x4_log8_k_over_k3_closes() {
    // √k×4·log(k)^8 / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase99_sqrt_k2_x4_log8_k_over_k5_closes() {
    // √(k²)×4·log(k)^8 / k⁵: effective_x2=8; 2·5=10 > 8 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k2]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1_2]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k5 = apply(sym(POW), vec![k.clone(), int(5)]);
    let kp1_5 = apply(sym(POW), vec![kp1.clone(), int(5)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k5]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_5]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase99_sqrt_k_x4_log8_k_times_k_over_k_refused() {
    // √k×4·log(k)^8·k / k: effective_x2=4+2=6; 2·1=2 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 100: Five-Sqrt × Eight-Log × polynomial numerator (completes Eight-Log family) ─

#[test]
fn phase100_sqrt_k_x5_log8_k_over_k3_closes() {
    // √k×5·log(k)^8 / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k3 = apply(sym(POW), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym(POW), vec![kp1.clone(), int(3)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase100_sqrt_k2_x5_log8_k_over_k6_closes() {
    // √(k²)×5·log(k)^8 / k⁶: effective_x2=10; 2·6=12 > 10 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(POW), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym(POW), vec![kp1.clone(), int(2)]);
    let sqrt_k2 = apply(sym("Sqrt"), vec![k2]);
    let sqrt_kp1_2 = apply(sym("Sqrt"), vec![kp1_2]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let k6 = apply(sym(POW), vec![k.clone(), int(6)]);
    let kp1_6 = apply(sym(POW), vec![kp1.clone(), int(6)]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2.clone(), sqrt_k2,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2.clone(), sqrt_kp1_2,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k6]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_6]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase100_sqrt_k_x5_log8_k_times_k_over_k_refused() {
    // √k×5·log(k)^8·k / k: effective_x2=5+2=7; 2·1=2 not > 7 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 101: Nine-Log × polynomial ──────────────────────────────────────────

#[test]
fn phase101_log9_k_over_k_closes() {
    // log(k)^9 / k: effective_x2=0; 2·1=2 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase101_log9_k_times_k_over_k2_closes() {
    // log(k)^9·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase101_log9_k_times_k_over_k_refused() {
    // log(k)^9·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 102: One-Sqrt × Nine-Log × polynomial ───────────────────────────────

#[test]
fn phase102_sqrt_k_log9_k_over_k_closes() {
    // √k·log(k)^9 / k: effective_x2=1; 2·1=2 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase102_sqrt_k_log9_k_times_k_over_k2_closes() {
    // √k·log(k)^9·k / k²: effective_x2=1+2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase102_sqrt_k_log9_k_times_k_over_k_refused() {
    // √k·log(k)^9·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 103: Two-Sqrt × Nine-Log × polynomial ───────────────────────────────

#[test]
fn phase103_sqrt_k_x2_log9_k_over_k2_closes() {
    // √k×2·log(k)^9 / k²: effective_x2=1+1=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase103_sqrt_k_x2_log9_k_times_k_over_k3_closes() {
    // √k×2·log(k)^9·k / k³: effective_x2=1+1+2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase103_sqrt_k_x2_log9_k_times_k_over_k_refused() {
    // √k×2·log(k)^9·k / k: effective_x2=4; 2·1=2 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 104: Three-Sqrt × Nine-Log × polynomial ─────────────────────────────

#[test]
fn phase104_sqrt_k_x3_log9_k_over_k2_closes() {
    // √k×3·log(k)^9 / k²: effective_x2=1+1+1=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase104_sqrt_k_x3_log9_k_times_k_over_k3_closes() {
    // √k×3·log(k)^9·k / k³: effective_x2=3+2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase104_sqrt_k_x3_log9_k_times_k_over_k2_refused() {
    // √k×3·log(k)^9·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 105: Four-Sqrt × Nine-Log × polynomial ──────────────────────────────

#[test]
fn phase105_sqrt_k_x4_log9_k_over_k3_closes() {
    // √k×4·log(k)^9 / k³: effective_x2=1+1+1+1=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase105_sqrt_k_x4_log9_k_times_k_over_k4_closes() {
    // √k×4·log(k)^9·k / k⁴: effective_x2=4+2=6; 2·4=8 > 6 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])])]);
    let kp1_4 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase105_sqrt_k_x4_log9_k_times_k_over_k3_refused() {
    // √k×4·log(k)^9·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 106: Five-Sqrt × Nine-Log × polynomial (completes Nine-Log family) ──

#[test]
fn phase106_sqrt_k_x5_log9_k_over_k3_closes() {
    // √k×5·log(k)^9 / k³: effective_x2=1+1+1+1+1=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase106_sqrt_k_x5_log9_k_times_k_over_k4_closes() {
    // √k×5·log(k)^9·k / k⁴: effective_x2=5+2=7; 2·4=8 > 7 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])])]);
    let kp1_4 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase106_sqrt_k_x5_log9_k_times_k_over_k3_refused() {
    // √k×5·log(k)^9·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 107: Ten-Log × polynomial ───────────────────────────────────────────

#[test]
fn phase107_log10_k_over_k_closes() {
    // log(k)^10 / k: effective_x2=0; 2·1=2 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase107_log10_k_times_k_over_k2_closes() {
    // log(k)^10·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase107_log10_k_times_k_over_k_refused() {
    // log(k)^10·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 108: One-Sqrt × Ten-Log × polynomial ─────────────────────────────────

#[test]
fn phase108_sqrt_k_log10_k_over_k_closes() {
    // √k·log(k)^10 / k: effective_x2=1; 2·1=2 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase108_sqrt_k_log10_k_times_k_over_k2_closes() {
    // √k·log(k)^10·k / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase108_sqrt_k_log10_k_times_k_over_k_refused() {
    // √k·log(k)^10·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 109: Two-Sqrt × Ten-Log × polynomial ─────────────────────────────────

#[test]
fn phase109_sqrt_k_x2_log10_k_over_k2_closes() {
    // √k×2·log(k)^10 / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase109_sqrt_k_x2_log10_k_times_k_over_k3_closes() {
    // √k×2·log(k)^10·k / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase109_sqrt_k_x2_log10_k_times_k_over_k_refused() {
    // √k×2·log(k)^10·k / k: effective_x2=4; 2·1=2 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 110: Three-Sqrt × Ten-Log × polynomial ───────────────────────────────

#[test]
fn phase110_sqrt_k_x3_log10_k_over_k2_closes() {
    // √k×3·log(k)^10 / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase110_sqrt_k_x3_log10_k_times_k_over_k3_closes() {
    // √k×3·log(k)^10·k / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase110_sqrt_k_x3_log10_k_times_k_over_k2_refused() {
    // √k×3·log(k)^10·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 111: Four-Sqrt × Ten-Log × polynomial ────────────────────────────────

#[test]
fn phase111_sqrt_k_x4_log10_k_over_k3_closes() {
    // √k×4·log(k)^10 / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase111_sqrt_k_x4_log10_k_times_k_over_k4_closes() {
    // √k×4·log(k)^10·k / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])])]);
    let kp1_4 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase111_sqrt_k_x4_log10_k_times_k_over_k3_refused() {
    // √k×4·log(k)^10·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 112: Five-Sqrt × Ten-Log × polynomial (completes Ten-Log family) ─────

#[test]
fn phase112_sqrt_k_x5_log10_k_over_k3_closes() {
    // √k×5·log(k)^10 / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase112_sqrt_k_x5_log10_k_times_k_over_k4_closes() {
    // √k×5·log(k)^10·k / k⁴: effective_x2=7; 2·4=8 > 7 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])])]);
    let kp1_4 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase112_sqrt_k_x5_log10_k_times_k_over_k3_refused() {
    // √k×5·log(k)^10·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 113: Eleven-Log × polynomial ─────────────────────────────────────────

#[test]
fn phase113_log11_k_over_k_closes() {
    // log(k)^11 / k: effective_x2=0; 2·1=2 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase113_log11_k_times_k_over_k2_closes() {
    // log(k)^11·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase113_log11_k_times_k_over_k_refused() {
    // log(k)^11·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 114: One-Sqrt × Eleven-Log × polynomial ──────────────────────────────

#[test]
fn phase114_sqrt_k_log11_k_over_k_closes() {
    // √k·log(k)^11 / k: effective_x2=1; 2·1=2 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase114_sqrt_k_log11_k_times_k_over_k2_closes() {
    // √k·log(k)^11·k / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase114_sqrt_k_log11_k_times_k_over_k_refused() {
    // √k·log(k)^11·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 115: Two-Sqrt × Eleven-Log × polynomial ──────────────────────────────

#[test]
fn phase115_sqrt_k_x2_log11_k_over_k2_closes() {
    // √k×2·log(k)^11 / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase115_sqrt_k_x2_log11_k_times_k_over_k3_closes() {
    // √k×2·log(k)^11·k / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase115_sqrt_k_x2_log11_k_times_k_over_k2_refused() {
    // √k×2·log(k)^11·k / k²: effective_x2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 116: Three-Sqrt × Eleven-Log × polynomial ────────────────────────────

#[test]
fn phase116_sqrt_k_x3_log11_k_over_k2_closes() {
    // √k×3·log(k)^11 / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase116_sqrt_k_x3_log11_k_times_k_over_k3_closes() {
    // √k×3·log(k)^11·k / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase116_sqrt_k_x3_log11_k_times_k_over_k2_refused() {
    // √k×3·log(k)^11·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 117: Four-Sqrt × Eleven-Log × polynomial ─────────────────────────────

#[test]
fn phase117_sqrt_k_x4_log11_k_over_k3_closes() {
    // √k×4·log(k)^11 / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase117_sqrt_k_x4_log11_k_times_k_over_k4_closes() {
    // √k×4·log(k)^11·k / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])])]);
    let kp1_4 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase117_sqrt_k_x4_log11_k_times_k_over_k3_refused() {
    // √k×4·log(k)^11·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 118: Five-Sqrt × Eleven-Log × polynomial ─────────────────────────────

#[test]
fn phase118_sqrt_k_x5_log11_k_over_k3_closes() {
    // √k×5·log(k)^11 / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase118_sqrt_k_x5_log11_k_times_k_over_k4_closes() {
    // √k×5·log(k)^11·k / k⁴: effective_x2=7; 2·4=8 > 7 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])])]);
    let kp1_4 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase118_sqrt_k_x5_log11_k_times_k_over_k3_refused() {
    // √k×5·log(k)^11·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 119: Zero-Sqrt × Twelve-Log × polynomial ──────────────────────────

#[test]
fn phase119_log12_k_over_k_closes() {
    // log(k)^12 / k: effective_x2=0; 2·1=2 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase119_log12_k_times_k_over_k2_closes() {
    // log(k)^12·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase119_log12_k_times_k_over_k_refused() {
    // log(k)^12·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 120: One-Sqrt × Twelve-Log × polynomial ───────────────────────────

#[test]
fn phase120_sqrt_k_log12_k_over_k_closes() {
    // √k·log(k)^12 / k: effective_x2=1; 2·1=2 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase120_sqrt_k_log12_k_times_k_over_k2_closes() {
    // √k·log(k)^12·k / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase120_sqrt_k_log12_k_times_k_over_k_refused() {
    // √k·log(k)^12·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 121: Two-Sqrt × Twelve-Log × polynomial ───────────────────────────

#[test]
fn phase121_sqrt_k_x2_log12_k_over_k2_closes() {
    // √k×2·log(k)^12 / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase121_sqrt_k_x2_log12_k_times_k_over_k3_closes() {
    // √k×2·log(k)^12·k / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase121_sqrt_k_x2_log12_k_times_k_over_k2_refused() {
    // √k×2·log(k)^12·k / k²: effective_x2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 122: Three-Sqrt × Twelve-Log × polynomial ─────────────────────────

#[test]
fn phase122_sqrt_k_x3_log12_k_over_k2_closes() {
    // √k×3·log(k)^12 / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase122_sqrt_k_x3_log12_k_times_k_over_k3_closes() {
    // √k×3·log(k)^12·k / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase122_sqrt_k_x3_log12_k_times_k_over_k2_refused() {
    // √k×3·log(k)^12·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 123: Four-Sqrt × Twelve-Log × polynomial ──────────────────────────

#[test]
fn phase123_sqrt_k_x4_log12_k_over_k3_closes() {
    // √k×4·log(k)^12 / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase123_sqrt_k_x4_log12_k_times_k_over_k4_closes() {
    // √k×4·log(k)^12·k / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])])]);
    let kp1_4 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase123_sqrt_k_x4_log12_k_times_k_over_k3_refused() {
    // √k×4·log(k)^12·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 124: Five-Sqrt × Twelve-Log × polynomial ──────────────────────────

#[test]
fn phase124_sqrt_k_x5_log12_k_over_k3_closes() {
    // √k×5·log(k)^12 / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase124_sqrt_k_x5_log12_k_times_k_over_k4_closes() {
    // √k×5·log(k)^12·k / k⁴: effective_x2=7; 2·4=8 > 7 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])])]);
    let kp1_4 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase124_sqrt_k_x5_log12_k_times_k_over_k3_refused() {
    // √k×5·log(k)^12·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym(MUL), vec![k.clone(), apply(sym(MUL), vec![k.clone(), k.clone()])]);
    let kp1_3 = apply(sym(MUL), vec![kp1.clone(), apply(sym(MUL), vec![kp1.clone(), kp1.clone()])]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

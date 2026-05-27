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

// ── Phase 125: Zero-Sqrt × Thirteen-Log × polynomial ─────────────────────────

#[test]
fn phase125_log13_k_over_k_closes() {
    // log(k)^13 / k: effective_x2=0; 2·1=2 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase125_log13_k_times_k_over_k2_closes() {
    // log(k)^13·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase125_log13_k_times_k_over_k_refused() {
    // log(k)^13·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 126: One-Sqrt × Thirteen-Log × polynomial ──────────────────────────

#[test]
fn phase126_sqrt_k_log13_k_over_k_closes() {
    // √k·log(k)^13 / k: effective_x2=1; 2·1=2 > 1 → closes.
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
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase126_sqrt_k_log13_k_times_k_over_k2_closes() {
    // √k·log(k)^13·k / k²: effective_x2=3; 2·2=4 > 3 → closes.
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
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase126_sqrt_k_log13_k_times_k_over_k_refused() {
    // √k·log(k)^13·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
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
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 127: Two-Sqrt × Thirteen-Log × polynomial ──────────────────────────

#[test]
fn phase127_sqrt_k_x2_log13_k_over_k2_closes() {
    // √k×2·log(k)^13 / k²: effective_x2=2; 2·2=4 > 2 → closes.
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
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase127_sqrt_k_x2_log13_k_times_k_over_k3_closes() {
    // √k×2·log(k)^13·k / k³: effective_x2=4; 2·3=6 > 4 → closes.
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
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase127_sqrt_k_x2_log13_k_times_k_over_k2_refused() {
    // √k×2·log(k)^13·k / k²: effective_x2=4; 2·2=4 not > 4 → refused.
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
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 128: Three-Sqrt × Thirteen-Log × polynomial ────────────────────────

#[test]
fn phase128_sqrt_k_x3_log13_k_over_k2_closes() {
    // √k×3·log(k)^13 / k²: effective_x2=3; 2·2=4 > 3 → closes.
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
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase128_sqrt_k_x3_log13_k_times_k_over_k3_closes() {
    // √k×3·log(k)^13·k / k³: effective_x2=5; 2·3=6 > 5 → closes.
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
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase128_sqrt_k_x3_log13_k_times_k_over_k2_refused() {
    // √k×3·log(k)^13·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
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
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 129: Four-Sqrt × Thirteen-Log × polynomial ─────────────────────────

#[test]
fn phase129_sqrt_k_x4_log13_k_over_k3_closes() {
    // √k×4·log(k)^13 / k³: effective_x2=4; 2·3=6 > 4 → closes.
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
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase129_sqrt_k_x4_log13_k_times_k_over_k4_closes() {
    // √k×4·log(k)^13·k / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
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
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase129_sqrt_k_x4_log13_k_times_k_over_k3_refused() {
    // √k×4·log(k)^13·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
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
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 130: Five-Sqrt × Thirteen-Log × polynomial ─────────────────────────

#[test]
fn phase130_sqrt_k_x5_log13_k_over_k3_closes() {
    // √k×5·log(k)^13 / k³: effective_x2=5; 2·3=6 > 5 → closes.
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
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase130_sqrt_k_x5_log13_k_times_k_over_k4_closes() {
    // √k×5·log(k)^13·k / k⁴: effective_x2=7; 2·4=8 > 7 → closes.
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
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase130_sqrt_k_x5_log13_k_times_k_over_k3_refused() {
    // √k×5·log(k)^13·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
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
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1 = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f = apply(sym(SUB), vec![g_k, g_kp1]);
    let out = evaluate_sum(f, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 131: Fourteen-Log × polynomial ─────────────────────────────────────

#[test]
fn phase131_log14_k_over_k_closes() {
    // log(k)^14 / k: effective_x2=0; 2·1=2 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase131_log14_k_times_k_over_k2_closes() {
    // log(k)^14·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase131_log14_k_times_k_over_k_refused() {
    // log(k)^14·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 132: One-Sqrt × Fourteen-Log × polynomial ──────────────────────────

#[test]
fn phase132_sqrt_k_log14_k_over_k_closes() {
    // √k·log(k)^14 / k: effective_x2=1; 2·1=2 > 1 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase132_sqrt_k_log14_k_times_k_over_k2_closes() {
    // √k·log(k)^14·k / k²: effective_x2=3; 2·2=4 > 3 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase132_sqrt_k_log14_k_times_k_over_k_refused() {
    // √k·log(k)^14·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 133: Two-Sqrt × Fourteen-Log × polynomial ──────────────────────────

#[test]
fn phase133_sqrt_k_x2_log14_k_over_k2_closes() {
    // √k×2·log(k)^14 / k²: effective_x2=2; 2·2=4 > 2 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase133_sqrt_k_x2_log14_k_times_k_over_k3_closes() {
    // √k×2·log(k)^14·k / k³: effective_x2=4; 2·3=6 > 4 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase133_sqrt_k_x2_log14_k_times_k_over_k2_refused() {
    // √k×2·log(k)^14·k / k²: effective_x2=4; 2·2=4 not > 4 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 134: Three-Sqrt × Fourteen-Log × polynomial ────────────────────────

#[test]
fn phase134_sqrt_k_x3_log14_k_over_k2_closes() {
    // √k×3·log(k)^14 / k²: effective_x2=3; 2·2=4 > 3 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase134_sqrt_k_x3_log14_k_times_k_over_k3_closes() {
    // √k×3·log(k)^14·k / k³: effective_x2=5; 2·3=6 > 5 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase134_sqrt_k_x3_log14_k_times_k_over_k2_refused() {
    // √k×3·log(k)^14·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 135: Four-Sqrt × Fourteen-Log × polynomial ─────────────────────────

#[test]
fn phase135_sqrt_k_x4_log14_k_over_k3_closes() {
    // √k×4·log(k)^14 / k³: effective_x2=4; 2·3=6 > 4 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase135_sqrt_k_x4_log14_k_times_k_over_k4_closes() {
    // √k×4·log(k)^14·k / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase135_sqrt_k_x4_log14_k_times_k_over_k3_refused() {
    // √k×4·log(k)^14·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 136: Five-Sqrt × Fourteen-Log × polynomial ─────────────────────────

#[test]
fn phase136_sqrt_k_x5_log14_k_over_k3_closes() {
    // √k×5·log(k)^14 / k³: effective_x2=5; 2·3=6 > 5 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase136_sqrt_k_x5_log14_k_times_k_over_k4_closes() {
    // √k×5·log(k)^14·k / k⁴: effective_x2=7; 2·4=8 > 7 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase136_sqrt_k_x5_log14_k_times_k_over_k3_refused() {
    // √k×5·log(k)^14·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 137: Fifteen-Log × polynomial ──────────────────────────────────────

#[test]
fn phase137_log15_k_over_k_closes() {
    // log(k)^15 / k: effective_x2=0; 2·1=2 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase137_log15_k_times_k_over_k2_closes() {
    // log(k)^15·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase137_log15_k_times_k_over_k_refused() {
    // log(k)^15·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 138: One-Sqrt × Fifteen-Log × polynomial ───────────────────────────

#[test]
fn phase138_sqrt_k_log15_k_over_k_closes() {
    // √k·log(k)^15 / k: effective_x2=1; 2·1=2 > 1 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase138_sqrt_k_log15_k_times_k_over_k2_closes() {
    // √k·log(k)^15·k / k²: effective_x2=3; 2·2=4 > 3 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase138_sqrt_k_log15_k_times_k_over_k_refused() {
    // √k·log(k)^15·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 139: Two-Sqrt × Fifteen-Log × polynomial ───────────────────────────

#[test]
fn phase139_sqrt_k_x2_log15_k_over_k2_closes() {
    // √k×2·log(k)^15 / k²: effective_x2=2; 2·2=4 > 2 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase139_sqrt_k_x2_log15_k_times_k_over_k3_closes() {
    // √k×2·log(k)^15·k / k³: effective_x2=4; 2·3=6 > 4 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase139_sqrt_k_x2_log15_k_times_k_over_k2_refused() {
    // √k×2·log(k)^15·k / k²: effective_x2=4; 2·2=4 not > 4 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 140: Three-Sqrt × Fifteen-Log × polynomial ─────────────────────────

#[test]
fn phase140_sqrt_k_x3_log15_k_over_k2_closes() {
    // √k×3·log(k)^15 / k²: effective_x2=3; 2·2=4 > 3 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase140_sqrt_k_x3_log15_k_times_k_over_k3_closes() {
    // √k×3·log(k)^15·k / k³: effective_x2=5; 2·3=6 > 5 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase140_sqrt_k_x3_log15_k_times_k_over_k2_refused() {
    // √k×3·log(k)^15·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 141: Four-Sqrt × Fifteen-Log × polynomial ──────────────────────────

#[test]
fn phase141_sqrt_k_x4_log15_k_over_k3_closes() {
    // √k×4·log(k)^15 / k³: effective_x2=4; 2·3=6 > 4 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase141_sqrt_k_x4_log15_k_times_k_over_k4_closes() {
    // √k×4·log(k)^15·k / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase141_sqrt_k_x4_log15_k_times_k_over_k3_refused() {
    // √k×4·log(k)^15·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 142: Five-Sqrt × Fifteen-Log × polynomial ──────────────────────────

#[test]
fn phase142_sqrt_k_x5_log15_k_over_k3_closes() {
    // √k×5·log(k)^15 / k³: effective_x2=5; 2·3=6 > 5 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase142_sqrt_k_x5_log15_k_times_k_over_k4_closes() {
    // √k×5·log(k)^15·k / k⁴: effective_x2=7; 2·4=8 > 7 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase142_sqrt_k_x5_log15_k_times_k_over_k3_refused() {
    // √k×5·log(k)^15·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 143: Sixteen-Log × polynomial ──────────────────────────────────────

#[test]
fn phase143_log16_k_over_k_closes() {
    // log(k)^16 / k: effective_x2=0; 2·1=2 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase143_log16_k_times_k_over_k2_closes() {
    // log(k)^16·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let kp1_2 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase143_log16_k_times_k_over_k_refused() {
    // log(k)^16·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 144: One-Sqrt × Sixteen-Log × polynomial ───────────────────────────

#[test]
fn phase144_sqrt_k_log16_k_over_k_closes() {
    // √k·log(k)^16 / k: effective_x2=1; 2·1=2 > 1 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase144_sqrt_k_log16_k_times_k_over_k2_closes() {
    // √k·log(k)^16·k / k²: effective_x2=3; 2·2=4 > 3 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase144_sqrt_k_log16_k_times_k_over_k_refused() {
    // √k·log(k)^16·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 145: Two-Sqrt × Sixteen-Log × polynomial ───────────────────────────

#[test]
fn phase145_sqrt_k_x2_log16_k_over_k_closes() {
    // √k×2·log(k)^16 / k: effective_x2=2; 2·1=2 not > 2 — use k2 to close.
    // Actually use den=k² to close: effective_x2=2; 2·2=4 > 2 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase145_sqrt_k_x2_log16_k_times_k_over_k3_closes() {
    // √k×2·log(k)^16·k / k³: effective_x2=4; 2·3=6 > 4 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase145_sqrt_k_x2_log16_k_times_k_over_k2_refused() {
    // √k×2·log(k)^16·k / k²: effective_x2=4; 2·2=4 not > 4 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 146: Three-Sqrt × Sixteen-Log × polynomial ─────────────────────────

#[test]
fn phase146_sqrt_k_x3_log16_k_over_k2_closes() {
    // √k×3·log(k)^16 / k²: effective_x2=3; 2·2=4 > 3 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase146_sqrt_k_x3_log16_k_times_k_over_k3_closes() {
    // √k×3·log(k)^16·k / k³: effective_x2=5; 2·3=6 > 5 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase146_sqrt_k_x3_log16_k_times_k_over_k2_refused() {
    // √k×3·log(k)^16·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 147: Four-Sqrt × Sixteen-Log × polynomial ──────────────────────────

#[test]
fn phase147_sqrt_k_x4_log16_k_over_k2_closes() {
    // √k×4·log(k)^16 / k²: effective_x2=4; 2·2=4 not > 4 — use k³ to close.
    // √k×4·log(k)^16 / k³: effective_x2=4; 2·3=6 > 4 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase147_sqrt_k_x4_log16_k_times_k_over_k4_closes() {
    // √k×4·log(k)^16·k / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase147_sqrt_k_x4_log16_k_times_k_over_k3_refused() {
    // √k×4·log(k)^16·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ── Phase 148: Five-Sqrt × Sixteen-Log × polynomial ──────────────────────────

#[test]
fn phase148_sqrt_k_x5_log16_k_over_k3_closes() {
    // √k×5·log(k)^16 / k³: effective_x2=5; 2·3=6 > 5 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase148_sqrt_k_x5_log16_k_times_k_over_k4_closes() {
    // √k×5·log(k)^16·k / k⁴: effective_x2=7; 2·4=8 > 7 → closes.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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
fn phase148_sqrt_k_x5_log16_k_times_k_over_k3_refused() {
    // √k×5·log(k)^16·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
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

// ---------------------------------------------------------------------------
// Phase 149 — Seventeen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase149_log17_k_over_k2_closes() {
    // log(k)^17 / k²: effective_x2=0; 2·2=4 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k149a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_149a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f149a = apply(sym(SUB), vec![g_k149a, g_kp1_149a]);
    let out = evaluate_sum(f149a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase149_log17_k_times_k_over_k2_closes() {
    // log(k)^17·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k149b = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_149b = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f149b = apply(sym(SUB), vec![g_k149b, g_kp1_149b]);
    let out = evaluate_sum(f149b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase149_log17_k_times_k_over_k_refused() {
    // log(k)^17·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k149c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_149c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f149c = apply(sym(SUB), vec![g_k149c, g_kp1_149c]);
    let out = evaluate_sum(f149c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 150 — One-Sqrt × Seventeen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase150_sqrt_k_log17_k_over_k2_closes() {
    // √k·log(k)^17 / k²: effective_x2=1; 2·2=4 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k150a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_150a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f150a = apply(sym(SUB), vec![g_k150a, g_kp1_150a]);
    let out = evaluate_sum(f150a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase150_sqrt_k_log17_k_times_k_over_k2_closes() {
    // √k·log(k)^17·k / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k150b = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_150b = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f150b = apply(sym(SUB), vec![g_k150b, g_kp1_150b]);
    let out = evaluate_sum(f150b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase150_sqrt_k_log17_k_times_k_over_k_refused() {
    // √k·log(k)^17·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k150c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_150c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f150c = apply(sym(SUB), vec![g_k150c, g_kp1_150c]);
    let out = evaluate_sum(f150c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 151 — Two-Sqrt × Seventeen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase151_sqrt_k_x2_log17_k_over_k2_closes() {
    // √k²·log(k)^17 / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k151a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_151a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f151a = apply(sym(SUB), vec![g_k151a, g_kp1_151a]);
    let out = evaluate_sum(f151a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase151_sqrt_k_x2_log17_k_times_k_over_k3_closes() {
    // √k²·log(k)^17·k / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3p = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3p = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k151b = apply(sym(DIV), vec![num_k, k3p]);
    let g_kp1_151b = apply(sym(DIV), vec![num_kp1, kp1_3p]);
    let f151b = apply(sym(SUB), vec![g_k151b, g_kp1_151b]);
    let out = evaluate_sum(f151b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase151_sqrt_k_x2_log17_k_times_k_over_k2_refused() {
    // √k²·log(k)^17·k / k²: effective_x2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2p = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2p = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k151c = apply(sym(DIV), vec![num_k, k2p]);
    let g_kp1_151c = apply(sym(DIV), vec![num_kp1, kp1_2p]);
    let f151c = apply(sym(SUB), vec![g_k151c, g_kp1_151c]);
    let out = evaluate_sum(f151c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 152 — Three-Sqrt × Seventeen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase152_sqrt_k_x3_log17_k_over_k2_closes() {
    // √k³·log(k)^17 / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k152a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_152a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f152a = apply(sym(SUB), vec![g_k152a, g_kp1_152a]);
    let out = evaluate_sum(f152a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase152_sqrt_k_x3_log17_k_times_k_over_k3_closes() {
    // √k³·log(k)^17·k / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k152b = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_152b = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f152b = apply(sym(SUB), vec![g_k152b, g_kp1_152b]);
    let out = evaluate_sum(f152b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase152_sqrt_k_x3_log17_k_times_k_over_k2_refused() {
    // √k³·log(k)^17·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2r = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2r = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k152c = apply(sym(DIV), vec![num_k, k2r]);
    let g_kp1_152c = apply(sym(DIV), vec![num_kp1, kp1_2r]);
    let f152c = apply(sym(SUB), vec![g_k152c, g_kp1_152c]);
    let out = evaluate_sum(f152c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 153 — Four-Sqrt × Seventeen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase153_sqrt_k_x4_log17_k_over_k3_closes() {
    // √k⁴·log(k)^17 / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k153a = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_153a = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f153a = apply(sym(SUB), vec![g_k153a, g_kp1_153a]);
    let out = evaluate_sum(f153a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase153_sqrt_k_x4_log17_k_times_k_over_k4_closes() {
    // √k⁴·log(k)^17·k / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym("Pow"), vec![k.clone(), int(4)]);
    let kp1_4 = apply(sym("Pow"), vec![kp1.clone(), int(4)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k153b = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1_153b = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f153b = apply(sym(SUB), vec![g_k153b, g_kp1_153b]);
    let out = evaluate_sum(f153b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase153_sqrt_k_x4_log17_k_times_k_over_k3_refused() {
    // √k⁴·log(k)^17·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3r = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3r = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k153c = apply(sym(DIV), vec![num_k, k3r]);
    let g_kp1_153c = apply(sym(DIV), vec![num_kp1, kp1_3r]);
    let f153c = apply(sym(SUB), vec![g_k153c, g_kp1_153c]);
    let out = evaluate_sum(f153c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 154 — Five-Sqrt × Seventeen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase154_sqrt_k_x5_log17_k_over_k3_closes() {
    // √k⁵·log(k)^17 / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let g_k154a = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_154a = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f154a = apply(sym(SUB), vec![g_k154a, g_kp1_154a]);
    let out = evaluate_sum(f154a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase154_sqrt_k_x5_log17_k_times_k_over_k4_closes() {
    // √k⁵·log(k)^17·k / k⁴: effective_x2=7; 2·4=8 > 7 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym("Pow"), vec![k.clone(), int(4)]);
    let kp1_4 = apply(sym("Pow"), vec![kp1.clone(), int(4)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k154b = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1_154b = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f154b = apply(sym(SUB), vec![g_k154b, g_kp1_154b]);
    let out = evaluate_sum(f154b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase154_sqrt_k_x5_log17_k_times_k_over_k3_refused() {
    // √k⁵·log(k)^17·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3s = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3s = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k154c = apply(sym(DIV), vec![num_k, k3s]);
    let g_kp1_154c = apply(sym(DIV), vec![num_kp1, kp1_3s]);
    let f154c = apply(sym(SUB), vec![g_k154c, g_kp1_154c]);
    let out = evaluate_sum(f154c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 155 — Eighteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase155_log18_k_over_k2_closes() {
    // log(k)^18 / k²: effective_x2=0; 2·2=4 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k155a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_155a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f155a = apply(sym(SUB), vec![g_k155a, g_kp1_155a]);
    let out = evaluate_sum(f155a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase155_log18_k_times_k_over_k2_closes() {
    // log(k)^18·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k155b = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_155b = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f155b = apply(sym(SUB), vec![g_k155b, g_kp1_155b]);
    let out = evaluate_sum(f155b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase155_log18_k_times_k_over_k_refused() {
    // log(k)^18·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k155c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_155c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f155c = apply(sym(SUB), vec![g_k155c, g_kp1_155c]);
    let out = evaluate_sum(f155c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 156 — One-Sqrt × Eighteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase156_sqrt_k_log18_k_over_k2_closes() {
    // √k·log(k)^18 / k²: effective_x2=1; 2·2=4 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k156a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_156a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f156a = apply(sym(SUB), vec![g_k156a, g_kp1_156a]);
    let out = evaluate_sum(f156a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase156_sqrt_k_log18_k_times_k_over_k2_closes() {
    // √k·log(k)^18·k / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k156b = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_156b = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f156b = apply(sym(SUB), vec![g_k156b, g_kp1_156b]);
    let out = evaluate_sum(f156b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase156_sqrt_k_log18_k_times_k_over_k_refused() {
    // √k·log(k)^18·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k156c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_156c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f156c = apply(sym(SUB), vec![g_k156c, g_kp1_156c]);
    let out = evaluate_sum(f156c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 157 — Two-Sqrt × Eighteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase157_sqrt_k_x2_log18_k_over_k2_closes() {
    // √k²·log(k)^18 / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k157a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_157a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f157a = apply(sym(SUB), vec![g_k157a, g_kp1_157a]);
    let out = evaluate_sum(f157a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase157_sqrt_k_x2_log18_k_times_k_over_k3_closes() {
    // √k²·log(k)^18·k / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k157b = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_157b = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f157b = apply(sym(SUB), vec![g_k157b, g_kp1_157b]);
    let out = evaluate_sum(f157b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase157_sqrt_k_x2_log18_k_times_k_over_k2_refused() {
    // √k²·log(k)^18·k / k²: effective_x2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k157c = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_157c = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f157c = apply(sym(SUB), vec![g_k157c, g_kp1_157c]);
    let out = evaluate_sum(f157c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 158 — Three-Sqrt × Eighteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase158_sqrt_k_x3_log18_k_over_k2_closes() {
    // √k³·log(k)^18 / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k158a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_158a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f158a = apply(sym(SUB), vec![g_k158a, g_kp1_158a]);
    let out = evaluate_sum(f158a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase158_sqrt_k_x3_log18_k_times_k_over_k3_closes() {
    // √k³·log(k)^18·k / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k158b = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_158b = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f158b = apply(sym(SUB), vec![g_k158b, g_kp1_158b]);
    let out = evaluate_sum(f158b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase158_sqrt_k_x3_log18_k_times_k_over_k2_refused() {
    // √k³·log(k)^18·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k158c = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_158c = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f158c = apply(sym(SUB), vec![g_k158c, g_kp1_158c]);
    let out = evaluate_sum(f158c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 159 — Four-Sqrt × Eighteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase159_sqrt_k_x4_log18_k_over_k3_closes() {
    // √k⁴·log(k)^18 / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k159a = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_159a = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f159a = apply(sym(SUB), vec![g_k159a, g_kp1_159a]);
    let out = evaluate_sum(f159a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase159_sqrt_k_x4_log18_k_times_k_over_k4_closes() {
    // √k⁴·log(k)^18·k / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym("Pow"), vec![k.clone(), int(4)]);
    let kp1_4 = apply(sym("Pow"), vec![kp1.clone(), int(4)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k159b = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1_159b = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f159b = apply(sym(SUB), vec![g_k159b, g_kp1_159b]);
    let out = evaluate_sum(f159b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase159_sqrt_k_x4_log18_k_times_k_over_k3_refused() {
    // √k⁴·log(k)^18·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3s = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3s = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k159c = apply(sym(DIV), vec![num_k, k3s]);
    let g_kp1_159c = apply(sym(DIV), vec![num_kp1, kp1_3s]);
    let f159c = apply(sym(SUB), vec![g_k159c, g_kp1_159c]);
    let out = evaluate_sum(f159c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 160 — Five-Sqrt × Eighteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase160_sqrt_k_x5_log18_k_over_k3_closes() {
    // √k⁵·log(k)^18 / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k160a = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_160a = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f160a = apply(sym(SUB), vec![g_k160a, g_kp1_160a]);
    let out = evaluate_sum(f160a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase160_sqrt_k_x5_log18_k_times_k_over_k4_closes() {
    // √k⁵·log(k)^18·k / k⁴: effective_x2=7; 2·4=8 > 7 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym("Pow"), vec![k.clone(), int(4)]);
    let kp1_4 = apply(sym("Pow"), vec![kp1.clone(), int(4)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k160b = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1_160b = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f160b = apply(sym(SUB), vec![g_k160b, g_kp1_160b]);
    let out = evaluate_sum(f160b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase160_sqrt_k_x5_log18_k_times_k_over_k3_refused() {
    // √k⁵·log(k)^18·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3s = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3s = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k160c = apply(sym(DIV), vec![num_k, k3s]);
    let g_kp1_160c = apply(sym(DIV), vec![num_kp1, kp1_3s]);
    let f160c = apply(sym(SUB), vec![g_k160c, g_kp1_160c]);
    let out = evaluate_sum(f160c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 161 — Nineteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase161_log19_k_over_k2_closes() {
    // log(k)^19 / k²: effective_x2=0; 2·2=4 > 0 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k161a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_161a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f161a = apply(sym(SUB), vec![g_k161a, g_kp1_161a]);
    let out = evaluate_sum(f161a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase161_log19_k_times_k_over_k2_closes() {
    // log(k)^19·k / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k161b = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_161b = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f161b = apply(sym(SUB), vec![g_k161b, g_kp1_161b]);
    let out = evaluate_sum(f161b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase161_log19_k_times_k_over_k_refused() {
    // log(k)^19·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k161c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_161c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f161c = apply(sym(SUB), vec![g_k161c, g_kp1_161c]);
    let out = evaluate_sum(f161c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 162 — One-Sqrt × Nineteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase162_sqrt_k_log19_k_over_k2_closes() {
    // √k·log(k)^19 / k²: effective_x2=1; 2·2=4 > 1 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k162a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_162a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f162a = apply(sym(SUB), vec![g_k162a, g_kp1_162a]);
    let out = evaluate_sum(f162a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase162_sqrt_k_log19_k_times_k_over_k2_closes() {
    // √k·log(k)^19·k / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k162b = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_162b = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f162b = apply(sym(SUB), vec![g_k162b, g_kp1_162b]);
    let out = evaluate_sum(f162b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase162_sqrt_k_log19_k_times_k_over_k_refused() {
    // √k·log(k)^19·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k162c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_162c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f162c = apply(sym(SUB), vec![g_k162c, g_kp1_162c]);
    let out = evaluate_sum(f162c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 163 — Two-Sqrt × Nineteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase163_sqrt_k_x2_log19_k_over_k2_closes() {
    // √k²·log(k)^19 / k²: effective_x2=2; 2·2=4 > 2 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k163a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_163a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f163a = apply(sym(SUB), vec![g_k163a, g_kp1_163a]);
    let out = evaluate_sum(f163a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase163_sqrt_k_x2_log19_k_times_k_over_k3_closes() {
    // √k²·log(k)^19·k / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k163b = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_163b = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f163b = apply(sym(SUB), vec![g_k163b, g_kp1_163b]);
    let out = evaluate_sum(f163b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase163_sqrt_k_x2_log19_k_times_k_over_k2_refused() {
    // √k²·log(k)^19·k / k²: effective_x2=4; 2·2=4 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k163c = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_163c = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f163c = apply(sym(SUB), vec![g_k163c, g_kp1_163c]);
    let out = evaluate_sum(f163c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 164 — Three-Sqrt × Nineteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase164_sqrt_k_x3_log19_k_over_k2_closes() {
    // √k³·log(k)^19 / k²: effective_x2=3; 2·2=4 > 3 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k164a = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_164a = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f164a = apply(sym(SUB), vec![g_k164a, g_kp1_164a]);
    let out = evaluate_sum(f164a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase164_sqrt_k_x3_log19_k_times_k_over_k3_closes() {
    // √k³·log(k)^19·k / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k164b = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_164b = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f164b = apply(sym(SUB), vec![g_k164b, g_kp1_164b]);
    let out = evaluate_sum(f164b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase164_sqrt_k_x3_log19_k_times_k_over_k2_refused() {
    // √k³·log(k)^19·k / k²: effective_x2=5; 2·2=4 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k2 = apply(sym("Pow"), vec![k.clone(), int(2)]);
    let kp1_2 = apply(sym("Pow"), vec![kp1.clone(), int(2)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k164c = apply(sym(DIV), vec![num_k, k2]);
    let g_kp1_164c = apply(sym(DIV), vec![num_kp1, kp1_2]);
    let f164c = apply(sym(SUB), vec![g_k164c, g_kp1_164c]);
    let out = evaluate_sum(f164c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 165 — Four-Sqrt × Nineteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase165_sqrt_k_x4_log19_k_over_k3_closes() {
    // √k⁴·log(k)^19 / k³: effective_x2=4; 2·3=6 > 4 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k165a = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_165a = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f165a = apply(sym(SUB), vec![g_k165a, g_kp1_165a]);
    let out = evaluate_sum(f165a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase165_sqrt_k_x4_log19_k_times_k_over_k4_closes() {
    // √k⁴·log(k)^19·k / k⁴: effective_x2=6; 2·4=8 > 6 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym("Pow"), vec![k.clone(), int(4)]);
    let kp1_4 = apply(sym("Pow"), vec![kp1.clone(), int(4)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k165b = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1_165b = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f165b = apply(sym(SUB), vec![g_k165b, g_kp1_165b]);
    let out = evaluate_sum(f165b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase165_sqrt_k_x4_log19_k_times_k_over_k3_refused() {
    // √k⁴·log(k)^19·k / k³: effective_x2=6; 2·3=6 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k165c = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_165c = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f165c = apply(sym(SUB), vec![g_k165c, g_kp1_165c]);
    let out = evaluate_sum(f165c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Phase 166 — Five-Sqrt × Nineteen-Log × polynomial numerator
// ---------------------------------------------------------------------------

#[test]
fn phase166_sqrt_k_x5_log19_k_over_k3_closes() {
    // √k⁵·log(k)^19 / k³: effective_x2=5; 2·3=6 > 5 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3 = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3 = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let g_k166a = apply(sym(DIV), vec![num_k, k3]);
    let g_kp1_166a = apply(sym(DIV), vec![num_kp1, kp1_3]);
    let f166a = apply(sym(SUB), vec![g_k166a, g_kp1_166a]);
    let out = evaluate_sum(f166a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase166_sqrt_k_x5_log19_k_times_k_over_k4_closes() {
    // √k⁵·log(k)^19·k / k⁴: effective_x2=7; 2·4=8 > 7 → closes.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k4 = apply(sym("Pow"), vec![k.clone(), int(4)]);
    let kp1_4 = apply(sym("Pow"), vec![kp1.clone(), int(4)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k166b = apply(sym(DIV), vec![num_k, k4]);
    let g_kp1_166b = apply(sym(DIV), vec![num_kp1, kp1_4]);
    let f166b = apply(sym(SUB), vec![g_k166b, g_kp1_166b]);
    let out = evaluate_sum(f166b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase166_sqrt_k_x5_log19_k_times_k_over_k3_refused() {
    // √k⁵·log(k)^19·k / k³: effective_x2=7; 2·3=6 not > 7 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let k3s = apply(sym("Pow"), vec![k.clone(), int(3)]);
    let kp1_3s = apply(sym("Pow"), vec![kp1.clone(), int(3)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k166c = apply(sym(DIV), vec![num_k, k3s]);
    let g_kp1_166c = apply(sym(DIV), vec![num_kp1, kp1_3s]);
    let f166c = apply(sym(SUB), vec![g_k166c, g_kp1_166c]);
    let out = evaluate_sum(f166c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}


// ── Phase 167: 0-sqrt × 20-log × polynomial ──────────────────────────────────

#[test]
fn phase167_log20_k_over_k3_converges() {
    // log(k)^20 / k^3: effective_x2=0; 2·3=6 > 0 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k167a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_167a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f167a = apply(sym(SUB), vec![g_k167a, g_kp1_167a]);
    let out = evaluate_sum(f167a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase167_log20_k_times_k_over_k4_converges() {
    // log(k)^20·k / k^4: effective_x2=2; 2·4=8 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k167b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_167b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f167b = apply(sym(SUB), vec![g_k167b, g_kp1_167b]);
    let out = evaluate_sum(f167b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase167_log20_k_over_k_refused() {
    // log(k)^20·k / k: effective_x2=2; 2·1=2 not > 2 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k167c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_167c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f167c = apply(sym(SUB), vec![g_k167c, g_kp1_167c]);
    let out = evaluate_sum(f167c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 168: 1-sqrt × 20-log × polynomial ──────────────────────────────────

#[test]
fn phase168_sqrt_k_log20_k_over_k3_converges() {
    // √k·log(k)^20 / k^3: effective_x2=1; 2·3=6 > 1 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k168a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_168a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f168a = apply(sym(SUB), vec![g_k168a, g_kp1_168a]);
    let out = evaluate_sum(f168a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase168_sqrt_k_log20_k_times_k_over_k4_converges() {
    // √k·log(k)^20·k / k^4: effective_x2=3; 2·4=8 > 3 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k168b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_168b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f168b = apply(sym(SUB), vec![g_k168b, g_kp1_168b]);
    let out = evaluate_sum(f168b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase168_sqrt_k_log20_k_times_k_over_k_refused() {
    // √k·log(k)^20·k / k: effective_x2=3; 2·1=2 not > 3 → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k168c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_168c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f168c = apply(sym(SUB), vec![g_k168c, g_kp1_168c]);
    let out = evaluate_sum(f168c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 169: 2-sqrt × 20-log × polynomial ──────────────────────────────────

#[test]
fn phase169_sqrt2_k_log20_k_over_k3_converges() {
    // √k·√k·log(k)^20 / k^3: effective_x2=2; 2·3=6 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k169a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_169a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f169a = apply(sym(SUB), vec![g_k169a, g_kp1_169a]);
    let out = evaluate_sum(f169a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase169_sqrt2_k_log20_k_times_k_over_k4_converges() {
    // √k·√k·log(k)^20·k / k^4: effective_x2=4; 2·4=8 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k169b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_169b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f169b = apply(sym(SUB), vec![g_k169b, g_kp1_169b]);
    let out = evaluate_sum(f169b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase169_sqrt2_k_log20_k_over_k_refused() {
    // √k·√k·log(k)^20·k / k: effective_x2=4; 2·1=2 not > 4 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k169c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_169c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f169c = apply(sym(SUB), vec![g_k169c, g_kp1_169c]);
    let out = evaluate_sum(f169c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 170: 3-sqrt × 20-log × polynomial ──────────────────────────────────

#[test]
fn phase170_sqrt3_k_log20_k_over_k3_converges() {
    // √k^3·log(k)^20 / k^3: effective_x2=3; 2·3=6 > 3 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k170a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_170a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f170a = apply(sym(SUB), vec![g_k170a, g_kp1_170a]);
    let out = evaluate_sum(f170a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase170_sqrt3_k_log20_k_times_k_over_k4_converges() {
    // √k^3·log(k)^20·k / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k170b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_170b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f170b = apply(sym(SUB), vec![g_k170b, g_kp1_170b]);
    let out = evaluate_sum(f170b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase170_sqrt3_k_log20_k_over_k_refused() {
    // √k^3·log(k)^20·k / k: effective_x2=5; 2·1=2 not > 5 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k170c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_170c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f170c = apply(sym(SUB), vec![g_k170c, g_kp1_170c]);
    let out = evaluate_sum(f170c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 171: 4-sqrt × 20-log × polynomial ──────────────────────────────────

#[test]
fn phase171_sqrt4_k_log20_k_over_k3_converges() {
    // √k^4·log(k)^20 / k^3: effective_x2=4; 2·3=6 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k171a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_171a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f171a = apply(sym(SUB), vec![g_k171a, g_kp1_171a]);
    let out = evaluate_sum(f171a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase171_sqrt4_k_log20_k_times_k_over_k4_converges() {
    // √k^4·log(k)^20·k / k^4: effective_x2=6; 2·4=8 > 6 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k171b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_171b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f171b = apply(sym(SUB), vec![g_k171b, g_kp1_171b]);
    let out = evaluate_sum(f171b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase171_sqrt4_k_log20_k_over_k_refused() {
    // √k^4·log(k)^20·k / k: effective_x2=6; 2·1=2 not > 6 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k171c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_171c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f171c = apply(sym(SUB), vec![g_k171c, g_kp1_171c]);
    let out = evaluate_sum(f171c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 172: 5-sqrt × 20-log × polynomial ──────────────────────────────────

#[test]
fn phase172_sqrt5_k_log20_k_over_k4_converges() {
    // √k^5·log(k)^20 / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k172a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_172a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f172a = apply(sym(SUB), vec![g_k172a, g_kp1_172a]);
    let out = evaluate_sum(f172a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase172_sqrt5_k_log20_k_times_k_over_k5_converges() {
    // √k^5·log(k)^20·k / k^5: effective_x2=7; 2·5=10 > 7 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k172b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_172b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f172b = apply(sym(SUB), vec![g_k172b, g_kp1_172b]);
    let out = evaluate_sum(f172b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase172_sqrt5_k_log20_k_over_k_refused() {
    // √k^5·log(k)^20·k / k: effective_x2=7; 2·1=2 not > 7 → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k172c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_172c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f172c = apply(sym(SUB), vec![g_k172c, g_kp1_172c]);
    let out = evaluate_sum(f172c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 173: 0-sqrt × 21-log × polynomial ──────────────────────────────────

#[test]
fn phase173_log21_k_over_k3_converges() {
    // log(k)^21 / k^3: effective_x2=0; 2·3=6 > 0 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k173a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_173a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f173a = apply(sym(SUB), vec![g_k173a, g_kp1_173a]);
    let out = evaluate_sum(f173a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase173_log21_k_times_k_over_k4_converges() {
    // log(k)^21·k / k^4: effective_x2=2; 2·4=8 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k173b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_173b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f173b = apply(sym(SUB), vec![g_k173b, g_kp1_173b]);
    let out = evaluate_sum(f173b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase173_log22_k_over_k_refused() {
    // log(k)^22·k / k: effective_x2=2; 2·1=2 not > 2 → refused (22 logs, not matched by phase173).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k173c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_173c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f173c = apply(sym(SUB), vec![g_k173c, g_kp1_173c]);
    let out = evaluate_sum(f173c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 174: 1-sqrt × 21-log × polynomial ──────────────────────────────────

#[test]
fn phase174_sqrt_k_log21_k_over_k3_converges() {
    // √k·log(k)^21 / k^3: effective_x2=1; 2·3=6 > 1 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k174a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_174a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f174a = apply(sym(SUB), vec![g_k174a, g_kp1_174a]);
    let out = evaluate_sum(f174a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase174_sqrt_k_log21_k_times_k_over_k4_converges() {
    // √k·log(k)^21·k / k^4: effective_x2=3; 2·4=8 > 3 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k174b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_174b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f174b = apply(sym(SUB), vec![g_k174b, g_kp1_174b]);
    let out = evaluate_sum(f174b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase174_sqrt_k_log22_k_times_k_over_k_refused() {
    // √k·log(k)^22·k / k: effective_x2=3; 2·1=2 not > 3 → refused (22 logs, not matched by phase174).
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k174c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_174c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f174c = apply(sym(SUB), vec![g_k174c, g_kp1_174c]);
    let out = evaluate_sum(f174c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 175: 2-sqrt × 21-log × polynomial ──────────────────────────────────

#[test]
fn phase175_sqrt2_k_log21_k_over_k3_converges() {
    // √k·√k·log(k)^21 / k^3: effective_x2=2; 2·3=6 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k175a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_175a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f175a = apply(sym(SUB), vec![g_k175a, g_kp1_175a]);
    let out = evaluate_sum(f175a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase175_sqrt2_k_log21_k_times_k_over_k4_converges() {
    // √k·√k·log(k)^21·k / k^4: effective_x2=4; 2·4=8 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k175b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_175b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f175b = apply(sym(SUB), vec![g_k175b, g_kp1_175b]);
    let out = evaluate_sum(f175b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase175_sqrt2_k_log22_k_over_k_refused() {
    // √k·√k·log(k)^22·k / k: effective_x2=4; 2·1=2 not > 4 → refused (22 logs, not matched by phase175).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k175c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_175c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f175c = apply(sym(SUB), vec![g_k175c, g_kp1_175c]);
    let out = evaluate_sum(f175c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 176: 3-sqrt × 21-log × polynomial ──────────────────────────────────

#[test]
fn phase176_sqrt3_k_log21_k_over_k3_converges() {
    // √k^3·log(k)^21 / k^3: effective_x2=3; 2·3=6 > 3 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k176a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_176a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f176a = apply(sym(SUB), vec![g_k176a, g_kp1_176a]);
    let out = evaluate_sum(f176a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase176_sqrt3_k_log21_k_times_k_over_k4_converges() {
    // √k^3·log(k)^21·k / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k176b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_176b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f176b = apply(sym(SUB), vec![g_k176b, g_kp1_176b]);
    let out = evaluate_sum(f176b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase176_sqrt3_k_log22_k_over_k_refused() {
    // √k^3·log(k)^22·k / k: effective_x2=5; 2·1=2 not > 5 → refused (22 logs, not matched by phase176).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k176c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_176c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f176c = apply(sym(SUB), vec![g_k176c, g_kp1_176c]);
    let out = evaluate_sum(f176c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 177: 4-sqrt × 21-log × polynomial ──────────────────────────────────

#[test]
fn phase177_sqrt4_k_log21_k_over_k3_converges() {
    // √k^4·log(k)^21 / k^3: effective_x2=4; 2·3=6 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k177a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_177a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f177a = apply(sym(SUB), vec![g_k177a, g_kp1_177a]);
    let out = evaluate_sum(f177a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase177_sqrt4_k_log21_k_times_k_over_k4_converges() {
    // √k^4·log(k)^21·k / k^4: effective_x2=6; 2·4=8 > 6 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k177b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_177b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f177b = apply(sym(SUB), vec![g_k177b, g_kp1_177b]);
    let out = evaluate_sum(f177b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase177_sqrt4_k_log22_k_over_k_refused() {
    // √k^4·log(k)^22·k / k: effective_x2=6; 2·1=2 not > 6 → refused (22 logs, not matched by phase177).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k177c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_177c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f177c = apply(sym(SUB), vec![g_k177c, g_kp1_177c]);
    let out = evaluate_sum(f177c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 178: 5-sqrt × 21-log × polynomial ──────────────────────────────────

#[test]
fn phase178_sqrt5_k_log21_k_over_k4_converges() {
    // √k^5·log(k)^21 / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k178a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_178a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f178a = apply(sym(SUB), vec![g_k178a, g_kp1_178a]);
    let out = evaluate_sum(f178a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase178_sqrt5_k_log21_k_times_k_over_k5_converges() {
    // √k^5·log(k)^21·k / k^5: effective_x2=7; 2·5=10 > 7 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k178b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_178b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f178b = apply(sym(SUB), vec![g_k178b, g_kp1_178b]);
    let out = evaluate_sum(f178b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase178_sqrt5_k_log22_k_over_k_refused() {
    // √k^5·log(k)^22·k / k: effective_x2=7; 2·1=2 not > 7 → refused (22 logs, not matched by phase178).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k178c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_178c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f178c = apply(sym(SUB), vec![g_k178c, g_kp1_178c]);
    let out = evaluate_sum(f178c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 179: 0-sqrt × 22-log × polynomial ──────────────────────────────────

#[test]
fn phase179_log22_k_over_k3_converges() {
    // log(k)^22 / k^3: effective_x2=0; 2·3=6 > 0 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k179a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_179a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f179a = apply(sym(SUB), vec![g_k179a, g_kp1_179a]);
    let out = evaluate_sum(f179a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase179_log22_k_times_k_over_k4_converges() {
    // log(k)^22·k / k^4: effective_x2=2; 2·4=8 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k179b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_179b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f179b = apply(sym(SUB), vec![g_k179b, g_kp1_179b]);
    let out = evaluate_sum(f179b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase179_log22_k_over_k_refused() {
    // log(k)^22·k / k: effective_x2=2; 2·1=2 not > 2 → refused (boundary: denom too small).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k179c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_179c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f179c = apply(sym(SUB), vec![g_k179c, g_kp1_179c]);
    let out = evaluate_sum(f179c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 180: 1-sqrt × 22-log × polynomial ──────────────────────────────────

#[test]
fn phase180_sqrt_k_log22_k_over_k3_converges() {
    // √k·log(k)^22 / k^3: effective_x2=1; 2·3=6 > 1 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k180a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_180a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f180a = apply(sym(SUB), vec![g_k180a, g_kp1_180a]);
    let out = evaluate_sum(f180a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase180_sqrt_k_log22_k_times_k_over_k4_converges() {
    // √k·log(k)^22·k / k^4: effective_x2=3; 2·4=8 > 3 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k180b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_180b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f180b = apply(sym(SUB), vec![g_k180b, g_kp1_180b]);
    let out = evaluate_sum(f180b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase180_sqrt_k_log22_k_over_k_refused() {
    // √k·log(k)^22·k / k: effective_x2=3; 2·1=2 not > 3 → refused (boundary: denom too small).
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k180c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_180c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f180c = apply(sym(SUB), vec![g_k180c, g_kp1_180c]);
    let out = evaluate_sum(f180c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 181: 2-sqrt × 22-log × polynomial ──────────────────────────────────

#[test]
fn phase181_sqrt2_k_log22_k_over_k3_converges() {
    // √k^2·log(k)^22 / k^3: effective_x2=2; 2·3=6 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k181a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_181a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f181a = apply(sym(SUB), vec![g_k181a, g_kp1_181a]);
    let out = evaluate_sum(f181a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase181_sqrt2_k_log22_k_times_k_over_k4_converges() {
    // √k^2·log(k)^22·k / k^4: effective_x2=4; 2·4=8 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k181b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_181b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f181b = apply(sym(SUB), vec![g_k181b, g_kp1_181b]);
    let out = evaluate_sum(f181b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase181_sqrt2_k_log22_k_over_k_refused() {
    // √k^2·log(k)^22·k / k: effective_x2=4; 2·1=2 not > 4 → refused (boundary: denom too small).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k181c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_181c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f181c = apply(sym(SUB), vec![g_k181c, g_kp1_181c]);
    let out = evaluate_sum(f181c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 182: 3-sqrt × 22-log × polynomial ──────────────────────────────────

#[test]
fn phase182_sqrt3_k_log22_k_over_k3_converges() {
    // √k^3·log(k)^22 / k^3: effective_x2=3; 2·3=6 > 3 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k182a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_182a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f182a = apply(sym(SUB), vec![g_k182a, g_kp1_182a]);
    let out = evaluate_sum(f182a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase182_sqrt3_k_log22_k_times_k_over_k4_converges() {
    // √k^3·log(k)^22·k / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k182b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_182b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f182b = apply(sym(SUB), vec![g_k182b, g_kp1_182b]);
    let out = evaluate_sum(f182b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase182_sqrt3_k_log22_k_over_k_refused() {
    // √k^3·log(k)^22·k / k: effective_x2=5; 2·1=2 not > 5 → refused (boundary: denom too small).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k182c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_182c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f182c = apply(sym(SUB), vec![g_k182c, g_kp1_182c]);
    let out = evaluate_sum(f182c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 183: 4-sqrt × 22-log × polynomial ──────────────────────────────────

#[test]
fn phase183_sqrt4_k_log22_k_over_k4_converges() {
    // √k^4·log(k)^22 / k^4: effective_x2=4; 2·4=8 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k183a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_183a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f183a = apply(sym(SUB), vec![g_k183a, g_kp1_183a]);
    let out = evaluate_sum(f183a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase183_sqrt4_k_log22_k_times_k_over_k5_converges() {
    // √k^4·log(k)^22·k / k^5: effective_x2=6; 2·5=10 > 6 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k183b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_183b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f183b = apply(sym(SUB), vec![g_k183b, g_kp1_183b]);
    let out = evaluate_sum(f183b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase183_sqrt4_k_log22_k_over_k_refused() {
    // √k^4·log(k)^22·k / k: effective_x2=6; 2·1=2 not > 6 → refused (boundary: denom too small).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k183c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_183c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f183c = apply(sym(SUB), vec![g_k183c, g_kp1_183c]);
    let out = evaluate_sum(f183c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 184: 5-sqrt × 22-log × polynomial ──────────────────────────────────

#[test]
fn phase184_sqrt5_k_log22_k_over_k4_converges() {
    // √k^5·log(k)^22 / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k184a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_184a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f184a = apply(sym(SUB), vec![g_k184a, g_kp1_184a]);
    let out = evaluate_sum(f184a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase184_sqrt5_k_log22_k_times_k_over_k5_converges() {
    // √k^5·log(k)^22·k / k^5: effective_x2=7; 2·5=10 > 7 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k184b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_184b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f184b = apply(sym(SUB), vec![g_k184b, g_kp1_184b]);
    let out = evaluate_sum(f184b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase184_sqrt5_k_log22_k_over_k_refused() {
    // √k^5·log(k)^22·k / k: effective_x2=7; 2·1=2 not > 7 → refused (boundary: denom too small).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let g_k184c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_184c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f184c = apply(sym(SUB), vec![g_k184c, g_kp1_184c]);
    let out = evaluate_sum(f184c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 185: 0-sqrt × 23-log × polynomial ──────────────────────────────────

#[test]
fn phase185_log23_k_over_k3_converges() {
    // log(k)^23 / k^3: effective_x2=0; 2·3=6 > 0 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k185a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_185a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f185a = apply(sym(SUB), vec![g_k185a, g_kp1_185a]);
    let out = evaluate_sum(f185a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase185_log23_k_times_k_over_k4_converges() {
    // log(k)^23·k / k^4: effective_x2=2; 2·4=8 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k185b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_185b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f185b = apply(sym(SUB), vec![g_k185b, g_kp1_185b]);
    let out = evaluate_sum(f185b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase185_log26_k_over_k_refused() {
    // log(k)^26·k / k: effective_x2=2; 2·1=2 not > 2 → refused (boundary: denom too small).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let g_k185c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_185c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f185c = apply(sym(SUB), vec![g_k185c, g_kp1_185c]);
    let out = evaluate_sum(f185c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 186: 1-sqrt × 23-log × polynomial ──────────────────────────────────

#[test]
fn phase186_sqrt_k_log23_k_over_k3_converges() {
    // √k·log(k)^23 / k^3: effective_x2=1; 2·3=6 > 1 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k186a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_186a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f186a = apply(sym(SUB), vec![g_k186a, g_kp1_186a]);
    let out = evaluate_sum(f186a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase186_sqrt_k_log23_k_times_k_over_k4_converges() {
    // √k·log(k)^23·k / k^4: effective_x2=3; 2·4=8 > 3 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k186b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_186b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f186b = apply(sym(SUB), vec![g_k186b, g_kp1_186b]);
    let out = evaluate_sum(f186b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase186_sqrt_k_log26_k_over_k_refused() {
    // √k·log(k)^26·k / k: effective_x2=3; 2·1=2 not > 3 → refused (boundary: denom too small).
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let g_k186c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_186c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f186c = apply(sym(SUB), vec![g_k186c, g_kp1_186c]);
    let out = evaluate_sum(f186c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 187: 2-sqrt × 23-log × polynomial ──────────────────────────────────

#[test]
fn phase187_sqrt2_k_log23_k_over_k3_converges() {
    // √k^2·log(k)^23 / k^3: effective_x2=2; 2·3=6 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k187a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_187a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f187a = apply(sym(SUB), vec![g_k187a, g_kp1_187a]);
    let out = evaluate_sum(f187a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase187_sqrt2_k_log23_k_times_k_over_k4_converges() {
    // √k^2·log(k)^23·k / k^4: effective_x2=4; 2·4=8 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k187b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_187b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f187b = apply(sym(SUB), vec![g_k187b, g_kp1_187b]);
    let out = evaluate_sum(f187b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase187_sqrt2_k_log26_k_over_k_refused() {
    // √k^2·log(k)^26·k / k: effective_x2=4; 2·1=2 not > 4 → refused (boundary: denom too small).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let g_k187c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_187c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f187c = apply(sym(SUB), vec![g_k187c, g_kp1_187c]);
    let out = evaluate_sum(f187c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 188: 3-sqrt × 23-log × polynomial ──────────────────────────────────

#[test]
fn phase188_sqrt3_k_log23_k_over_k3_converges() {
    // √k^3·log(k)^23 / k^3: effective_x2=3; 2·3=6 > 3 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k188a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_188a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f188a = apply(sym(SUB), vec![g_k188a, g_kp1_188a]);
    let out = evaluate_sum(f188a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase188_sqrt3_k_log23_k_times_k_over_k4_converges() {
    // √k^3·log(k)^23·k / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k188b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_188b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f188b = apply(sym(SUB), vec![g_k188b, g_kp1_188b]);
    let out = evaluate_sum(f188b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase188_sqrt3_k_log26_k_over_k_refused() {
    // √k^3·log(k)^26·k / k: effective_x2=5; 2·1=2 not > 5 → refused (boundary: denom too small).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let g_k188c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_188c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f188c = apply(sym(SUB), vec![g_k188c, g_kp1_188c]);
    let out = evaluate_sum(f188c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 189: 4-sqrt × 23-log × polynomial ──────────────────────────────────

#[test]
fn phase189_sqrt4_k_log23_k_over_k4_converges() {
    // √k^4·log(k)^23 / k^4: effective_x2=4; 2·4=8 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k189a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_189a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f189a = apply(sym(SUB), vec![g_k189a, g_kp1_189a]);
    let out = evaluate_sum(f189a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase189_sqrt4_k_log23_k_times_k_over_k5_converges() {
    // √k^4·log(k)^23·k / k^5: effective_x2=6; 2·5=10 > 6 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k189b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_189b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f189b = apply(sym(SUB), vec![g_k189b, g_kp1_189b]);
    let out = evaluate_sum(f189b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase189_sqrt4_k_log26_k_over_k_refused() {
    // √k^4·log(k)^26·k / k: effective_x2=6; 2·1=2 not > 6 → refused (boundary: denom too small).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let g_k189c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_189c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f189c = apply(sym(SUB), vec![g_k189c, g_kp1_189c]);
    let out = evaluate_sum(f189c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 190: 5-sqrt × 23-log × polynomial ──────────────────────────────────

#[test]
fn phase190_sqrt5_k_log23_k_over_k4_converges() {
    // √k^5·log(k)^23 / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k190a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_190a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f190a = apply(sym(SUB), vec![g_k190a, g_kp1_190a]);
    let out = evaluate_sum(f190a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase190_sqrt5_k_log23_k_times_k_over_k5_converges() {
    // √k^5·log(k)^23·k / k^5: effective_x2=7; 2·5=10 > 7 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k190b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_190b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f190b = apply(sym(SUB), vec![g_k190b, g_kp1_190b]);
    let out = evaluate_sum(f190b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase190_sqrt5_k_log26_k_over_k_refused() {
    // √k^5·log(k)^26·k / k: effective_x2=7; 2·1=2 not > 7 → refused (boundary: denom too small).
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let g_k190c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_190c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f190c = apply(sym(SUB), vec![g_k190c, g_kp1_190c]);
    let out = evaluate_sum(f190c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 191: 0-sqrt × 24-log × polynomial ──────────────────────────────────

#[test]
fn phase191_log24_k_over_k2_converges() {
    // log(k)^24 / k^2: effective_x2=0; 2·2=4 > 0 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let g_k191a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_191a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f191a = apply(sym(SUB), vec![g_k191a, g_kp1_191a]);
    let out = evaluate_sum(f191a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase191_log24_k_times_k_over_k3_converges() {
    // log(k)^24·k / k^3: effective_x2=2; 2·3=6 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k191b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_191b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f191b = apply(sym(SUB), vec![g_k191b, g_kp1_191b]);
    let out = evaluate_sum(f191b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase191_log26_k_times_k_over_k2_refused() {
    // log(k)^26·k / k^2: effective_x2=2; 2·2=4 not > 2 with 26 logs → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let g_k191c = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_191c = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f191c = apply(sym(SUB), vec![g_k191c, g_kp1_191c]);
    let out = evaluate_sum(f191c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 192: 1-sqrt × 24-log × polynomial ──────────────────────────────────

#[test]
fn phase192_sqrt_k_log24_k_over_k3_converges() {
    // √k·log(k)^24 / k^3: effective_x2=1; 2·3=6 > 1 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k192a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_192a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f192a = apply(sym(SUB), vec![g_k192a, g_kp1_192a]);
    let out = evaluate_sum(f192a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase192_sqrt_k_log24_k_times_k_over_k4_converges() {
    // √k·log(k)^24·k / k^4: effective_x2=3; 2·4=8 > 3 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k192b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_192b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f192b = apply(sym(SUB), vec![g_k192b, g_kp1_192b]);
    let out = evaluate_sum(f192b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase192_sqrt_k_log26_k_times_k_over_k2_refused() {
    // √k·log(k)^26·k / k^2: 26 logs → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let g_k192c = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_192c = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f192c = apply(sym(SUB), vec![g_k192c, g_kp1_192c]);
    let out = evaluate_sum(f192c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 193: 2-sqrt × 24-log × polynomial ──────────────────────────────────

#[test]
fn phase193_sqrt2_k_log24_k_over_k3_converges() {
    // √k^2·log(k)^24 / k^3: effective_x2=2; 2·3=6 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k193a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_193a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f193a = apply(sym(SUB), vec![g_k193a, g_kp1_193a]);
    let out = evaluate_sum(f193a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase193_sqrt2_k_log24_k_times_k_over_k4_converges() {
    // √k^2·log(k)^24·k / k^4: effective_x2=4; 2·4=8 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k193b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_193b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f193b = apply(sym(SUB), vec![g_k193b, g_kp1_193b]);
    let out = evaluate_sum(f193b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase193_sqrt2_k_log26_k_over_k_refused() {
    // √k^2·log(k)^26·k / k: 26 logs → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let g_k193c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_193c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f193c = apply(sym(SUB), vec![g_k193c, g_kp1_193c]);
    let out = evaluate_sum(f193c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 194: 3-sqrt × 24-log × polynomial ──────────────────────────────────

#[test]
fn phase194_sqrt3_k_log24_k_over_k3_converges() {
    // √k^3·log(k)^24 / k^3: effective_x2=3; 2·3=6 > 3 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k194a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_194a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f194a = apply(sym(SUB), vec![g_k194a, g_kp1_194a]);
    let out = evaluate_sum(f194a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase194_sqrt3_k_log24_k_times_k_over_k4_converges() {
    // √k^3·log(k)^24·k / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k194b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_194b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f194b = apply(sym(SUB), vec![g_k194b, g_kp1_194b]);
    let out = evaluate_sum(f194b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase194_sqrt3_k_log26_k_over_k_refused() {
    // √k^3·log(k)^26·k / k: 26 logs → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let g_k194c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_194c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f194c = apply(sym(SUB), vec![g_k194c, g_kp1_194c]);
    let out = evaluate_sum(f194c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 195: 4-sqrt × 24-log × polynomial ──────────────────────────────────

#[test]
fn phase195_sqrt4_k_log24_k_over_k4_converges() {
    // √k^4·log(k)^24 / k^4: effective_x2=4; 2·4=8 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k195a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_195a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f195a = apply(sym(SUB), vec![g_k195a, g_kp1_195a]);
    let out = evaluate_sum(f195a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase195_sqrt4_k_log24_k_times_k_over_k5_converges() {
    // √k^4·log(k)^24·k / k^5: effective_x2=6; 2·5=10 > 6 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k195b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_195b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f195b = apply(sym(SUB), vec![g_k195b, g_kp1_195b]);
    let out = evaluate_sum(f195b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase195_sqrt4_k_log26_k_over_k_refused() {
    // √k^4·log(k)^26·k / k: 26 logs → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let g_k195c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_195c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f195c = apply(sym(SUB), vec![g_k195c, g_kp1_195c]);
    let out = evaluate_sum(f195c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 196: 5-sqrt × 24-log × polynomial ──────────────────────────────────

#[test]
fn phase196_sqrt5_k_log24_k_over_k4_converges() {
    // √k^5·log(k)^24 / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k196a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_196a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f196a = apply(sym(SUB), vec![g_k196a, g_kp1_196a]);
    let out = evaluate_sum(f196a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase196_sqrt5_k_log24_k_times_k_over_k5_converges() {
    // √k^5·log(k)^24·k / k^5: effective_x2=7; 2·5=10 > 7 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k,
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1,
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k196b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_196b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f196b = apply(sym(SUB), vec![g_k196b, g_kp1_196b]);
    let out = evaluate_sum(f196b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase196_sqrt5_k_log26_k_over_k_refused() {
    // √k^5·log(k)^26·k / k: 26 logs → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26th log
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26th log
        kp1.clone(),
    ]);
    let g_k196c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_196c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f196c = apply(sym(SUB), vec![g_k196c, g_kp1_196c]);
    let out = evaluate_sum(f196c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 197: 0-sqrt × 25-log × polynomial ──────────────────────────────────

#[test]
fn phase197_log25_k_over_k2_converges() {
    // log(k)^25 / k^2: effective_x2=0; 2·2=4 > 0 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let g_k197a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_197a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f197a = apply(sym(SUB), vec![g_k197a, g_kp1_197a]);
    let out = evaluate_sum(f197a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase197_log25_k_times_k_over_k3_converges() {
    // log(k)^25·k / k^3: effective_x2=2; 2·3=6 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k197b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_197b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f197b = apply(sym(SUB), vec![g_k197b, g_kp1_197b]);
    let out = evaluate_sum(f197b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase197_log26_k_times_k_over_k2_refused() {
    // log(k)^26·k / k^2: 26 logs → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26 logs
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone()]);
    let g_k197c = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_197c = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f197c = apply(sym(SUB), vec![g_k197c, g_kp1_197c]);
    let out = evaluate_sum(f197c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 198: 1-sqrt × 25-log × polynomial ──────────────────────────────────

#[test]
fn phase198_sqrt_k_log25_k_over_k3_converges() {
    // √k·log(k)^25 / k^3: effective_x2=1; 2·3=6 > 1 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k198a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_198a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f198a = apply(sym(SUB), vec![g_k198a, g_kp1_198a]);
    let out = evaluate_sum(f198a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase198_sqrt_k_log25_k_times_k_over_k4_converges() {
    // √k·log(k)^25·k / k^4: effective_x2=3; 2·4=8 > 3 → converges.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k198b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_198b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f198b = apply(sym(SUB), vec![g_k198b, g_kp1_198b]);
    let out = evaluate_sum(f198b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase198_sqrt_k_log26_k_over_k_refused() {
    // √k·log(k)^26·k / k: 26 logs → refused.
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
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26 logs
        kp1.clone(),
    ]);
    let g_k198c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_198c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f198c = apply(sym(SUB), vec![g_k198c, g_kp1_198c]);
    let out = evaluate_sum(f198c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 199: 2-sqrt × 25-log × polynomial ──────────────────────────────────

#[test]
fn phase199_sqrt2_k_log25_k_over_k3_converges() {
    // √k^2·log(k)^25 / k^3: effective_x2=2; 2·3=6 > 2 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k199a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_199a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f199a = apply(sym(SUB), vec![g_k199a, g_kp1_199a]);
    let out = evaluate_sum(f199a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase199_sqrt2_k_log25_k_times_k_over_k4_converges() {
    // √k^2·log(k)^25·k / k^4: effective_x2=4; 2·4=8 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k199b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_199b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f199b = apply(sym(SUB), vec![g_k199b, g_kp1_199b]);
    let out = evaluate_sum(f199b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase199_sqrt2_k_log26_k_over_k_refused() {
    // √k^2·log(k)^26·k / k: 26 logs → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26 logs
        kp1.clone(),
    ]);
    let g_k199c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_199c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f199c = apply(sym(SUB), vec![g_k199c, g_kp1_199c]);
    let out = evaluate_sum(f199c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 200: 3-sqrt × 25-log × polynomial ──────────────────────────────────

#[test]
fn phase200_sqrt3_k_log25_k_over_k3_converges() {
    // √k^3·log(k)^25 / k^3: effective_x2=3; 2·3=6 > 3 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k200a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_200a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f200a = apply(sym(SUB), vec![g_k200a, g_kp1_200a]);
    let out = evaluate_sum(f200a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase200_sqrt3_k_log25_k_times_k_over_k4_converges() {
    // √k^3·log(k)^25·k / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k200b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_200b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f200b = apply(sym(SUB), vec![g_k200b, g_kp1_200b]);
    let out = evaluate_sum(f200b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase200_sqrt3_k_log26_k_over_k_refused() {
    // √k^3·log(k)^26·k / k: 26 logs → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26 logs
        kp1.clone(),
    ]);
    let g_k200c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_200c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f200c = apply(sym(SUB), vec![g_k200c, g_kp1_200c]);
    let out = evaluate_sum(f200c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 201: 4-sqrt × 25-log × polynomial ──────────────────────────────────

#[test]
fn phase201_sqrt4_k_log25_k_over_k4_converges() {
    // √k^4·log(k)^25 / k^4: effective_x2=4; 2·4=8 > 4 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k201a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_201a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f201a = apply(sym(SUB), vec![g_k201a, g_kp1_201a]);
    let out = evaluate_sum(f201a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase201_sqrt4_k_log25_k_times_k_over_k5_converges() {
    // √k^4·log(k)^25·k / k^5: effective_x2=6; 2·5=10 > 6 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k201b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_201b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f201b = apply(sym(SUB), vec![g_k201b, g_kp1_201b]);
    let out = evaluate_sum(f201b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase201_sqrt4_k_log26_k_over_k_refused() {
    // √k^4·log(k)^26·k / k: 26 logs → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26 logs
        kp1.clone(),
    ]);
    let g_k201c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_201c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f201c = apply(sym(SUB), vec![g_k201c, g_kp1_201c]);
    let out = evaluate_sum(f201c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

// ── Phase 202: 5-sqrt × 25-log × polynomial ──────────────────────────────────

#[test]
fn phase202_sqrt5_k_log25_k_over_k4_converges() {
    // √k^5·log(k)^25 / k^4: effective_x2=5; 2·4=8 > 5 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k202a = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_202a = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f202a = apply(sym(SUB), vec![g_k202a, g_kp1_202a]);
    let out = evaluate_sum(f202a, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase202_sqrt5_k_log25_k_times_k_over_k5_converges() {
    // √k^5·log(k)^25·k / k^5: effective_x2=7; 2·5=10 > 7 → converges.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 25 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 25 logs
        kp1.clone(),
    ]);
    let den_k = apply(sym(MUL), vec![k.clone(), k.clone(), k.clone(), k.clone(), k.clone()]);
    let den_kp1 = apply(sym(MUL), vec![kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone(), kp1.clone()]);
    let g_k202b = apply(sym(DIV), vec![num_k, den_k]);
    let g_kp1_202b = apply(sym(DIV), vec![num_kp1, den_kp1]);
    let f202b = apply(sym(SUB), vec![g_k202b, g_kp1_202b]);
    let out = evaluate_sum(f202b, k, int(1), sym("%inf"), eval);
    assert!(!matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

#[test]
fn phase202_sqrt5_k_log26_k_over_k_refused() {
    // √k^5·log(k)^26·k / k: 26 logs → refused.
    let k = sym("k");
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let sqrt_k = apply(sym("Sqrt"), vec![k.clone()]);
    let sqrt_kp1 = apply(sym("Sqrt"), vec![kp1.clone()]);
    let log_k = apply(sym(LOG), vec![k.clone()]);
    let log_kp1 = apply(sym(LOG), vec![kp1.clone()]);
    let num_k = apply(sym(MUL), vec![
        sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k.clone(), sqrt_k,
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(),
        log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k.clone(), log_k, // 26 logs
        k.clone(),
    ]);
    let num_kp1 = apply(sym(MUL), vec![
        sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1.clone(), sqrt_kp1,
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(),
        log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1.clone(), log_kp1, // 26 logs
        kp1.clone(),
    ]);
    let g_k202c = apply(sym(DIV), vec![num_k, k.clone()]);
    let g_kp1_202c = apply(sym(DIV), vec![num_kp1, kp1.clone()]);
    let f202c = apply(sym(SUB), vec![g_k202c, g_kp1_202c]);
    let out = evaluate_sum(f202c, k, int(1), sym("%inf"), eval);
    assert!(matches!(&out, IRNode::Apply(node) if node.head == sym(SUM)));
}

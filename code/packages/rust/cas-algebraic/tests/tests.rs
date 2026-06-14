use cas_algebraic::{
    alg_factor_handler, alg_factor_handler_with_eval, alg_factor_ir,
    build_alg_factor_handler_table, extract_radical_d, factor_over_extension, rational_square_root,
    try_split_depressed_quartic, try_split_quadratic, Rational, ALG_FACTOR,
};
use symbolic_ir::{apply, int, rat, sym, IRApply, IRNode, ADD, MUL, POW, SQRT, SUB};

fn sqrt_d(d: i64) -> symbolic_ir::IRNode {
    apply(sym(SQRT), vec![int(d)])
}

fn alg_factor_apply(args: Vec<IRNode>) -> IRApply {
    IRApply {
        head: sym(ALG_FACTOR),
        args,
    }
}

#[test]
fn rational_square_root_detects_exact_squares() {
    assert_eq!(
        rational_square_root(Rational::from_int(4)),
        Some(Rational::from_int(2))
    );
    assert_eq!(
        rational_square_root(Rational::new(1, 4)),
        Some(Rational::new(1, 2))
    );
    assert_eq!(rational_square_root(Rational::from_int(2)), None);
    assert_eq!(rational_square_root(Rational::from_int(-1)), None);
}

#[test]
fn x2_minus_d_splits_over_matching_extension() {
    let result = try_split_quadratic(&[-2, 0, 1], 2).expect("x^2 - 2 should split");
    assert_eq!(result.len(), 2);

    let radicals = [result[0][0].radical, result[1][0].radical];
    assert!(radicals.contains(&Rational::from_int(-1)));
    assert!(radicals.contains(&Rational::from_int(1)));
}

#[test]
fn quadratic_with_wrong_discriminant_does_not_split() {
    assert!(try_split_quadratic(&[2, 0, 1], 2).is_none());
    assert!(try_split_quadratic(&[1, 1, 1], 2).is_none());
}

#[test]
fn x4_plus_one_splits_over_sqrt_two() {
    let result =
        factor_over_extension(&[1, 0, 0, 0, 1], 2).expect("x^4 + 1 should split over sqrt(2)");
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].len(), 3);
    assert_eq!(result[1].len(), 3);

    let radicals = [result[0][1].radical, result[1][1].radical];
    assert!(radicals.contains(&Rational::from_int(-1)));
    assert!(radicals.contains(&Rational::from_int(1)));
}

#[test]
fn x4_plus_one_does_not_split_over_sqrt_three() {
    assert!(factor_over_extension(&[1, 0, 0, 0, 1], 3).is_none());
}

#[test]
fn depressed_quartic_checks_shape() {
    assert!(try_split_depressed_quartic(&[1, 0, 0, 1, 1], 2).is_none());
    assert!(try_split_depressed_quartic(&[2, 0, 0, 0, 1], 2).is_none());
}

#[test]
fn keeps_rational_factors_when_residual_splits() {
    // (x - 1) * (x^2 - 2) should keep the rational linear factor and split
    // the quadratic over Q[sqrt(2)].
    let result = factor_over_extension(&[2, -2, -1, 1], 2).expect("residual should split");
    assert_eq!(result.len(), 3);
    assert!(result
        .iter()
        .any(|factor| factor.len() == 2 && factor[0].rational == Rational::from_int(-1)));
}

#[test]
fn extract_radical_rejects_trivial_or_malformed_extensions() {
    assert_eq!(extract_radical_d(&sqrt_d(2)), Some(2));
    assert_eq!(extract_radical_d(&sqrt_d(4)), None);
    assert_eq!(extract_radical_d(&int(2)), None);
}

#[test]
fn ir_adapter_factors_pow_polynomial() {
    let x = sym("x");
    let x2_minus_2 = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(2)],
    );
    let result = alg_factor_ir(&x2_minus_2, &sqrt_d(2), &x).expect("IR should factor");
    assert!(matches!(result, symbolic_ir::IRNode::Apply(_)));
}

#[test]
fn ir_adapter_factors_nested_mul_polynomial() {
    let x = sym("x");
    let x4 = apply(sym(MUL), vec![x.clone(), x.clone(), x.clone(), x.clone()]);
    let poly = apply(sym(ADD), vec![x4, int(1)]);
    let result = alg_factor_ir(&poly, &sqrt_d(2), &x).expect("IR should factor");
    assert!(matches!(result, symbolic_ir::IRNode::Apply(_)));
}

#[test]
fn ir_adapter_returns_none_for_non_polynomial() {
    let x = sym("x");
    let sin_x = apply(sym("Sin"), vec![x.clone()]);
    assert!(alg_factor_ir(&sin_x, &sqrt_d(2), &x).is_none());
}

#[test]
fn handler_table_registers_alg_factor() {
    let table = build_alg_factor_handler_table();
    assert!(table.contains_key(ALG_FACTOR));
}

#[test]
fn handler_returns_original_for_wrong_arity() {
    let expr = alg_factor_apply(vec![int(1)]);
    let original = IRNode::Apply(Box::new(expr.clone()));

    assert_eq!(alg_factor_handler(&expr), original);
}

#[test]
fn handler_returns_original_for_non_sqrt_extension() {
    let x = sym("x");
    let poly = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(2)],
    );
    let expr = alg_factor_apply(vec![poly, int(2)]);
    let original = IRNode::Apply(Box::new(expr.clone()));

    assert_eq!(alg_factor_handler(&expr), original);
}

#[test]
fn handler_returns_original_for_non_polynomial() {
    let x = sym("x");
    let sin_x = apply(sym("Sin"), vec![x.clone()]);
    let expr = alg_factor_apply(vec![sin_x, sqrt_d(2)]);
    let original = IRNode::Apply(Box::new(expr.clone()));

    assert_eq!(alg_factor_handler(&expr), original);
}

#[test]
fn handler_returns_original_for_irreducible_polynomial() {
    let x = sym("x");
    let x2_plus_1 = apply(
        sym(ADD),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(1)],
    );
    let expr = alg_factor_apply(vec![x2_plus_1, sqrt_d(2)]);
    let original = IRNode::Apply(Box::new(expr.clone()));

    assert_eq!(alg_factor_handler(&expr), original);
}

#[test]
fn handler_factors_x2_minus_2_over_sqrt_two() {
    let x = sym("x");
    let x2_minus_2 = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(2)],
    );
    let expr = alg_factor_apply(vec![x2_minus_2, sqrt_d(2)]);

    let result = alg_factor_handler(&expr);

    match result {
        IRNode::Apply(apply) => assert_eq!(apply.head, sym(MUL)),
        other => panic!("expected product IR, got {other:?}"),
    }
}

#[test]
fn handler_clears_rational_coefficients_before_factoring() {
    let x = sym("x");
    let half_x2 = apply(
        sym(MUL),
        vec![rat(1, 2), apply(sym(POW), vec![x.clone(), int(2)])],
    );
    let poly = apply(sym(SUB), vec![half_x2, int(1)]);
    let expr = alg_factor_apply(vec![poly, sqrt_d(2)]);

    let result = alg_factor_handler(&expr);

    match result {
        IRNode::Apply(apply) => assert_eq!(apply.head, sym(MUL)),
        other => panic!("expected product IR after clearing denominators, got {other:?}"),
    }
}

#[test]
fn handler_evaluates_polynomial_before_conversion() {
    let x = sym("x");
    let normalized = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(2)],
    );
    let expr = alg_factor_apply(vec![apply(sym("NormalizeMe"), vec![x]), sqrt_d(2)]);
    let original = IRNode::Apply(Box::new(expr.clone()));

    assert_eq!(alg_factor_handler(&expr), original);

    let result = alg_factor_handler_with_eval(&expr, |_| normalized);

    match result {
        IRNode::Apply(apply) => assert_eq!(apply.head, sym(MUL)),
        other => panic!("expected product IR after eval callback, got {other:?}"),
    }
}

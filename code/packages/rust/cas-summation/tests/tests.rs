use cas_summation::{
    evaluate_product, evaluate_product_expr, evaluate_sum, faulhaber_ir, geometric_sum_ir,
    poly_sum_ir, rational_value, try_special_infinite, Rational, GAMMA_FUNC, PRODUCT, SUM,
};
use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, DIV, EXP, MUL, NEG, POW, SUB};

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

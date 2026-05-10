use cas_mnewton::{ir_to_float, mnewton_solve, MNewtonError, MNewtonOptions};
use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, COS, MUL, POW, SIN, SUB};

fn eval(node: IRNode) -> IRNode {
    match node {
        IRNode::Apply(apply_node) => {
            let head = apply_node.head.clone();
            let args: Vec<IRNode> = apply_node.args.into_iter().map(eval).collect();
            let name = match &head {
                IRNode::Symbol(name) => name.as_str(),
                _ => return IRNode::Apply(Box::new(symbolic_ir::IRApply { head, args })),
            };
            match (name, args.as_slice()) {
                (ADD, [a, b]) => numeric_binary(a, b, |x, y| x + y),
                (SUB, [a, b]) => numeric_binary(a, b, |x, y| x - y),
                (MUL, [a, b]) => numeric_binary(a, b, |x, y| x * y),
                (POW, [a, b]) => numeric_binary(a, b, |x, y| x.powf(y)),
                (SIN, [a]) => ir_to_float(a).map(|v| IRNode::Float(v.sin())),
                (COS, [a]) => ir_to_float(a).map(|v| IRNode::Float(v.cos())),
                _ => None,
            }
            .unwrap_or_else(|| IRNode::Apply(Box::new(symbolic_ir::IRApply { head, args })))
        }
        other => other,
    }
}

fn numeric_binary(a: &IRNode, b: &IRNode, op: impl FnOnce(f64, f64) -> f64) -> Option<IRNode> {
    Some(IRNode::Float(op(ir_to_float(a)?, ir_to_float(b)?)))
}

fn diff(node: &IRNode, var: &IRNode) -> IRNode {
    if node == var {
        return int(1);
    }
    match node {
        IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_) => int(0),
        IRNode::Symbol(_) => int(0),
        IRNode::Apply(apply_node) => {
            let head_name = match &apply_node.head {
                IRNode::Symbol(name) => name.as_str(),
                _ => return int(0),
            };
            match (head_name, apply_node.args.as_slice()) {
                (ADD, [a, b]) => apply(sym(ADD), vec![diff(a, var), diff(b, var)]),
                (SUB, [a, b]) => apply(sym(SUB), vec![diff(a, var), diff(b, var)]),
                (MUL, [a, b]) => apply(
                    sym(ADD),
                    vec![
                        apply(sym(MUL), vec![diff(a, var), b.clone()]),
                        apply(sym(MUL), vec![a.clone(), diff(b, var)]),
                    ],
                ),
                (POW, [base, IRNode::Integer(exp)]) if *exp >= 1 => {
                    let coefficient = int(*exp);
                    let lowered = if *exp == 1 {
                        int(1)
                    } else {
                        apply(sym(POW), vec![base.clone(), int(exp - 1)])
                    };
                    apply(
                        sym(MUL),
                        vec![coefficient, apply(sym(MUL), vec![lowered, diff(base, var)])],
                    )
                }
                (SIN, [arg]) => apply(
                    sym(MUL),
                    vec![apply(sym(COS), vec![arg.clone()]), diff(arg, var)],
                ),
                _ => int(0),
            }
        }
        IRNode::Str(_) => int(0),
    }
}

fn solve(f: IRNode, x0: IRNode) -> IRNode {
    let x = sym("x");
    mnewton_solve(&f, &x, &x0, eval, diff, MNewtonOptions::default()).unwrap()
}

fn assert_close(node: IRNode, expected: f64, tol: f64) {
    let actual = ir_to_float(&node).expect("expected numeric result");
    assert!(
        (actual - expected).abs() < tol,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn ir_to_float_accepts_numeric_literals() {
    assert_eq!(ir_to_float(&int(3)), Some(3.0));
    assert_eq!(ir_to_float(&IRNode::Float(1.5)), Some(1.5));
    assert_eq!(ir_to_float(&rat(1, 2)), Some(0.5));
    assert_eq!(ir_to_float(&sym("x")), None);
    assert_eq!(ir_to_float(&apply(sym(ADD), vec![int(1), int(2)])), None);
}

#[test]
fn solves_linear_functions() {
    let x = sym("x");
    assert_close(
        solve(apply(sym(SUB), vec![x.clone(), int(2)]), IRNode::Float(0.0)),
        2.0,
        1e-9,
    );
    assert_close(solve(apply(sym(SUB), vec![x, int(7)]), int(0)), 7.0, 1e-9);
}

#[test]
fn solves_quadratic_roots_from_starting_side() {
    let x = sym("x");
    let f = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(4)],
    );
    assert_close(solve(f.clone(), IRNode::Float(3.0)), 2.0, 1e-9);
    assert_close(solve(f, IRNode::Float(-3.0)), -2.0, 1e-9);
}

#[test]
fn solves_sqrt_two_and_cubic() {
    let x = sym("x");
    let sqrt_two = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(2)],
    );
    assert_close(solve(sqrt_two, IRNode::Float(1.5)), 2.0_f64.sqrt(), 1e-8);

    let cubic = apply(sym(SUB), vec![apply(sym(POW), vec![x, int(3)]), int(8)]);
    assert_close(solve(cubic, IRNode::Float(1.0)), 2.0, 1e-8);
}

#[test]
fn accepts_rational_initial_guess() {
    let x = sym("x");
    let f = apply(sym(SUB), vec![x, int(2)]);
    assert_close(solve(f, rat(3, 2)), 2.0, 1e-9);
}

#[test]
fn returns_initial_guess_when_already_at_root() {
    let x = sym("x");
    let f = apply(sym(SUB), vec![x, int(5)]);
    assert_eq!(solve(f, IRNode::Float(5.0)), IRNode::Float(5.0));
}

#[test]
fn returns_original_expression_for_symbolic_initial_guess_or_non_numeric_eval() {
    let x = sym("x");
    let y = sym("y");
    let f = apply(sym(SUB), vec![x.clone(), int(2)]);
    let out = mnewton_solve(&f, &x, &sym("a"), eval, diff, MNewtonOptions::default()).unwrap();
    assert_eq!(out, f);

    let f_with_extra_symbol = apply(sym(ADD), vec![x.clone(), y]);
    let out = mnewton_solve(
        &f_with_extra_symbol,
        &x,
        &IRNode::Float(1.0),
        eval,
        diff,
        MNewtonOptions::default(),
    )
    .unwrap();
    assert_eq!(out, f_with_extra_symbol);
}

#[test]
fn reports_zero_derivative_before_newton_step() {
    let x = sym("x");
    let f = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(1)],
    );
    let err = mnewton_solve(
        &f,
        &x,
        &IRNode::Float(0.0),
        eval,
        diff,
        MNewtonOptions::default(),
    )
    .unwrap_err();
    assert_eq!(err, MNewtonError::ZeroDerivative { x: 0.0 });
}

#[test]
fn honors_custom_tolerance_and_max_iter() {
    let x = sym("x");
    let f = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(2)],
    );
    let out = mnewton_solve(
        &f,
        &x,
        &IRNode::Float(1.5),
        eval,
        diff,
        MNewtonOptions {
            tol: 1e-4,
            max_iter: 50,
        },
    )
    .unwrap();
    assert_close(out, 2.0_f64.sqrt(), 1e-3);
}

#[test]
fn solves_sin_root_near_pi() {
    let x = sym("x");
    let f = apply(sym(SIN), vec![x]);
    assert_close(solve(f, IRNode::Float(3.0)), std::f64::consts::PI, 1e-8);
}

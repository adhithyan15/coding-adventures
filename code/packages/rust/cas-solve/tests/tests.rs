// Integration tests for cas-solve.
//
// Mirrors the Python reference tests in
// code/packages/python/cas-solve/tests/.

use cas_solve::frac::Frac;
use cas_solve::{
    nsolve_fraction_poly, nsolve_poly, roots_to_ir, solve_cubic, solve_linear, solve_linear_system,
    solve_quadratic, solve_quartic, Complex, SolveResult,
};
use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, EQUAL, MUL, POW, RULE, SUB};

fn frac(n: i64, d: i64) -> Frac {
    Frac::new(n, d)
}
fn fi(n: i64) -> Frac {
    Frac::from_int(n)
}

fn c(re: f64, im: f64) -> Complex {
    Complex::new(re, im)
}

fn assert_numeric_roots_close(actual: &[Complex], expected: &[Complex]) {
    assert_eq!(actual.len(), expected.len());
    let mut used = vec![false; actual.len()];
    for want in expected {
        let mut matched = false;
        for (i, got) in actual.iter().enumerate() {
            if used[i] {
                continue;
            }
            let distance = ((*got - *want).abs()).min((*got - c(want.re, -want.im)).abs());
            if distance < 1e-7 {
                used[i] = true;
                matched = true;
                break;
            }
        }
        assert!(matched, "missing root {want:?} in {actual:?}");
    }
}

fn eq(lhs: IRNode, rhs: IRNode) -> IRNode {
    apply(sym(EQUAL), vec![lhs, rhs])
}

fn add(args: Vec<IRNode>) -> IRNode {
    apply(sym(ADD), args)
}

fn sub(lhs: IRNode, rhs: IRNode) -> IRNode {
    apply(sym(SUB), vec![lhs, rhs])
}

fn mul(args: Vec<IRNode>) -> IRNode {
    apply(sym(MUL), args)
}

fn pow(base: IRNode, exponent: IRNode) -> IRNode {
    apply(sym(POW), vec![base, exponent])
}

fn rule_value(rules: &[IRNode], variable: &IRNode) -> IRNode {
    rules
        .iter()
        .find_map(|rule| match rule {
            IRNode::Apply(apply_node)
                if apply_node.head == sym(RULE) && apply_node.args.first() == Some(variable) =>
            {
                Some(apply_node.args[1].clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing rule for {variable:?}"))
}

// ---------------------------------------------------------------------------
// solve_linear
// ---------------------------------------------------------------------------

#[test]
fn linear_basic() {
    // 2x + 3 = 0 → x = -3/2
    let r = solve_linear(fi(2), fi(3));
    assert_eq!(r, SolveResult::Solutions(vec![rat(-3, 2)]));
}

#[test]
fn linear_integer_solution() {
    // x - 5 = 0 → x = 5
    let r = solve_linear(fi(1), fi(-5));
    assert_eq!(r, SolveResult::Solutions(vec![int(5)]));
}

#[test]
fn linear_no_solution() {
    // 0x + 5 = 0 → no solution
    let r = solve_linear(fi(0), fi(5));
    assert_eq!(r, SolveResult::Solutions(vec![]));
}

#[test]
fn linear_all_solutions() {
    // 0x + 0 = 0 → ALL
    let r = solve_linear(fi(0), fi(0));
    assert_eq!(r, SolveResult::All);
}

#[test]
fn linear_zero_constant() {
    // 3x = 0 → x = 0
    let r = solve_linear(fi(3), fi(0));
    assert_eq!(r, SolveResult::Solutions(vec![int(0)]));
}

#[test]
fn linear_rational_coefficients() {
    // (1/2)x + (1/4) = 0 → x = -1/2
    let r = solve_linear(frac(1, 2), frac(1, 4));
    assert_eq!(r, SolveResult::Solutions(vec![rat(-1, 2)]));
}

// ---------------------------------------------------------------------------
// solve_quadratic
// ---------------------------------------------------------------------------

#[test]
fn quadratic_two_distinct_real_roots() {
    // x^2 - 5x + 6 = 0 → {2, 3}
    let r = solve_quadratic(fi(1), fi(-5), fi(6));
    assert_eq!(r, SolveResult::Solutions(vec![int(2), int(3)]));
}

#[test]
fn quadratic_double_root() {
    // x^2 - 4x + 4 = 0 → x = 2 (repeated)
    let r = solve_quadratic(fi(1), fi(-4), fi(4));
    assert_eq!(r, SolveResult::Solutions(vec![int(2)]));
}

#[test]
fn quadratic_complex_roots() {
    // x^2 + 1 = 0 → ±i
    let r = solve_quadratic(fi(1), fi(0), fi(1));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 2);
            // Both roots should contain %i symbol somewhere.
            let text = format!("{roots:?}");
            assert!(text.contains("%i"), "expected %i in {text}");
        }
        SolveResult::All => panic!("expected Solutions, got All"),
    }
}

#[test]
fn quadratic_zero_leading_falls_back_to_linear() {
    // 0x^2 + 2x + 4 = 0 → x = -2
    let r = solve_quadratic(fi(0), fi(2), fi(4));
    assert_eq!(r, SolveResult::Solutions(vec![int(-2)]));
}

#[test]
fn quadratic_irrational_discriminant() {
    // x^2 - 2 = 0 → roots involve Sqrt(2)
    let r = solve_quadratic(fi(1), fi(0), fi(-2));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 2);
            let text = format!("{roots:?}");
            assert!(text.contains("Sqrt"), "expected Sqrt in {text}");
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

#[test]
fn quadratic_perfect_square_discriminant_rational_coeffs() {
    // (2x-1)(2x+1) = 4x^2 - 1 → x in {1/2, -1/2}
    let r = solve_quadratic(fi(4), fi(0), fi(-1));
    match r {
        SolveResult::Solutions(mut roots) => {
            roots.sort_by_key(|n| match n {
                IRNode::Rational(a, _) => *a,
                IRNode::Integer(a) => *a,
                _ => 0,
            });
            assert_eq!(roots, vec![rat(-1, 2), rat(1, 2)]);
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

#[test]
fn quadratic_all_zero() {
    // 0x^2 + 0x + 0 = 0 → ALL (falls back to linear then ALL)
    let r = solve_quadratic(fi(0), fi(0), fi(0));
    assert_eq!(r, SolveResult::All);
}

#[test]
fn quadratic_roots_form_check() {
    // x^2 - 1 = 0 → {-1, 1}
    let r = solve_quadratic(fi(1), fi(0), fi(-1));
    assert_eq!(r, SolveResult::Solutions(vec![int(-1), int(1)]));
}

#[test]
fn quadratic_large_discriminant() {
    // x^2 - 100x + 2499 = (x-49)(x-51) → roots 49 and 51
    // disc = 10000 - 4*2499 = 10000 - 9996 = 4 (perfect square = 2)
    let r = solve_quadratic(fi(1), fi(-100), fi(2499));
    assert_eq!(r, SolveResult::Solutions(vec![int(49), int(51)]));
}

#[test]
fn quadratic_no_roots_all_complex() {
    // x^2 + x + 1 = 0 → disc = 1 - 4 = -3 < 0, complex roots
    let r = solve_quadratic(fi(1), fi(1), fi(1));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 2);
            let text = format!("{roots:?}");
            assert!(text.contains("%i"), "expected complex roots in {text}");
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

#[test]
fn quadratic_irrational_sqrt_node_shape() {
    // x^2 - 3 = 0 → roots = ±Sqrt(3)
    // Expect: Div(Add(-0, Sqrt(3)), 2) and Div(Sub(-0, Sqrt(3)), 2)
    let r = solve_quadratic(fi(1), fi(0), fi(-3));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 2);
            // Both roots should contain Sqrt(3) in their IR.
            let text = format!("{roots:?}");
            assert!(text.contains("Sqrt"), "expected Sqrt in {text}");
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

// ---------------------------------------------------------------------------
// solve_cubic
// ---------------------------------------------------------------------------

#[test]
fn cubic_zero_leading_falls_back_to_quadratic() {
    // 0x^3 + x^2 - 5x + 6 = 0 -> {2, 3}
    let r = solve_cubic(fi(0), fi(1), fi(-5), fi(6));
    assert_eq!(r, SolveResult::Solutions(vec![int(2), int(3)]));
}

#[test]
fn cubic_three_rational_roots() {
    // x^3 - 6x^2 + 11x - 6 = 0 -> {1, 2, 3}
    let r = solve_cubic(fi(1), fi(-6), fi(11), fi(-6));
    assert_eq!(r, SolveResult::Solutions(vec![int(1), int(2), int(3)]));
}

#[test]
fn cubic_repeated_roots_are_deduplicated() {
    // x^3 - 3x - 2 = (x + 1)^2(x - 2) -> {-1, 2}
    let r = solve_cubic(fi(1), fi(0), fi(-3), fi(-2));
    assert_eq!(r, SolveResult::Solutions(vec![int(-1), int(2)]));
}

#[test]
fn cubic_rational_fraction_root() {
    // 2x^3 - 3x^2 - 11x + 6 = (x + 2)(2x - 1)(x - 3)
    let r = solve_cubic(fi(2), fi(-3), fi(-11), fi(6));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 3);
            assert!(roots.contains(&int(-2)));
            assert!(roots.contains(&rat(1, 2)));
            assert!(roots.contains(&int(3)));
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

#[test]
fn cubic_rational_root_with_complex_pair() {
    // x^3 + 1 = 0 -> -1 and a complex conjugate pair.
    let r = solve_cubic(fi(1), fi(0), fi(0), fi(1));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 3);
            assert!(roots.contains(&int(-1)));
            let text = format!("{roots:?}");
            assert!(text.contains("%i"), "expected complex roots in {text}");
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

#[test]
fn cubic_cardano_symbolic_one_real_two_complex() {
    // x^3 + x + 1 has no rational root and D_cardano > 0.
    let r = solve_cubic(fi(1), fi(0), fi(1), fi(1));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 3);
            let text = format!("{roots:?}");
            assert!(text.contains("Cbrt"), "expected Cbrt in {text}");
            assert!(text.contains("%i"), "expected complex roots in {text}");
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

#[test]
fn cubic_casus_irreducibilis_returns_empty() {
    // x^3 - 3x + 1 has three real irrational roots in the Python reference.
    let r = solve_cubic(fi(1), fi(0), fi(-3), fi(1));
    assert_eq!(r, SolveResult::Solutions(vec![]));
}

// ---------------------------------------------------------------------------
// solve_quartic
// ---------------------------------------------------------------------------

#[test]
fn quartic_zero_leading_falls_back_to_cubic() {
    let r = solve_quartic(fi(0), fi(1), fi(-6), fi(11), fi(-6));
    assert_eq!(r, SolveResult::Solutions(vec![int(1), int(2), int(3)]));
}

#[test]
fn quartic_four_rational_roots() {
    // x^4 - 10x^2 + 9 = (x^2 - 1)(x^2 - 9)
    let r = solve_quartic(fi(1), fi(0), fi(-10), fi(0), fi(9));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 4);
            assert!(roots.contains(&int(-3)));
            assert!(roots.contains(&int(-1)));
            assert!(roots.contains(&int(1)));
            assert!(roots.contains(&int(3)));
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

#[test]
fn quartic_all_integer_roots_positive() {
    // (x-1)(x-2)(x-3)(x-4)
    let r = solve_quartic(fi(1), fi(-10), fi(35), fi(-50), fi(24));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 4);
            assert!(roots.contains(&int(1)));
            assert!(roots.contains(&int(2)));
            assert!(roots.contains(&int(3)));
            assert!(roots.contains(&int(4)));
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

#[test]
fn quartic_zero_root_is_deduplicated() {
    let r = solve_quartic(fi(1), fi(-1), fi(0), fi(0), fi(0));
    assert_eq!(r, SolveResult::Solutions(vec![int(0), int(1)]));
}

#[test]
fn quartic_biquadratic_complex_roots() {
    // x^4 + 4x^2 + 3 = (x^2 + 1)(x^2 + 3)
    let r = solve_quartic(fi(1), fi(0), fi(4), fi(0), fi(3));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 4);
            let text = format!("{roots:?}");
            assert!(text.contains("Sqrt"), "expected Sqrt in {text}");
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

#[test]
fn quartic_ferrari_complex_roots() {
    // Full Ferrari path with rational resolvent root m=2.
    let r = solve_quartic(fi(1), fi(0), fi(1), fi(2), fi(6));
    match r {
        SolveResult::Solutions(roots) => {
            assert_eq!(roots.len(), 4);
            let text = format!("{roots:?}");
            assert!(text.contains("%i"), "expected complex roots in {text}");
        }
        SolveResult::All => panic!("expected Solutions"),
    }
}

#[test]
fn quartic_no_usable_resolvent_root_returns_empty() {
    let r = solve_quartic(fi(1), fi(0), fi(0), fi(1), fi(1));
    assert_eq!(r, SolveResult::Solutions(vec![]));
}

// ---------------------------------------------------------------------------
// nsolve_poly
// ---------------------------------------------------------------------------

#[test]
fn nsolve_linear_root() {
    let roots = nsolve_poly(&[c(2.0, 0.0), c(-4.0, 0.0)], 200, 1e-12);
    assert_numeric_roots_close(&roots, &[c(2.0, 0.0)]);
}

#[test]
fn nsolve_quadratic_real_roots() {
    let roots = nsolve_poly(&[c(1.0, 0.0), c(0.0, 0.0), c(-1.0, 0.0)], 200, 1e-12);
    assert_numeric_roots_close(&roots, &[c(-1.0, 0.0), c(1.0, 0.0)]);
}

#[test]
fn nsolve_quadratic_complex_roots() {
    let roots = nsolve_poly(&[c(1.0, 0.0), c(0.0, 0.0), c(1.0, 0.0)], 200, 1e-12);
    assert_numeric_roots_close(&roots, &[c(0.0, 1.0), c(0.0, -1.0)]);
}

#[test]
fn nsolve_cubic_three_real_roots() {
    let roots = nsolve_poly(
        &[c(1.0, 0.0), c(-6.0, 0.0), c(11.0, 0.0), c(-6.0, 0.0)],
        200,
        1e-12,
    );
    assert_numeric_roots_close(&roots, &[c(1.0, 0.0), c(2.0, 0.0), c(3.0, 0.0)]);
}

#[test]
fn nsolve_quintic_unit_roots() {
    let roots = nsolve_poly(
        &[
            c(1.0, 0.0),
            c(0.0, 0.0),
            c(0.0, 0.0),
            c(0.0, 0.0),
            c(0.0, 0.0),
            c(-1.0, 0.0),
        ],
        300,
        1e-12,
    );
    assert_eq!(roots.len(), 5);
    assert!(roots.iter().all(|root| (root.abs() - 1.0).abs() < 1e-8));
}

#[test]
fn nsolve_constant_polynomial_returns_empty() {
    assert!(nsolve_poly(&[c(5.0, 0.0)], 200, 1e-12).is_empty());
}

#[test]
fn roots_to_ir_preserves_real_and_complex_roots() {
    let roots = [c(2.0, 0.0), c(1.5, -0.25)];
    let ir = roots_to_ir(&roots);
    assert_eq!(ir.len(), 2);
    assert_eq!(ir[0], IRNode::Float(2.0));
    let text = format!("{:?}", ir[1]);
    assert!(text.contains("%i"), "expected complex IR in {text}");
}

#[test]
fn nsolve_fraction_poly_returns_float_ir_roots() {
    let ir = nsolve_fraction_poly(&[fi(1), fi(-6), fi(11), fi(-6)]);
    assert_eq!(ir.len(), 3);
    let mut values: Vec<f64> = ir
        .iter()
        .map(|node| match node {
            IRNode::Float(value) => *value,
            other => panic!("expected float root, got {other:?}"),
        })
        .collect();
    values.sort_by(f64::total_cmp);
    assert!((values[0] - 1.0).abs() < 1e-7);
    assert!((values[1] - 2.0).abs() < 1e-7);
    assert!((values[2] - 3.0).abs() < 1e-7);
}

// ---------------------------------------------------------------------------
// solve_linear_system
// ---------------------------------------------------------------------------

#[test]
fn linear_system_2x2_simple() {
    let x = sym("x");
    let y = sym("y");
    let result = solve_linear_system(
        &[
            eq(add(vec![x.clone(), y.clone()]), int(3)),
            eq(sub(x.clone(), y.clone()), int(1)),
        ],
        &[x.clone(), y.clone()],
    )
    .expect("expected unique solution");

    assert_eq!(rule_value(&result, &x), int(2));
    assert_eq!(rule_value(&result, &y), int(1));
}

#[test]
fn linear_system_rational_solution() {
    let x = sym("x");
    let y = sym("y");
    let result = solve_linear_system(
        &[
            eq(
                add(vec![
                    mul(vec![int(2), x.clone()]),
                    mul(vec![int(3), y.clone()]),
                ]),
                int(7),
            ),
            eq(sub(mul(vec![int(4), x.clone()]), y.clone()), int(1)),
        ],
        &[x.clone(), y.clone()],
    )
    .expect("expected unique solution");

    assert_eq!(rule_value(&result, &x), rat(5, 7));
    assert_eq!(rule_value(&result, &y), rat(13, 7));
}

#[test]
fn linear_system_3x3_and_zero_form() {
    let x = sym("x");
    let y = sym("y");
    let z = sym("z");
    let result = solve_linear_system(
        &[
            eq(add(vec![x.clone(), y.clone(), z.clone()]), int(6)),
            eq(add(vec![mul(vec![int(2), x.clone()]), y.clone()]), int(5)),
            eq(z.clone(), int(3)),
        ],
        &[x.clone(), y.clone(), z.clone()],
    )
    .expect("expected unique solution");

    assert_eq!(rule_value(&result, &x), int(2));
    assert_eq!(rule_value(&result, &y), int(1));
    assert_eq!(rule_value(&result, &z), int(3));

    let zero_form = solve_linear_system(
        &[add(vec![x.clone(), y.clone()]), sub(x.clone(), y.clone())],
        &[x.clone(), y.clone()],
    )
    .expect("expected unique zero-form solution");
    assert_eq!(rule_value(&zero_form, &x), int(0));
    assert_eq!(rule_value(&zero_form, &y), int(0));
}

#[test]
fn linear_system_rejects_bad_systems() {
    let x = sym("x");
    let y = sym("y");
    assert!(solve_linear_system(
        &[eq(add(vec![x.clone(), y.clone()]), int(3))],
        &[x.clone(), y.clone()]
    )
    .is_none());
    assert!(solve_linear_system(&[], &[]).is_none());
    assert!(solve_linear_system(
        &[
            eq(add(vec![x.clone(), y.clone()]), int(1)),
            eq(
                add(vec![
                    mul(vec![int(2), x.clone()]),
                    mul(vec![int(2), y.clone()])
                ]),
                int(2)
            ),
        ],
        &[x.clone(), y.clone()],
    )
    .is_none());
    assert!(solve_linear_system(&[eq(pow(x.clone(), int(2)), int(4))], &[x]).is_none());
}

#[test]
fn linear_system_returns_rule_nodes_in_variable_order() {
    let x = sym("x");
    let y = sym("y");
    let result = solve_linear_system(
        &[
            eq(add(vec![x.clone(), y.clone()]), int(3)),
            eq(sub(x.clone(), y.clone()), int(1)),
        ],
        &[x.clone(), y.clone()],
    )
    .expect("expected unique solution");

    assert_eq!(result[0], apply(sym(RULE), vec![x, int(2)]));
    assert_eq!(result[1], apply(sym(RULE), vec![y, int(1)]));
}

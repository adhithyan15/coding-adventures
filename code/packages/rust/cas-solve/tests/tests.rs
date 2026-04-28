// Integration tests for cas-solve.
//
// Mirrors the Python reference tests in
// code/packages/python/cas-solve/tests/.

use cas_solve::{solve_linear, solve_quadratic, SolveResult};
use cas_solve::frac::Frac;
use symbolic_ir::{int, rat, IRNode};

fn frac(n: i64, d: i64) -> Frac { Frac::new(n, d) }
fn fi(n: i64) -> Frac { Frac::from_int(n) }

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

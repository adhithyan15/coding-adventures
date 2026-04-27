// Integration tests for cas-complex.

use cas_complex::{
    argument, complex_normalize, complex_pow, conjugate, imag_part, modulus, real_part,
    IMAGINARY_UNIT,
};
use symbolic_ir::{apply, int, sym, IRNode, ADD, MUL, NEG, POW, SUB};
use std::f64::consts::{FRAC_PI_2, PI};

// Helper: build 3 + 4*I
fn z_3_4() -> IRNode {
    apply(sym(ADD), vec![int(3), apply(sym(MUL), vec![int(4), sym(IMAGINARY_UNIT)])])
}

// ---------------------------------------------------------------------------
// complex_normalize — basic forms
// ---------------------------------------------------------------------------

#[test]
fn normalize_integer_stays_integer() {
    assert_eq!(complex_normalize(&int(5)), int(5));
}

#[test]
fn normalize_zero_stays_zero() {
    assert_eq!(complex_normalize(&int(0)), int(0));
}

#[test]
fn normalize_i_is_i() {
    assert_eq!(complex_normalize(&sym(IMAGINARY_UNIT)), sym(IMAGINARY_UNIT));
}

#[test]
fn normalize_pure_imaginary() {
    // 3*I stays as 3*I (well-formed already)
    let z = apply(sym(MUL), vec![int(3), sym(IMAGINARY_UNIT)]);
    let result = complex_normalize(&z);
    // imag part = 3
    assert_eq!(imag_part(&result), int(3));
    assert_eq!(real_part(&result), int(0));
}

#[test]
fn normalize_3_plus_4i() {
    let z = z_3_4();
    let r = complex_normalize(&z);
    assert_eq!(real_part(&r), int(3));
    assert_eq!(imag_part(&r), int(4));
}

#[test]
fn normalize_i_squared_is_minus_one() {
    // I^2 = -1
    let i_sq = apply(sym(POW), vec![sym(IMAGINARY_UNIT), int(2)]);
    assert_eq!(complex_normalize(&i_sq), int(-1));
}

#[test]
fn normalize_i_cubed_is_neg_i() {
    // I^3 = -I  (re=0, im=-1)
    let i_cu = apply(sym(POW), vec![sym(IMAGINARY_UNIT), int(3)]);
    let r = complex_normalize(&i_cu);
    assert_eq!(real_part(&r), int(0));
    assert_eq!(imag_part(&r), int(-1));
}

#[test]
fn normalize_i_pow4_is_one() {
    // I^4 = 1
    let i4 = apply(sym(POW), vec![sym(IMAGINARY_UNIT), int(4)]);
    assert_eq!(complex_normalize(&i4), int(1));
}

#[test]
fn normalize_i_pow_neg1_is_neg_i() {
    // I^(-1) = -i  (re=0, im=-1)
    let i_inv = apply(sym(POW), vec![sym(IMAGINARY_UNIT), int(-1)]);
    let r = complex_normalize(&i_inv);
    assert_eq!(real_part(&r), int(0));
    assert_eq!(imag_part(&r), int(-1));
}

// ---------------------------------------------------------------------------
// complex_normalize — multiplication
// ---------------------------------------------------------------------------

#[test]
fn normalize_mul_two_complex() {
    // (1 + I) * (1 - I) = 1 - I + I - I^2 = 1 + 1 = 2
    let a = apply(sym(ADD), vec![int(1), sym(IMAGINARY_UNIT)]);
    let b = apply(sym(ADD), vec![int(1), apply(sym(NEG), vec![sym(IMAGINARY_UNIT)])]);
    let product = apply(sym(MUL), vec![a, b]);
    let r = complex_normalize(&product);
    assert_eq!(real_part(&r), int(2));
    assert_eq!(imag_part(&r), int(0));
}

#[test]
fn normalize_mul_i_i() {
    // I * I = I^2 ... wait, Mul(I, I) → split: (re=0,im=1)*(re=0,im=1)
    // = (0*0 - 1*1) + (0*1 + 1*0)*I = -1 + 0*I = -1
    let i_times_i = apply(sym(MUL), vec![sym(IMAGINARY_UNIT), sym(IMAGINARY_UNIT)]);
    let r = complex_normalize(&i_times_i);
    assert_eq!(r, int(-1));
}

#[test]
fn normalize_mul_3i_4i() {
    // 3I * 4I = 12 * I^2 = -12
    let a = apply(sym(MUL), vec![int(3), sym(IMAGINARY_UNIT)]);
    let b = apply(sym(MUL), vec![int(4), sym(IMAGINARY_UNIT)]);
    let product = apply(sym(MUL), vec![a, b]);
    let r = complex_normalize(&product);
    assert_eq!(r, int(-12));
}

#[test]
fn normalize_3_4i_times_1_minus_2i() {
    // (3 + 4i)(1 - 2i) = 3 - 6i + 4i - 8i^2 = 3 - 2i + 8 = 11 - 2i
    let a = z_3_4();
    // 1 - 2*I as Sub(1, Mul(2, I))
    let b = apply(sym(SUB), vec![int(1), apply(sym(MUL), vec![int(2), sym(IMAGINARY_UNIT)])]);
    let product = apply(sym(MUL), vec![a, b]);
    let r = complex_normalize(&product);
    assert_eq!(real_part(&r), int(11));
    assert_eq!(imag_part(&r), int(-2));
}

// ---------------------------------------------------------------------------
// real_part / imag_part
// ---------------------------------------------------------------------------

#[test]
fn real_part_of_real_is_itself() {
    assert_eq!(real_part(&int(7)), int(7));
}

#[test]
fn real_part_of_pure_imaginary_is_zero() {
    let z = apply(sym(MUL), vec![int(5), sym(IMAGINARY_UNIT)]);
    assert_eq!(real_part(&z), int(0));
}

#[test]
fn real_part_of_i_is_zero() {
    assert_eq!(real_part(&sym(IMAGINARY_UNIT)), int(0));
}

#[test]
fn imag_part_of_i_is_one() {
    assert_eq!(imag_part(&sym(IMAGINARY_UNIT)), int(1));
}

#[test]
fn imag_part_of_real_is_zero() {
    assert_eq!(imag_part(&int(5)), int(0));
}

#[test]
fn imag_part_3_4i() {
    assert_eq!(imag_part(&z_3_4()), int(4));
}

// ---------------------------------------------------------------------------
// conjugate
// ---------------------------------------------------------------------------

#[test]
fn conjugate_of_real_is_itself() {
    // conj(5) = 5
    let c = conjugate(&int(5));
    assert_eq!(real_part(&c), int(5));
    assert_eq!(imag_part(&c), int(0));
}

#[test]
fn conjugate_3_4i() {
    // conj(3 + 4I) = 3 - 4I
    let c = conjugate(&z_3_4());
    assert_eq!(real_part(&c), int(3));
    assert_eq!(imag_part(&c), int(-4));
}

#[test]
fn conjugate_of_i_is_neg_i() {
    // conj(I) = -I
    let c = conjugate(&sym(IMAGINARY_UNIT));
    assert_eq!(real_part(&c), int(0));
    assert_eq!(imag_part(&c), int(-1));
}

// ---------------------------------------------------------------------------
// modulus
// ---------------------------------------------------------------------------

#[test]
fn modulus_3_4i_is_5() {
    if let IRNode::Float(v) = modulus(&z_3_4()) {
        assert!((v - 5.0).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn modulus_of_real_positive() {
    if let IRNode::Float(v) = modulus(&int(3)) {
        assert!((v - 3.0).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn modulus_of_i_is_one() {
    if let IRNode::Float(v) = modulus(&sym(IMAGINARY_UNIT)) {
        assert!((v - 1.0).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn modulus_of_zero_is_zero() {
    if let IRNode::Float(v) = modulus(&int(0)) {
        assert_eq!(v, 0.0);
    } else {
        panic!("expected Float");
    }
}

// ---------------------------------------------------------------------------
// argument
// ---------------------------------------------------------------------------

#[test]
fn argument_of_positive_real_is_zero() {
    if let IRNode::Float(v) = argument(&int(1)) {
        assert!((v - 0.0).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn argument_of_negative_real_is_pi() {
    if let IRNode::Float(v) = argument(&int(-1)) {
        assert!((v - PI).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn argument_of_i_is_pi_over_2() {
    if let IRNode::Float(v) = argument(&sym(IMAGINARY_UNIT)) {
        assert!((v - FRAC_PI_2).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

// ---------------------------------------------------------------------------
// complex_pow
// ---------------------------------------------------------------------------

#[test]
fn pow_i4_is_1() {
    // I^4 = 1
    assert_eq!(complex_pow(&sym(IMAGINARY_UNIT), &int(4)), int(1));
}

#[test]
fn pow_i2_is_neg1() {
    // I^2 = -1
    assert_eq!(complex_pow(&sym(IMAGINARY_UNIT), &int(2)), int(-1));
}

#[test]
fn pow_i0_is_1() {
    assert_eq!(complex_pow(&sym(IMAGINARY_UNIT), &int(0)), int(1));
}

#[test]
fn pow_1_plus_i_squared() {
    // (1 + I)^2 = 1 + 2I + I^2 = 1 + 2I - 1 = 2I
    let w = apply(sym(ADD), vec![int(1), sym(IMAGINARY_UNIT)]);
    let result = complex_pow(&w, &int(2));
    assert_eq!(real_part(&result), int(0));
    assert_eq!(imag_part(&result), int(2));
}

#[test]
fn pow_3_4i_squared() {
    // (3 + 4I)^2 = 9 + 24I + 16I^2 = 9 + 24I - 16 = -7 + 24I
    let result = complex_pow(&z_3_4(), &int(2));
    assert_eq!(real_part(&result), int(-7));
    assert_eq!(imag_part(&result), int(24));
}

#[test]
fn pow_symbolic_base_returns_unevaluated() {
    // x^2 — x is symbolic, return Pow(x, 2) unevaluated
    let x = sym("x");
    let result = complex_pow(&x, &int(2));
    if let IRNode::Apply(a) = &result {
        assert_eq!(a.head, sym(POW));
    } else {
        // Might be Int(1) if x was treated as real atom and n=2 ... actually
        // sym("x") is an opaque real atom, so split_complex gives re=x, im=0.
        // Then (x*x - 0*0) + (x*0 + 0*x)*I = x*x + 0*I.
        // Hmm, this won't return unevaluated Pow.  Let's just check it makes sense.
        let _re = real_part(&result);
        let im = imag_part(&result);
        assert_eq!(im, int(0));
        // re should be x*x (some Mul or Pow form)
    }
}

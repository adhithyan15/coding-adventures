// Integration tests for cas-number-theory.

use cas_number_theory::{
    crt, extended_gcd, factor_integer, factorize_ir, gcd, is_prime, lcm, mod_inverse, mod_pow,
    next_prime, nth_prime, primes_up_to, totient,
};
use symbolic_ir::{apply, int, sym, IRNode, MUL, POW};

// ---------------------------------------------------------------------------
// arithmetic — gcd
// ---------------------------------------------------------------------------

#[test]
fn gcd_basic() {
    assert_eq!(gcd(12, 8), 4);
    assert_eq!(gcd(9, 6), 3);
    assert_eq!(gcd(7, 3), 1);
}

#[test]
fn gcd_with_zero() {
    assert_eq!(gcd(0, 5), 5);
    assert_eq!(gcd(5, 0), 5);
    assert_eq!(gcd(0, 0), 0);
}

#[test]
fn gcd_negative_inputs() {
    assert_eq!(gcd(-12, 8), 4);
    assert_eq!(gcd(12, -8), 4);
    assert_eq!(gcd(-12, -8), 4);
}

#[test]
fn gcd_one() {
    assert_eq!(gcd(1, 100), 1);
    assert_eq!(gcd(100, 1), 1);
}

#[test]
fn gcd_equal_inputs() {
    assert_eq!(gcd(7, 7), 7);
    assert_eq!(gcd(100, 100), 100);
}

// ---------------------------------------------------------------------------
// arithmetic — lcm
// ---------------------------------------------------------------------------

#[test]
fn lcm_basic() {
    assert_eq!(lcm(4, 6), 12);
    assert_eq!(lcm(3, 4), 12);
    assert_eq!(lcm(7, 5), 35);
}

#[test]
fn lcm_with_zero() {
    assert_eq!(lcm(0, 5), 0);
    assert_eq!(lcm(5, 0), 0);
}

#[test]
fn lcm_coprime() {
    assert_eq!(lcm(7, 11), 77);
}

#[test]
fn lcm_one_divides() {
    // lcm(3, 6) = 6 since 3 | 6
    assert_eq!(lcm(3, 6), 6);
}

// ---------------------------------------------------------------------------
// arithmetic — extended_gcd
// ---------------------------------------------------------------------------

#[test]
fn extended_gcd_bezout_identity() {
    // Verify a*s + b*t = g for various inputs.
    for (a, b) in &[(3i64, 5), (12, 8), (100, 37), (7, 13), (0, 5)] {
        let (g, s, t) = extended_gcd(*a, *b);
        assert_eq!(a * s + b * t, g, "Bézout identity failed for ({}, {})", a, b);
        assert_eq!(g, gcd(*a, *b));
    }
}

#[test]
fn extended_gcd_coprime_gives_inverse() {
    // extended_gcd(3, 7) → s is the modular inverse of 3 mod 7 = 5
    let (g, s, _) = extended_gcd(3, 7);
    assert_eq!(g, 1);
    assert_eq!(s.rem_euclid(7), 5); // 3*5 = 15 ≡ 1 (mod 7)
}

// ---------------------------------------------------------------------------
// arithmetic — totient
// ---------------------------------------------------------------------------

#[test]
fn totient_one() {
    assert_eq!(totient(1), 1);
}

#[test]
fn totient_prime() {
    assert_eq!(totient(7), 6);
    assert_eq!(totient(13), 12);
    assert_eq!(totient(97), 96);
}

#[test]
fn totient_prime_power() {
    // φ(p^k) = p^(k-1)·(p-1)
    assert_eq!(totient(4), 2);   // φ(2²) = 2
    assert_eq!(totient(8), 4);   // φ(2³) = 4
    assert_eq!(totient(9), 6);   // φ(3²) = 6
}

#[test]
fn totient_composite() {
    assert_eq!(totient(12), 4);   // φ(4·3) = 2·2
    assert_eq!(totient(36), 12);  // φ(4·9) = 2·6
    assert_eq!(totient(30), 8);   // φ(2·3·5) = 1·2·4
}

#[test]
fn totient_nonpositive() {
    assert_eq!(totient(0), 0);
    assert_eq!(totient(-5), 0);
}

// ---------------------------------------------------------------------------
// arithmetic — mod_inverse
// ---------------------------------------------------------------------------

#[test]
fn mod_inverse_basic() {
    assert_eq!(mod_inverse(3, 7), Some(5));   // 3*5=15≡1 (mod 7)
    assert_eq!(mod_inverse(1, 5), Some(1));
}

#[test]
fn mod_inverse_no_inverse() {
    assert_eq!(mod_inverse(2, 4), None);   // gcd(2,4)=2≠1
    assert_eq!(mod_inverse(6, 9), None);
}

// ---------------------------------------------------------------------------
// arithmetic — mod_pow
// ---------------------------------------------------------------------------

#[test]
fn mod_pow_basic() {
    assert_eq!(mod_pow(2, 10, 1000), 24);   // 2^10 = 1024 ≡ 24 (mod 1000)
    assert_eq!(mod_pow(3, 4, 7), 4);         // 81 ≡ 4 (mod 7)
    assert_eq!(mod_pow(3, 0, 7), 1);         // anything^0 = 1
}

#[test]
fn mod_pow_modulus_one() {
    assert_eq!(mod_pow(100, 100, 1), 0);
}

// ---------------------------------------------------------------------------
// primality — is_prime
// ---------------------------------------------------------------------------

#[test]
fn is_prime_small() {
    assert!(!is_prime(0));
    assert!(!is_prime(1));
    assert!(is_prime(2));
    assert!(is_prime(3));
    assert!(!is_prime(4));
    assert!(is_prime(5));
}

#[test]
fn is_prime_composites() {
    assert!(!is_prime(100));
    assert!(!is_prime(561)); // Carmichael number — NOT prime
    assert!(!is_prime(1000000));
}

#[test]
fn is_prime_large() {
    assert!(is_prime(97));
    assert!(is_prime(9973));
    assert!(is_prime(104729));
}

#[test]
fn is_prime_negative() {
    assert!(!is_prime(-7));
}

// ---------------------------------------------------------------------------
// primality — primes_up_to
// ---------------------------------------------------------------------------

#[test]
fn primes_up_to_small() {
    assert_eq!(primes_up_to(10), vec![2, 3, 5, 7]);
    assert_eq!(primes_up_to(20), vec![2, 3, 5, 7, 11, 13, 17, 19]);
    assert_eq!(primes_up_to(2), vec![2]);
    assert_eq!(primes_up_to(1), vec![]);
    assert_eq!(primes_up_to(0), vec![]);
}

#[test]
fn primes_up_to_count() {
    // There are 25 primes ≤ 99 (2, 3, 5, …, 89, 97).
    // 100 is not prime, so primes_up_to(100) is also 25.
    assert_eq!(primes_up_to(99).len(), 25);
    assert_eq!(primes_up_to(100).len(), 25);
}

// ---------------------------------------------------------------------------
// primality — next_prime / nth_prime
// ---------------------------------------------------------------------------

#[test]
fn next_prime_basic() {
    assert_eq!(next_prime(0), 2);
    assert_eq!(next_prime(1), 2);
    assert_eq!(next_prime(2), 3);
    assert_eq!(next_prime(10), 11);
    assert_eq!(next_prime(13), 17);
}

#[test]
fn nth_prime_basic() {
    assert_eq!(nth_prime(1), 2);
    assert_eq!(nth_prime(2), 3);
    assert_eq!(nth_prime(4), 7);
    assert_eq!(nth_prime(10), 29);
    assert_eq!(nth_prime(25), 97);
}

// ---------------------------------------------------------------------------
// factorize — factor_integer
// ---------------------------------------------------------------------------

#[test]
fn factor_integer_prime() {
    assert_eq!(factor_integer(7), vec![(7, 1)]);
    assert_eq!(factor_integer(97), vec![(97, 1)]);
}

#[test]
fn factor_integer_composite() {
    assert_eq!(factor_integer(12), vec![(2, 2), (3, 1)]);
    assert_eq!(factor_integer(360), vec![(2, 3), (3, 2), (5, 1)]);
    assert_eq!(factor_integer(1024), vec![(2, 10)]);
}

#[test]
fn factor_integer_trivial() {
    assert_eq!(factor_integer(0), vec![]);
    assert_eq!(factor_integer(1), vec![]);
    assert_eq!(factor_integer(-1), vec![]);
}

#[test]
fn factor_integer_negative() {
    // Sign is stripped; only the absolute value is factored.
    assert_eq!(factor_integer(-12), vec![(2, 2), (3, 1)]);
}

#[test]
fn factor_integer_reconstruct() {
    // Verify the factorization: ∏ p^e = |n|
    for n in &[2i64, 6, 12, 100, 360, 1023, 9973] {
        let factors = factor_integer(*n);
        let product: i64 = factors.iter().map(|(p, e)| p.pow(*e)).product();
        assert_eq!(product, *n, "reconstruction failed for {}", n);
    }
}

// ---------------------------------------------------------------------------
// factorize — factorize_ir
// ---------------------------------------------------------------------------

#[test]
fn factorize_ir_prime_unchanged() {
    assert_eq!(factorize_ir(&int(7)), int(7));
    assert_eq!(factorize_ir(&int(2)), int(2));
}

#[test]
fn factorize_ir_one_zero_unchanged() {
    assert_eq!(factorize_ir(&int(0)), int(0));
    assert_eq!(factorize_ir(&int(1)), int(1));
    assert_eq!(factorize_ir(&int(-1)), int(-1));
}

#[test]
fn factorize_ir_composite() {
    // 12 = 2² × 3  →  Mul(Pow(2, 2), 3)
    let result = factorize_ir(&int(12));
    if let IRNode::Apply(a) = &result {
        assert_eq!(a.head, sym(MUL));
        let has_pow_2_2 = a.args.contains(&apply(sym(POW), vec![int(2), int(2)]));
        let has_3 = a.args.contains(&int(3));
        assert!(has_pow_2_2, "expected Pow(2, 2) in {:?}", result);
        assert!(has_3, "expected 3 in {:?}", result);
    } else {
        panic!("expected Mul node, got {:?}", result);
    }
}

#[test]
fn factorize_ir_negative() {
    // -6 = (-1) × 2 × 3  →  Mul(-1, 2, 3)
    let result = factorize_ir(&int(-6));
    if let IRNode::Apply(a) = &result {
        assert_eq!(a.head, sym(MUL));
        assert!(a.args.contains(&int(-1)), "expected -1 in {:?}", result);
        assert!(a.args.contains(&int(2)), "expected 2 in {:?}", result);
        assert!(a.args.contains(&int(3)), "expected 3 in {:?}", result);
    } else {
        panic!("expected Mul node, got {:?}", result);
    }
}

#[test]
fn factorize_ir_non_integer_unchanged() {
    let sym_x = sym("x");
    assert_eq!(factorize_ir(&sym_x), sym_x);
}

// ---------------------------------------------------------------------------
// crt
// ---------------------------------------------------------------------------

#[test]
fn crt_classic_example() {
    // x ≡ 2 (mod 3),  x ≡ 3 (mod 5),  x ≡ 2 (mod 7)  →  23
    assert_eq!(crt(&[2, 3, 2], &[3, 5, 7]), Some(23));
}

#[test]
fn crt_single_congruence() {
    assert_eq!(crt(&[5], &[7]), Some(5));
    assert_eq!(crt(&[0], &[3]), Some(0));
}

#[test]
fn crt_two_congruences() {
    // x ≡ 0 (mod 3), x ≡ 0 (mod 5)  →  0
    assert_eq!(crt(&[0, 0], &[3, 5]), Some(0));
    // x ≡ 1 (mod 2), x ≡ 0 (mod 3)  →  3
    assert_eq!(crt(&[1, 0], &[2, 3]), Some(3));
}

#[test]
fn crt_inconsistent() {
    // x ≡ 0 (mod 4) and x ≡ 1 (mod 2) conflict (0 is even, 1 is odd).
    assert_eq!(crt(&[0, 1], &[4, 2]), None);
}

#[test]
fn crt_empty_returns_none() {
    assert_eq!(crt(&[], &[]), None);
}

#[test]
fn crt_invalid_modulus() {
    assert_eq!(crt(&[1], &[0]), None);
    assert_eq!(crt(&[1], &[-3]), None);
}

#[test]
fn crt_solution_unique_mod_lcm() {
    // Result should be in [0, lcm(moduli)).
    let result = crt(&[2, 3, 2], &[3, 5, 7]).unwrap();
    // lcm(3,5,7) = 105
    assert!(result >= 0 && result < 105, "result {} not in [0, 105)", result);
    // Verify each congruence.
    assert_eq!(result % 3, 2);
    assert_eq!(result % 5, 3);
    assert_eq!(result % 7, 2);
}

#[test]
fn crt_large_moduli() {
    // x ≡ 1 (mod 101), x ≡ 2 (mod 103)
    let result = crt(&[1, 2], &[101, 103]).unwrap();
    assert_eq!(result % 101, 1);
    assert_eq!(result % 103, 2);
}

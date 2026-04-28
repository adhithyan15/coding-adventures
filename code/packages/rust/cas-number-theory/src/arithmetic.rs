//! Basic integer arithmetic: GCD, LCM, extended Euclidean algorithm, and
//! Euler's totient function.
//!
//! All functions work over `i64`.  Inputs are treated by absolute value
//! unless stated otherwise; results are non-negative unless stated otherwise.
//!
//! # GCD by example
//!
//! ```text
//!  gcd(12, 8):
//!    a=12, b=8 → a=8, b=4 → a=4, b=0 → return 4
//!
//!  gcd(7, 0):
//!    a=7, b=0 → return 7
//! ```
//!
//! # Extended Euclidean algorithm
//!
//! Given integers `a` and `b`, finds `(g, s, t)` such that:
//!
//! ```text
//! a·s + b·t = g = gcd(a, b)
//! ```
//!
//! Useful for computing modular inverses: when `g = 1`, `s` is the modular
//! inverse of `a` modulo `b`.
//!
//! # Euler's totient function φ(n)
//!
//! `φ(n)` counts the integers in `[1, n]` that are coprime to `n`.
//!
//! ```text
//! φ(1)  = 1
//! φ(p)  = p − 1  (p prime)
//! φ(p^k) = p^(k−1) · (p−1)
//! φ(mn) = φ(m)·φ(n)  when gcd(m, n) = 1
//! ```
//!
//! The formula used here: for each prime factor `p` of `n`,
//! multiply the running total by `(p − 1) / p`.

// ---------------------------------------------------------------------------
// GCD / LCM
// ---------------------------------------------------------------------------

/// Greatest common divisor of `a` and `b`.
///
/// Uses the Euclidean algorithm on `|a|` and `|b|`.  Returns `0` when both
/// inputs are zero (by convention, GCD(0,0) = 0).
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::gcd;
///
/// assert_eq!(gcd(12, 8), 4);
/// assert_eq!(gcd(-12, 8), 4);   // sign-agnostic
/// assert_eq!(gcd(7, 0), 7);
/// assert_eq!(gcd(0, 0), 0);
/// ```
pub fn gcd(a: i64, b: i64) -> i64 {
    let mut a = a.abs();
    let mut b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Least common multiple of `a` and `b`.
///
/// Returns `0` if either input is zero.
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::lcm;
///
/// assert_eq!(lcm(4, 6), 12);
/// assert_eq!(lcm(0, 5), 0);
/// assert_eq!(lcm(-4, 6), 12);   // sign-agnostic
/// ```
pub fn lcm(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 {
        return 0;
    }
    (a.abs() / gcd(a, b)) * b.abs()
}

// ---------------------------------------------------------------------------
// Extended GCD
// ---------------------------------------------------------------------------

/// Extended Euclidean algorithm.
///
/// Returns `(g, s, t)` satisfying `a·s + b·t = g = gcd(|a|, |b|)`.
///
/// When `gcd(a, b) = 1`, the coefficient `s` is the modular inverse of `a`
/// modulo `b`.
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::extended_gcd;
///
/// let (g, s, t) = extended_gcd(3, 5);
/// assert_eq!(g, 1);
/// assert_eq!(3 * s + 5 * t, 1);   // Bézout identity holds
///
/// let (g2, s2, t2) = extended_gcd(12, 8);
/// assert_eq!(g2, 4);
/// assert_eq!(12 * s2 + 8 * t2, 4);
/// ```
pub fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    // Recursive extended Euclidean algorithm.
    // Base case: gcd(a, 0) = a, with a·1 + 0·0 = a.
    if b == 0 {
        return (a, 1, 0);
    }
    // Recursive step: extended_gcd(b, a % b) → (g, s', t')
    //   b·s' + (a%b)·t' = g
    //   b·s' + (a - (a/b)·b)·t' = g
    //   a·t' + b·(s' - (a/b)·t') = g
    // So: s = t', t = s' - (a/b)·t'
    let (g, s, t) = extended_gcd(b, a % b);
    (g, t, s - (a / b) * t)
}

// ---------------------------------------------------------------------------
// Euler's totient
// ---------------------------------------------------------------------------

/// Euler's totient function φ(n).
///
/// Returns the count of integers in `[1, n]` that are coprime to `n`.
///
/// Returns `0` for `n ≤ 0`.
///
/// # Algorithm
///
/// Factor `n` by trial division.  For each prime factor `p`,
/// multiply the running total by `(p − 1) / p`:
///
/// ```text
/// φ(n) = n · ∏_{p | n, p prime}  (1 − 1/p)
/// ```
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::totient;
///
/// assert_eq!(totient(1), 1);
/// assert_eq!(totient(7), 6);    // prime
/// assert_eq!(totient(12), 4);   // φ(4·3) = 2·2 = 4
/// assert_eq!(totient(36), 12);
/// ```
pub fn totient(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut result = n;
    let mut n = n;
    let mut p = 2i64;
    // For each prime p dividing n, apply φ(n) *= (p-1)/p.
    while p * p <= n {
        if n % p == 0 {
            // Remove all copies of p from n so we don't count p twice.
            while n % p == 0 {
                n /= p;
            }
            // result = result * (p - 1) / p.  We know p divides result at
            // this point (since p | original n), so the division is exact.
            result -= result / p;
        }
        p += 1;
    }
    // If any prime factor > sqrt(original n) remains.
    if n > 1 {
        result -= result / n;
    }
    result
}

// ---------------------------------------------------------------------------
// Modular arithmetic helpers
// ---------------------------------------------------------------------------

/// Modular inverse of `a` modulo `m`.
///
/// Returns `Some(x)` where `a * x ≡ 1 (mod m)`, or `None` if
/// `gcd(a, m) ≠ 1` (inverse doesn't exist).
///
/// # Example
///
/// ```rust
/// use cas_number_theory::mod_inverse;
///
/// assert_eq!(mod_inverse(3, 7), Some(5));  // 3*5 = 15 ≡ 1 (mod 7)
/// assert_eq!(mod_inverse(2, 4), None);     // gcd(2, 4) = 2 ≠ 1
/// ```
pub fn mod_inverse(a: i64, m: i64) -> Option<i64> {
    let (g, s, _) = extended_gcd(a, m);
    if g != 1 {
        return None;
    }
    // s may be negative; normalise to [0, m).
    Some(s.rem_euclid(m))
}

/// Fast modular exponentiation: `base^exp mod modulus`.
///
/// Uses repeated squaring (O(log exp) multiplications).
///
/// # Example
///
/// ```rust
/// use cas_number_theory::mod_pow;
///
/// assert_eq!(mod_pow(2, 10, 1000), 24);   // 2^10 = 1024 ≡ 24 (mod 1000)
/// assert_eq!(mod_pow(3, 0, 7), 1);         // any^0 = 1
/// ```
pub fn mod_pow(mut base: i64, mut exp: u64, modulus: i64) -> i64 {
    if modulus == 1 {
        return 0;
    }
    let mut result = 1i64;
    base = base.rem_euclid(modulus);
    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * base).rem_euclid(modulus);
        }
        exp /= 2;
        base = (base * base).rem_euclid(modulus);
    }
    result
}

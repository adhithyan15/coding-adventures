//! Chinese Remainder Theorem (CRT).
//!
//! Given a system of simultaneous congruences:
//!
//! ```text
//! x ≡ r₀  (mod m₀)
//! x ≡ r₁  (mod m₁)
//!    ⋮
//! x ≡ rₖ  (mod mₖ)
//! ```
//!
//! The CRT states: if the moduli are pairwise coprime, a unique solution `x`
//! exists modulo `M = m₀ × m₁ × … × mₖ`.
//!
//! # Algorithm (iterative pairwise combination)
//!
//! Start with `x ≡ r₀ (mod m₀)`.  Repeatedly incorporate one new congruence
//! at a time:
//!
//! 1. Current state: `x ≡ a (mod M)`.
//! 2. New congruence: `x ≡ b (mod n)`.
//! 3. We need `a + M·t ≡ b (mod n)`, so `M·t ≡ (b − a) (mod n)`.
//! 4. Let `g = gcd(M, n)`.  Solution exists iff `g | (b − a)`.
//! 5. `t ≡ (b−a)/g · (M/g)⁻¹ (mod n/g)`.
//! 6. New state: `x ← (a + M·t) mod lcm(M, n)`.
//!
//! This correctly handles non-pairwise-coprime moduli as long as the
//! congruences are consistent (no conflicting residues).
//!
//! # Example
//!
//! ```text
//! x ≡ 2 (mod 3)
//! x ≡ 3 (mod 5)
//! x ≡ 2 (mod 7)
//!
//! Step 1: x=2, M=3
//! Step 2: combine with r=3, n=5 → x=8, M=15
//! Step 3: combine with r=2, n=7 → x=23, M=105
//!
//! Answer: x ≡ 23 (mod 105)
//! ```

use crate::arithmetic::{extended_gcd, lcm};

/// Solve the system of congruences `x ≡ remainders[i] (mod moduli[i])`.
///
/// Returns `Some(x)` where `x` is the unique solution in `[0, M)` and
/// `M = lcm(moduli)`, or `None` if:
///
/// - The slices are empty or of different lengths.
/// - Any modulus is ≤ 0.
/// - The congruences are inconsistent (no solution exists).
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::crt;
///
/// // x ≡ 2 (mod 3),  x ≡ 3 (mod 5),  x ≡ 2 (mod 7)  →  23
/// assert_eq!(crt(&[2, 3, 2], &[3, 5, 7]), Some(23));
///
/// // Single congruence — trivial.
/// assert_eq!(crt(&[5], &[7]), Some(5));
///
/// // Inconsistent: x ≡ 0 (mod 4) and x ≡ 1 (mod 2) conflict.
/// assert_eq!(crt(&[0, 1], &[4, 2]), None);
/// ```
pub fn crt(remainders: &[i64], moduli: &[i64]) -> Option<i64> {
    if remainders.is_empty() || remainders.len() != moduli.len() {
        return None;
    }
    if moduli.iter().any(|&m| m <= 0) {
        return None;
    }

    // Initial state: x ≡ r₀ (mod m₀), normalised to [0, m₀).
    let mut x = remainders[0].rem_euclid(moduli[0]);
    let mut m = moduli[0];

    for i in 1..remainders.len() {
        let r = remainders[i];
        let n = moduli[i];

        // Combine x ≡ a (mod M) with x ≡ b (mod n):
        //   a + M·t ≡ b (mod n)
        //   M·t ≡ (b − a) (mod n)
        let (g, s, _) = extended_gcd(m, n);
        let diff = r - x;

        // A solution exists iff g | diff.
        if diff % g != 0 {
            return None;
        }

        // t ≡ (diff / g) · (M/g)⁻¹ (mod n/g)
        //
        // extended_gcd(M, n) gives M·s + n·t₂ = g, so
        // (M/g)·s ≡ 1 (mod n/g), i.e. s is the inverse of M/g mod n/g.
        let n_g = n / g;
        // Avoid overflow by using i128 for the intermediate product.
        let t =
            ((diff / g).rem_euclid(n_g) as i128 * s.rem_euclid(n_g) as i128 % n_g as i128) as i64;
        let t = t.rem_euclid(n_g); // ensure non-negative

        // New modulus is lcm(M, n).
        let new_m = lcm(m, n);
        // Use i128 to avoid overflow when m and t are both large.
        x = ((x as i128 + m as i128 * t as i128).rem_euclid(new_m as i128)) as i64;
        m = new_m;
    }

    Some(x)
}

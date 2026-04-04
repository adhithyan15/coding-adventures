//! # reed-solomon — Error-Correcting Codes over GF(256)
//!
//! **Reed-Solomon** is a block error-correcting code that adds redundancy bytes
//! to a message. Even if some bytes are corrupted in transit, the decoder can
//! recover the original data — as long as the number of corrupted bytes does not
//! exceed the *correction capacity* `t`.
//!
//! ## Where RS is Used
//!
//! | System | How RS helps |
//! |--------|-------------|
//! | QR codes | Up to 30% of a QR code can be scratched away and still decode |
//! | CDs / DVDs | CIRC two-level RS corrects scratches |
//! | Hard drives | Firmware corrects sector-level errors |
//! | Deep-space probes | Voyager transmits photos across billions of kilometres |
//! | RAID-6 | The two parity drives are exactly an (n, n-2) RS code |
//!
//! ## Building Blocks
//!
//! This crate sits at the top of the MA (Mathematical Algorithms) stack:
//!
//! ```text
//! MA00  polynomial   — coefficient-array polynomial arithmetic over f64
//! MA01  gf256        — GF(2^8) field arithmetic (add=XOR, mul=table lookup)
//! MA02  reed-solomon ← THIS CRATE — RS encoding and decoding over GF(256)
//! ```
//!
//! All coefficient arithmetic (addition, multiplication, division) in this crate
//! uses the `gf256` crate's log/antilog-table operations.
//!
//! ## Code Parameters
//!
//! An RS code is described by **[n, k, d]**:
//!
//! | Symbol | Name | Meaning |
//! |--------|------|---------|
//! | `n` | block length | Total bytes in a codeword |
//! | `k = n - n_check` | message length | Data bytes |
//! | `n_check` | check symbol count | Redundancy bytes added by the encoder |
//! | `t = n_check / 2` | correction capacity | Maximum errors correctable |
//!
//! **Constraint**: `n_check` must be even and ≥ 2; `k + n_check ≤ 255`.
//!
//! ## Quick Example
//!
//! ```rust
//! use reed_solomon::{encode, decode};
//!
//! let message = b"hello";
//! let n_check = 8;                              // t = 4 errors correctable
//!
//! let mut codeword = encode(message, n_check).unwrap();
//! assert_eq!(&codeword[..message.len()], message); // message bytes unchanged
//!
//! // Corrupt 4 bytes — the decoder can still recover
//! codeword[0] ^= 0xFF;
//! codeword[2] ^= 0xAA;
//! codeword[4] ^= 0x55;
//! codeword[7] ^= 0x11;
//!
//! let recovered = decode(&codeword, n_check).unwrap();
//! assert_eq!(recovered, message);
//! ```
//!
//! ## Polynomial Conventions — One System, Applied Consistently
//!
//! All codeword bytes are treated as a **big-endian** polynomial:
//!
//! ```text
//! codeword[0] · x^{n-1}  +  codeword[1] · x^{n-2}  +  …  +  codeword[n-1]
//!    ↑ highest degree                                             ↑ constant term
//! ```
//!
//! The **systematic** codeword lays out as:
//!
//! ```text
//! [ message[0] … message[k-1] | check[0] … check[n_check-1] ]
//!   degree n-1 … degree n_check    degree n_check-1 … degree 0
//! ```
//!
//! The message occupies the *high-degree* positions and the check bytes the
//! *low-degree* positions. This is the standard RS / QR-code big-endian convention.
//!
//! ### Internal representations
//!
//! | Data | Representation | Why |
//! |------|----------------|-----|
//! | Codeword byte array | Big-endian (BE) | Public API; index = physical byte position |
//! | Syndrome evaluation | `poly_eval_be` on BE codeword | Gives `C(αⁱ)` directly |
//! | Generator g(x) | Little-endian (LE) in `build_generator`; reversed to BE for division | LE is natural for convolution; BE for long division |
//! | Berlekamp-Massey / Λ(x) / Ω(x) | Little-endian (LE) | LE is natural for LFSR polynomials |
//!
//! ### Error locator numbers
//!
//! For a **big-endian** codeword of length `n`, the **locator number** for the
//! byte at position `p` (0-indexed from the first byte) is:
//!
//! ```text
//! X_p = α^{n-1-p}
//! ```
//!
//! `X_p` has this form because `codeword[p]` contributes to degree `n-1-p`.
//! The syndrome `Sⱼ = C(αʲ)` thus satisfies `Sⱼ = Σ e_p · X_p^j`.
//!
//! The **inverse locator** (root of Λ) is:
//!
//! ```text
//! X_p⁻¹ = α^{-(n-1-p)} = α^{(p + 256 - n) mod 255}
//! ```
//!
//! This formula is used consistently in both Chien search and Forney.

use gf256::{add, divide, multiply, power};

// =============================================================================
// Error type
// =============================================================================

/// Errors returned by RS encoding and decoding.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RSError {
    /// The received codeword has more than `t = n_check/2` errors. The data
    /// cannot be recovered — too many bytes were corrupted.
    TooManyErrors,
    /// The input arguments are invalid (e.g. `n_check` is odd, zero, or the
    /// total codeword length would exceed 255 bytes).
    InvalidInput(String),
}

impl std::fmt::Display for RSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RSError::TooManyErrors => write!(f, "too many errors: codeword is unrecoverable"),
            RSError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
        }
    }
}

impl std::error::Error for RSError {}

// =============================================================================
// Generator Polynomial
// =============================================================================

/// Build the RS generator polynomial for a given number of check bytes.
///
/// The generator is the product of `n_check` linear factors:
///
/// ```text
/// g(x) = (x + α¹)(x + α²)…(x + α^{n_check})
/// ```
///
/// where `α = 2` is the primitive element of GF(256).
///
/// ## Return value
///
/// A **little-endian** coefficient array (index = degree), length `n_check + 1`.
/// The leading coefficient (last element) is always `1` (monic).
///
/// ## Algorithm
///
/// Start with `g = [1]` (the constant 1). At each step multiply by `(αⁱ + x)`:
///
/// ```text
/// new_g[j] = GF256.mul(αⁱ, g[j]) XOR g[j-1]
/// ```
///
/// where out-of-bounds reads are 0.
///
/// ## Example: n_check = 2
///
/// ```text
/// Start: g = [1]
/// i=1: [α¹, 1] = [2, 1]   (g = multiply([1], [2, 1]))
/// i=2: [GF256.mul(2,4), 2 XOR 4, 1] = [8, 6, 1]   (8 + 6x + x²)
/// ```
///
/// Verify root α¹: g(2) = 8 XOR GF256.mul(6,2) XOR GF256.mul(1,4) = 8 XOR 12 XOR 4 = 0 ✓
pub fn build_generator(n_check: usize) -> Result<Vec<u8>, RSError> {
    if n_check == 0 || n_check % 2 != 0 {
        return Err(RSError::InvalidInput(format!(
            "n_check must be a positive even number, got {n_check}"
        )));
    }

    // Start with g(x) = 1.
    let mut g = vec![1u8];

    for i in 1..=n_check {
        let alpha_i = power(2, i as u32);
        let mut new_g = vec![0u8; g.len() + 1];
        for (j, &coeff) in g.iter().enumerate() {
            new_g[j] ^= multiply(coeff, alpha_i);
            new_g[j + 1] ^= coeff;
        }
        g = new_g;
    }

    Ok(g)
}

// =============================================================================
// GF(256) Polynomial Helpers
// =============================================================================

/// Evaluate a **big-endian** GF(256) polynomial at field element `x`.
///
/// In the big-endian representation, `p[0]` is the *highest-degree* coefficient.
/// Horner's method reads left-to-right:
///
/// ```text
/// acc = 0
/// for each byte b in the array (highest degree first):
///     acc = acc · x + b
/// ```
///
/// This is the convention used for codeword bytes and syndrome evaluation:
/// `S_j = poly_eval_be(codeword, α^j) = C(α^j)`.
fn poly_eval_be(p: &[u8], x: u8) -> u8 {
    p.iter().fold(0u8, |acc, &c| add(multiply(acc, x), c))
}

/// Evaluate a **little-endian** GF(256) polynomial at `x`.
///
/// `p[i]` is the coefficient of `xⁱ`. Horner iterates from high to low degree:
///
/// ```text
/// acc = p[n], then: acc = acc·x + p[n-1], …, acc = acc·x + p[0]
/// ```
fn poly_eval_le(p: &[u8], x: u8) -> u8 {
    p.iter().rev().fold(0u8, |acc, &c| add(multiply(acc, x), c))
}

/// Multiply two **little-endian** GF(256) polynomials (convolution).
///
/// `result[i+j] XOR= a[i] · b[j]`
fn poly_mul_le(a: &[u8], b: &[u8]) -> Vec<u8> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut result = vec![0u8; a.len() + b.len() - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            result[i + j] ^= multiply(ai, bj);
        }
    }
    result
}

/// Compute the remainder of **big-endian** polynomial division in GF(256).
///
/// `dividend` and `divisor` are both big-endian (first element = highest degree).
/// The `divisor` must be **monic** (leading coefficient = 1).
///
/// ## Algorithm (big-endian long division)
///
/// Process the dividend from left (highest degree) to right.  At each step,
/// the current leading term is eliminated by subtracting a scaled copy of the
/// divisor:
///
/// ```text
/// for i = 0 .. (len(dividend) - len(divisor)):
///     coeff = dividend[i]          // leading term; divisor is monic so no division
///     for j = 0 .. len(divisor):
///         dividend[i+j] ^= coeff · divisor[j]
/// ```
///
/// After all steps, `dividend[0..k]` are zeroed out and `dividend[k..]`
/// contains the remainder (length = `len(divisor) - 1`).
///
/// ## Why big-endian for encoding
///
/// The codeword polynomial `C(x) = M(x)·x^{n_check} + R(x)` (big-endian) is
/// exactly `dividend = message || zeros`, and `R` is the last `n_check` bytes
/// of `dividend` after this division.  This guarantees `C(αⁱ) = 0` for all
/// roots `αⁱ` of `g`.
fn poly_mod_be(dividend: &[u8], divisor: &[u8]) -> Vec<u8> {
    assert!(!divisor.is_empty(), "polynomial division by zero");
    // divisor[0] == 1 because we only call this with the monic generator.
    debug_assert_eq!(
        divisor[0], 1,
        "poly_mod_be requires a monic divisor (leading coefficient 1)"
    );

    let mut rem = dividend.to_vec();
    let div_len = divisor.len();

    if rem.len() < div_len {
        // Dividend has lower degree than divisor: remainder is the dividend itself.
        return rem;
    }

    let steps = rem.len() - div_len + 1;
    for i in 0..steps {
        let coeff = rem[i];
        if coeff == 0 {
            continue;
        }
        // Eliminate term i by subtracting coeff · (divisor shifted to position i).
        // Since divisor[0]=1: rem[i] ^= coeff*1 = coeff → becomes 0 automatically.
        // We still XOR it to be explicit (no branch needed for the monic term).
        for (j, &d) in divisor.iter().enumerate() {
            rem[i + j] ^= multiply(coeff, d);
        }
    }

    // Remainder is the last (div_len - 1) bytes.
    rem[rem.len() - (div_len - 1)..].to_vec()
}

// =============================================================================
// Encoding
// =============================================================================

/// Encode a message using systematic Reed-Solomon.
///
/// **Systematic** means the original message bytes appear at the *start* of the
/// output (high-degree positions), followed by `n_check` computed check bytes:
///
/// ```text
/// output = [ message bytes  |  check bytes ]
///            degree n-1..n_check  degree n_check-1..0
/// ```
///
/// ## Algorithm
///
/// 1. Build the **big-endian** generator `g_BE` (reversed from the LE output of
///    `build_generator`).
/// 2. Append `n_check` zero bytes to the message: `shifted = message || 000…0`.
///    In polynomial terms this is `M(x) · x^{n_check}`.
/// 3. Compute `R = shifted mod g_BE` using big-endian long division.
///    `R` has exactly `n_check` bytes (padded with leading zeros if needed).
/// 4. Output `message || R`.
///
/// ## Why this produces a valid codeword
///
/// `C(x) = M(x)·x^{n_check} XOR R(x)` satisfies `C(αⁱ) = 0` for `i=1…n_check`
/// because `R(x) = M(x)·x^{n_check} mod g(x)` by construction:
///
/// ```text
/// C(x) = M(x)·x^{n_check} + R(x)
///       = M(x)·x^{n_check} + (M(x)·x^{n_check} mod g(x))
///       = Q(x)·g(x)    (the quotient times the divisor)
/// ```
///
/// So `C(αⁱ) = Q(αⁱ)·g(αⁱ) = Q(αⁱ)·0 = 0`.
///
/// ## Constraints
///
/// - `n_check` must be even and ≥ 2.
/// - `message.len() + n_check ≤ 255`.
pub fn encode(message: &[u8], n_check: usize) -> Result<Vec<u8>, RSError> {
    if n_check == 0 || n_check % 2 != 0 {
        return Err(RSError::InvalidInput(format!(
            "n_check must be a positive even number, got {n_check}"
        )));
    }
    let n = message.len() + n_check;
    if n > 255 {
        return Err(RSError::InvalidInput(format!(
            "total codeword length {n} exceeds GF(256) block size limit of 255"
        )));
    }

    // Build generator in big-endian form (reversed from the LE output).
    let g_le = build_generator(n_check)?;
    let g_be: Vec<u8> = g_le.iter().rev().cloned().collect();
    // g_be[0] = 1 (monic, leading coefficient) ✓

    // Shift: append n_check zeros to the message (represents M(x)·x^{n_check}).
    let mut shifted = message.to_vec();
    shifted.resize(n, 0);

    // Compute the remainder R = shifted mod g_be.
    let remainder = poly_mod_be(&shifted, &g_be);
    // remainder has length n_check (exactly n_check bytes; pad leading zeros if shorter).

    // Assemble: message bytes then check bytes.
    let mut codeword = message.to_vec();
    let pad = n_check - remainder.len();
    for _ in 0..pad {
        codeword.push(0);
    }
    codeword.extend_from_slice(&remainder);

    Ok(codeword)
}

// =============================================================================
// Decoding
// =============================================================================

/// Compute the `n_check` syndromes of a received codeword.
///
/// `Sⱼ = received(αʲ)` for `j = 1, …, n_check`.
///
/// If the codeword has no errors, all syndromes are zero (because every valid
/// codeword is divisible by `g(x)`, so it evaluates to zero at each root αⁱ).
///
/// ## Evaluation convention (big-endian)
///
/// `received[0]` is the highest-degree coefficient, `received[n-1]` is the
/// constant term.  Horner's method gives `Sⱼ = received(αʲ)`:
///
/// ```text
/// Sⱼ = received[0]·(αʲ)^{n-1} + received[1]·(αʲ)^{n-2} + … + received[n-1]
/// ```
///
/// An error at position `p` (big-endian index) with magnitude `e` contributes
/// `e · (αʲ)^{n-1-p} = e · Xₚʲ` where the **locator number** `Xₚ = α^{n-1-p}`.
pub fn syndromes(received: &[u8], n_check: usize) -> Vec<u8> {
    (1..=n_check)
        .map(|i| {
            let alpha_i = power(2, i as u32);
            poly_eval_be(received, alpha_i)
        })
        .collect()
}

/// Check if all syndromes are zero (no errors detected).
fn all_zero(s: &[u8]) -> bool {
    s.iter().all(|&x| x == 0)
}

/// Berlekamp-Massey: find the shortest LFSR generating the syndrome sequence.
///
/// Returns `(Λ, L)` where `Λ` is the **error locator polynomial** in
/// **little-endian** form (`Λ[0] = 1`) and `L` is the number of errors found.
///
/// ## Background
///
/// Think of `S₁, S₂, …, S_{2t}` as the output of a linear feedback shift
/// register (LFSR). BM finds the shortest LFSR that generates that sequence.
/// The LFSR *connection polynomial* is `Λ(x)`.
///
/// If the codeword has `v ≤ t` errors with locator numbers `X₁, …, Xᵥ`:
///
/// ```text
/// Λ(x) = ∏_{k=1}^{v} (1 - X_k · x)
/// ```
///
/// The **roots** of `Λ` are `Xₖ⁻¹`.  These are found via Chien search.
///
/// ## Algorithm
///
/// ```text
/// C = [1], B = [1], L = 0, x_shift = 1, b = 1
///
/// for n = 0 to 2t-1:
///     d = S[n] XOR ∑_{j=1}^{L} C[j] · S[n-j]   ← discrepancy
///
///     if d == 0:
///         x_shift += 1
///     elif 2L ≤ n:
///         T = C.clone()
///         C = C XOR (d/b) · x^{x_shift} · B
///         L = n + 1 - L;  B = T;  b = d;  x_shift = 1
///     else:
///         C = C XOR (d/b) · x^{x_shift} · B
///         x_shift += 1
/// ```
fn berlekamp_massey(syndromes: &[u8]) -> (Vec<u8>, usize) {
    let two_t = syndromes.len();

    let mut c = vec![1u8]; // current locator (LE)
    let mut b = vec![1u8]; // previous locator (LE)
    let mut big_l: usize = 0;
    let mut x: usize = 1;
    let mut b_scale: u8 = 1;

    for n in 0..two_t {
        // Discrepancy: d = S[n] + ∑_{j=1}^{L} C[j]·S[n-j]
        let mut d = syndromes[n];
        for j in 1..=big_l {
            if j < c.len() && n >= j {
                d ^= multiply(c[j], syndromes[n - j]);
            }
        }

        if d == 0 {
            x += 1;
        } else if 2 * big_l <= n {
            let t_save = c.clone();

            let scale = divide(d, b_scale);
            let shifted_len = x + b.len();
            if c.len() < shifted_len {
                c.resize(shifted_len, 0);
            }
            for (k, &bk) in b.iter().enumerate() {
                c[x + k] ^= multiply(scale, bk);
            }

            big_l = n + 1 - big_l;
            b = t_save;
            b_scale = d;
            x = 1;
        } else {
            let scale = divide(d, b_scale);
            let shifted_len = x + b.len();
            if c.len() < shifted_len {
                c.resize(shifted_len, 0);
            }
            for (k, &bk) in b.iter().enumerate() {
                c[x + k] ^= multiply(scale, bk);
            }
            x += 1;
        }
    }

    (c, big_l)
}

/// Compute the inverse locator for byte position `p` in a codeword of length `n`.
///
/// In the big-endian codeword polynomial, position `p` has degree `n-1-p`.
/// The locator number is `Xₚ = α^{n-1-p}`, so its inverse is:
///
/// ```text
/// Xₚ⁻¹ = α^{-(n-1-p)} = α^{(p + 256 - n) mod 255}
/// ```
///
/// Special cases:
/// - `p = 0` (first byte, highest degree): `Xₚ⁻¹ = α^{256-n mod 255}`
/// - `p = n-1` (last byte, degree 0): `Xₚ⁻¹ = α^{255 mod 255} = α⁰ = 1`
///
/// This formula is used identically in both Chien search and Forney.
#[inline]
fn inv_locator(p: usize, n: usize) -> u8 {
    let exp = (p + 256 - n) % 255;
    power(2, exp as u32)
}

/// Chien Search: find which positions are error locations.
///
/// Position `p` is an error location if `Λ(Xₚ⁻¹) = 0`, i.e. if `Xₚ⁻¹` is
/// a root of `Λ`.  We check all `n` positions using `inv_locator(p, n)`.
///
/// ## Return
///
/// Sorted list of error positions `p` (0-indexed, big-endian).
/// If the count of found positions differs from `degree(Λ)`, the codeword is
/// unrecoverable (Chien found fewer roots than expected).
fn chien_search(lambda: &[u8], n: usize) -> Vec<usize> {
    let mut positions = Vec::new();

    for p in 0..n {
        let xi_inv = inv_locator(p, n);
        if poly_eval_le(lambda, xi_inv) == 0 {
            positions.push(p);
        }
    }

    positions
}

/// Forney Algorithm: compute error magnitudes from positions.
///
/// Given error positions `{p₁, …, pᵥ}`, XOR-ing each received byte with
/// its magnitude recovers the original.
///
/// ## The three polynomials
///
/// **Syndrome polynomial** `S(x)` (little-endian):
/// ```text
/// S(x) = S₁ + S₂x + … + S_{2t}x^{2t-1}
/// ```
///
/// **Error evaluator** `Ω(x) = (S(x) · Λ(x)) mod x^{2t}` (little-endian).
/// Truncating to `2t` terms removes the high-degree part.
///
/// **Forney formula** for position `p` with locator `Xₚ = α^{n-1-p}`:
/// ```text
/// eₚ = Ω(Xₚ⁻¹) / Λ'(Xₚ⁻¹)
/// ```
///
/// ## Formal derivative in characteristic 2
///
/// `Λ'(x)` keeps only the odd-indexed coefficients of `Λ`, each reduced in
/// degree by 1 (even terms vanish because `j·Λⱼ = 0` for even `j` in char 2):
///
/// ```text
/// Λ'(x) = Λ₁ + Λ₃x² + Λ₅x⁴ + …
/// ```
///
/// So `Λ'[k] = Λ[k+1]` when `k+1` is odd (k is even), else `0`.
fn forney(
    lambda: &[u8],
    syndromes: &[u8],
    positions: &[usize],
    n: usize,
) -> Result<Vec<u8>, RSError> {
    let two_t = syndromes.len();

    // S(x) in LE: S[0] = S₁, S[1] = S₂, …
    let s_poly: Vec<u8> = syndromes.to_vec();

    // Ω(x) = S(x) · Λ(x) mod x^{2t}: keep only the first 2t coefficients.
    let omega_full = poly_mul_le(&s_poly, lambda);
    let omega: Vec<u8> = omega_full.into_iter().take(two_t).collect();

    // Formal derivative Λ'(x) in char 2.
    // Λ'[k] = Λ[k+1] if k+1 is odd (i.e. k is even), else 0.
    let mut lambda_prime = vec![0u8; lambda.len().saturating_sub(1)];
    for (j, &lj) in lambda.iter().enumerate() {
        if j % 2 == 1 {
            // j is odd → contributes to Λ'[j-1]
            let out_idx = j - 1;
            if out_idx < lambda_prime.len() {
                lambda_prime[out_idx] ^= lj;
            }
        }
    }

    // Compute each error magnitude using the inverse locator.
    let mut magnitudes = Vec::with_capacity(positions.len());
    for &pos in positions {
        let xi_inv = inv_locator(pos, n);

        let omega_val = poly_eval_le(&omega, xi_inv);
        let lambda_prime_val = poly_eval_le(&lambda_prime, xi_inv);

        if lambda_prime_val == 0 {
            return Err(RSError::TooManyErrors);
        }

        magnitudes.push(divide(omega_val, lambda_prime_val));
    }

    Ok(magnitudes)
}

/// Decode a received Reed-Solomon codeword, correcting up to `t = n_check/2` errors.
///
/// ## Pipeline
///
/// ```text
/// received
///    │
///    ▼ Step 1: Compute syndromes S₁ … S_{n_check}
///    │         all zero? → no errors, return message bytes directly
///    │
///    ▼ Step 2: Berlekamp-Massey → error locator Λ(x), error count L
///    │         L > t? → TooManyErrors
///    │
///    ▼ Step 3: Chien search → error positions {p₁, …, pᵥ}
///    │         |positions| ≠ L? → TooManyErrors
///    │
///    ▼ Step 4: Forney → error magnitudes {e₁, …, eᵥ}
///    │
///    ▼ Step 5: Correct: received[pₖ] XOR= eₖ
///    │
///    ▼ Return first k = len - n_check bytes (strip check bytes)
/// ```
pub fn decode(received: &[u8], n_check: usize) -> Result<Vec<u8>, RSError> {
    if n_check == 0 || n_check % 2 != 0 {
        return Err(RSError::InvalidInput(format!(
            "n_check must be a positive even number, got {n_check}"
        )));
    }
    if received.len() < n_check {
        return Err(RSError::InvalidInput(format!(
            "received length {} < n_check {}",
            received.len(),
            n_check
        )));
    }

    let t = n_check / 2;
    let n = received.len();
    let k = n - n_check;

    // --- Step 1: Syndromes ---
    let synds = syndromes(received, n_check);

    if all_zero(&synds) {
        return Ok(received[..k].to_vec());
    }

    // --- Step 2: Berlekamp-Massey ---
    let (lambda, num_errors) = berlekamp_massey(&synds);

    if num_errors > t {
        return Err(RSError::TooManyErrors);
    }

    // --- Step 3: Chien Search ---
    let positions = chien_search(&lambda, n);

    if positions.len() != num_errors {
        return Err(RSError::TooManyErrors);
    }

    // --- Step 4: Forney ---
    let magnitudes = forney(&lambda, &synds, &positions, n)?;

    // --- Step 5: Apply corrections ---
    let mut corrected = received.to_vec();
    for (&pos, &mag) in positions.iter().zip(magnitudes.iter()) {
        corrected[pos] ^= mag;
    }

    Ok(corrected[..k].to_vec())
}

/// Compute the error locator polynomial directly from syndromes.
///
/// Exposed as a public function for testing and for higher-level tools
/// (e.g. QR decoders) that may want to inspect the LFSR structure.
///
/// Returns `Λ(x)` in **little-endian** form, `Λ[0] = 1`.
pub fn error_locator(syndromes: &[u8]) -> Vec<u8> {
    let (lambda, _) = berlekamp_massey(syndromes);
    lambda
}

// ReedSolomon.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Reed-Solomon Error-Correcting Codes over GF(256)
// ============================================================================
//
// Reed-Solomon (RS) codes are a family of block error-correcting codes invented
// by Irving Reed and Gustave Solomon in 1960. They are used everywhere:
//
//   - QR codes: up to 30% of the symbol can be scratched and still decoded.
//   - CDs and DVDs: CIRC two-level RS corrects scratches and burst errors.
//   - Hard drives: firmware sector-level error correction.
//   - Voyager probes: images sent across 20+ billion kilometres.
//   - RAID-6: the two parity drives ARE an (n, n-2) RS code over GF(256).
//
// The core insight is simple: add redundancy to a message so that even after
// some bytes are corrupted the original can be reconstructed.
//
// ============================================================================
// How It Fits in the MA Series
// ============================================================================
//
//   MA00 polynomial   — coefficient-array polynomial arithmetic over Double
//   MA01 gf256        — GF(2^8) field arithmetic (add=XOR, mul=table lookup)
//   MA02 reed-solomon — RS encoding / decoding (THIS PACKAGE)
//
// RS encoding is just polynomial multiplication over GF(256).
// RS decoding is Berlekamp-Massey + Chien search + Forney — all polynomial
// operations over GF(256), composed into a 5-step pipeline.
//
// ============================================================================
// Polynomial Conventions (CRITICAL — must match all other languages)
// ============================================================================
//
// Two conventions coexist in this file:
//
//   Big-endian (BE): codeword[0] is the highest-degree coefficient.
//     Used for codewords, received messages, and syndrome evaluation.
//     codeword = [a_{n-1}, a_{n-2}, ..., a_1, a_0]
//
//   Little-endian (LE): poly[i] is the coefficient of x^i.
//     Used for internal polynomial algebra (generator, locator, omega).
//     poly = [a_0, a_1, a_2, ..., a_n]
//
// The boundary between the two conventions is clearly marked in each function.
//
// The systematic codeword layout is:
//   [ message bytes (k) | check bytes (n_check) ]
//     degree n-1 … n_check   degree n_check-1 … 0
//
// This is the standard RS / QR code convention.
//
// ============================================================================
// Error-Correction Capacity
// ============================================================================
//
// An RS code with n_check check bytes can correct up to t = n_check / 2 byte
// errors in unknown positions. The check bytes must be a positive even number.
//
// Why even? Because the decoding pipeline (Berlekamp-Massey) needs 2t
// syndromes to locate t errors. Odd n_check wastes one syndrome.
//
// ============================================================================
// The Five-Step Decoding Pipeline
// ============================================================================
//
//   received bytes
//        │
//        ▼  Step 1: Syndromes S₁ … S_{n_check}
//        │          all zero → no errors, return message directly
//        │
//        ▼  Step 2: Berlekamp-Massey → Λ(x), error count L
//        │          L > t → TooManyErrors
//        │
//        ▼  Step 3: Chien search → error positions {p₁ … pᵥ}
//        │          |positions| ≠ L → TooManyErrors
//        │
//        ▼  Step 4: Forney → error magnitudes {e₁ … eᵥ}
//        │
//        ▼  Step 5: corrected[p_k] ^= e_k  for each k
//        │
//        ▼  Return first k = len(received) - n_check bytes
//
// ============================================================================

import GF256

// ============================================================================
// MARK: - Public Namespace
// ============================================================================
//
// We wrap the entire RS API in `public enum ReedSolomon` (a namespace enum).
// Swift enums with no cases cannot be instantiated — they are pure namespaces.
// This keeps call sites clean: `ReedSolomon.encode(...)`.

/// Reed-Solomon error-correcting codes over GF(2^8).
///
/// Elements are `[UInt8]` byte arrays. All operations work in GF(256):
/// addition is XOR, multiplication uses log/antilog tables.
///
/// ## Quick Reference
///
///   ReedSolomon.encode([72, 101, 108], nCheck: 4)  // → 7-byte codeword
///   ReedSolomon.decode(corrupted, nCheck: 4)        // → original message
///   ReedSolomon.syndromes(codeword, nCheck: 4)      // → [0,0,0,0] if valid
///   ReedSolomon.buildGenerator(4)                   // → LE generator poly
///   ReedSolomon.errorLocator(syndromes)             // → LE locator poly Λ(x)
///
public enum ReedSolomon {

    // ========================================================================
    // MARK: - Error Types
    // ========================================================================

    /// Thrown when decoding fails because there are more errors than t = nCheck/2.
    ///
    /// The code can correct at most t byte errors. If more are present the
    /// codeword is unrecoverable and this error is thrown rather than silently
    /// returning wrong data.
    public struct TooManyErrors: Error {
        public init() {}
    }

    /// Thrown when encode / decode receives invalid parameters.
    ///
    /// Common causes:
    /// - nCheck is 0 or odd (must be a positive even number)
    /// - total codeword length exceeds 255 (the GF(256) block size limit)
    /// - received codeword is shorter than nCheck
    public struct InvalidInput: Error {
        public let reason: String
        public init(_ reason: String) { self.reason = reason }
    }

    // ========================================================================
    // MARK: - Generator Polynomial
    // ========================================================================
    //
    // The RS generator polynomial for n_check check bytes is the product of
    // n_check linear factors:
    //
    //   g(x) = (x + α¹)(x + α²) … (x + α^{n_check})
    //
    // where α = 2 is the primitive element of GF(256).
    //
    // The result is in LITTLE-ENDIAN form: g[i] is the coefficient of x^i.
    // The last element is always 1 (the monic leading coefficient of x^{n_check}).
    //
    // Example: buildGenerator(2)
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~
    //
    //   Start: g = [1]
    //   i=1: α¹ = 2, factor = (x + 2) = [2, 1]
    //     poly_mul_le([1], [2, 1]) = [2, 1]
    //   i=2: α² = 4, factor = (x + 4) = [4, 1]
    //     poly_mul_le([2, 1], [4, 1]):
    //       [0] = 2*4 = 8
    //       [1] = 2*1 ^ 1*4 = 2^4 = 6
    //       [2] = 1*1 = 1
    //     g = [8, 6, 1]
    //
    // Verify α¹=2 is a root (all GF(256)):
    //   g(2) = 8 + 6·2 + 1·4
    //        = 8 ^ mul(6,2) ^ mul(1,4)
    //        = 8 ^ 12 ^ 4
    //        = 0  ✓
    //
    // Cross-language test vector: buildGenerator(2) must return [8, 6, 1].

    /// Build the RS generator polynomial for a given number of check bytes.
    ///
    /// - Parameter nCheck: Number of check bytes (must be even and > 0).
    /// - Returns: Little-endian coefficient array of length nCheck+1.
    public static func buildGenerator(_ nCheck: Int) -> [UInt8] {
        // Start with the constant polynomial 1.
        var g: [UInt8] = [1]

        // Multiply in each linear factor (x + α^i) for i = 1 to nCheck.
        // In little-endian: [α^i, 1] means α^i·x⁰ + 1·x¹ = x + α^i.
        for i in 1...nCheck {
            let factor: [UInt8] = [GF256.power(2, UInt32(i)), 1]
            g = polyMulLE(g, factor)
        }

        return g
    }

    // ========================================================================
    // MARK: - Encoding
    // ========================================================================
    //
    // Systematic encoding: the output codeword is [ message | check_bytes ].
    //
    // The check bytes are the remainder of dividing M(x)·x^{n_check} by g(x),
    // where M(x) is the message polynomial (big-endian) and g(x) is the
    // generator (little-endian, then reversed to big-endian for division).
    //
    // Why does this give a valid codeword?
    //
    //   C(x) = M(x)·x^{n_check}  XOR  R(x)
    //        = Q(x)·g(x)         (by the definition of remainder)
    //
    // So C(α^i) = Q(α^i)·g(α^i) = 0 for i = 1 … n_check, because α^i is a
    // root of g(x). This is the property the decoder exploits.

    /// Encode a message with Reed-Solomon, producing a systematic codeword.
    ///
    /// - Parameters:
    ///   - message: Raw data bytes (arbitrary content).
    ///   - nCheck: Number of check bytes to append. Must be a positive even integer.
    /// - Returns: Codeword of length message.count + nCheck.
    /// - Throws: `InvalidInput` if nCheck is 0 or odd, or total length > 255.
    public static func encode(_ message: [UInt8], nCheck: Int) throws -> [UInt8] {
        // Validate parameters.
        guard nCheck > 0, nCheck % 2 == 0 else {
            throw InvalidInput("nCheck must be a positive even number, got \(nCheck)")
        }
        let n = message.count + nCheck
        guard n <= 255 else {
            throw InvalidInput(
                "total codeword length \(n) exceeds GF(256) block size limit of 255"
            )
        }

        // Build generator in LE form.
        // polyModBE takes the generator in LE form and reverses it internally.
        let gen = buildGenerator(nCheck)

        // Shifted message: M(x)·x^{n_check} in big-endian form.
        // Appending n_check zeros shifts the polynomial degree up by n_check.
        let padded = message + [UInt8](repeating: 0, count: nCheck)

        // Remainder = padded mod gen (check bytes, big-endian, length nCheck).
        let check = polyModBE(padded, gen)

        // Codeword = message || check_bytes.
        return message + check
    }

    // ========================================================================
    // MARK: - Syndrome Computation
    // ========================================================================
    //
    // The syndrome S_j = received(α^j) evaluates the received polynomial at
    // successive powers of the primitive element α = 2.
    //
    // For a valid codeword C(x) divisible by g(x) = ∏(x + α^i):
    //   C(α^i) = 0   for  i = 1 … n_check
    //
    // So all syndromes are zero for a valid codeword.
    //
    // An error at position p adds e · (α^j)^{n-1-p} = e · X_p^j to S_j,
    // where X_p = α^{n-1-p} is the error locator number for position p.
    //
    // If every syndrome is zero → no errors detected → return message directly.
    // Any non-zero syndrome → at least one error exists.

    /// Compute the nCheck syndrome values of a received codeword.
    ///
    /// - Parameters:
    ///   - received: Codeword bytes (possibly corrupted), big-endian.
    ///   - nCheck: Number of check bytes.
    /// - Returns: Array of nCheck syndromes. All-zero means no errors detected.
    public static func syndromes(_ received: [UInt8], nCheck: Int) -> [UInt8] {
        // S_j = received(α^j) for j = 1, 2, ..., n_check.
        // We evaluate the big-endian polynomial using Horner's method.
        return (1...nCheck).map { j in
            polyEvalBE(received, GF256.power(2, UInt32(j)))
        }
    }

    // ========================================================================
    // MARK: - Error Locator Polynomial
    // ========================================================================
    //
    // The error locator polynomial Λ(x) encodes the positions of all errors:
    //
    //   Λ(x) = ∏_{k=1}^{v} (1 - X_k · x)    where Λ(0) = 1
    //
    // Its roots at x = X_k^{-1} reveal error positions via Chien search.
    //
    // We compute Λ using the Berlekamp-Massey (BM) algorithm, which finds the
    // shortest linear-feedback shift register (LFSR) that generates the syndrome
    // sequence. The LFSR connection polynomial IS the error locator polynomial.
    //
    // Berlekamp-Massey Algorithm
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~
    //
    //   Inputs: syndromes S[0..2t-1] (0-based)
    //   Output: Λ(x) in LE form with Λ(0) = 1
    //
    //   Initialize: c = [1], b = [1], l = 0, x = 1
    //
    //   For n from 0 to 2t-1:
    //     Compute discrepancy:
    //       d = S[n] XOR Σ_{j=1}^{l} c[j] · S[n-j]
    //
    //     If d == 0: no update needed, advance shift counter
    //
    //     Else:
    //       bShifted = [0]*x + b        (shift b left by x positions)
    //       t_new    = c XOR (d · bShifted)   (update candidate)
    //
    //       If 2·l <= n (need more error capacity):
    //         l   = n + 1 - l           (new error count)
    //         b   = (1/d) · c           (save current locator scaled by 1/d)
    //         c   = t_new
    //         x   = 1
    //       Else:
    //         c = t_new
    //         x += 1

    /// Compute the error locator polynomial Λ(x) from a syndrome array.
    ///
    /// Runs the Berlekamp-Massey algorithm. Returns Λ in **little-endian** form
    /// with Λ[0] = 1. The degree of Λ equals the number of errors detected.
    ///
    /// - Parameter syndromes: Syndrome array of length 2t.
    /// - Returns: Error locator polynomial in LE form.
    public static func errorLocator(_ syndromes: [UInt8]) -> [UInt8] {
        let n = syndromes.count

        // c = current error locator polynomial Λ (LE, starts as [1] = constant 1)
        var c: [UInt8] = [1]
        // b = previous Λ saved for the update step (LE)
        var b: [UInt8] = [1]
        // l = current number of errors tracked by c
        var l = 0
        // x = number of iterations since last significant update
        var x = 1

        for i in 0..<n {
            // ----------------------------------------------------------------
            // Step 1: Compute discrepancy d.
            //
            // d = S[i] XOR Σ_{j=1}^{l}  c[j] · S[i-j]
            //
            // If d == 0, the current Λ already predicts S[i] correctly.
            // If d ≠ 0, the locator needs updating.
            // ----------------------------------------------------------------
            var d: UInt8 = syndromes[i]
            for j in 1..<c.count {
                d = GF256.add(d, GF256.multiply(c[j], syndromes[i - j]))
            }

            if d == 0 {
                // No discrepancy: advance the shift counter, keep c unchanged.
                x += 1
            } else {
                // bShifted = [0, 0, ..., 0, b[0], b[1], ...]
                //            (x leading zeros, then b's coefficients)
                let bShifted = [UInt8](repeating: 0, count: x) + b

                // t_new = c XOR (d · bShifted)
                let t = polyAddLE(c, polyScaleLE(bShifted, d))

                if 2 * l <= i {
                    // Need to increase error capacity.
                    l = i + 1 - l
                    // Save current c scaled by 1/d as new b.
                    b = polyScaleLE(c, GF256.inverse(d))
                    c = t
                    x = 1
                } else {
                    // Consistent update: adjust c without growing degree.
                    c = t
                    x += 1
                }
            }
        }

        return c
    }

    // ========================================================================
    // MARK: - Full Decode
    // ========================================================================
    //
    // The full five-step decoder pipeline:
    //
    //   1. Syndromes: S_j = received(α^j)
    //   2. Berlekamp-Massey: Λ(x), num_errors
    //   3. Chien search: positions where Λ(X_p^{-1}) = 0
    //   4. Forney: error magnitudes e_p = Ω(X_p^{-1}) / Λ'(X_p^{-1})
    //   5. Apply corrections: received[p] ^= e_p
    //
    // Chien Search
    // ~~~~~~~~~~~~
    //
    // For each position p in 0..<n, compute X_p^{-1} = α^{(p + 256 - n) mod 255}.
    // If Λ(X_p^{-1}) = 0, then X_p is a root of Λ, meaning position p has an error.
    //
    // The formula (p + 256 - n) mod 255 handles the mapping from codeword position
    // to the cyclic group exponent. Position p (big-endian) corresponds to degree
    // n-1-p, so the locator X_p = α^{n-1-p} and its inverse X_p^{-1} = α^{p+1-n}.
    // Adding 256 keeps the exponent non-negative; mod 255 maps into the group.
    //
    // Forney Algorithm
    // ~~~~~~~~~~~~~~~~
    //
    // The error evaluator polynomial is:
    //   Ω(x) = S(x) · Λ(x)  mod  x^{2t}
    //
    // where S(x) = S₁ + S₂x + … + S_{2t}x^{2t-1} is the syndrome polynomial (LE).
    //
    // The formal derivative of Λ in GF(2^8):
    //   Λ'(x) = Λ₁ + Λ₃x² + Λ₅x⁴ + …
    // (In characteristic 2, even-degree terms vanish: d/dx(ax^{2k}) = 2k·ax^{2k-1} = 0)
    //
    // The error magnitude at position p:
    //   e_p = Ω(X_p^{-1}) / Λ'(X_p^{-1})

    /// Decode a received codeword, correcting up to t = nCheck/2 byte errors.
    ///
    /// - Parameters:
    ///   - received: Possibly corrupted codeword bytes (big-endian).
    ///   - nCheck: Number of check bytes (must be even ≥ 2).
    /// - Returns: Recovered message of length received.count - nCheck.
    /// - Throws: `InvalidInput` or `TooManyErrors`.
    public static func decode(_ received: [UInt8], nCheck: Int) throws -> [UInt8] {
        // Validate parameters.
        guard nCheck > 0, nCheck % 2 == 0 else {
            throw InvalidInput("nCheck must be a positive even number, got \(nCheck)")
        }
        guard received.count >= nCheck else {
            throw InvalidInput(
                "received length \(received.count) < nCheck \(nCheck)"
            )
        }

        let t = nCheck / 2
        let n = received.count
        let k = n - nCheck

        // ----------------------------------------------------------------
        // Step 1: Syndromes
        //
        // If all syndromes are zero, the codeword is valid. Return message.
        // ----------------------------------------------------------------
        let synds = syndromes(received, nCheck: nCheck)
        if synds.allSatisfy({ $0 == 0 }) {
            return Array(received.prefix(k))
        }

        // ----------------------------------------------------------------
        // Step 2: Berlekamp-Massey → Λ(x)
        //
        // The error locator polynomial has degree = number of errors.
        // If that exceeds t, we cannot correct — give up.
        // ----------------------------------------------------------------
        let lam = errorLocator(synds)
        let numErrors = lam.count - 1   // degree of Λ = number of errors
        guard numErrors <= t else {
            throw TooManyErrors()
        }

        // ----------------------------------------------------------------
        // Step 3: Chien search → error positions
        //
        // Test each candidate position p in 0..<n.
        // Position p is an error iff Λ(X_p^{-1}) = 0.
        // ----------------------------------------------------------------
        var positions: [Int] = []
        for p in 0..<n {
            // X_p^{-1} = α^{(p + 256 - n) mod 255}
            let xInv = GF256.power(2, UInt32((p + 256 - n) % 255))
            if polyEvalLE(lam, xInv) == 0 {
                positions.append(p)
            }
        }

        // If we found a different number of positions than expected, the
        // codeword is too corrupted to correct.
        guard positions.count == numErrors else {
            throw TooManyErrors()
        }

        // ----------------------------------------------------------------
        // Step 4: Forney → error magnitudes
        //
        // Ω(x) = S(x) · Λ(x)  mod  x^{n_check}  (LE, truncated)
        //
        // Formal derivative Λ'(x): in GF(2^8), even-degree terms vanish.
        //   Λ'[k] = Λ[k+1]  if (k+1) is odd, else 0
        //   i.e.: keep coefficients at odd positions, shift down by 1.
        //
        // Error magnitude at position p:
        //   e_p = Ω(X_p^{-1}) / Λ'(X_p^{-1})
        // ----------------------------------------------------------------
        let omega = Array(polyMulLE(synds, lam).prefix(nCheck))

        // Formal derivative: drop index 0, keep odd-indexed coefficients of Λ.
        // lambda_prime[k] = lam[k+1] if (k+1) is odd (i.e. k is even), else 0
        var lambdaPrime = Array(lam.dropFirst())
        for k in 0..<lambdaPrime.count {
            if (k + 1) % 2 == 0 { lambdaPrime[k] = 0 }
        }

        // Compute magnitudes and apply corrections.
        var corrected = received
        for p in positions {
            let xInv = GF256.power(2, UInt32((p + 256 - n) % 255))
            let omegaVal = polyEvalLE(omega, xInv)
            let lpVal = polyEvalLE(lambdaPrime, xInv)
            guard lpVal != 0 else { throw TooManyErrors() }
            let magnitude = GF256.divide(omegaVal, lpVal)
            corrected[p] = GF256.add(corrected[p], magnitude)
        }

        // ----------------------------------------------------------------
        // Step 5: Return the message portion (first k bytes).
        // ----------------------------------------------------------------
        return Array(corrected.prefix(k))
    }
}

// ============================================================================
// MARK: - Private Polynomial Helpers
// ============================================================================
//
// These helpers implement basic polynomial arithmetic over GF(256).
// They are NOT part of the public API — they are implementation details.
//
// Two conventions are in play:
//
//   BE (big-endian):  poly[0] = highest-degree coefficient
//   LE (little-endian): poly[i] = coefficient of x^i
//
// Functions are suffixed with BE or LE to make the convention explicit.

// ============================================================================
// polyEvalBE — Evaluate a big-endian polynomial using Horner's method
// ============================================================================
//
// Horner's method rewrites p(x) = a_n·x^n + … + a_0 as:
//   (…((a_n · x + a_{n-1}) · x + a_{n-2})…) · x + a_0
//
// This reduces n multiplications and n additions to exactly n of each,
// reading coefficients left-to-right in big-endian order.
//
// Used in syndrome evaluation: S_j = received(α^j).

/// Evaluate a big-endian GF(256) polynomial at `x` using Horner's method.
///
/// `poly[0]` is the highest-degree coefficient. Iteration goes left to right.
private func polyEvalBE(_ poly: [UInt8], _ x: UInt8) -> UInt8 {
    var acc: UInt8 = 0
    for b in poly {
        // acc = acc·x + b  (in GF(256))
        acc = GF256.add(GF256.multiply(acc, x), b)
    }
    return acc
}

// ============================================================================
// polyEvalLE — Evaluate a little-endian polynomial using Horner's method
// ============================================================================
//
// For LE polys, we read coefficients in reverse (highest degree first).
// Used for evaluating Λ(x), Ω(x), and Λ'(x) in the Chien / Forney steps.

/// Evaluate a little-endian GF(256) polynomial at `x` using Horner's method.
///
/// `poly[i]` is the coefficient of x^i. Iteration goes from highest degree down.
private func polyEvalLE(_ poly: [UInt8], _ x: UInt8) -> UInt8 {
    var acc: UInt8 = 0
    for coeff in poly.reversed() {
        acc = GF256.add(GF256.multiply(acc, x), coeff)
    }
    return acc
}

// ============================================================================
// polyMulLE — Multiply two little-endian polynomials (convolution in GF(256))
// ============================================================================
//
// Schoolbook polynomial multiplication:
//   (Σ_i a_i · x^i) · (Σ_j b_j · x^j) = Σ_{k} (Σ_{i+j=k} a_i · b_j) · x^k
//
// The result has degree deg(a) + deg(b), so length len(a) + len(b) - 1.
// In GF(256), addition is XOR (^=).
//
// Used in generator polynomial construction and in the Forney step
// to compute Ω(x) = S(x) · Λ(x).

/// Multiply two little-endian GF(256) polynomials (schoolbook convolution).
private func polyMulLE(_ a: [UInt8], _ b: [UInt8]) -> [UInt8] {
    guard !a.isEmpty, !b.isEmpty else { return [] }
    var result = [UInt8](repeating: 0, count: a.count + b.count - 1)
    for (i, ai) in a.enumerated() {
        for (j, bj) in b.enumerated() {
            result[i + j] = GF256.add(result[i + j], GF256.multiply(ai, bj))
        }
    }
    return result
}

// ============================================================================
// polyAddLE — Add two little-endian polynomials (XOR coefficient-wise)
// ============================================================================
//
// In GF(256), addition is XOR. Coefficient-wise XOR of two LE polynomials
// gives their sum. We pad the shorter one with zeros to match lengths.
//
// Used in Berlekamp-Massey to update the locator polynomial:
//   c_new = c  XOR  (d · bShifted)

/// Add two little-endian GF(256) polynomials (XOR each coefficient).
private func polyAddLE(_ a: [UInt8], _ b: [UInt8]) -> [UInt8] {
    let maxLen = max(a.count, b.count)
    var result = [UInt8](repeating: 0, count: maxLen)
    for i in 0..<a.count { result[i] = GF256.add(result[i], a[i]) }
    for i in 0..<b.count { result[i] = GF256.add(result[i], b[i]) }
    return result
}

// ============================================================================
// polyScaleLE — Multiply every coefficient of a little-endian polynomial
//               by a GF(256) scalar
// ============================================================================
//
// polyScaleLE([a_0, a_1, ..., a_n], s) = [s·a_0, s·a_1, ..., s·a_n]
//
// Used in Berlekamp-Massey to scale the auxiliary polynomial b:
//   new_b = (1/d) · c   (i.e. scale by inverse of discrepancy)

/// Multiply every coefficient of a little-endian polynomial by a GF(256) scalar.
private func polyScaleLE(_ poly: [UInt8], _ scalar: UInt8) -> [UInt8] {
    return poly.map { GF256.multiply($0, scalar) }
}

// ============================================================================
// polyModBE — Remainder of big-endian GF(256) polynomial long division
// ============================================================================
//
// Both dividend and generator are provided. The generator is in LE form (as
// returned by buildGenerator) and is reversed to BE internally for alignment.
//
// The generator is monic (leading coefficient = 1) by construction, which
// simplifies the division: the leading term of each step is just work[i].
//
// Algorithm (schoolbook long division):
//
//   genBE = reverse(genLE)     // convert LE to BE
//   work  = copy(dividend)
//   for i in 0...(len(work) - len(genBE)):
//       lead = work[i]
//       if lead != 0:
//           for j in 0..<len(genBE):
//               work[i+j] ^= lead · genBE[j]
//
// After the loop, the remainder sits in the last (len(genBE) - 1) = nCheck bytes.
//
// Why this works:
//   At each step we zero out work[i] by subtracting lead · genBE · x^{...}.
//   In GF(2^8), subtraction = addition = XOR, so ^= does both.
//
// Returns: last nCheck bytes of work (the remainder = check bytes).

/// Compute the remainder of big-endian GF(256) polynomial long division.
///
/// - Parameters:
///   - dividend: Big-endian polynomial (message padded with nCheck zeros).
///   - genLE: Generator polynomial in little-endian form (from buildGenerator).
/// - Returns: nCheck check bytes (big-endian remainder).
private func polyModBE(_ dividend: [UInt8], _ genLE: [UInt8]) -> [UInt8] {
    // Reverse the LE generator to BE so it aligns with the BE dividend.
    // genLE = [a_0, a_1, ..., a_k] → genBE = [a_k, ..., a_1, a_0]
    // Since the generator is monic, genBE[0] = 1 (the leading coefficient).
    let genBE = Array(genLE.reversed())
    var work = dividend
    let genLen = genBE.count

    // Long division: zero out each leading term by subtracting a multiple of genBE.
    // Guard against underflow: if work is shorter than genBE, no steps needed.
    guard work.count >= genLen else {
        return Array(work.suffix(genLen - 1))
    }
    for i in 0...(work.count - genLen) {
        let lead = work[i]
        if lead != 0 {
            for j in 0..<genLen {
                // work[i+j] ^= lead · genBE[j]
                // This subtracts (= adds in GF(2^8)) lead · x^{...} · genBE
                // from work, zeroing out work[i].
                work[i + j] = GF256.add(work[i + j], GF256.multiply(lead, genBE[j]))
            }
        }
    }

    // The remainder is the last (genLen - 1) = nCheck bytes of work.
    return Array(work.suffix(genLen - 1))
}

/**
 * Reed-Solomon error-correcting codes over GF(256).
 *
 * Reed-Solomon (RS) is a block error-correcting code. You add `nCheck`
 * redundancy bytes to a message, and the decoder can recover the original
 * message even if up to `t = nCheck / 2` bytes are corrupted anywhere in
 * the codeword.
 *
 * ## Where RS is used
 *
 * | System           | How RS helps                                        |
 * |------------------|-----------------------------------------------------|
 * | QR codes         | Up to 30% of a QR symbol can be destroyed/scratched |
 * | CDs / DVDs       | CIRC two-level RS corrects scratches and defects    |
 * | Hard drives      | Firmware sector-level error correction              |
 * | Voyager probes   | Transmit images across 20+ billion km               |
 * | RAID-6           | Two parity drives are exactly an (n, n-2) RS code   |
 *
 * ## Building blocks
 *
 * ```
 * MA01  gf256        — GF(2^8) field: add=XOR, mul=table lookup
 * MA00  polynomial   — GF(256) polynomial arithmetic (convolution, divmod, eval)
 * MA02  reed-solomon ← THIS PACKAGE
 * ```
 *
 * ## Polynomial conventions
 *
 * Codeword bytes are interpreted as a **big-endian** polynomial:
 *
 *   codeword[0]·x^{n-1} + codeword[1]·x^{n-2} + … + codeword[n-1]
 *
 * Systematic codeword layout:
 *
 *   [ message[0] … message[k-1] | check[0] … check[nCheck-1] ]
 *     degree n-1 … degree nCheck    degree nCheck-1 … degree 0
 *
 * The generator polynomial roots use the `b=1` convention: α^1, α^2, …, α^nCheck,
 * where α = 2 is the primitive element of GF(256).
 *
 * Spec: MA02-reed-solomon.md
 */
package com.codingadventures.reedsolomon

import com.codingadventures.gf256.GF256

/** Version of this package. */
const val VERSION = "0.1.0"

// =============================================================================
// Errors
// =============================================================================

/**
 * Thrown when a received codeword has more errors than the correction capacity
 * `t = nCheck / 2`.
 *
 * The codeword is unrecoverable. The caller should request a retransmission or
 * report a permanent error.
 */
class TooManyErrorsException(message: String = "reed-solomon: too many errors — codeword is unrecoverable") :
    Exception(message)

/**
 * Thrown when encode or decode receive invalid parameters.
 *
 * Common causes:
 * - nCheck is 0 or odd
 * - Total codeword length exceeds 255 (GF(256) block size limit)
 * - Received codeword is shorter than nCheck
 */
class InvalidInputException(message: String) :
    Exception("reed-solomon: invalid input — $message")

// =============================================================================
// Generator Polynomial
// =============================================================================

/**
 * Build the RS generator polynomial for [nCheck] check bytes.
 *
 * The generator is the product of `nCheck` linear factors:
 *
 *   g(x) = (x + α^1)(x + α^2)…(x + α^{nCheck})
 *
 * where α = 2 is the primitive element of GF(256).
 *
 * ## Algorithm
 *
 * Start with `g = [1]` (little-endian: constant 1).
 * At each step i from 1 to nCheck, multiply by `(x + α^i)`:
 *
 *   new_g[j] = GF256.mul(α^i, g[j]) XOR g[j-1]
 *
 * The resulting polynomial has nCheck+1 coefficients, with leading coefficient 1 (monic).
 *
 * ## Example: nCheck = 2
 *
 * ```
 * Start: g = [1]
 * i=1:   g = [mul(1, alpha^1)=2, 1] = [2, 1]        → (x + 2)
 * i=2:   new_g[0] = mul(4, 2) = 8
 *        new_g[1] = mul(4, 1) XOR 2 = 4 XOR 2 = 6
 *        new_g[2] = 1
 *        g = [8, 6, 1]
 * ```
 *
 * Verify root α^1=2: g(2) = 8 XOR mul(6,2) XOR mul(1,4) = 8 XOR 12 XOR 4 = 0 ✓
 * Verify root α^2=4: g(4) = 8 XOR mul(6,4) XOR mul(1,16) = 8 XOR 24 XOR 16 = 0 ✓
 *
 * @throws InvalidInputException if nCheck is 0 or odd
 */
fun buildGenerator(nCheck: Int): IntArray {
    if (nCheck == 0 || nCheck % 2 != 0) {
        throw InvalidInputException("nCheck must be a positive even number, got $nCheck")
    }

    var g = IntArray(1) { 1 }  // start: [1]

    for (i in 1..nCheck) {
        val alphaI = GF256.pow(2, i)
        val newG = IntArray(g.size + 1)
        for (j in g.indices) {
            newG[j] = GF256.add(newG[j], GF256.mul(g[j], alphaI))
            newG[j + 1] = GF256.add(newG[j + 1], g[j])
        }
        g = newG
    }

    return g  // little-endian, length nCheck+1, monic (last coeff = 1)
}

// =============================================================================
// Internal Polynomial Helpers
// =============================================================================

/**
 * Evaluate a **big-endian** GF(256) polynomial at [x] using Horner's method.
 *
 * `p[0]` is the coefficient of the highest-degree term.
 * Horner left-to-right: `acc = acc·x XOR b` for each byte b in p.
 *
 * Used for syndrome computation: `S_j = polyEvalBE(codeword, α^j)`.
 */
private fun polyEvalBE(p: IntArray, x: Int): Int {
    var acc = 0
    for (b in p) {
        acc = GF256.add(GF256.mul(acc, x), b)
    }
    return acc
}

/**
 * Evaluate a **little-endian** GF(256) polynomial at [x] using Horner's method.
 *
 * `p[i]` is the coefficient of `x^i`. Iterate from high degree downto 0.
 *
 * Used for Chien search (evaluating the error-locator polynomial) and Forney
 * (evaluating the error-evaluator polynomial).
 */
private fun polyEvalLE(p: IntArray, x: Int): Int {
    var acc = 0
    for (i in p.size - 1 downTo 0) {
        acc = GF256.add(GF256.mul(acc, x), p[i])
    }
    return acc
}

/**
 * Multiply two **little-endian** GF(256) polynomials.
 *
 * `result[i+j] ^= a[i] · b[j]`
 */
private fun polyMulLE(a: IntArray, b: IntArray): IntArray {
    if (a.isEmpty() || b.isEmpty()) return IntArray(0)
    val result = IntArray(a.size + b.size - 1)
    for (i in a.indices) {
        for (j in b.indices) {
            result[i + j] = GF256.add(result[i + j], GF256.mul(a[i], b[j]))
        }
    }
    return result
}

/**
 * Compute the remainder of **big-endian** polynomial division in GF(256).
 *
 * Both [dividend] and [divisor] are big-endian (first byte = highest degree).
 * The divisor must be **monic** (leading coefficient = 1).
 *
 * ## Algorithm
 *
 * At each step, eliminate the current leading term by XOR-ing a scaled copy
 * of the divisor:
 *
 * ```
 * for i in 0..(dividend.length - divisor.length):
 *   coeff = dividend[i]          (since divisor is monic: coeff = dividend[i] / 1)
 *   for j in 0..divisor.length:
 *     dividend[i+j] ^= mul(coeff, divisor[j])
 * ```
 *
 * The last `(divisor.length - 1)` bytes are the remainder.
 *
 * Used in RS encoding to compute `M(x)·x^{nCheck} mod g(x)`.
 */
private fun polyModBE(dividend: IntArray, divisor: IntArray): IntArray {
    val rem = dividend.copyOf()
    val divLen = divisor.size

    if (rem.size < divLen) return rem

    val steps = rem.size - divLen + 1
    for (i in 0 until steps) {
        val coeff = rem[i]
        if (coeff == 0) continue
        for (j in 0 until divLen) {
            rem[i + j] = GF256.add(rem[i + j], GF256.mul(coeff, divisor[j]))
        }
    }

    return rem.copyOfRange(rem.size - (divLen - 1), rem.size)
}

/**
 * Compute the inverse locator `X_p⁻¹` for byte position [p] in a codeword
 * of length [n].
 *
 * In big-endian convention, position `p` has degree `n-1-p`.
 * The locator is `X_p = α^{n-1-p}`, so `X_p⁻¹ = α^{(p + 256 - n) mod 255}`.
 *
 * Special cases:
 * - `p = n-1` (last byte): `X_p⁻¹ = α^{255 mod 255} = α^0 = 1`
 * - `p = 0` (first byte): `X_p⁻¹ = α^{(256 - n) mod 255}`
 *
 * Internally used by Chien search and Forney algorithm.
 */
private fun invLocator(p: Int, n: Int): Int {
    val exp = (p + 256 - n) % 255
    return GF256.pow(2, exp)
}

// =============================================================================
// Encoding
// =============================================================================

/**
 * Encode a message with Reed-Solomon, producing a systematic codeword.
 *
 * **Systematic** means the original message bytes are unchanged in the output:
 *
 * ```
 * output = [ message bytes | check bytes ]
 *            message[0..k-1]  check[0..nCheck-1]
 * ```
 *
 * ## Algorithm
 *
 * 1. Build generator `g` (little-endian), then reverse to big-endian `gBE`.
 * 2. Append `nCheck` zero bytes: `shifted = message || 000…0`
 *    (this represents `M(x)·x^{nCheck}` in big-endian).
 * 3. Compute remainder `R = shifted mod gBE`.
 * 4. Output `message || R` (padded to exactly `nCheck` bytes).
 *
 * ## Why it works
 *
 * `C(x) = M(x)·x^{nCheck} + R(x) = Q(x)·g(x)` (by the division algorithm).
 * Therefore `C(α^i) = Q(α^i)·g(α^i) = 0` for i = 1…nCheck,
 * which is the defining property of a valid RS codeword.
 *
 * @param message  raw data bytes as IntArray (values 0..255)
 * @param nCheck   number of check bytes to add (must be even ≥ 2)
 * @return systematic codeword of length `message.size + nCheck`
 * @throws InvalidInputException if nCheck is invalid or total length > 255
 */
fun encode(message: IntArray, nCheck: Int): IntArray {
    if (nCheck == 0 || nCheck % 2 != 0) {
        throw InvalidInputException("nCheck must be a positive even number, got $nCheck")
    }
    val n = message.size + nCheck
    if (n > 255) {
        throw InvalidInputException(
            "total codeword length $n exceeds GF(256) block size limit of 255"
        )
    }

    val gLE = buildGenerator(nCheck)
    // Reverse to big-endian for division: gLE[last]=1 becomes gBE[0]=1 (monic head).
    val gBE = gLE.reversedArray()

    // shifted = message || zeros  (big-endian representation of M(x)·x^{nCheck})
    val shifted = IntArray(n)
    for (i in message.indices) shifted[i] = message[i]
    // trailing nCheck slots remain 0

    val remainder = polyModBE(shifted, gBE)

    // Codeword = message || check bytes (padded to exactly nCheck bytes if remainder is shorter)
    val codeword = IntArray(n)
    for (i in message.indices) codeword[i] = message[i]
    val pad = nCheck - remainder.size
    for (i in remainder.indices) codeword[message.size + pad + i] = remainder[i]

    return codeword
}

// =============================================================================
// Decoding
// =============================================================================

/**
 * Compute the [nCheck] syndrome values of a received codeword.
 *
 * `S_j = received(α^j)` for `j = 1, …, nCheck`.
 *
 * If all syndromes are zero, the codeword has no errors. Any non-zero syndrome
 * reveals corruption.
 *
 * The codeword is evaluated as a **big-endian** polynomial. An error at position
 * `p` contributes `e · (α^j)^{n-1-p} = e · X_p^j` where `X_p = α^{n-1-p}`.
 *
 * @param received  received codeword bytes (possibly corrupted)
 * @param nCheck    number of check bytes in the codeword
 * @return IntArray of nCheck syndrome values
 */
fun syndromes(received: IntArray, nCheck: Int): IntArray {
    return IntArray(nCheck) { i -> polyEvalBE(received, GF256.pow(2, i + 1)) }
}

/**
 * Berlekamp-Massey algorithm: find the shortest LFSR generating the syndrome
 * sequence.
 *
 * Returns `(Λ, L)` where Λ is the **error locator polynomial** (little-endian,
 * Λ[0] = 1) and L is the number of errors.
 *
 * The LFSR connection polynomial satisfies:
 *
 *   Λ(x) = ∏_{k=1}^{v} (1 - X_k · x)
 *
 * where X_k are the error locator numbers.  The roots of Λ are X_k⁻¹, found
 * by Chien search.
 *
 * ## Algorithm
 *
 * ```
 * C = [1], B = [1], L = 0, xShift = 1, b = 1
 *
 * for n = 0 to 2t-1:
 *   d = S[n] XOR ∑_{j=1}^{L} C[j]·S[n-j]   ← discrepancy
 *
 *   if d == 0:
 *     xShift++
 *   elif 2L ≤ n:
 *     T = C.clone()
 *     C = C XOR (d/b)·x^{xShift}·B
 *     L = n+1-L;  B = T;  b = d;  xShift = 1
 *   else:
 *     C = C XOR (d/b)·x^{xShift}·B
 *     xShift++
 * ```
 */
private fun berlekampMassey(synds: IntArray): Pair<IntArray, Int> {
    val twoT = synds.size

    var c = IntArray(1) { 1 }   // connection polynomial Λ(x), LE, starts as [1]
    var b = IntArray(1) { 1 }   // previous connection polynomial
    var bigL = 0
    var xShift = 1
    var bScale = 1

    for (n in 0 until twoT) {
        // Compute discrepancy: d = S[n] XOR ∑_{j=1}^{L} C[j]·S[n-j]
        var d = synds[n]
        for (j in 1..bigL) {
            if (j < c.size && n >= j) {
                d = GF256.add(d, GF256.mul(c[j], synds[n - j]))
            }
        }

        if (d == 0) {
            xShift++
        } else if (2 * bigL <= n) {
            val tSave = c.copyOf()
            val scale = GF256.div(d, bScale)
            val shiftedLen = xShift + b.size
            if (c.size < shiftedLen) {
                val cNew = IntArray(shiftedLen)
                c.copyInto(cNew)
                c = cNew
            }
            for (k in b.indices) {
                c[xShift + k] = GF256.add(c[xShift + k], GF256.mul(scale, b[k]))
            }
            bigL = n + 1 - bigL
            b = tSave
            bScale = d
            xShift = 1
        } else {
            val scale = GF256.div(d, bScale)
            val shiftedLen = xShift + b.size
            if (c.size < shiftedLen) {
                val cNew = IntArray(shiftedLen)
                c.copyInto(cNew)
                c = cNew
            }
            for (k in b.indices) {
                c[xShift + k] = GF256.add(c[xShift + k], GF256.mul(scale, b[k]))
            }
            xShift++
        }
    }

    return Pair(c, bigL)
}

/**
 * Chien Search: find which byte positions are error locations.
 *
 * Position `p` is an error location if `Λ(X_p⁻¹) = 0`, where
 * `X_p⁻¹ = α^{(p+256-n) mod 255}` for a codeword of length `n`.
 *
 * Named after Robert Chien who described this exhaustive search in 1964.
 *
 * @return sorted list of error positions (0-indexed, big-endian)
 */
private fun chienSearch(lambda: IntArray, n: Int): List<Int> {
    val positions = mutableListOf<Int>()
    for (p in 0 until n) {
        val xiInv = invLocator(p, n)
        if (polyEvalLE(lambda, xiInv) == 0) {
            positions.add(p)
        }
    }
    return positions
}

/**
 * Forney Algorithm: compute error magnitudes from positions.
 *
 * For each error at position `p`:
 *
 *   e_p = Ω(X_p⁻¹) / Λ'(X_p⁻¹)
 *
 * where:
 * - Ω(x) = (S(x) · Λ(x)) mod x^{2t}  — error evaluator polynomial
 * - S(x) = S₁ + S₂x + … + S_{2t}x^{2t-1}  — syndrome polynomial (LE)
 * - Λ'(x) — formal derivative of Λ in GF(2^8)
 *
 * ## Formal derivative in characteristic 2
 *
 * Only odd-indexed coefficients survive (even terms vanish because 2 = 0):
 *
 *   Λ'(x) = Λ₁ + Λ₃x² + Λ₅x⁴ + …
 *
 * @throws TooManyErrorsException if the denominator evaluates to zero
 */
private fun forney(
    lambda: IntArray,
    synds: IntArray,
    positions: List<Int>,
    n: Int
): List<Int> {
    val twoT = synds.size

    // Ω = S(x) · Λ(x) mod x^{2t}: truncate to first 2t terms
    val omegaFull = polyMulLE(synds, lambda)
    val omega = omegaFull.copyOf(minOf(twoT, omegaFull.size))

    // Formal derivative Λ'(x): only odd-indexed Λ terms contribute
    // Λ'[k] = Λ[k+1] if (k+1) is odd (i.e., k is even), else 0
    val lambdaPrimeSize = maxOf(0, lambda.size - 1)
    val lambdaPrime = IntArray(lambdaPrimeSize)
    for (j in 1 until lambda.size) {
        if (j % 2 == 1) {
            lambdaPrime[j - 1] = GF256.add(lambdaPrime[j - 1], lambda[j])
        }
    }

    return positions.map { pos ->
        val xiInv = invLocator(pos, n)
        val omegaVal = polyEvalLE(omega, xiInv)
        val lpVal = polyEvalLE(lambdaPrime, xiInv)
        if (lpVal == 0) throw TooManyErrorsException()
        GF256.div(omegaVal, lpVal)
    }
}

/**
 * Decode a received Reed-Solomon codeword, correcting up to `t = nCheck/2` errors.
 *
 * ## Pipeline
 *
 * ```
 * received
 *   │
 *   ▼ Step 1: Compute syndromes S₁…S_{nCheck}
 *   │         All zero? → no errors, return message directly
 *   │
 *   ▼ Step 2: Berlekamp-Massey → Λ(x), error count L
 *   │         L > t? → TooManyErrorsException
 *   │
 *   ▼ Step 3: Chien search → error positions {p₁…pᵥ}
 *   │         |positions| ≠ L? → TooManyErrorsException
 *   │
 *   ▼ Step 4: Forney → error magnitudes {e₁…eᵥ}
 *   │
 *   ▼ Step 5: Correct: received[pₖ] XOR= eₖ
 *   │
 *   ▼ Return first k = received.size - nCheck bytes
 * ```
 *
 * @param received  received codeword bytes (possibly corrupted)
 * @param nCheck    number of check bytes (must be even ≥ 2)
 * @return recovered message bytes (length = received.size - nCheck)
 * @throws InvalidInputException   if nCheck is invalid or received is too short
 * @throws TooManyErrorsException  if more than t errors are present
 */
fun decode(received: IntArray, nCheck: Int): IntArray {
    if (nCheck == 0 || nCheck % 2 != 0) {
        throw InvalidInputException("nCheck must be a positive even number, got $nCheck")
    }
    if (received.size < nCheck) {
        throw InvalidInputException("received length ${received.size} < nCheck $nCheck")
    }

    val t = nCheck / 2
    val n = received.size
    val k = n - nCheck

    // Step 1: Syndromes
    val synds = syndromes(received, nCheck)
    if (synds.all { it == 0 }) {
        return received.copyOf(k)  // no errors
    }

    // Step 2: Berlekamp-Massey
    val (lambda, numErrors) = berlekampMassey(synds)
    if (numErrors > t) throw TooManyErrorsException()

    // Step 3: Chien Search
    val positions = chienSearch(lambda, n)
    if (positions.size != numErrors) throw TooManyErrorsException()

    // Step 4: Forney
    val magnitudes = forney(lambda, synds, positions, n)

    // Step 5: Apply corrections (XOR the error magnitude into each error position)
    val corrected = received.copyOf()
    for (i in positions.indices) {
        corrected[positions[i]] = GF256.add(corrected[positions[i]], magnitudes[i])
    }

    return corrected.copyOf(k)
}

/**
 * Compute the error locator polynomial from a syndrome array.
 *
 * Exposed for advanced use (QR decoders, diagnostics).
 * Returns Λ(x) in **little-endian** form with Λ[0] = 1.
 *
 * @param synds syndrome array (length = nCheck)
 */
fun errorLocator(synds: IntArray): IntArray {
    val (lambda, _) = berlekampMassey(synds)
    return lambda
}

/**
 * GF(2^8) — Galois Field arithmetic over 256 elements.
 *
 * GF(256) is the finite field whose elements are the integers 0..255.
 * It is the mathematical backbone of Reed-Solomon error correction (QR codes,
 * CDs, hard drives, deep-space probes) and AES encryption.
 *
 * ## Why a finite field?
 *
 * Error-correction codes need to do polynomial arithmetic over the data bytes.
 * Ordinary integer arithmetic does not close neatly: multiplying two bytes can
 * produce values larger than 255. A finite field keeps everything in range and
 * ensures every non-zero element has a multiplicative inverse — properties
 * critical for the decoding algorithms.
 *
 * ## The primitive polynomial
 *
 * Elements of GF(2^8) are polynomials over GF(2) of degree ≤ 7:
 *
 *   a₇x⁷ + a₆x⁶ + … + a₁x + a₀   where each aᵢ ∈ {0, 1}
 *
 * When multiplication would exceed degree 7 (byte overflow), we reduce modulo
 * an irreducible polynomial of degree 8.  This implementation uses:
 *
 *   p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
 *
 * This polynomial is both irreducible (cannot be factored) and primitive (the
 * element g=2 generates the entire multiplicative group of order 255).
 *
 * ## Add = XOR = Subtract
 *
 * In any characteristic-2 field, 1 + 1 = 0.  So −1 = 1, meaning subtraction
 * and addition are the same operation: bitwise XOR.  No carries, no overflow.
 *
 * ## Multiplication via logarithms
 *
 * Because g=2 generates all 255 non-zero elements, we precompute:
 *
 *   EXP[i] = g^i mod p(x)           — the antilogarithm ("exp") table
 *   LOG[x] = i such that g^i = x    — the logarithm table
 *
 * Then: a × b = EXP[(LOG[a] + LOG[b]) mod 255]
 *
 * Two table lookups and one addition replace a complex bit-level polynomial
 * multiplication.  O(1) per operation.
 *
 * Spec: MA01-gf256.md
 */
package com.codingadventures.gf256

/** Version of this package. */
const val VERSION = "0.1.0"

/**
 * GF256 contains all GF(2^8) arithmetic operations over the Reed-Solomon
 * primitive polynomial 0x11D.
 *
 * All values passed to and returned from these functions are bytes in 0..255.
 * Kotlin uses Int for clarity and to avoid signed-byte confusion.
 *
 * Design: a Kotlin object (singleton) holding precomputed tables and functions,
 * matching the typical module-level function style of the TypeScript reference
 * implementation but using Kotlin idioms.
 */
object GF256 {

    /**
     * The primitive (irreducible, primitive) polynomial used for reduction.
     *
     *   p(x) = x^8 + x^4 + x^3 + x^2 + 1
     *
     * Binary representation:
     *   bit 8 → x^8 = 256
     *   bit 4 → x^4 = 16
     *   bit 3 → x^3 = 8
     *   bit 2 → x^2 = 4
     *   bit 0 → 1
     *
     *   256 + 16 + 8 + 4 + 1 = 285 = 0x11D
     *
     * Why primitive?  Because g=2 (the polynomial "x") satisfies g^255 = 1
     * and all of g^0, g^1, …, g^254 are distinct — i.e., g generates the
     * complete multiplicative group.  This is what makes logarithm tables
     * possible.
     */
    const val PRIMITIVE_POLY: Int = 0x11D

    // =========================================================================
    // Log / Antilog (Exp) Tables
    // =========================================================================
    //
    // We precompute two 256-element arrays at object initialisation.
    //
    // EXP[i] = 2^i mod p(x)  for i in 0..510
    //   (doubled to 512 so we can use LOG[a] + LOG[b] without modular wrap
    //    in the hot-path multiply — some implementations do this for speed.
    //    We keep the standard 512-entry version from the TypeScript source.)
    //
    // LOG[x] = i such that 2^i = x  for x in 1..255
    // LOG[0] = -1  (0 has no logarithm; there is no power of 2 that equals 0)
    //
    // Construction:
    //   Start with val = 1.
    //   Each iteration: val = val * 2 in GF(256)
    //     = (val shl 1) XOR 0x11D   when bit 8 is set after the shift
    //     = (val shl 1)              otherwise
    //   Record EXP[i] = val, LOG[val] = i.
    //   Duplicate EXP[0..254] into EXP[255..509] to allow LOG[a]+LOG[b] without mod.

    /** Antilogarithm (exp) table of size 512 for multiplication without modular wrap. */
    val EXP: IntArray = IntArray(512)

    /** Logarithm table. LOG[0] = -1 (undefined). LOG[x] for x in 1..255 = i where 2^i = x. */
    val LOG: IntArray = IntArray(256)

    init {
        // Fill EXP[0..254] and LOG[1..255].
        var x = 1
        for (i in 0 until 255) {
            EXP[i] = x
            LOG[x] = i
            // Multiply x by 2 (left-shift by 1).
            x = x shl 1
            // If bit 8 is set, reduce modulo the primitive polynomial.
            if (x and 0x100 != 0) {
                x = x xor PRIMITIVE_POLY
            }
        }
        // EXP[255] through EXP[511]: duplicate the first 255 entries so that
        //   LOG[a] + LOG[b] can range up to 508 without overflow in mul().
        for (i in 255 until 512) {
            EXP[i] = EXP[i - 255]
        }
        // LOG[0] is explicitly -1 (no logarithm exists for 0).
        LOG[0] = -1
    }

    // =========================================================================
    // Fundamental Operations
    // =========================================================================

    /**
     * Add two GF(256) elements.
     *
     * In characteristic-2 arithmetic, addition is bitwise XOR.  Each bit
     * represents a coefficient in GF(2), and 1 + 1 = 0 mod 2.
     *
     * Truth table for a single bit:
     *   0 + 0 = 0
     *   0 + 1 = 1
     *   1 + 0 = 1
     *   1 + 1 = 0  ← this is the "characteristic 2" property
     *
     * Examples:
     *   add(0x53, 0xCA) = 0x53 XOR 0xCA = 0x99
     *   add(x, x)       = 0 for all x  (every element is its own inverse)
     */
    fun add(a: Int, b: Int): Int = a xor b

    /**
     * Subtract two GF(256) elements.
     *
     * In characteristic-2 fields, −1 = 1, so subtraction equals addition: XOR.
     * This simplifies error-correction algorithms: a "syndrome" computed via
     * subtraction uses identical hardware/logic to addition.
     */
    fun sub(a: Int, b: Int): Int = a xor b

    /**
     * Multiply two GF(256) elements using logarithm/antilogarithm tables.
     *
     * Identity: a × b = g^(LOG[a] + LOG[b])  = EXP[LOG[a] + LOG[b]]
     *
     * The extended EXP table (size 512) avoids the modulo-255 wrap in the hot
     * path: LOG[a] + LOG[b] can be at most 254 + 254 = 508, which is within
     * bounds.  The duplication ensures EXP[k] = EXP[k mod 255] for k ≥ 255.
     *
     * Special case: 0 × anything = 0.  (Zero has no logarithm, so this must
     * be handled explicitly.)
     *
     * Time complexity: O(1) — two table lookups and one addition.
     */
    fun mul(a: Int, b: Int): Int {
        if (a == 0 || b == 0) return 0
        return EXP[LOG[a] + LOG[b]]
    }

    /**
     * Divide a by b in GF(256).
     *
     * a / b = g^(LOG[a] − LOG[b])
     *
     * The +255 before the mod ensures a non-negative result when LOG[a] < LOG[b].
     * (Kotlin's % operator can return negative values for negative operands.)
     *
     * Special case: 0 / b = 0.
     *
     * @throws ArithmeticException if b = 0 (division by zero is undefined)
     */
    fun div(a: Int, b: Int): Int {
        if (b == 0) throw ArithmeticException("GF256: division by zero")
        if (a == 0) return 0
        return EXP[(LOG[a] - LOG[b] + 255) % 255]
    }

    /**
     * Raise a GF(256) element to a non-negative integer power.
     *
     * base^exp = EXP[(LOG[base] * exp) mod 255]
     *
     * The modulo 255 reflects the order of the multiplicative group: by
     * Fermat's little theorem for finite fields, every non-zero element
     * satisfies a^255 = 1.
     *
     * Special cases:
     *   0^0 = 1 by convention (consistent with the spec and most libraries)
     *   0^n = 0 for n > 0
     *
     * @throws IllegalArgumentException if exp is negative
     */
    fun pow(base: Int, n: Int): Int {
        require(n >= 0) { "GF256: exponent must be non-negative, got $n" }
        if (n == 0) return 1
        if (base == 0) return 0
        return EXP[(LOG[base].toLong() * n % 255).toInt().let { if (it < 0) it + 255 else it }]
    }

    /**
     * Compute the multiplicative inverse of a GF(256) element.
     *
     * The inverse of a satisfies: a × inv(a) = 1.
     *
     * Derivation:
     *   a × a^(−1) = 1 = g^0 = g^255
     *   LOG[a] + LOG[a^(−1)] ≡ 0 (mod 255)
     *   LOG[a^(−1)] = 255 − LOG[a]
     *   a^(−1) = EXP[255 − LOG[a]]
     *
     * This is used in Reed-Solomon decoding (Forney algorithm) and AES SubBytes.
     *
     * @throws ArithmeticException if a = 0 (zero has no multiplicative inverse)
     */
    fun inv(a: Int): Int {
        if (a == 0) throw ArithmeticException("GF256: zero has no multiplicative inverse")
        return EXP[255 - LOG[a]]
    }
}

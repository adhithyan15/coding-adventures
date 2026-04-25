// ============================================================================
// Rng.kt — Three Classic Pseudorandom Number Generators
// ============================================================================
//
// This package implements three progressively more sophisticated PRNGs:
//
//   ┌─────────────┬──────────────┬──────────────┬────────────────────────────┐
//   │ Algorithm   │ State (bits) │ Period       │ Notes                      │
//   ├─────────────┼──────────────┼──────────────┼────────────────────────────┤
//   │ Lcg         │ 64           │ 2^64         │ Simplest; correlated bits  │
//   │ Xorshift64  │ 64           │ 2^64 − 1     │ No multiply; seed ≠ 0      │
//   │ Pcg32       │ 64           │ 2^64         │ LCG + output permutation   │
//   └─────────────┴──────────────┴──────────────┴────────────────────────────┘
//
// All three expose the same API:
//   - nextU32()           — uniform UInt32 value returned as Long in [0, 2^32)
//   - nextU64()           — uniform 64-bit value as Long
//   - nextFloat()         — uniform Double in [0.0, 1.0)
//   - nextIntInRange(a,b) — uniform Long in [a, b] inclusive
//
// Kotlin note: Kotlin's Long is signed 64-bit but two's-complement overflow
// wraps exactly as needed for mod-2^64 arithmetic.  No masking required for
// multiply/add.  We use ushr (unsigned shift right) wherever bits must be
// treated as unsigned.
//
// Spec: code/specs/rng.md
// ============================================================================

package com.codingadventures.rng

// =========================================================================
// Shared constants (Knuth / Numerical Recipes)
// =========================================================================

/** LCG multiplier: 6364136223846793005. Satisfies Hull-Dobell with INCREMENT. */
private const val LCG_MULTIPLIER: Long = 0x5851F42D4C957F2DL.toLong()  // 6364136223846793005

/**
 * LCG increment: 1442695040888963407 (odd, ensuring full 2^64 period).
 *
 * Also used as PCG32's stream increment.
 */
private const val LCG_INCREMENT: Long = 0x14057B7EF767814FL.toLong()   // 1442695040888963407

/** 2^32 as a Double — normalises a uint32 to [0.0, 1.0). */
private const val FLOAT_DIV: Double = 4294967296.0  // 1L shl 32

// Helper: treat a Long as unsigned and mask to 32 bits.
// Returns a Long whose value is in [0, 2^32).
private fun Long.toU32(): Long = this and 0xFFFFFFFFL

// =========================================================================
// Lcg — Linear Congruential Generator (Knuth 1948)
// =========================================================================

/**
 * LCG (Linear Congruential Generator, Knuth 1948).
 *
 * Recurrence: `state = (state × a + c) mod 2^64`
 * Output: upper 32 bits of state (lower bits have shorter sub-periods).
 *
 * LCG is the simplest useful PRNG.  It is fast and has a full period of
 * 2^64, but consecutive outputs are linearly correlated — do not use it
 * for cryptography or simulations requiring statistical independence.
 *
 * ```
 *  state_n+1 = state_n × 6364136223846793005 + 1442695040888963407
 *  output    = state_n+1 ushr 32
 * ```
 *
 * Example (seed = 1):
 * ```
 *  call 1 → 1817669548
 *  call 2 → 2187888307
 *  call 3 → 2784682393
 * ```
 *
 * @param seed initial state (any 64-bit value)
 */
class Lcg(seed: Long) {
    private var state: Long = seed

    /**
     * Advance the state and return the upper 32 bits as an unsigned value
     * in [0, 2^32), represented as a non-negative [Long].
     */
    fun nextU32(): Long {
        state = state * LCG_MULTIPLIER + LCG_INCREMENT
        return state ushr 32
    }

    /**
     * Return a 64-bit value composed of two consecutive [nextU32] calls:
     * `(hi shl 32) or lo`.
     */
    fun nextU64(): Long {
        val hi = nextU32()
        val lo = nextU32()
        return (hi shl 32) or lo
    }

    /**
     * Return a [Double] uniformly distributed in [0.0, 1.0).
     *
     * Computed as `nextU32() / 2^32`.
     */
    fun nextFloat(): Double = nextU32().toDouble() / FLOAT_DIV

    /**
     * Return a uniform random [Long] in [[min], [max]] inclusive.
     *
     * Uses rejection sampling to eliminate modulo bias.  Naïve
     * `value % range` over-samples low values when 2^32 is not divisible
     * by range.
     *
     * ```
     *  threshold = (-range) mod range   (unsigned 32-bit arithmetic)
     *  discard draws below threshold; expected extra draws < 2.
     * ```
     *
     * @param min lower bound (inclusive)
     * @param max upper bound (inclusive), must be ≥ min
     */
    fun nextIntInRange(min: Long, max: Long): Long {
        require(min <= max) { "nextIntInRange requires min <= max, got $min > $max" }
        val rangeSize = max - min + 1L
        val threshold = java.lang.Long.remainderUnsigned(-rangeSize, rangeSize)
        while (true) {
            val r = nextU32()
            if (java.lang.Long.compareUnsigned(r, threshold) >= 0) {
                return min + java.lang.Long.remainderUnsigned(r, rangeSize)
            }
        }
    }
}

// =========================================================================
// Xorshift64 — Marsaglia (2003)
// =========================================================================

/**
 * Xorshift64 (Marsaglia 2003).
 *
 * Three XOR-shift operations scramble a 64-bit state with no multiplication:
 * ```
 *   x = x xor (x shl 13)
 *   x = x xor (x ushr 7)
 *   x = x xor (x shl 17)
 * ```
 *
 * Period: 2^64 − 1.  State 0 is a fixed point (all shifts yield 0), so
 * seed 0 is silently replaced with 1.  Output is the lower 32 bits.
 *
 * Xorshift is faster than PCG32 on hardware without fast multipliers and
 * passes many statistical tests, but fails some modern test suites.
 *
 * Example (seed = 1):
 * ```
 *  call 1 → 1082269761
 *  call 2 →  201397313
 *  call 3 → 1854285353
 * ```
 *
 * @param seed initial state (0 is replaced with 1)
 */
class Xorshift64(seed: Long) {
    private var state: Long = if (seed == 0L) 1L else seed

    /**
     * Apply the three XOR-shifts and return the lower 32 bits as an unsigned
     * value in [0, 2^32), represented as a non-negative [Long].
     */
    fun nextU32(): Long {
        var x = state
        x = x xor (x shl 13)
        x = x xor (x ushr 7)
        x = x xor (x shl 17)
        state = x
        return x.toU32()
    }

    /**
     * Return a 64-bit value composed of two consecutive [nextU32] calls:
     * `(hi shl 32) or lo`.
     */
    fun nextU64(): Long {
        val hi = nextU32()
        val lo = nextU32()
        return (hi shl 32) or lo
    }

    /** Return a [Double] uniformly distributed in [0.0, 1.0). */
    fun nextFloat(): Double = nextU32().toDouble() / FLOAT_DIV

    /**
     * Return a uniform random [Long] in [[min], [max]] inclusive.
     *
     * Same rejection-sampling algorithm as [Lcg.nextIntInRange].
     *
     * @param min lower bound (inclusive)
     * @param max upper bound (inclusive), must be ≥ min
     */
    fun nextIntInRange(min: Long, max: Long): Long {
        require(min <= max) { "nextIntInRange requires min <= max, got $min > $max" }
        val rangeSize = max - min + 1L
        val threshold = java.lang.Long.remainderUnsigned(-rangeSize, rangeSize)
        while (true) {
            val r = nextU32()
            if (java.lang.Long.compareUnsigned(r, threshold) >= 0) {
                return min + java.lang.Long.remainderUnsigned(r, rangeSize)
            }
        }
    }
}

// =========================================================================
// Pcg32 — Permuted Congruential Generator (O'Neill 2014)
// =========================================================================

/**
 * PCG32 (Permuted Congruential Generator, O'Neill 2014).
 *
 * Uses the same LCG recurrence as [Lcg] but applies an XSH RR
 * (XOR-Shift High / Random Rotate) output permutation before returning:
 *
 * ```
 *  1. xorshifted = (((oldState ushr 18) xor oldState) ushr 27).toInt()
 *  2. rot        = (oldState ushr 59).toInt()
 *  3. output     = xorshifted.rotateRight(rot)
 * ```
 *
 * The permutation mixes information from the upper bits (which have long
 * sub-periods in a plain LCG) into the output, producing excellent
 * statistical quality.  PCG32 passes all known test suites.
 *
 * **Seeding (initseq warm-up):**
 * ```
 *  state = 0
 *  advance once (incorporates increment)
 *  state += seed
 *  advance once (scatters seed bits)
 * ```
 *
 * Example (seed = 1):
 * ```
 *  call 1 → 1412771199
 *  call 2 → 1791099446
 *  call 3 →  124312908
 * ```
 *
 * @param seed initial seed (any 64-bit value)
 */
class Pcg32(seed: Long) {
    private var state: Long
    private val increment: Long = LCG_INCREMENT or 1L  // must be odd

    init {
        state = 0L
        // Advance once to incorporate the increment
        state = state * LCG_MULTIPLIER + increment
        // Mix in the seed
        state += seed
        // Advance once more to scatter seed bits throughout state
        state = state * LCG_MULTIPLIER + increment
    }

    /**
     * Advance the PCG32 state and return the XSH RR permuted output as an
     * unsigned value in [0, 2^32), represented as a non-negative [Long].
     *
     * The output-before-advance model captures the old state, advances, then
     * permutes the captured value so every bit participates in the output.
     */
    fun nextU32(): Long {
        val oldState = state
        // Advance LCG
        state = oldState * LCG_MULTIPLIER + increment
        // XSH RR permutation on old state
        val xorshifted: Int = (((oldState ushr 18) xor oldState) ushr 27).toInt()
        val rot: Int = (oldState ushr 59).toInt()
        return xorshifted.rotateRight(rot).toLong() and 0xFFFFFFFFL
    }

    /**
     * Return a 64-bit value composed of two consecutive [nextU32] calls:
     * `(hi shl 32) or lo`.
     */
    fun nextU64(): Long {
        val hi = nextU32()
        val lo = nextU32()
        return (hi shl 32) or lo
    }

    /** Return a [Double] uniformly distributed in [0.0, 1.0). */
    fun nextFloat(): Double = nextU32().toDouble() / FLOAT_DIV

    /**
     * Return a uniform random [Long] in [[min], [max]] inclusive.
     *
     * Same rejection-sampling algorithm as [Lcg.nextIntInRange].
     *
     * @param min lower bound (inclusive)
     * @param max upper bound (inclusive), must be ≥ min
     */
    fun nextIntInRange(min: Long, max: Long): Long {
        require(min <= max) { "nextIntInRange requires min <= max, got $min > $max" }
        val rangeSize = max - min + 1L
        val threshold = java.lang.Long.remainderUnsigned(-rangeSize, rangeSize)
        while (true) {
            val r = nextU32()
            if (java.lang.Long.compareUnsigned(r, threshold) >= 0) {
                return min + java.lang.Long.remainderUnsigned(r, rangeSize)
            }
        }
    }
}

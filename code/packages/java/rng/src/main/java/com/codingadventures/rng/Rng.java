// ============================================================================
// Rng.java — Three Classic Pseudorandom Number Generators
// ============================================================================
//
// This package implements three progressively more sophisticated PRNGs:
//
//   ┌─────────────┬──────────────┬──────────────┬────────────────────────────┐
//   │ Algorithm   │ State (bits) │ Period       │ Notes                      │
//   ├─────────────┼──────────────┼──────────────┼────────────────────────────┤
//   │ LCG         │ 64           │ 2^64         │ Simplest; correlated bits  │
//   │ Xorshift64  │ 64           │ 2^64 − 1     │ No multiply; seed≠0        │
//   │ PCG32       │ 64           │ 2^64         │ LCG + output permutation   │
//   └─────────────┴──────────────┴──────────────┴────────────────────────────┘
//
// All three implement the same API:
//   - nextU32()           — uniform uint32 in [0, 2^32)
//   - nextU64()           — uniform uint64 in [0, 2^64)
//   - nextFloat()         — uniform double in [0.0, 1.0)
//   - nextIntInRange(a,b) — uniform long in [a, b] inclusive
//
// Java note: Java's long is signed 64-bit, but two's-complement overflow
// wraps exactly as we need for mod 2^64 arithmetic.  No masking required.
// We use >>> (unsigned right shift) whenever treating bits as unsigned.
//
// Spec: code/specs/rng.md
// ============================================================================

package com.codingadventures.rng;

/**
 * Namespace class — instantiate {@link Lcg}, {@link Xorshift64}, or
 * {@link Pcg32} directly.
 */
public final class Rng {
    private Rng() {}

    // =========================================================================
    // Shared constants (Knuth / Numerical Recipes)
    // =========================================================================

    /**
     * LCG multiplier: 6364136223846793005.
     *
     * <p>Together with {@link #LCG_INCREMENT} this satisfies the Hull-Dobell
     * theorem and gives a full period of 2^64.</p>
     */
    static final long LCG_MULTIPLIER = 0x5851F42D4C957F2DL; // 6364136223846793005

    /**
     * LCG increment: 1442695040888963407 (must be odd for full period).
     *
     * <p>This is also used as PCG32's stream identifier.</p>
     */
    static final long LCG_INCREMENT  = 0x14057B7EF767814FL; // 1442695040888963407

    /** 2^32 as a double — normalises a uint32 to [0.0, 1.0). */
    static final double FLOAT_DIV = 4294967296.0; // 1L << 32

    // =========================================================================
    // LCG — Linear Congruential Generator
    // =========================================================================

    /**
     * LCG (Linear Congruential Generator, Knuth 1948).
     *
     * <p>Recurrence: {@code state = (state × a + c) mod 2^64}<br>
     * Output: upper 32 bits of state (lower bits have shorter sub-periods).
     *
     * <p>LCG is the simplest useful PRNG.  It is fast and has a full period,
     * but consecutive outputs are linearly correlated — don't use it for
     * cryptography or statistical simulations that need independence.
     *
     * <pre>
     *  state_n+1 = state_n × 6364136223846793005 + 1442695040888963407
     *  output    = state_n+1 >>> 32
     * </pre>
     *
     * Example (seed = 1):
     * <pre>
     *  call 1 → 1817669548
     *  call 2 → 2187888307
     *  call 3 → 2784682393
     * </pre>
     */
    public static final class Lcg {
        private long state;

        private Lcg(long seed) {
            this.state = seed;
        }

        /**
         * Create an LCG seeded with the given value.  Any seed is valid.
         *
         * @param seed initial state (any 64-bit value)
         * @return a new {@link Lcg} instance
         */
        public static Lcg of(long seed) {
            return new Lcg(seed);
        }

        /**
         * Advance the state and return the upper 32 bits as an unsigned int.
         *
         * <p>The return type is {@code long} to represent the unsigned range
         * [0, 2^32).  If you need an {@code int} for bit-twiddling, cast with
         * {@code (int) nextU32()} — the bit pattern is identical.
         */
        public long nextU32() {
            state = state * LCG_MULTIPLIER + LCG_INCREMENT;
            return (state >>> 32) & 0xFFFFFFFFL;
        }

        /**
         * Return a 64-bit value composed of two consecutive {@link #nextU32}
         * calls: {@code (hi << 32) | lo}.
         */
        public long nextU64() {
            long hi = nextU32();
            long lo = nextU32();
            return (hi << 32) | lo;
        }

        /**
         * Return a double uniformly distributed in [0.0, 1.0).
         *
         * <p>Computed as {@code nextU32() / 2^32}.
         */
        public double nextFloat() {
            return (double) nextU32() / FLOAT_DIV;
        }

        /**
         * Return a uniform random long in [{@code min}, {@code max}] inclusive.
         *
         * <p>Uses rejection sampling to eliminate modulo bias.  Naïve
         * {@code value % range} over-samples low values when 2^32 is not
         * divisible by range.
         *
         * <pre>
         *   threshold = (-range) mod range  (in unsigned 32-bit arithmetic)
         *   discard draws below threshold; expected extra draws &lt; 2.
         * </pre>
         *
         * @param min lower bound (inclusive)
         * @param max upper bound (inclusive), must be &ge; min
         */
        public long nextIntInRange(long min, long max) {
            if (min > max) {
                throw new IllegalArgumentException("nextIntInRange requires min <= max, got " + min + " > " + max);
            }
            long rangeSize = max - min + 1;
            long threshold = Long.remainderUnsigned(-rangeSize, rangeSize);
            while (true) {
                long r = nextU32();
                if (Long.compareUnsigned(r, threshold) >= 0) {
                    return min + Long.remainderUnsigned(r, rangeSize);
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
     * <p>Three XOR-shift operations scramble a 64-bit state with no
     * multiplication:
     * <pre>
     *   x ^= x &lt;&lt; 13
     *   x ^= x &gt;&gt;&gt; 7
     *   x ^= x &lt;&lt; 17
     * </pre>
     *
     * <p>Period: 2^64 − 1.  State 0 is a fixed point (all shifts produce 0)
     * so seed 0 is replaced with 1.  Output is the lower 32 bits.
     *
     * <p>Xorshift is faster than PCG32 on hardware without fast multipliers,
     * and passes many statistical tests, but fails some modern suites.
     *
     * Example (seed = 1):
     * <pre>
     *  call 1 → 1082269761
     *  call 2 →  201397313
     *  call 3 → 1854285353
     * </pre>
     */
    public static final class Xorshift64 {
        private long state;

        private Xorshift64(long seed) {
            this.state = (seed == 0) ? 1L : seed;
        }

        /**
         * Create an Xorshift64 seeded with the given value.
         *
         * <p>Seed 0 is replaced with 1 to avoid the zero fixed point.
         *
         * @param seed initial state (any 64-bit value; 0 becomes 1)
         * @return a new {@link Xorshift64} instance
         */
        public static Xorshift64 of(long seed) {
            return new Xorshift64(seed);
        }

        /**
         * Apply the three XOR-shifts and return the lower 32 bits as an
         * unsigned int in [0, 2^32).
         *
         * <p>The bit pattern of the returned {@code long}'s low 32 bits
         * equals the generator's raw output.
         */
        public long nextU32() {
            long x = state;
            x ^= x << 13;
            x ^= x >>> 7;
            x ^= x << 17;
            state = x;
            return x & 0xFFFFFFFFL;
        }

        /**
         * Return a 64-bit value composed of two consecutive {@link #nextU32}
         * calls: {@code (hi << 32) | lo}.
         */
        public long nextU64() {
            long hi = nextU32();
            long lo = nextU32();
            return (hi << 32) | lo;
        }

        /**
         * Return a double uniformly distributed in [0.0, 1.0).
         */
        public double nextFloat() {
            return (double) nextU32() / FLOAT_DIV;
        }

        /**
         * Return a uniform random long in [{@code min}, {@code max}] inclusive.
         *
         * <p>Same rejection-sampling algorithm as {@link Lcg#nextIntInRange}.
         *
         * @param min lower bound (inclusive)
         * @param max upper bound (inclusive), must be &ge; min
         */
        public long nextIntInRange(long min, long max) {
            if (min > max) {
                throw new IllegalArgumentException("nextIntInRange requires min <= max, got " + min + " > " + max);
            }
            long rangeSize = max - min + 1;
            long threshold = Long.remainderUnsigned(-rangeSize, rangeSize);
            while (true) {
                long r = nextU32();
                if (Long.compareUnsigned(r, threshold) >= 0) {
                    return min + Long.remainderUnsigned(r, rangeSize);
                }
            }
        }
    }

    // =========================================================================
    // PCG32 — Permuted Congruential Generator (O'Neill 2014)
    // =========================================================================

    /**
     * PCG32 (Permuted Congruential Generator, O'Neill 2014).
     *
     * <p>Uses the same LCG recurrence as {@link Lcg} but applies an
     * XSH RR (XOR-Shift High / Random Rotate) output permutation before
     * returning:
     *
     * <pre>
     *  1. xorshifted = (int)(((oldState &gt;&gt;&gt; 18) ^ oldState) &gt;&gt;&gt; 27)
     *  2. rot        = (int)(oldState &gt;&gt;&gt; 59)
     *  3. output     = Integer.rotateRight(xorshifted, rot)
     * </pre>
     *
     * <p>The permutation mixes information from the upper bits (which have
     * long sub-periods in a plain LCG) into the output, producing excellent
     * statistical quality.  PCG32 passes all known test suites.
     *
     * <p><b>Seeding (initseq warm-up):</b>
     * <pre>
     *   state = 0
     *   advance once (incorporates increment)
     *   state += seed
     *   advance once (scatters seed bits)
     * </pre>
     *
     * Example (seed = 1):
     * <pre>
     *  call 1 → 1412771199
     *  call 2 → 1791099446
     *  call 3 →  124312908
     * </pre>
     */
    public static final class Pcg32 {
        private long state;
        private final long increment;

        private Pcg32(long seed) {
            // increment must be odd for full period
            this.increment = LCG_INCREMENT | 1L;
            this.state = 0L;
            // Advance once to incorporate the increment
            this.state = this.state * LCG_MULTIPLIER + this.increment;
            // Mix in the seed
            this.state += seed;
            // Advance once more to scatter seed bits
            this.state = this.state * LCG_MULTIPLIER + this.increment;
        }

        /**
         * Create a PCG32 seeded with the given value.
         *
         * <p>The "initseq" warm-up is applied so that seeds 0 and 1 produce
         * well-distributed initial sequences.
         *
         * @param seed initial seed (any 64-bit value)
         * @return a new {@link Pcg32} instance
         */
        public static Pcg32 of(long seed) {
            return new Pcg32(seed);
        }

        /**
         * Advance the PCG32 state and return the XSH RR permuted output.
         *
         * <p>The output-before-advance model captures the old state,
         * advances, then permutes the captured value.  This way every bit
         * of state participates in the output before it is overwritten.
         */
        public long nextU32() {
            long oldState = state;
            // Advance LCG
            state = oldState * LCG_MULTIPLIER + increment;
            // XSH RR permutation on old state
            int xorshifted = (int)(((oldState >>> 18) ^ oldState) >>> 27);
            int rot = (int)(oldState >>> 59);
            return Integer.toUnsignedLong(Integer.rotateRight(xorshifted, rot));
        }

        /**
         * Return a 64-bit value composed of two consecutive {@link #nextU32}
         * calls: {@code (hi << 32) | lo}.
         */
        public long nextU64() {
            long hi = nextU32();
            long lo = nextU32();
            return (hi << 32) | lo;
        }

        /**
         * Return a double uniformly distributed in [0.0, 1.0).
         */
        public double nextFloat() {
            return (double) nextU32() / FLOAT_DIV;
        }

        /**
         * Return a uniform random long in [{@code min}, {@code max}] inclusive.
         *
         * <p>Same rejection-sampling algorithm as {@link Lcg#nextIntInRange}.
         *
         * @param min lower bound (inclusive)
         * @param max upper bound (inclusive), must be &ge; min
         */
        public long nextIntInRange(long min, long max) {
            if (min > max) {
                throw new IllegalArgumentException("nextIntInRange requires min <= max, got " + min + " > " + max);
            }
            long rangeSize = max - min + 1;
            long threshold = Long.remainderUnsigned(-rangeSize, rangeSize);
            while (true) {
                long r = nextU32();
                if (Long.compareUnsigned(r, threshold) >= 0) {
                    return min + Long.remainderUnsigned(r, rangeSize);
                }
            }
        }
    }
}

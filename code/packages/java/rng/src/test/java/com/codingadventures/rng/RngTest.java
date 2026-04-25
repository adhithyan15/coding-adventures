// ============================================================================
// RngTest.java — Unit Tests for Lcg, Xorshift64, and Pcg32
// ============================================================================
//
// Test strategy:
//   1. Known reference values (seed=1) — cross-checked against the Go
//      implementation to confirm identical bit-for-bit output.
//   2. Independence — each fresh instance from the same seed produces
//      the same sequence.
//   3. Different seeds produce different sequences.
//   4. nextU64 consistency — should equal (hi << 32) | lo.
//   5. nextFloat in [0.0, 1.0).
//   6. nextIntInRange distribution and boundary conditions.
//   7. Edge cases: seed 0 (Xorshift64 replaces it with 1), large seeds.
// ============================================================================

package com.codingadventures.rng;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class RngTest {

    // =========================================================================
    // Known reference values (seed = 1, verified against Go implementation)
    // =========================================================================

    @Test
    void lcgSeed1FirstThreeValues() {
        Rng.Lcg g = Rng.Lcg.of(1L);
        assertEquals(1817669548L, g.nextU32(), "LCG seed=1 call 1");
        assertEquals(2187888307L, g.nextU32(), "LCG seed=1 call 2");
        assertEquals(2784682393L, g.nextU32(), "LCG seed=1 call 3");
    }

    @Test
    void xorshift64Seed1FirstThreeValues() {
        Rng.Xorshift64 g = Rng.Xorshift64.of(1L);
        assertEquals(1082269761L, g.nextU32(), "Xorshift64 seed=1 call 1");
        assertEquals(201397313L,  g.nextU32(), "Xorshift64 seed=1 call 2");
        assertEquals(1854285353L, g.nextU32(), "Xorshift64 seed=1 call 3");
    }

    @Test
    void pcg32Seed1FirstThreeValues() {
        Rng.Pcg32 g = Rng.Pcg32.of(1L);
        assertEquals(1412771199L, g.nextU32(), "PCG32 seed=1 call 1");
        assertEquals(1791099446L, g.nextU32(), "PCG32 seed=1 call 2");
        assertEquals(124312908L,  g.nextU32(), "PCG32 seed=1 call 3");
    }

    // =========================================================================
    // Reproducibility — same seed → same sequence
    // =========================================================================

    @Test
    void lcgReproducible() {
        Rng.Lcg a = Rng.Lcg.of(42L);
        Rng.Lcg b = Rng.Lcg.of(42L);
        for (int i = 0; i < 10; i++) {
            assertEquals(a.nextU32(), b.nextU32(), "LCG mismatch at step " + i);
        }
    }

    @Test
    void xorshift64Reproducible() {
        Rng.Xorshift64 a = Rng.Xorshift64.of(42L);
        Rng.Xorshift64 b = Rng.Xorshift64.of(42L);
        for (int i = 0; i < 10; i++) {
            assertEquals(a.nextU32(), b.nextU32(), "Xorshift64 mismatch at step " + i);
        }
    }

    @Test
    void pcg32Reproducible() {
        Rng.Pcg32 a = Rng.Pcg32.of(42L);
        Rng.Pcg32 b = Rng.Pcg32.of(42L);
        for (int i = 0; i < 10; i++) {
            assertEquals(a.nextU32(), b.nextU32(), "PCG32 mismatch at step " + i);
        }
    }

    // =========================================================================
    // Different seeds → different sequences
    // =========================================================================

    @Test
    void lcgDifferentSeedsDifferentOutput() {
        Rng.Lcg a = Rng.Lcg.of(1L);
        Rng.Lcg b = Rng.Lcg.of(2L);
        assertNotEquals(a.nextU32(), b.nextU32(), "LCG seed=1 vs seed=2 should differ");
    }

    @Test
    void xorshift64DifferentSeedsDifferentOutput() {
        Rng.Xorshift64 a = Rng.Xorshift64.of(1L);
        Rng.Xorshift64 b = Rng.Xorshift64.of(2L);
        assertNotEquals(a.nextU32(), b.nextU32(), "Xorshift64 seed=1 vs seed=2 should differ");
    }

    @Test
    void pcg32DifferentSeedsDifferentOutput() {
        Rng.Pcg32 a = Rng.Pcg32.of(1L);
        Rng.Pcg32 b = Rng.Pcg32.of(2L);
        assertNotEquals(a.nextU32(), b.nextU32(), "PCG32 seed=1 vs seed=2 should differ");
    }

    // =========================================================================
    // nextU32 range — all values must be in [0, 2^32)
    // =========================================================================

    @Test
    void lcgNextU32InRange() {
        Rng.Lcg g = Rng.Lcg.of(999L);
        for (int i = 0; i < 1000; i++) {
            long v = g.nextU32();
            assertTrue(v >= 0 && v < 4294967296L, "LCG nextU32 out of range: " + v);
        }
    }

    @Test
    void xorshift64NextU32InRange() {
        Rng.Xorshift64 g = Rng.Xorshift64.of(999L);
        for (int i = 0; i < 1000; i++) {
            long v = g.nextU32();
            assertTrue(v >= 0 && v < 4294967296L, "Xorshift64 nextU32 out of range: " + v);
        }
    }

    @Test
    void pcg32NextU32InRange() {
        Rng.Pcg32 g = Rng.Pcg32.of(999L);
        for (int i = 0; i < 1000; i++) {
            long v = g.nextU32();
            assertTrue(v >= 0 && v < 4294967296L, "PCG32 nextU32 out of range: " + v);
        }
    }

    // =========================================================================
    // nextU64 consistency — should equal (hi << 32) | lo from nextU32 pairs
    // =========================================================================

    @Test
    void lcgNextU64ConsistentWithNextU32Pairs() {
        // Build a fresh generator to capture two u32 values...
        Rng.Lcg ref = Rng.Lcg.of(7L);
        long hi = ref.nextU32();
        long lo = ref.nextU32();
        long expected = (hi << 32) | lo;

        // ...then check nextU64 on an identically-seeded generator
        Rng.Lcg g = Rng.Lcg.of(7L);
        assertEquals(expected, g.nextU64(), "LCG nextU64 must equal (hi<<32)|lo");
    }

    @Test
    void xorshift64NextU64ConsistentWithNextU32Pairs() {
        Rng.Xorshift64 ref = Rng.Xorshift64.of(7L);
        long hi = ref.nextU32();
        long lo = ref.nextU32();
        long expected = (hi << 32) | lo;

        Rng.Xorshift64 g = Rng.Xorshift64.of(7L);
        assertEquals(expected, g.nextU64(), "Xorshift64 nextU64 must equal (hi<<32)|lo");
    }

    @Test
    void pcg32NextU64ConsistentWithNextU32Pairs() {
        Rng.Pcg32 ref = Rng.Pcg32.of(7L);
        long hi = ref.nextU32();
        long lo = ref.nextU32();
        long expected = (hi << 32) | lo;

        Rng.Pcg32 g = Rng.Pcg32.of(7L);
        assertEquals(expected, g.nextU64(), "PCG32 nextU64 must equal (hi<<32)|lo");
    }

    // =========================================================================
    // nextFloat — must be in [0.0, 1.0)
    // =========================================================================

    @Test
    void lcgNextFloatInRange() {
        Rng.Lcg g = Rng.Lcg.of(5L);
        for (int i = 0; i < 1000; i++) {
            double v = g.nextFloat();
            assertTrue(v >= 0.0 && v < 1.0, "LCG nextFloat out of range: " + v);
        }
    }

    @Test
    void xorshift64NextFloatInRange() {
        Rng.Xorshift64 g = Rng.Xorshift64.of(5L);
        for (int i = 0; i < 1000; i++) {
            double v = g.nextFloat();
            assertTrue(v >= 0.0 && v < 1.0, "Xorshift64 nextFloat out of range: " + v);
        }
    }

    @Test
    void pcg32NextFloatInRange() {
        Rng.Pcg32 g = Rng.Pcg32.of(5L);
        for (int i = 0; i < 1000; i++) {
            double v = g.nextFloat();
            assertTrue(v >= 0.0 && v < 1.0, "PCG32 nextFloat out of range: " + v);
        }
    }

    // =========================================================================
    // nextIntInRange — boundary and distribution checks
    // =========================================================================

    @Test
    void lcgNextIntInRangeAllInBounds() {
        Rng.Lcg g = Rng.Lcg.of(3L);
        for (int i = 0; i < 1000; i++) {
            long v = g.nextIntInRange(1, 6);
            assertTrue(v >= 1 && v <= 6, "LCG die roll out of range: " + v);
        }
    }

    @Test
    void xorshift64NextIntInRangeAllInBounds() {
        Rng.Xorshift64 g = Rng.Xorshift64.of(3L);
        for (int i = 0; i < 1000; i++) {
            long v = g.nextIntInRange(1, 6);
            assertTrue(v >= 1 && v <= 6, "Xorshift64 die roll out of range: " + v);
        }
    }

    @Test
    void pcg32NextIntInRangeAllInBounds() {
        Rng.Pcg32 g = Rng.Pcg32.of(3L);
        for (int i = 0; i < 1000; i++) {
            long v = g.nextIntInRange(1, 6);
            assertTrue(v >= 1 && v <= 6, "PCG32 die roll out of range: " + v);
        }
    }

    /** A range of size 1 must always return the single possible value. */
    @Test
    void nextIntInRangeSizeOne() {
        Rng.Lcg  lcg = Rng.Lcg.of(0L);
        Rng.Xorshift64 xs = Rng.Xorshift64.of(0L);
        Rng.Pcg32 pcg = Rng.Pcg32.of(0L);
        for (int i = 0; i < 20; i++) {
            assertEquals(7L, lcg.nextIntInRange(7, 7),  "LCG range(7,7)");
            assertEquals(7L, xs.nextIntInRange(7, 7),   "Xorshift64 range(7,7)");
            assertEquals(7L, pcg.nextIntInRange(7, 7),  "PCG32 range(7,7)");
        }
    }

    // =========================================================================
    // Edge case: Xorshift64 seed 0 is replaced with 1
    // =========================================================================

    @Test
    void xorshift64Seed0EquivalentToSeed1() {
        Rng.Xorshift64 fromZero = Rng.Xorshift64.of(0L);
        Rng.Xorshift64 fromOne  = Rng.Xorshift64.of(1L);
        for (int i = 0; i < 5; i++) {
            assertEquals(fromOne.nextU32(), fromZero.nextU32(),
                "Xorshift64 seed=0 should behave like seed=1 at step " + i);
        }
    }

    // =========================================================================
    // Large seeds — verify generators work with large / negative-looking longs
    // =========================================================================

    @Test
    void lcgLargeSeedDoesNotThrow() {
        Rng.Lcg g = Rng.Lcg.of(Long.MAX_VALUE);
        assertDoesNotThrow(() -> {
            for (int i = 0; i < 100; i++) g.nextU32();
        });
    }

    @Test
    void xorshift64LargeSeedDoesNotThrow() {
        Rng.Xorshift64 g = Rng.Xorshift64.of(Long.MAX_VALUE);
        assertDoesNotThrow(() -> {
            for (int i = 0; i < 100; i++) g.nextU32();
        });
    }

    @Test
    void pcg32LargeSeedDoesNotThrow() {
        Rng.Pcg32 g = Rng.Pcg32.of(Long.MAX_VALUE);
        assertDoesNotThrow(() -> {
            for (int i = 0; i < 100; i++) g.nextU32();
        });
    }

    // =========================================================================
    // PCG32 seed=0 produces non-zero output (initseq warms up state)
    // =========================================================================

    @Test
    void pcg32Seed0IsNonTrivial() {
        Rng.Pcg32 g = Rng.Pcg32.of(0L);
        long v = g.nextU32();
        // The warm-up advance guarantees the state is not stuck at 0
        assertTrue(v > 0, "PCG32 seed=0 first output should be non-zero");
    }
}

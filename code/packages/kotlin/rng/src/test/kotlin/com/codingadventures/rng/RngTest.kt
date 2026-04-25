// ============================================================================
// RngTest.kt — Unit Tests for Lcg, Xorshift64, and Pcg32
// ============================================================================
//
// Test strategy:
//   1. Known reference values (seed=1) — cross-checked against the Go
//      implementation to confirm identical bit-for-bit output.
//   2. Reproducibility — same seed → same sequence (independent instances).
//   3. Different seeds produce different sequences.
//   4. nextU64 consistency — must equal (hi shl 32) or lo.
//   5. nextFloat in [0.0, 1.0).
//   6. nextIntInRange bounds and edge cases.
//   7. Edge cases: seed 0 (Xorshift64), large seeds, PCG32 seed 0 non-trivial.
// ============================================================================

package com.codingadventures.rng

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class RngTest {

    // =========================================================================
    // Known reference values (seed = 1, verified against Go implementation)
    // =========================================================================

    @Test
    fun lcgSeed1FirstThreeValues() {
        val g = Lcg(1L)
        assertEquals(1817669548L, g.nextU32(), "LCG seed=1 call 1")
        assertEquals(2187888307L, g.nextU32(), "LCG seed=1 call 2")
        assertEquals(2784682393L, g.nextU32(), "LCG seed=1 call 3")
    }

    @Test
    fun xorshift64Seed1FirstThreeValues() {
        val g = Xorshift64(1L)
        assertEquals(1082269761L, g.nextU32(), "Xorshift64 seed=1 call 1")
        assertEquals(201397313L,  g.nextU32(), "Xorshift64 seed=1 call 2")
        assertEquals(1854285353L, g.nextU32(), "Xorshift64 seed=1 call 3")
    }

    @Test
    fun pcg32Seed1FirstThreeValues() {
        val g = Pcg32(1L)
        assertEquals(1412771199L, g.nextU32(), "PCG32 seed=1 call 1")
        assertEquals(1791099446L, g.nextU32(), "PCG32 seed=1 call 2")
        assertEquals(124312908L,  g.nextU32(), "PCG32 seed=1 call 3")
    }

    // =========================================================================
    // Reproducibility — same seed → same sequence
    // =========================================================================

    @Test
    fun lcgReproducible() {
        val a = Lcg(42L)
        val b = Lcg(42L)
        repeat(10) { i ->
            assertEquals(a.nextU32(), b.nextU32(), "LCG mismatch at step $i")
        }
    }

    @Test
    fun xorshift64Reproducible() {
        val a = Xorshift64(42L)
        val b = Xorshift64(42L)
        repeat(10) { i ->
            assertEquals(a.nextU32(), b.nextU32(), "Xorshift64 mismatch at step $i")
        }
    }

    @Test
    fun pcg32Reproducible() {
        val a = Pcg32(42L)
        val b = Pcg32(42L)
        repeat(10) { i ->
            assertEquals(a.nextU32(), b.nextU32(), "PCG32 mismatch at step $i")
        }
    }

    // =========================================================================
    // Different seeds → different sequences
    // =========================================================================

    @Test
    fun lcgDifferentSeedsDifferentOutput() {
        val a = Lcg(1L)
        val b = Lcg(2L)
        assertNotEquals(a.nextU32(), b.nextU32(), "LCG seed=1 vs seed=2 should differ")
    }

    @Test
    fun xorshift64DifferentSeedsDifferentOutput() {
        val a = Xorshift64(1L)
        val b = Xorshift64(2L)
        assertNotEquals(a.nextU32(), b.nextU32(), "Xorshift64 seed=1 vs seed=2 should differ")
    }

    @Test
    fun pcg32DifferentSeedsDifferentOutput() {
        val a = Pcg32(1L)
        val b = Pcg32(2L)
        assertNotEquals(a.nextU32(), b.nextU32(), "PCG32 seed=1 vs seed=2 should differ")
    }

    // =========================================================================
    // nextU32 range — all values must be in [0, 2^32)
    // =========================================================================

    @Test
    fun lcgNextU32InRange() {
        val g = Lcg(999L)
        repeat(1000) {
            val v = g.nextU32()
            assertTrue(v in 0L..4294967295L, "LCG nextU32 out of range: $v")
        }
    }

    @Test
    fun xorshift64NextU32InRange() {
        val g = Xorshift64(999L)
        repeat(1000) {
            val v = g.nextU32()
            assertTrue(v in 0L..4294967295L, "Xorshift64 nextU32 out of range: $v")
        }
    }

    @Test
    fun pcg32NextU32InRange() {
        val g = Pcg32(999L)
        repeat(1000) {
            val v = g.nextU32()
            assertTrue(v in 0L..4294967295L, "PCG32 nextU32 out of range: $v")
        }
    }

    // =========================================================================
    // nextU64 consistency — must equal (hi shl 32) or lo from nextU32 pairs
    // =========================================================================

    @Test
    fun lcgNextU64ConsistentWithNextU32Pairs() {
        val ref = Lcg(7L)
        val hi = ref.nextU32()
        val lo = ref.nextU32()
        val expected = (hi shl 32) or lo

        val g = Lcg(7L)
        assertEquals(expected, g.nextU64(), "LCG nextU64 must equal (hi shl 32) or lo")
    }

    @Test
    fun xorshift64NextU64ConsistentWithNextU32Pairs() {
        val ref = Xorshift64(7L)
        val hi = ref.nextU32()
        val lo = ref.nextU32()
        val expected = (hi shl 32) or lo

        val g = Xorshift64(7L)
        assertEquals(expected, g.nextU64(), "Xorshift64 nextU64 must equal (hi shl 32) or lo")
    }

    @Test
    fun pcg32NextU64ConsistentWithNextU32Pairs() {
        val ref = Pcg32(7L)
        val hi = ref.nextU32()
        val lo = ref.nextU32()
        val expected = (hi shl 32) or lo

        val g = Pcg32(7L)
        assertEquals(expected, g.nextU64(), "PCG32 nextU64 must equal (hi shl 32) or lo")
    }

    // =========================================================================
    // nextFloat — must be in [0.0, 1.0)
    // =========================================================================

    @Test
    fun lcgNextFloatInRange() {
        val g = Lcg(5L)
        repeat(1000) {
            val v = g.nextFloat()
            assertTrue(v >= 0.0 && v < 1.0, "LCG nextFloat out of range: $v")
        }
    }

    @Test
    fun xorshift64NextFloatInRange() {
        val g = Xorshift64(5L)
        repeat(1000) {
            val v = g.nextFloat()
            assertTrue(v >= 0.0 && v < 1.0, "Xorshift64 nextFloat out of range: $v")
        }
    }

    @Test
    fun pcg32NextFloatInRange() {
        val g = Pcg32(5L)
        repeat(1000) {
            val v = g.nextFloat()
            assertTrue(v >= 0.0 && v < 1.0, "PCG32 nextFloat out of range: $v")
        }
    }

    // =========================================================================
    // nextIntInRange — boundary and distribution checks
    // =========================================================================

    @Test
    fun lcgNextIntInRangeAllInBounds() {
        val g = Lcg(3L)
        repeat(1000) {
            val v = g.nextIntInRange(1L, 6L)
            assertTrue(v in 1L..6L, "LCG die roll out of range: $v")
        }
    }

    @Test
    fun xorshift64NextIntInRangeAllInBounds() {
        val g = Xorshift64(3L)
        repeat(1000) {
            val v = g.nextIntInRange(1L, 6L)
            assertTrue(v in 1L..6L, "Xorshift64 die roll out of range: $v")
        }
    }

    @Test
    fun pcg32NextIntInRangeAllInBounds() {
        val g = Pcg32(3L)
        repeat(1000) {
            val v = g.nextIntInRange(1L, 6L)
            assertTrue(v in 1L..6L, "PCG32 die roll out of range: $v")
        }
    }

    /** A range of size 1 must always return the single possible value. */
    @Test
    fun nextIntInRangeSizeOne() {
        val lcg = Lcg(0L)
        val xs  = Xorshift64(0L)
        val pcg = Pcg32(0L)
        repeat(20) {
            assertEquals(7L, lcg.nextIntInRange(7L, 7L), "LCG range(7,7)")
            assertEquals(7L, xs.nextIntInRange(7L, 7L),  "Xorshift64 range(7,7)")
            assertEquals(7L, pcg.nextIntInRange(7L, 7L), "PCG32 range(7,7)")
        }
    }

    // =========================================================================
    // Edge case: Xorshift64 seed 0 is replaced with 1
    // =========================================================================

    @Test
    fun xorshift64Seed0EquivalentToSeed1() {
        val fromZero = Xorshift64(0L)
        val fromOne  = Xorshift64(1L)
        repeat(5) { i ->
            assertEquals(fromOne.nextU32(), fromZero.nextU32(),
                "Xorshift64 seed=0 should behave like seed=1 at step $i")
        }
    }

    // =========================================================================
    // Large seeds — verify generators work with large / negative-looking longs
    // =========================================================================

    @Test
    fun lcgLargeSeedDoesNotThrow() {
        val g = Lcg(Long.MAX_VALUE)
        repeat(100) { g.nextU32() }
    }

    @Test
    fun xorshift64LargeSeedDoesNotThrow() {
        val g = Xorshift64(Long.MAX_VALUE)
        repeat(100) { g.nextU32() }
    }

    @Test
    fun pcg32LargeSeedDoesNotThrow() {
        val g = Pcg32(Long.MAX_VALUE)
        repeat(100) { g.nextU32() }
    }

    // =========================================================================
    // PCG32 seed=0 produces non-zero output (initseq warms up state)
    // =========================================================================

    @Test
    fun pcg32Seed0IsNonTrivial() {
        val g = Pcg32(0L)
        val v = g.nextU32()
        assertTrue(v > 0L, "PCG32 seed=0 first output should be non-zero")
    }
}

import XCTest
@testable import Rng

// ============================================================================
// RngTests — Comprehensive tests for LCG, Xorshift64, and PCG32
// ============================================================================
//
// Reference values (seed = 1, from Go implementation):
//   LCG:        [1817669548, 2187888307, 2784682393]
//   Xorshift64: [1082269761, 201397313,  1854285353]
//   PCG32:      [1412771199, 1791099446, 124312908]
//
// The tests are grouped into sections:
//   1. Known-value tests (deterministic correctness against Go reference)
//   2. API shape tests (types, return ranges)
//   3. Statistical sanity tests (mean, variance, chi-squared lite)
//   4. Edge-case tests (seed 0, seed max, range = 1)
// ============================================================================

final class RngTests: XCTestCase {

    // ========================================================================
    // MARK: - 1. LCG Known-Value Tests
    // ========================================================================

    /// LCG with seed=1 must produce the three reference values in order.
    func testLCGKnownValues() {
        var g = LCG(seed: 1)
        XCTAssertEqual(g.nextU32(), 1817669548)
        XCTAssertEqual(g.nextU32(), 2187888307)
        XCTAssertEqual(g.nextU32(), 2784682393)
    }

    /// LCG with seed=0 should not crash and should produce a deterministic value.
    func testLCGSeedZero() {
        var g = LCG(seed: 0)
        // state after one step: 0 * mult + inc = lcgIncrement
        // output = lcgIncrement >> 32
        let expected = UInt32(truncatingIfNeeded: UInt64(1442695040888963407) >> 32)
        XCTAssertEqual(g.nextU32(), expected)
    }

    /// LCG seed=1 first three values match exactly.
    func testLCGFirstThreeExact() {
        var g = LCG(seed: 1)
        let vals = [g.nextU32(), g.nextU32(), g.nextU32()]
        XCTAssertEqual(vals[0], 1817669548)
        XCTAssertEqual(vals[1], 2187888307)
        XCTAssertEqual(vals[2], 2784682393)
    }

    /// Two LCGs with the same seed produce identical sequences.
    func testLCGReproducibility() {
        var g1 = LCG(seed: 99999)
        var g2 = LCG(seed: 99999)
        for _ in 0..<50 {
            XCTAssertEqual(g1.nextU32(), g2.nextU32())
        }
    }

    /// Two LCGs with different seeds should diverge immediately.
    func testLCGDifferentSeedsProduceDifferentOutput() {
        var g1 = LCG(seed: 1)
        var g2 = LCG(seed: 2)
        XCTAssertNotEqual(g1.nextU32(), g2.nextU32())
    }

    // ========================================================================
    // MARK: - 2. LCG Derived-Output Tests
    // ========================================================================

    /// LCG nextU64 must equal (hi << 32) | lo from two consecutive nextU32.
    func testLCGNextU64IsHiLoComposition() {
        var ref = LCG(seed: 42)
        var tst = LCG(seed: 42)
        for _ in 0..<20 {
            let hi = UInt64(ref.nextU32())
            let lo = UInt64(ref.nextU32())
            let expected = (hi << 32) | lo
            XCTAssertEqual(tst.nextU64(), expected)
        }
    }

    /// LCG nextFloat must be in [0.0, 1.0) for all draws.
    func testLCGNextFloatIsInUnitInterval() {
        var g = LCG(seed: 7)
        for _ in 0..<1000 {
            let f = g.nextFloat()
            XCTAssertGreaterThanOrEqual(f, 0.0)
            XCTAssertLessThan(f, 1.0)
        }
    }

    /// LCG nextIntInRange must stay within [min, max] inclusive.
    func testLCGNextIntInRangeStaysInBounds() {
        var g = LCG(seed: 42)
        for _ in 0..<2000 {
            let v = g.nextIntInRange(min: -5, max: 5)
            XCTAssertGreaterThanOrEqual(v, -5)
            XCTAssertLessThanOrEqual(v, 5)
        }
    }

    /// LCG nextIntInRange with range=1 always returns min.
    func testLCGNextIntInRangeOfOne() {
        var g = LCG(seed: 0)
        for _ in 0..<100 {
            XCTAssertEqual(g.nextIntInRange(min: 42, max: 42), 42)
        }
    }

    // ========================================================================
    // MARK: - 3. Xorshift64 Known-Value Tests
    // ========================================================================

    /// Xorshift64 with seed=1 must produce the three reference values.
    func testXorshift64KnownValues() {
        var g = Xorshift64(seed: 1)
        XCTAssertEqual(g.nextU32(), 1082269761)
        XCTAssertEqual(g.nextU32(), 201397313)
        XCTAssertEqual(g.nextU32(), 1854285353)
    }

    /// Xorshift64 replaces seed 0 with 1 — same output as seed=1.
    func testXorshift64SeedZeroReplacedWithOne() {
        var g0 = Xorshift64(seed: 0)
        var g1 = Xorshift64(seed: 1)
        for _ in 0..<20 {
            XCTAssertEqual(g0.nextU32(), g1.nextU32())
        }
    }

    /// Xorshift64 is reproducible.
    func testXorshift64Reproducibility() {
        var g1 = Xorshift64(seed: 123456789)
        var g2 = Xorshift64(seed: 123456789)
        for _ in 0..<50 {
            XCTAssertEqual(g1.nextU32(), g2.nextU32())
        }
    }

    /// Xorshift64 nextFloat is in [0.0, 1.0).
    func testXorshift64NextFloatIsInUnitInterval() {
        var g = Xorshift64(seed: 31415926)
        for _ in 0..<1000 {
            let f = g.nextFloat()
            XCTAssertGreaterThanOrEqual(f, 0.0)
            XCTAssertLessThan(f, 1.0)
        }
    }

    /// Xorshift64 nextIntInRange stays within bounds.
    func testXorshift64NextIntInRangeStaysInBounds() {
        var g = Xorshift64(seed: 42)
        for _ in 0..<2000 {
            let v = g.nextIntInRange(min: 1, max: 6)
            XCTAssertGreaterThanOrEqual(v, 1)
            XCTAssertLessThanOrEqual(v, 6)
        }
    }

    // ========================================================================
    // MARK: - 4. PCG32 Known-Value Tests
    // ========================================================================

    /// PCG32 with seed=1 must produce the three reference values.
    func testPCG32KnownValues() {
        var g = PCG32(seed: 1)
        XCTAssertEqual(g.nextU32(), 1412771199)
        XCTAssertEqual(g.nextU32(), 1791099446)
        XCTAssertEqual(g.nextU32(), 124312908)
    }

    /// PCG32 is reproducible.
    func testPCG32Reproducibility() {
        var g1 = PCG32(seed: 7777)
        var g2 = PCG32(seed: 7777)
        for _ in 0..<50 {
            XCTAssertEqual(g1.nextU32(), g2.nextU32())
        }
    }

    /// PCG32 seed=0 does not crash and is different from seed=1.
    func testPCG32SeedZeroRunsAndDiffersFromSeedOne() {
        var g0 = PCG32(seed: 0)
        var g1 = PCG32(seed: 1)
        // At least one of the first three values should differ.
        let v0 = [g0.nextU32(), g0.nextU32(), g0.nextU32()]
        let v1 = [g1.nextU32(), g1.nextU32(), g1.nextU32()]
        XCTAssertNotEqual(v0, v1)
    }

    /// PCG32 nextU64 is (hi << 32) | lo.
    func testPCG32NextU64IsHiLoComposition() {
        var ref = PCG32(seed: 99)
        var tst = PCG32(seed: 99)
        for _ in 0..<20 {
            let hi = UInt64(ref.nextU32())
            let lo = UInt64(ref.nextU32())
            let expected = (hi << 32) | lo
            XCTAssertEqual(tst.nextU64(), expected)
        }
    }

    /// PCG32 nextFloat is in [0.0, 1.0).
    func testPCG32NextFloatIsInUnitInterval() {
        var g = PCG32(seed: 2718281828)
        for _ in 0..<1000 {
            let f = g.nextFloat()
            XCTAssertGreaterThanOrEqual(f, 0.0)
            XCTAssertLessThan(f, 1.0)
        }
    }

    /// PCG32 nextIntInRange stays within bounds.
    func testPCG32NextIntInRangeStaysInBounds() {
        var g = PCG32(seed: 42)
        for _ in 0..<2000 {
            let v = g.nextIntInRange(min: 0, max: 100)
            XCTAssertGreaterThanOrEqual(v, 0)
            XCTAssertLessThanOrEqual(v, 100)
        }
    }

    /// PCG32 nextIntInRange with range=1 always returns min.
    func testPCG32NextIntInRangeOfOne() {
        var g = PCG32(seed: 0)
        for _ in 0..<100 {
            XCTAssertEqual(g.nextIntInRange(min: -7, max: -7), -7)
        }
    }

    // ========================================================================
    // MARK: - 5. Cross-generator independence tests
    // ========================================================================

    /// All three generators with the same seed must produce different values.
    ///
    /// If all three returned the same value it would strongly suggest a bug
    /// where one implementation is calling another.
    func testAllThreeGeneratorsProduceDifferentFirstValues() {
        var lcg = LCG(seed: 1)
        var xs  = Xorshift64(seed: 1)
        var pcg = PCG32(seed: 1)
        let a = lcg.nextU32()
        let b = xs.nextU32()
        let c = pcg.nextU32()
        // All three algorithms are different, so all three values must differ.
        XCTAssertNotEqual(a, b)
        XCTAssertNotEqual(b, c)
        XCTAssertNotEqual(a, c)
    }

    // ========================================================================
    // MARK: - 6. Statistical sanity (chi-squared, mean, coverage)
    // ========================================================================

    /// PCG32 nextFloat mean should be near 0.5 for 10,000 draws.
    ///
    /// For a U[0,1) distribution, E[X] = 0.5.  With 10k samples the
    /// standard error is sqrt(1/12 / 10000) ≈ 0.0029.  We allow ±0.02.
    func testPCG32FloatMeanNearHalf() {
        var g = PCG32(seed: 12345)
        var sum = 0.0
        let n = 10000
        for _ in 0..<n {
            sum += g.nextFloat()
        }
        let mean = sum / Double(n)
        XCTAssertGreaterThan(mean, 0.48)
        XCTAssertLessThan(mean, 0.52)
    }

    /// LCG should produce all values in [1, 6] over 1200 rolls.
    ///
    /// For a fair 6-sided die, the probability of *never* seeing a specific
    /// face in 1200 rolls is (5/6)^1200 ≈ 3.7 × 10^-95 — astronomically
    /// small.  Any PRNG that misses a face has a serious bug.
    func testLCGDieCoverageAllValues() {
        var g = LCG(seed: 42)
        var seen = Set<Int64>()
        for _ in 0..<1200 {
            seen.insert(g.nextIntInRange(min: 1, max: 6))
        }
        XCTAssertEqual(seen, [1, 2, 3, 4, 5, 6])
    }

    /// Xorshift64 should produce all values in [0, 9] over 2000 draws.
    func testXorshift64RangeCoverageAllValues() {
        var g = Xorshift64(seed: 314159)
        var seen = Set<Int64>()
        for _ in 0..<2000 {
            seen.insert(g.nextIntInRange(min: 0, max: 9))
        }
        XCTAssertEqual(seen, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
    }

    /// struct value-type semantics: copying a generator preserves state.
    ///
    /// Because Swift structs are value types, assigning `var b = a` gives
    /// an independent copy.  Both should produce the same subsequent sequence.
    func testStructValueTypeSemanticsLCG() {
        var a = LCG(seed: 555)
        _ = a.nextU32() // advance once to a non-trivial state
        var b = a       // copy
        XCTAssertEqual(a.nextU32(), b.nextU32())
        XCTAssertEqual(a.nextU32(), b.nextU32())
    }

    func testStructValueTypeSematicsPCG32() {
        var a = PCG32(seed: 555)
        _ = a.nextU32()
        var b = a
        XCTAssertEqual(a.nextU32(), b.nextU32())
        XCTAssertEqual(a.nextU32(), b.nextU32())
    }
}

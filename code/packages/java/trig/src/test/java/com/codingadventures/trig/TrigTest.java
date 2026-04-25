// ============================================================================
// TrigTest.java — Unit Tests for Trig
// ============================================================================
//
// Tests are organized into 10 sections:
//   1. sin — special values, symmetry (odd function), Pythagorean identity
//   2. cos — special values, symmetry (even function), Pythagorean identity
//   3. tan — special values, near-pole guard
//   4. sqrt — exact squares, Newton convergence, negative domain error
//   5. radians — degree → radian conversions
//   6. degrees — radian → degree conversions
//   7. roundtrip — degrees ↔ radians inverse
//   8. large inputs — stress range reduction
//   9. atan — special values, range reduction, round-trip
//  10. atan2 — quadrant correctness, axes, origin
// ============================================================================

package com.codingadventures.trig;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class TrigTest {

    /** Absolute tolerance used for all approximate equality checks. */
    private static final double TOL = 1e-10;

    // =========================================================================
    // 1. sin — special values
    // =========================================================================

    @Test
    void sinZero() {
        assertEquals(0.0, Trig.sin(0.0), TOL);
    }

    @Test
    void sinPiOver2() {
        assertEquals(1.0, Trig.sin(Trig.PI / 2), TOL);
    }

    @Test
    void sinPi() {
        assertEquals(0.0, Trig.sin(Trig.PI), TOL);
    }

    @Test
    void sin3PiOver2() {
        assertEquals(-1.0, Trig.sin(3 * Trig.PI / 2), TOL);
    }

    @Test
    void sin2Pi() {
        assertEquals(0.0, Trig.sin(2 * Trig.PI), TOL);
    }

    @Test
    void sinPiOver6() {
        assertEquals(0.5, Trig.sin(Trig.PI / 6), TOL);
    }

    @Test
    void sinPiOver4() {
        // sin(π/4) = √2/2 ≈ 0.7071067811865476
        assertEquals(0.7071067811865476, Trig.sin(Trig.PI / 4), TOL);
    }

    @Test
    void sinPiOver3() {
        // sin(π/3) = √3/2 ≈ 0.8660254037844386
        assertEquals(0.8660254037844386, Trig.sin(Trig.PI / 3), TOL);
    }

    // sin is an ODD function: sin(-x) = -sin(x)
    @Test
    void sinIsOdd() {
        double[] angles = {0.5, 1.0, 1.5, 2.0, 2.7, Trig.PI / 4, Trig.PI / 3};
        for (double a : angles) {
            assertEquals(-Trig.sin(a), Trig.sin(-a), TOL,
                "sin(-" + a + ") should equal -sin(" + a + ")");
        }
    }

    // =========================================================================
    // 2. cos — special values
    // =========================================================================

    @Test
    void cosZero() {
        assertEquals(1.0, Trig.cos(0.0), TOL);
    }

    @Test
    void cosPiOver2() {
        assertEquals(0.0, Trig.cos(Trig.PI / 2), TOL);
    }

    @Test
    void cosPi() {
        assertEquals(-1.0, Trig.cos(Trig.PI), TOL);
    }

    @Test
    void cos3PiOver2() {
        assertEquals(0.0, Trig.cos(3 * Trig.PI / 2), TOL);
    }

    @Test
    void cos2Pi() {
        assertEquals(1.0, Trig.cos(2 * Trig.PI), TOL);
    }

    @Test
    void cosPiOver6() {
        // cos(π/6) = √3/2 ≈ 0.8660254037844386
        assertEquals(0.8660254037844386, Trig.cos(Trig.PI / 6), TOL);
    }

    @Test
    void cosPiOver4() {
        // cos(π/4) = √2/2 ≈ 0.7071067811865476
        assertEquals(0.7071067811865476, Trig.cos(Trig.PI / 4), TOL);
    }

    @Test
    void cosPiOver3() {
        assertEquals(0.5, Trig.cos(Trig.PI / 3), TOL);
    }

    // cos is an EVEN function: cos(-x) = cos(x)
    @Test
    void cosIsEven() {
        double[] angles = {0.5, 1.0, 1.5, 2.0, 2.7, Trig.PI / 4, Trig.PI / 3};
        for (double a : angles) {
            assertEquals(Trig.cos(a), Trig.cos(-a), TOL,
                "cos(-" + a + ") should equal cos(" + a + ")");
        }
    }

    // =========================================================================
    // 3. Pythagorean identity: sin²(x) + cos²(x) = 1
    // =========================================================================

    @Test
    void pythagoreanIdentity() {
        double[] angles = {
            0, Trig.PI / 6, Trig.PI / 4, Trig.PI / 3, Trig.PI / 2,
            Trig.PI, 3 * Trig.PI / 2, 2 * Trig.PI,
            -1.0, -2.5, 0.1, 3.0, 5.5
        };
        for (double x : angles) {
            double s = Trig.sin(x);
            double c = Trig.cos(x);
            assertEquals(1.0, s * s + c * c, TOL,
                "sin²(" + x + ") + cos²(" + x + ") should equal 1");
        }
    }

    // =========================================================================
    // 4. tan — special values and pole guard
    // =========================================================================

    @Test
    void tanZero() {
        assertEquals(0.0, Trig.tan(0.0), TOL);
    }

    @Test
    void tanPiOver4() {
        // tan(π/4) = 1
        assertEquals(1.0, Trig.tan(Trig.PI / 4), TOL);
    }

    @Test
    void tanPiOver6() {
        // tan(π/6) = 1/√3 ≈ 0.5773...
        assertEquals(1.0 / Trig.sqrt(3.0), Trig.tan(Trig.PI / 6), TOL);
    }

    @Test
    void tanNegativePiOver4() {
        assertEquals(-1.0, Trig.tan(-Trig.PI / 4), TOL);
    }

    // Near a pole (π/2), tan should return a very large finite value.
    @Test
    void tanNearPoleIsLargeFinite() {
        double t = Trig.tan(Trig.PI / 2);
        assertTrue(Double.isFinite(t),   "tan(π/2) should be finite (not NaN/Inf)");
        assertTrue(Math.abs(t) > 1e100,  "tan(π/2) should have very large magnitude");
    }

    // =========================================================================
    // 5. sqrt
    // =========================================================================

    @Test
    void sqrtZero() {
        assertEquals(0.0, Trig.sqrt(0.0), TOL);
    }

    @Test
    void sqrtOne() {
        assertEquals(1.0, Trig.sqrt(1.0), TOL);
    }

    @Test
    void sqrtFour() {
        assertEquals(2.0, Trig.sqrt(4.0), TOL);
    }

    @Test
    void sqrtNine() {
        assertEquals(3.0, Trig.sqrt(9.0), TOL);
    }

    @Test
    void sqrtTwo() {
        assertEquals(1.41421356237, Trig.sqrt(2.0), TOL);
    }

    @Test
    void sqrtQuarter() {
        assertEquals(0.5, Trig.sqrt(0.25), TOL);
    }

    @Test
    void sqrtLarge() {
        // sqrt(1e10) ≈ 1e5; use relative tolerance for large values
        assertEquals(1e5, Trig.sqrt(1e10), 1e5 * 1e-9);
    }

    @Test
    void sqrtRoundtrip() {
        // sqrt(2) * sqrt(2) should recover 2.0
        double s = Trig.sqrt(2.0);
        assertEquals(2.0, s * s, TOL);
    }

    @Test
    void sqrtNegativeThrows() {
        assertThrows(ArithmeticException.class, () -> Trig.sqrt(-1.0));
    }

    // =========================================================================
    // 6. radians — degree → radian conversion
    // =========================================================================

    @Test
    void radians0() {
        assertEquals(0.0, Trig.radians(0.0), TOL);
    }

    @Test
    void radians90() {
        assertEquals(Trig.PI / 2, Trig.radians(90.0), TOL);
    }

    @Test
    void radians180() {
        assertEquals(Trig.PI, Trig.radians(180.0), TOL);
    }

    @Test
    void radians360() {
        assertEquals(2 * Trig.PI, Trig.radians(360.0), TOL);
    }

    // =========================================================================
    // 7. degrees — radian → degree conversion
    // =========================================================================

    @Test
    void degrees0() {
        assertEquals(0.0, Trig.degrees(0.0), TOL);
    }

    @Test
    void degreesPiOver2() {
        assertEquals(90.0, Trig.degrees(Trig.PI / 2), TOL);
    }

    @Test
    void degreesPi() {
        assertEquals(180.0, Trig.degrees(Trig.PI), TOL);
    }

    // =========================================================================
    // 8. Roundtrip — degrees ↔ radians
    // =========================================================================

    @Test
    void roundtripDegreesToRadians() {
        int[] degs = {0, 30, 45, 60, 90, 120, 180, 270, 360};
        for (int d : degs) {
            assertEquals((double) d, Trig.degrees(Trig.radians(d)), TOL,
                "degrees(radians(" + d + ")) should be " + d);
        }
    }

    @Test
    void roundtripRadiansToDegrees() {
        double[] rads = {0, Trig.PI / 6, Trig.PI / 4, Trig.PI / 3,
                         Trig.PI / 2, Trig.PI, 2 * Trig.PI};
        for (double r : rads) {
            assertEquals(r, Trig.radians(Trig.degrees(r)), TOL,
                "radians(degrees(" + r + ")) should be " + r);
        }
    }

    // =========================================================================
    // 9. Large inputs — stress range reduction
    // =========================================================================

    @Test
    void sin1000Pi() {
        // 1000 * π is a multiple of 2π, so sin ≈ 0
        assertEquals(0.0, Trig.sin(1000 * Trig.PI), TOL);
    }

    @Test
    void cos1000Pi() {
        // 1000 is even → cos(1000π) = cos(0) = 1
        assertEquals(1.0, Trig.cos(1000 * Trig.PI), TOL);
    }

    @Test
    void pythagoreanIdentityLargeInput() {
        double s = Trig.sin(100.0);
        double c = Trig.cos(100.0);
        assertEquals(1.0, s * s + c * c, TOL);
    }

    @Test
    void sinLargeNegativeIsOdd() {
        assertEquals(-Trig.sin(100.0), Trig.sin(-100.0), TOL);
    }

    // =========================================================================
    // 10. Integration — sin/cos with degree input
    // =========================================================================

    @Test
    void sin30Degrees() {
        assertEquals(0.5, Trig.sin(Trig.radians(30)), TOL);
    }

    @Test
    void cos60Degrees() {
        assertEquals(0.5, Trig.cos(Trig.radians(60)), TOL);
    }

    @Test
    void sin45Degrees() {
        assertEquals(0.7071067811865476, Trig.sin(Trig.radians(45)), TOL);
    }

    // =========================================================================
    // 11. atan
    // =========================================================================

    @Test
    void atanZero() {
        assertEquals(0.0, Trig.atan(0.0), TOL);
    }

    @Test
    void atanOne() {
        // atan(1) = π/4
        assertEquals(Trig.PI / 4, Trig.atan(1.0), TOL);
    }

    @Test
    void atanMinusOne() {
        assertEquals(-Trig.PI / 4, Trig.atan(-1.0), TOL);
    }

    @Test
    void atanSqrt3() {
        // atan(√3) = π/3
        assertEquals(Trig.PI / 3, Trig.atan(Trig.sqrt(3.0)), TOL);
    }

    @Test
    void atanInvSqrt3() {
        // atan(1/√3) = π/6
        assertEquals(Trig.PI / 6, Trig.atan(1.0 / Trig.sqrt(3.0)), TOL);
    }

    @Test
    void atanLargeApproachesPiOver2() {
        assertEquals(Trig.PI / 2, Trig.atan(1e10), 1e-5);
    }

    @Test
    void atanLargeNegativeApproachesMinusPiOver2() {
        assertEquals(-Trig.PI / 2, Trig.atan(-1e10), 1e-5);
    }

    @Test
    void atanTanRoundtrip() {
        // atan(tan(π/4)) ≈ π/4
        assertEquals(Trig.PI / 4, Trig.atan(Trig.tan(Trig.PI / 4)), TOL);
    }

    // =========================================================================
    // 12. atan2 — quadrant correctness
    // =========================================================================

    @Test
    void atan2PositiveXAxis() {
        // atan2(0, 1) = 0  (positive x-axis)
        assertEquals(0.0, Trig.atan2(0, 1), TOL);
    }

    @Test
    void atan2PositiveYAxis() {
        // atan2(1, 0) = π/2  (positive y-axis)
        assertEquals(Trig.PI / 2, Trig.atan2(1, 0), TOL);
    }

    @Test
    void atan2NegativeXAxis() {
        // atan2(0, -1) = π  (negative x-axis)
        assertEquals(Trig.PI, Trig.atan2(0, -1), TOL);
    }

    @Test
    void atan2NegativeYAxis() {
        // atan2(-1, 0) = -π/2  (negative y-axis)
        assertEquals(-Trig.PI / 2, Trig.atan2(-1, 0), TOL);
    }

    @Test
    void atan2Q1() {
        // (1,1) → π/4 (first quadrant)
        assertEquals(Trig.PI / 4, Trig.atan2(1, 1), TOL);
    }

    @Test
    void atan2Q2() {
        // (1,-1) → 3π/4 (second quadrant)
        assertEquals(3 * Trig.PI / 4, Trig.atan2(1, -1), TOL);
    }

    @Test
    void atan2Q3() {
        // (-1,-1) → -3π/4 (third quadrant)
        assertEquals(-3 * Trig.PI / 4, Trig.atan2(-1, -1), TOL);
    }

    @Test
    void atan2Q4() {
        // (-1,1) → -π/4 (fourth quadrant)
        assertEquals(-Trig.PI / 4, Trig.atan2(-1, 1), TOL);
    }

    @Test
    void atan2Origin() {
        // atan2(0, 0) = 0 by convention
        assertEquals(0.0, Trig.atan2(0, 0), TOL);
    }
}

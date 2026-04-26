// ============================================================================
// TrigTest.kt — Unit Tests for Trig (Kotlin)
// ============================================================================
//
// Tests mirror the Java implementation with idiomatic Kotlin style.
// Sections:
//   1. sin — special values, odd-function symmetry
//   2. cos — special values, even-function symmetry
//   3. Pythagorean identity
//   4. tan — special values, pole guard
//   5. sqrt — Newton's method, domain error
//   6. radians / degrees conversions
//   7. Roundtrip conversions
//   8. Large inputs (stress range reduction)
//   9. atan — special values, round-trip
//  10. atan2 — all four quadrants and axes
// ============================================================================

package com.codingadventures.trig

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.math.abs
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class TrigTest {

    /** Absolute tolerance used for all approximate equality checks. */
    private val TOL = 1e-10

    // =========================================================================
    // 1. sin — special values
    // =========================================================================

    @Test
    fun sinZero() = assertEquals(0.0, Trig.sin(0.0), TOL)

    @Test
    fun sinPiOver2() = assertEquals(1.0, Trig.sin(Trig.PI / 2), TOL)

    @Test
    fun sinPi() = assertEquals(0.0, Trig.sin(Trig.PI), TOL)

    @Test
    fun sin3PiOver2() = assertEquals(-1.0, Trig.sin(3 * Trig.PI / 2), TOL)

    @Test
    fun sin2Pi() = assertEquals(0.0, Trig.sin(2 * Trig.PI), TOL)

    @Test
    fun sinPiOver6() = assertEquals(0.5, Trig.sin(Trig.PI / 6), TOL)

    @Test
    fun sinPiOver4() {
        // sin(π/4) = √2/2 ≈ 0.7071067811865476
        assertEquals(0.7071067811865476, Trig.sin(Trig.PI / 4), TOL)
    }

    @Test
    fun sinPiOver3() {
        // sin(π/3) = √3/2 ≈ 0.8660254037844386
        assertEquals(0.8660254037844386, Trig.sin(Trig.PI / 3), TOL)
    }

    // sin is an ODD function: sin(-x) = -sin(x)
    @Test
    fun sinIsOdd() {
        listOf(0.5, 1.0, 1.5, 2.0, 2.7, Trig.PI / 4, Trig.PI / 3).forEach { a ->
            assertEquals(-Trig.sin(a), Trig.sin(-a), TOL,
                "sin(-$a) should equal -sin($a)")
        }
    }

    // =========================================================================
    // 2. cos — special values
    // =========================================================================

    @Test
    fun cosZero() = assertEquals(1.0, Trig.cos(0.0), TOL)

    @Test
    fun cosPiOver2() = assertEquals(0.0, Trig.cos(Trig.PI / 2), TOL)

    @Test
    fun cosPi() = assertEquals(-1.0, Trig.cos(Trig.PI), TOL)

    @Test
    fun cos3PiOver2() = assertEquals(0.0, Trig.cos(3 * Trig.PI / 2), TOL)

    @Test
    fun cos2Pi() = assertEquals(1.0, Trig.cos(2 * Trig.PI), TOL)

    @Test
    fun cosPiOver6() {
        // cos(π/6) = √3/2 ≈ 0.8660254037844386
        assertEquals(0.8660254037844386, Trig.cos(Trig.PI / 6), TOL)
    }

    @Test
    fun cosPiOver4() {
        // cos(π/4) = √2/2 ≈ 0.7071067811865476
        assertEquals(0.7071067811865476, Trig.cos(Trig.PI / 4), TOL)
    }

    @Test
    fun cosPiOver3() = assertEquals(0.5, Trig.cos(Trig.PI / 3), TOL)

    // cos is an EVEN function: cos(-x) = cos(x)
    @Test
    fun cosIsEven() {
        listOf(0.5, 1.0, 1.5, 2.0, 2.7, Trig.PI / 4, Trig.PI / 3).forEach { a ->
            assertEquals(Trig.cos(a), Trig.cos(-a), TOL,
                "cos(-$a) should equal cos($a)")
        }
    }

    // =========================================================================
    // 3. Pythagorean identity: sin²(x) + cos²(x) = 1
    // =========================================================================

    @Test
    fun pythagoreanIdentity() {
        listOf(
            0.0, Trig.PI / 6, Trig.PI / 4, Trig.PI / 3, Trig.PI / 2,
            Trig.PI, 3 * Trig.PI / 2, 2 * Trig.PI,
            -1.0, -2.5, 0.1, 3.0, 5.5
        ).forEach { x ->
            val s = Trig.sin(x)
            val c = Trig.cos(x)
            assertEquals(1.0, s * s + c * c, TOL,
                "sin²($x) + cos²($x) should equal 1")
        }
    }

    // =========================================================================
    // 4. tan — special values and pole guard
    // =========================================================================

    @Test
    fun tanZero() = assertEquals(0.0, Trig.tan(0.0), TOL)

    @Test
    fun tanPiOver4() = assertEquals(1.0, Trig.tan(Trig.PI / 4), TOL)

    @Test
    fun tanPiOver6() {
        // tan(π/6) = 1/√3
        assertEquals(1.0 / Trig.sqrt(3.0), Trig.tan(Trig.PI / 6), TOL)
    }

    @Test
    fun tanNegativePiOver4() = assertEquals(-1.0, Trig.tan(-Trig.PI / 4), TOL)

    @Test
    fun tanNearPoleIsLargeFinite() {
        val t = Trig.tan(Trig.PI / 2)
        assertTrue(t.isFinite(), "tan(π/2) should be finite")
        assertTrue(abs(t) > 1e100, "tan(π/2) should have very large magnitude")
    }

    // =========================================================================
    // 5. sqrt
    // =========================================================================

    @Test
    fun sqrtZero() = assertEquals(0.0, Trig.sqrt(0.0), TOL)

    @Test
    fun sqrtOne() = assertEquals(1.0, Trig.sqrt(1.0), TOL)

    @Test
    fun sqrtFour() = assertEquals(2.0, Trig.sqrt(4.0), TOL)

    @Test
    fun sqrtNine() = assertEquals(3.0, Trig.sqrt(9.0), TOL)

    @Test
    fun sqrtTwo() = assertEquals(1.41421356237, Trig.sqrt(2.0), TOL)

    @Test
    fun sqrtQuarter() = assertEquals(0.5, Trig.sqrt(0.25), TOL)

    @Test
    fun sqrtLarge() {
        assertEquals(1e5, Trig.sqrt(1e10), 1e5 * 1e-9)
    }

    @Test
    fun sqrtRoundtrip() {
        val s = Trig.sqrt(2.0)
        assertEquals(2.0, s * s, TOL)
    }

    @Test
    fun sqrtNegativeThrows() {
        assertThrows<ArithmeticException> { Trig.sqrt(-1.0) }
    }

    // =========================================================================
    // 6. radians / degrees conversions
    // =========================================================================

    @Test
    fun radians0() = assertEquals(0.0, Trig.radians(0.0), TOL)

    @Test
    fun radians90() = assertEquals(Trig.PI / 2, Trig.radians(90.0), TOL)

    @Test
    fun radians180() = assertEquals(Trig.PI, Trig.radians(180.0), TOL)

    @Test
    fun radians360() = assertEquals(2 * Trig.PI, Trig.radians(360.0), TOL)

    @Test
    fun degrees0() = assertEquals(0.0, Trig.degrees(0.0), TOL)

    @Test
    fun degreesPiOver2() = assertEquals(90.0, Trig.degrees(Trig.PI / 2), TOL)

    @Test
    fun degreesPi() = assertEquals(180.0, Trig.degrees(Trig.PI), TOL)

    // =========================================================================
    // 7. Roundtrip conversions
    // =========================================================================

    @Test
    fun roundtripDegreesToRadians() {
        listOf(0, 30, 45, 60, 90, 120, 180, 270, 360).forEach { d ->
            assertEquals(d.toDouble(), Trig.degrees(Trig.radians(d.toDouble())), TOL,
                "degrees(radians($d)) should be $d")
        }
    }

    @Test
    fun roundtripRadiansToDegrees() {
        listOf(0.0, Trig.PI / 6, Trig.PI / 4, Trig.PI / 3,
               Trig.PI / 2, Trig.PI, 2 * Trig.PI).forEach { r ->
            assertEquals(r, Trig.radians(Trig.degrees(r)), TOL,
                "radians(degrees($r)) should be $r")
        }
    }

    // =========================================================================
    // 8. Large inputs — stress range reduction
    // =========================================================================

    @Test
    fun sin1000Pi() {
        assertEquals(0.0, Trig.sin(1000 * Trig.PI), TOL)
    }

    @Test
    fun cos1000Pi() {
        // 1000 is even → cos(1000π) = cos(0) = 1
        assertEquals(1.0, Trig.cos(1000 * Trig.PI), TOL)
    }

    @Test
    fun pythagoreanLargeInput() {
        val s = Trig.sin(100.0)
        val c = Trig.cos(100.0)
        assertEquals(1.0, s * s + c * c, TOL)
    }

    @Test
    fun sinLargeNegativeIsOdd() {
        assertEquals(-Trig.sin(100.0), Trig.sin(-100.0), TOL)
    }

    // =========================================================================
    // 9. Integration — sin/cos with degree input
    // =========================================================================

    @Test
    fun sin30Degrees() = assertEquals(0.5, Trig.sin(Trig.radians(30.0)), TOL)

    @Test
    fun cos60Degrees() = assertEquals(0.5, Trig.cos(Trig.radians(60.0)), TOL)

    @Test
    fun sin45Degrees() {
        assertEquals(0.7071067811865476, Trig.sin(Trig.radians(45.0)), TOL)
    }

    // =========================================================================
    // 10. atan
    // =========================================================================

    @Test
    fun atanZero() = assertEquals(0.0, Trig.atan(0.0), TOL)

    @Test
    fun atanOne() = assertEquals(Trig.PI / 4, Trig.atan(1.0), TOL)

    @Test
    fun atanMinusOne() = assertEquals(-Trig.PI / 4, Trig.atan(-1.0), TOL)

    @Test
    fun atanSqrt3() {
        // atan(√3) = π/3
        assertEquals(Trig.PI / 3, Trig.atan(Trig.sqrt(3.0)), TOL)
    }

    @Test
    fun atanInvSqrt3() {
        // atan(1/√3) = π/6
        assertEquals(Trig.PI / 6, Trig.atan(1.0 / Trig.sqrt(3.0)), TOL)
    }

    @Test
    fun atanLargeApproachesPiOver2() {
        assertEquals(Trig.PI / 2, Trig.atan(1e10), 1e-5)
    }

    @Test
    fun atanLargeNegativeApproachesMinusPiOver2() {
        assertEquals(-Trig.PI / 2, Trig.atan(-1e10), 1e-5)
    }

    @Test
    fun atanTanRoundtrip() {
        assertEquals(Trig.PI / 4, Trig.atan(Trig.tan(Trig.PI / 4)), TOL)
    }

    // =========================================================================
    // 11. atan2 — quadrant correctness
    // =========================================================================

    @Test
    fun atan2PositiveXAxis() = assertEquals(0.0, Trig.atan2(0.0, 1.0), TOL)

    @Test
    fun atan2PositiveYAxis() = assertEquals(Trig.PI / 2, Trig.atan2(1.0, 0.0), TOL)

    @Test
    fun atan2NegativeXAxis() = assertEquals(Trig.PI, Trig.atan2(0.0, -1.0), TOL)

    @Test
    fun atan2NegativeYAxis() = assertEquals(-Trig.PI / 2, Trig.atan2(-1.0, 0.0), TOL)

    @Test
    fun atan2Q1() = assertEquals(Trig.PI / 4, Trig.atan2(1.0, 1.0), TOL)

    @Test
    fun atan2Q2() = assertEquals(3 * Trig.PI / 4, Trig.atan2(1.0, -1.0), TOL)

    @Test
    fun atan2Q3() = assertEquals(-3 * Trig.PI / 4, Trig.atan2(-1.0, -1.0), TOL)

    @Test
    fun atan2Q4() = assertEquals(-Trig.PI / 4, Trig.atan2(-1.0, 1.0), TOL)

    @Test
    fun atan2Origin() = assertEquals(0.0, Trig.atan2(0.0, 0.0), TOL)
}

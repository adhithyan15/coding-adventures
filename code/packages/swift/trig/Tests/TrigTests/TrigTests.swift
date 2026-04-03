// ============================================================================
// TrigTests.swift — Tests for the Trig package
// ============================================================================
//
// We verify all public functions (sin, cos, tan, atan, atan2, sqrt, radians,
// degrees) against known mathematical identities and special values.
//
// Why approximate equality?
// -------------------------
// Floating-point arithmetic cannot represent most real numbers exactly.
// sin(π) won't be exactly 0 — it'll be something like 1.2e-16. So we need
// a tolerance. We use 1e-10 throughout, which is far tighter than any
// physical measurement but allows for the tiny rounding errors in our
// series computation.
//
// Layout:
//   1. sin / cos landmark values
//   2. Symmetry and Pythagorean identity
//   3. sqrt correctness and roundtrip
//   4. tan function
//   5. atan function
//   6. atan2 four-quadrant correctness
//   7. Angle conversion

import XCTest
@testable import Trig

final class TrigTests: XCTestCase {

    // -------------------------------------------------------------------------
    // Helper
    // -------------------------------------------------------------------------

    /// Tolerance for all approximate comparisons.
    let eps = 1e-10

    /// Returns true if |a - b| < tolerance.
    func approxEqual(_ a: Double, _ b: Double, tol: Double? = nil) -> Bool {
        return abs(a - b) < (tol ?? eps)
    }

    // =========================================================================
    // sin tests
    // =========================================================================

    func testSinZero() {
        XCTAssertEqual(Trig.sin(0.0), 0.0, accuracy: eps)
    }

    func testSinPiOver2() {
        XCTAssertEqual(Trig.sin(PI / 2), 1.0, accuracy: eps)
    }

    func testSinPi() {
        XCTAssertEqual(Trig.sin(PI), 0.0, accuracy: eps)
    }

    func testSin3PiOver2() {
        XCTAssertEqual(Trig.sin(3 * PI / 2), -1.0, accuracy: eps)
    }

    func testSin2Pi() {
        XCTAssertEqual(Trig.sin(2 * PI), 0.0, accuracy: eps)
    }

    func testSinPiOver6() {
        // sin(30°) = 0.5 exactly
        XCTAssertEqual(Trig.sin(PI / 6), 0.5, accuracy: eps)
    }

    func testSinNegative() {
        // sin is an odd function: sin(-x) = -sin(x)
        XCTAssertEqual(Trig.sin(-PI / 4), -Trig.sin(PI / 4), accuracy: eps)
    }

    // =========================================================================
    // cos tests
    // =========================================================================

    func testCosZero() {
        XCTAssertEqual(Trig.cos(0.0), 1.0, accuracy: eps)
    }

    func testCosPiOver2() {
        XCTAssertEqual(Trig.cos(PI / 2), 0.0, accuracy: eps)
    }

    func testCosPi() {
        XCTAssertEqual(Trig.cos(PI), -1.0, accuracy: eps)
    }

    func testCos2Pi() {
        XCTAssertEqual(Trig.cos(2 * PI), 1.0, accuracy: eps)
    }

    func testCosEven() {
        // cos is an even function: cos(-x) = cos(x)
        let x = 1.23
        XCTAssertEqual(Trig.cos(-x), Trig.cos(x), accuracy: eps)
    }

    // Pythagorean identity: sin²(x) + cos²(x) = 1 for all x
    func testPythagoreanIdentity() {
        let angles = [0.0, 0.5, 1.0, PI / 4, PI / 3, PI / 2, PI, 2.5, -1.7]
        for x in angles {
            let s = Trig.sin(x)
            let c = Trig.cos(x)
            XCTAssertEqual(s * s + c * c, 1.0, accuracy: eps,
                           "Pythagorean identity failed at x = \(x)")
        }
    }

    // =========================================================================
    // sqrt tests
    // =========================================================================

    func testSqrtZero() {
        XCTAssertEqual(Trig.sqrt(0.0), 0.0)
    }

    func testSqrtOne() {
        XCTAssertEqual(Trig.sqrt(1.0), 1.0, accuracy: eps)
    }

    func testSqrtFour() {
        XCTAssertEqual(Trig.sqrt(4.0), 2.0, accuracy: eps)
    }

    func testSqrtNine() {
        XCTAssertEqual(Trig.sqrt(9.0), 3.0, accuracy: eps)
    }

    func testSqrtTwo() {
        // sqrt(2) ≈ 1.41421356237...
        XCTAssertEqual(Trig.sqrt(2.0), 1.41421356237, accuracy: eps)
    }

    func testSqrtQuarter() {
        XCTAssertEqual(Trig.sqrt(0.25), 0.5, accuracy: eps)
    }

    func testSqrtLarge() {
        // sqrt(1e10) = 1e5
        XCTAssertEqual(Trig.sqrt(1e10), 1e5, accuracy: 1e-4)
    }

    func testSqrtRoundtrip() {
        // sqrt(2) * sqrt(2) should recover 2.0
        let s = Trig.sqrt(2.0)
        XCTAssertEqual(s * s, 2.0, accuracy: eps)
    }

    // =========================================================================
    // tan tests
    // =========================================================================

    func testTanZero() {
        XCTAssertEqual(Trig.tan(0.0), 0.0, accuracy: eps)
    }

    func testTanPiOver4() {
        // tan(45°) = 1 exactly
        XCTAssertEqual(Trig.tan(PI / 4), 1.0, accuracy: eps)
    }

    func testTanPiOver6() {
        // tan(30°) = 1/sqrt(3)
        let expected = 1.0 / Trig.sqrt(3.0)
        XCTAssertEqual(Trig.tan(PI / 6), expected, accuracy: eps)
    }

    func testTanNegativePiOver4() {
        XCTAssertEqual(Trig.tan(-PI / 4), -1.0, accuracy: eps)
    }

    // =========================================================================
    // atan tests
    // =========================================================================

    func testAtanZero() {
        XCTAssertEqual(Trig.atan(0.0), 0.0)
    }

    func testAtanOne() {
        // atan(1) = π/4 (45°)
        XCTAssertEqual(Trig.atan(1.0), PI / 4, accuracy: eps)
    }

    func testAtanMinusOne() {
        XCTAssertEqual(Trig.atan(-1.0), -PI / 4, accuracy: eps)
    }

    func testAtanSqrt3() {
        // atan(√3) = π/3 (60°)
        XCTAssertEqual(Trig.atan(Trig.sqrt(3.0)), PI / 3, accuracy: eps)
    }

    func testAtanInvSqrt3() {
        // atan(1/√3) = π/6 (30°)
        XCTAssertEqual(Trig.atan(1.0 / Trig.sqrt(3.0)), PI / 6, accuracy: eps)
    }

    func testAtanLargePositive() {
        // atan of a very large number → π/2
        XCTAssertEqual(Trig.atan(1e10), PI / 2, accuracy: 1e-5)
    }

    func testAtanLargeNegative() {
        XCTAssertEqual(Trig.atan(-1e10), -PI / 2, accuracy: 1e-5)
    }

    func testAtanTanRoundtrip() {
        // atan(tan(π/4)) = π/4
        XCTAssertEqual(Trig.atan(Trig.tan(PI / 4)), PI / 4, accuracy: eps)
    }

    // =========================================================================
    // atan2 tests
    // =========================================================================

    func testAtan2PositiveXAxis() {
        // atan2(0, 1) = 0  (pointing right)
        XCTAssertEqual(Trig.atan2(0.0, 1.0), 0.0, accuracy: eps)
    }

    func testAtan2PositiveYAxis() {
        // atan2(1, 0) = π/2  (pointing up)
        XCTAssertEqual(Trig.atan2(1.0, 0.0), PI / 2, accuracy: eps)
    }

    func testAtan2NegativeXAxis() {
        // atan2(0, -1) = π  (pointing left)
        XCTAssertEqual(Trig.atan2(0.0, -1.0), PI, accuracy: eps)
    }

    func testAtan2NegativeYAxis() {
        // atan2(-1, 0) = -π/2  (pointing down)
        XCTAssertEqual(Trig.atan2(-1.0, 0.0), -PI / 2, accuracy: eps)
    }

    func testAtan2Q1() {
        // atan2(1, 1) = π/4  (northeast, Q1)
        XCTAssertEqual(Trig.atan2(1.0, 1.0), PI / 4, accuracy: eps)
    }

    func testAtan2Q2() {
        // atan2(1, -1) = 3π/4  (northwest, Q2)
        XCTAssertEqual(Trig.atan2(1.0, -1.0), 3 * PI / 4, accuracy: eps)
    }

    func testAtan2Q3() {
        // atan2(-1, -1) = -3π/4  (southwest, Q3)
        XCTAssertEqual(Trig.atan2(-1.0, -1.0), -3 * PI / 4, accuracy: eps)
    }

    func testAtan2Q4() {
        // atan2(-1, 1) = -π/4  (southeast, Q4)
        XCTAssertEqual(Trig.atan2(-1.0, 1.0), -PI / 4, accuracy: eps)
    }

    // =========================================================================
    // Angle conversion tests
    // =========================================================================

    func testRadiansZero() {
        XCTAssertEqual(Trig.radians(0.0), 0.0, accuracy: eps)
    }

    func testRadians180() {
        XCTAssertEqual(Trig.radians(180.0), PI, accuracy: eps)
    }

    func testRadians90() {
        XCTAssertEqual(Trig.radians(90.0), PI / 2, accuracy: eps)
    }

    func testDegreesPI() {
        XCTAssertEqual(Trig.degrees(PI), 180.0, accuracy: eps)
    }

    func testDegreesHalfPI() {
        XCTAssertEqual(Trig.degrees(PI / 2), 90.0, accuracy: eps)
    }

    func testDegreesRadiansRoundtrip() {
        for deg in [0.0, 30.0, 45.0, 60.0, 90.0, 120.0, 180.0, 360.0, -45.0] {
            XCTAssertEqual(Trig.degrees(Trig.radians(deg)), deg, accuracy: eps,
                           "Round-trip failed for \(deg)°")
        }
    }
}

import XCTest
@testable import Polynomial

// ============================================================================
// PolynomialTests — Comprehensive tests for the Polynomial library
// ============================================================================
//
// These tests cover:
//   1.  normalize — stripping trailing near-zero coefficients
//   2.  degree — index of highest non-zero coefficient
//   3.  zero and one — identity elements
//   4.  add — term-by-term addition
//   5.  subtract — term-by-term subtraction
//   6.  multiply — polynomial convolution
//   7.  divmod — polynomial long division
//   8.  divide and mod — quotient and remainder helpers
//   9.  evaluate — Horner's method
//  10.  gcd — Euclidean GCD
//  11.  edge cases — zero polynomial, single terms, large degrees
//
// ============================================================================

// MARK: - Normalize Tests

final class NormalizeTests: XCTestCase {

    /// Already-normalized polynomial should be returned unchanged.
    func testAlreadyNormalized() {
        XCTAssertEqual(Polynomial.normalize([1.0, 2.0, 3.0]), [1.0, 2.0, 3.0])
    }

    /// Single trailing zero should be stripped.
    func testOneTrailingZero() {
        XCTAssertEqual(Polynomial.normalize([1.0, 2.0, 0.0]), [1.0, 2.0])
    }

    /// Multiple trailing zeros should all be stripped.
    func testMultipleTrailingZeros() {
        XCTAssertEqual(Polynomial.normalize([1.0, 0.0, 0.0, 0.0]), [1.0])
    }

    /// All-zero input normalizes to [0.0].
    func testAllZeros() {
        XCTAssertEqual(Polynomial.normalize([0.0, 0.0, 0.0]), [0.0])
    }

    /// Single-zero input stays [0.0].
    func testSingleZero() {
        XCTAssertEqual(Polynomial.normalize([0.0]), [0.0])
    }

    /// Empty input should return [0.0] (the zero polynomial).
    func testEmpty() {
        XCTAssertEqual(Polynomial.normalize([]), [0.0])
    }

    /// Near-zero values below the threshold are treated as zero.
    func testNearZeroThreshold() {
        // 1e-11 is below the 1e-10 threshold → treated as zero.
        let result = Polynomial.normalize([1.0, 1e-11])
        XCTAssertEqual(result, [1.0])
    }

    /// Values at or above the threshold are kept.
    func testAboveThresholdKept() {
        // 2e-10 is above the threshold → kept.
        let result = Polynomial.normalize([1.0, 2e-10])
        XCTAssertEqual(result.count, 2)
    }
}


// MARK: - Degree Tests

final class DegreeTests: XCTestCase {

    /// Zero polynomial has degree 0 (constant zero).
    func testDegreeOfZero() {
        XCTAssertEqual(Polynomial.degree([0.0]), 0)
    }

    /// Constant polynomial has degree 0.
    func testDegreeConstant() {
        XCTAssertEqual(Polynomial.degree([5.0]), 0)
    }

    /// Linear polynomial has degree 1.
    func testDegreeLinear() {
        XCTAssertEqual(Polynomial.degree([1.0, 2.0]), 1)
    }

    /// Quadratic polynomial has degree 2.
    func testDegreeQuadratic() {
        XCTAssertEqual(Polynomial.degree([1.0, 2.0, 3.0]), 2)
    }

    /// Trailing zeros don't affect the degree.
    func testDegreeWithTrailingZeros() {
        XCTAssertEqual(Polynomial.degree([1.0, 2.0, 0.0, 0.0]), 1)
    }
}


// MARK: - Zero and One Tests

final class IdentityTests: XCTestCase {

    /// zero() should return [0.0].
    func testZero() {
        XCTAssertEqual(Polynomial.zero(), [0.0])
    }

    /// one() should return [1.0].
    func testOne() {
        XCTAssertEqual(Polynomial.one(), [1.0])
    }

    /// Adding zero to a polynomial should return the original.
    func testAddZeroIdentity() {
        let p = [3.0, 2.0, 1.0]
        XCTAssertEqual(Polynomial.add(p, Polynomial.zero()), p)
    }

    /// Multiplying by one should return the original polynomial.
    func testMultiplyOneIdentity() {
        let p = [3.0, 2.0, 1.0]
        XCTAssertEqual(Polynomial.multiply(p, Polynomial.one()), p)
    }
}


// MARK: - Addition Tests

final class AdditionTests: XCTestCase {

    /// Two simple polynomials.
    func testBasicAdd() {
        // (1 + 2x) + (3 + 4x) = 4 + 6x
        XCTAssertEqual(
            Polynomial.add([1.0, 2.0], [3.0, 4.0]),
            [4.0, 6.0]
        )
    }

    /// Different lengths: shorter one is zero-padded.
    func testAddDifferentLengths() {
        // (1 + 2x + 3x²) + (4 + 5x) = 5 + 7x + 3x²
        XCTAssertEqual(
            Polynomial.add([1.0, 2.0, 3.0], [4.0, 5.0]),
            [5.0, 7.0, 3.0]
        )
    }

    /// Adding a polynomial to zero returns the original.
    func testAddToZero() {
        let p = [2.0, 3.0, 4.0]
        XCTAssertEqual(Polynomial.add(p, Polynomial.zero()), p)
    }

    /// Adding a polynomial to itself doubles each coefficient.
    func testAddToItself() {
        let p = [1.0, 2.0, 3.0]
        XCTAssertEqual(Polynomial.add(p, p), [2.0, 4.0, 6.0])
    }

    /// Adding p and -p should give zero.
    func testAddCancelation() {
        let p = [1.0, 2.0, 3.0]
        let negP = [-1.0, -2.0, -3.0]
        XCTAssertEqual(Polynomial.add(p, negP), [0.0])
    }

    /// Commutativity: add(a, b) = add(b, a).
    func testAddCommutativity() {
        let a = [1.0, 2.0, 3.0]
        let b = [4.0, 5.0]
        XCTAssertEqual(Polynomial.add(a, b), Polynomial.add(b, a))
    }
}


// MARK: - Subtraction Tests

final class SubtractionTests: XCTestCase {

    /// Basic subtraction.
    func testBasicSubtract() {
        // (5 + 7x) - (1 + 2x) = 4 + 5x
        XCTAssertEqual(
            Polynomial.subtract([5.0, 7.0], [1.0, 2.0]),
            [4.0, 5.0]
        )
    }

    /// Subtracting a polynomial from itself yields zero.
    func testSubtractSelf() {
        let p = [3.0, 5.0, 2.0]
        XCTAssertEqual(Polynomial.subtract(p, p), [0.0])
    }

    /// Subtracting zero returns the original.
    func testSubtractZero() {
        let p = [2.0, 3.0]
        XCTAssertEqual(Polynomial.subtract(p, Polynomial.zero()), p)
    }

    /// Subtracting from zero negates the polynomial.
    func testSubtractFromZero() {
        let p = [1.0, 2.0, 3.0]
        XCTAssertEqual(Polynomial.subtract(Polynomial.zero(), p), [-1.0, -2.0, -3.0])
    }

    /// Result should be normalized (trailing zeros stripped).
    func testSubtractNormalized() {
        // (5 + 7x + 3x²) - (1 + 2x + 3x²) = 4 + 5x
        let result = Polynomial.subtract([5.0, 7.0, 3.0], [1.0, 2.0, 3.0])
        XCTAssertEqual(result, [4.0, 5.0])
    }
}


// MARK: - Multiplication Tests

final class MultiplicationTests: XCTestCase {

    /// Basic multiplication: (1 + 2x)(3 + 4x) = 3 + 10x + 8x².
    func testBasicMultiply() {
        XCTAssertEqual(
            Polynomial.multiply([1.0, 2.0], [3.0, 4.0]),
            [3.0, 10.0, 8.0]
        )
    }

    /// Multiply by zero yields zero.
    func testMultiplyByZero() {
        XCTAssertEqual(Polynomial.multiply([1.0, 2.0, 3.0], Polynomial.zero()), [0.0])
    }

    /// Multiply by one returns the original polynomial.
    func testMultiplyByOne() {
        let p = [2.0, 3.0, 4.0]
        XCTAssertEqual(Polynomial.multiply(p, Polynomial.one()), p)
    }

    /// Commutativity: multiply(a, b) = multiply(b, a).
    func testMultiplyCommutativity() {
        let a = [1.0, 2.0, 3.0]
        let b = [4.0, 5.0]
        XCTAssertEqual(Polynomial.multiply(a, b), Polynomial.multiply(b, a))
    }

    /// Degree of product equals sum of degrees.
    func testMultiplyDegree() {
        let a = [1.0, 2.0, 3.0]  // degree 2
        let b = [4.0, 5.0, 6.0]  // degree 2
        let result = Polynomial.multiply(a, b)
        XCTAssertEqual(Polynomial.degree(result), 4)  // degree 4
    }

    /// Multiplying constant by a polynomial scales all coefficients.
    func testMultiplyByConstant() {
        let p = [1.0, 2.0, 3.0]
        let c = [2.0]  // constant 2
        XCTAssertEqual(Polynomial.multiply(p, c), [2.0, 4.0, 6.0])
    }

    /// (x - 1)(x + 1) = x² - 1.
    func testMultiplyFactoredForm() {
        let a = [-1.0, 1.0]  // x - 1
        let b = [1.0, 1.0]   // x + 1
        XCTAssertEqual(Polynomial.multiply(a, b), [-1.0, 0.0, 1.0])
    }
}


// MARK: - Division Tests

final class DivisionTests: XCTestCase {

    /// Basic division: (x² - 1) / (x - 1) = x + 1.
    func testBasicDivide() {
        // x² - 1 = [-1.0, 0.0, 1.0]
        // x - 1  = [-1.0, 1.0]
        // quotient should be x + 1 = [1.0, 1.0], remainder = [0.0]
        let (q, r) = Polynomial.divmod([-1.0, 0.0, 1.0], [-1.0, 1.0])
        XCTAssertEqual(q, [1.0, 1.0])
        XCTAssertEqual(r, [0.0])
    }

    /// When dividend degree < divisor degree, quotient is zero, remainder is dividend.
    func testDivideLowerDegree() {
        let a = [3.0, 2.0]      // 3 + 2x (degree 1)
        let b = [1.0, 0.0, 1.0] // 1 + x² (degree 2)
        let (q, r) = Polynomial.divmod(a, b)
        XCTAssertEqual(q, [0.0])
        XCTAssertEqual(r, [3.0, 2.0])
    }

    /// Verify the fundamental property: dividend = divisor × quotient + remainder.
    func testDivmodFundamentalProperty() {
        let dividend = [5.0, 1.0, 3.0, 2.0]  // 5 + x + 3x² + 2x³
        let divisor  = [2.0, 1.0]            // 2 + x
        let (q, r) = Polynomial.divmod(dividend, divisor)
        // Reconstruct: divisor × quotient + remainder should equal dividend.
        let reconstructed = Polynomial.add(Polynomial.multiply(divisor, q), r)
        // Compare element-by-element with tolerance.
        XCTAssertEqual(reconstructed.count, dividend.count)
        for (a, b) in zip(reconstructed, dividend) {
            XCTAssertEqual(a, b, accuracy: 1e-9)
        }
    }

    /// divide() returns just the quotient.
    func testDivideHelper() {
        let result = Polynomial.divide([-1.0, 0.0, 1.0], [-1.0, 1.0])
        XCTAssertEqual(result, [1.0, 1.0])
    }

    /// mod() returns just the remainder.
    func testModHelper() {
        let result = Polynomial.mod([5.0, 1.0, 3.0, 2.0], [2.0, 1.0])
        // From the worked example: remainder = [-1.0]
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0], -1.0, accuracy: 1e-9)
    }

    /// Dividing by a constant divides all coefficients.
    func testDivideByConstant() {
        let p = [2.0, 4.0, 6.0]  // 2 + 4x + 6x²
        let (q, r) = Polynomial.divmod(p, [2.0])  // divide by constant 2
        XCTAssertEqual(q, [1.0, 2.0, 3.0])
        XCTAssertEqual(r, [0.0])
    }

    /// Dividing a polynomial by itself yields quotient 1, remainder 0.
    func testDivideSelf() {
        let p = [1.0, 2.0, 3.0]
        let (q, r) = Polynomial.divmod(p, p)
        XCTAssertEqual(q.count, 1)
        XCTAssertEqual(q[0], 1.0, accuracy: 1e-9)
        XCTAssertEqual(r, [0.0])
    }
}


// MARK: - Evaluate Tests

final class EvaluateTests: XCTestCase {

    /// Zero polynomial evaluates to 0 at any point.
    func testEvaluateZero() {
        XCTAssertEqual(Polynomial.evaluate(Polynomial.zero(), 5.0), 0.0, accuracy: 1e-10)
        XCTAssertEqual(Polynomial.evaluate(Polynomial.zero(), 0.0), 0.0, accuracy: 1e-10)
    }

    /// Constant polynomial evaluates to the constant.
    func testEvaluateConstant() {
        XCTAssertEqual(Polynomial.evaluate([7.0], 100.0), 7.0, accuracy: 1e-10)
    }

    /// Linear polynomial: 3 + x at x = 5 should be 8.
    func testEvaluateLinear() {
        XCTAssertEqual(Polynomial.evaluate([3.0, 1.0], 5.0), 8.0, accuracy: 1e-10)
    }

    /// Quadratic: 3 + x + 2x² at x = 4 should be 39.
    func testEvaluateQuadratic() {
        // 3 + 4 + 2*16 = 3 + 4 + 32 = 39
        XCTAssertEqual(Polynomial.evaluate([3.0, 1.0, 2.0], 4.0), 39.0, accuracy: 1e-10)
    }

    /// Evaluate at x = 0 always returns the constant term.
    func testEvaluateAtZero() {
        XCTAssertEqual(Polynomial.evaluate([5.0, 3.0, 2.0], 0.0), 5.0, accuracy: 1e-10)
    }

    /// Evaluate at x = 1 returns the sum of all coefficients.
    func testEvaluateAtOne() {
        // 1 + 2 + 3 = 6
        XCTAssertEqual(Polynomial.evaluate([1.0, 2.0, 3.0], 1.0), 6.0, accuracy: 1e-10)
    }

    /// Evaluate at x = -1 alternates signs: a₀ - a₁ + a₂ - ...
    func testEvaluateAtNegativeOne() {
        // 1 - 2 + 3 = 2
        XCTAssertEqual(Polynomial.evaluate([1.0, 2.0, 3.0], -1.0), 2.0, accuracy: 1e-10)
    }

    /// Horner's method should match naive evaluation.
    func testEvaluateMatchesNaive() {
        let poly = [1.0, 0.0, -2.0, 3.0]  // 1 - 2x² + 3x³
        let x = 2.0
        // Naive: 1 + 0*2 + (-2)*4 + 3*8 = 1 + 0 - 8 + 24 = 17
        let naive = 1.0 + 0.0 * x + (-2.0) * x * x + 3.0 * x * x * x
        XCTAssertEqual(Polynomial.evaluate(poly, x), naive, accuracy: 1e-10)
    }
}


// MARK: - GCD Tests

final class GCDTests: XCTestCase {

    /// GCD of a polynomial with itself should be itself (monic).
    func testGCDWithSelf() {
        let p = [2.0, 2.0]  // 2(1 + x), monic form is (1 + x)
        let g = Polynomial.gcd(p, p)
        // Should be [1.0, 1.0] after making monic.
        XCTAssertEqual(g.count, 2)
        XCTAssertEqual(g[0], 1.0, accuracy: 1e-9)
        XCTAssertEqual(g[1], 1.0, accuracy: 1e-9)
    }

    /// GCD of (x² - 1) and (x - 1) should be (x - 1) (monic).
    func testGCDFactoredPolynomials() {
        // x² - 1 = [-1.0, 0.0, 1.0]
        // x - 1  = [-1.0, 1.0]
        let g = Polynomial.gcd([-1.0, 0.0, 1.0], [-1.0, 1.0])
        // GCD should be x - 1 = [-1.0, 1.0]
        XCTAssertEqual(g.count, 2)
        XCTAssertEqual(g[g.count - 1], 1.0, accuracy: 1e-9)  // monic: leading coeff = 1
    }

    /// GCD of coprime polynomials should be 1 (monic constant).
    func testGCDCoprime() {
        // x and (x + 1) are coprime over the reals.
        let g = Polynomial.gcd([0.0, 1.0], [1.0, 1.0])  // x and x+1
        XCTAssertEqual(g.count, 1)
        XCTAssertEqual(g[0], 1.0, accuracy: 1e-9)
    }

    /// GCD(p, zero) = p (monic).
    func testGCDWithZero() {
        let p = [2.0, 4.0]  // 2 + 4x → monic [0.5, 1.0]
        let g = Polynomial.gcd(p, Polynomial.zero())
        // GCD(p, 0) = p, made monic.
        XCTAssertEqual(g[g.count - 1], 1.0, accuracy: 1e-9)
    }

    /// GCD is commutative.
    func testGCDCommutativity() {
        let a = [-1.0, 0.0, 1.0]  // x² - 1
        let b = [-1.0, 1.0]       // x - 1
        let g1 = Polynomial.gcd(a, b)
        let g2 = Polynomial.gcd(b, a)
        XCTAssertEqual(g1.count, g2.count)
        for (x, y) in zip(g1, g2) {
            XCTAssertEqual(x, y, accuracy: 1e-9)
        }
    }
}


// MARK: - Edge Case Tests

final class EdgeCaseTests: XCTestCase {

    /// Large polynomial multiplication preserves degree.
    func testLargePolynomialDegree() {
        // Multiplying two degree-5 polynomials yields degree-10.
        let a = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0]  // 1 + x + x² + x³ + x⁴ + x⁵
        let b = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0]
        let result = Polynomial.multiply(a, b)
        XCTAssertEqual(Polynomial.degree(result), 10)
    }

    /// Add-then-subtract round-trip.
    func testAddSubtractRoundTrip() {
        let a = [1.0, 2.0, 3.0]
        let b = [4.0, 5.0, 6.0]
        let sum = Polynomial.add(a, b)
        let back = Polynomial.subtract(sum, b)
        XCTAssertEqual(back.count, a.count)
        for (x, y) in zip(back, a) {
            XCTAssertEqual(x, y, accuracy: 1e-9)
        }
    }

    /// Multiply-then-divide round-trip (exact integer coefficients).
    func testMultiplyDivideRoundTrip() {
        let a = [1.0, 2.0, 3.0]
        let b = [1.0, 1.0]
        let product = Polynomial.multiply(a, b)
        let (q, r) = Polynomial.divmod(product, b)
        // Quotient should be a, remainder should be zero.
        XCTAssertEqual(q.count, a.count)
        for (x, y) in zip(q, a) {
            XCTAssertEqual(x, y, accuracy: 1e-9)
        }
        XCTAssertEqual(r, [0.0])
    }

    /// Evaluate then check against known values of x² + 1.
    func testEvaluateKnownFunction() {
        let p = [1.0, 0.0, 1.0]  // 1 + x²
        XCTAssertEqual(Polynomial.evaluate(p, 0.0), 1.0, accuracy: 1e-10)
        XCTAssertEqual(Polynomial.evaluate(p, 1.0), 2.0, accuracy: 1e-10)
        XCTAssertEqual(Polynomial.evaluate(p, 2.0), 5.0, accuracy: 1e-10)
        XCTAssertEqual(Polynomial.evaluate(p, -1.0), 2.0, accuracy: 1e-10)
    }

    /// degree of zero polynomial is 0.
    func testDegreeZeroPolynomial() {
        XCTAssertEqual(Polynomial.degree(Polynomial.zero()), 0)
    }

    /// Normalize of one() is one().
    func testNormalizeOne() {
        XCTAssertEqual(Polynomial.normalize(Polynomial.one()), [1.0])
    }
}

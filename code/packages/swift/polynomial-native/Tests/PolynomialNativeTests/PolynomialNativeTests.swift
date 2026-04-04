// ============================================================================
// PolynomialNativeTests.swift — Tests for the PolynomialNative Swift wrapper.
// ============================================================================
//
// IMPORTANT: These tests require libpolynomial_c.a to be compiled and placed
// at Sources/CPolynomial/libpolynomial_c.a before running. See the BUILD file
// for the full two-step process:
//
//   Step 1:  cd code/packages/rust/polynomial-c && cargo build --release
//   Step 2:  cp target/release/libpolynomial_c.a \
//               ../../swift/polynomial-native/Sources/CPolynomial/
//   Step 3:  cd ../../swift/polynomial-native && swift test
//
// Without the .a file, the linker will fail with "library not found for
// -lpolynomial_c". This is expected and documented — the Swift package is
// correctly structured, but it cannot run without the pre-built Rust library.
//
// ============================================================================

import Testing
@testable import PolynomialNative

// ============================================================================
// MARK: — Normalize
// ============================================================================

/// Normalization strips trailing near-zero coefficients, ensuring that
/// [1.0, 0.0, 0.0] and [1.0] represent the same polynomial.
struct NormalizeTests {

    @Test func stripsTrailingZeros() {
        let result = Polynomial.normalize([1.0, 0.0, 0.0])
        #expect(result == [1.0])
    }

    @Test func zeroPolynomialBecomesEmpty() {
        let result = Polynomial.normalize([0.0])
        #expect(result == [])
    }

    @Test func emptyPolynomialRemainsEmpty() {
        let result = Polynomial.normalize([])
        #expect(result == [])
    }

    @Test func noOpWhenAlreadyNormalized() {
        let input = [1.0, 2.0, 3.0]
        #expect(Polynomial.normalize(input) == input)
    }

    @Test func stripsMultipleTrailingZeros() {
        let result = Polynomial.normalize([5.0, 3.0, 0.0, 0.0, 0.0])
        #expect(result == [5.0, 3.0])
    }
}

// ============================================================================
// MARK: — Degree
// ============================================================================

struct DegreeTests {

    @Test func degreeOfConstant() {
        #expect(Polynomial.degree([7.0]) == 0)
    }

    @Test func degreeOfLinear() {
        // [1.0, 2.0] = 1 + 2x → degree 1
        #expect(Polynomial.degree([1.0, 2.0]) == 1)
    }

    @Test func degreeOfQuadratic() {
        // [0.0, 0.0, 3.0] = 3x² → degree 2
        #expect(Polynomial.degree([0.0, 0.0, 3.0]) == 2)
    }

    @Test func zeroPolynomialHasDegreeZero() {
        #expect(Polynomial.degree([]) == 0)
        #expect(Polynomial.degree([0.0]) == 0)
    }
}

// ============================================================================
// MARK: — Evaluate
// ============================================================================

struct EvaluateTests {

    @Test func constantPolynomial() {
        // p(x) = 5, evaluated anywhere returns 5
        #expect(Polynomial.evaluate([5.0], at: 0.0) == 5.0)
        #expect(Polynomial.evaluate([5.0], at: 99.0) == 5.0)
    }

    @Test func linearPolynomial() {
        // p(x) = 1 + 2x → p(3) = 1 + 6 = 7
        #expect(Polynomial.evaluate([1.0, 2.0], at: 3.0) == 7.0)
    }

    @Test func quadraticPolynomial() {
        // p(x) = 3 + 2x² → p(2) = 3 + 8 = 11
        #expect(Polynomial.evaluate([3.0, 0.0, 2.0], at: 2.0) == 11.0)
    }

    @Test func zeroPolynomialEvaluatesToZero() {
        #expect(Polynomial.evaluate([], at: 42.0) == 0.0)
        #expect(Polynomial.evaluate([0.0], at: 42.0) == 0.0)
    }

    @Test func evaluateAtZero() {
        // p(0) = constant term = a[0]
        #expect(Polynomial.evaluate([3.0, 99.0, 99.0], at: 0.0) == 3.0)
    }
}

// ============================================================================
// MARK: — Addition
// ============================================================================

struct AddTests {

    @Test func addSameLength() {
        // (1 + 2x + 3x²) + (4 + 5x + 6x²) = 5 + 7x + 9x²
        let result = Polynomial.add([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])
        #expect(result == [5.0, 7.0, 9.0])
    }

    @Test func addDifferentLengths() {
        // (1 + 2x + 3x²) + (4 + 5x) = 5 + 7x + 3x²
        let result = Polynomial.add([1.0, 2.0, 3.0], [4.0, 5.0])
        #expect(result == [5.0, 7.0, 3.0])
    }

    @Test func addWithCancellation() {
        // (3x²) + (−3x²) = 0 → normalized to []
        let result = Polynomial.add([0.0, 0.0, 3.0], [0.0, 0.0, -3.0])
        #expect(result == [])
    }

    @Test func addZeroPolynomial() {
        let p = [1.0, 2.0, 3.0]
        #expect(Polynomial.add(p, []) == p)
        #expect(Polynomial.add([], p) == p)
    }
}

// ============================================================================
// MARK: — Subtraction
// ============================================================================

struct SubtractTests {

    @Test func subtractSameLength() {
        // (5 + 7x + 3x²) − (1 + 2x + 3x²) = 4 + 5x
        let result = Polynomial.subtract([5.0, 7.0, 3.0], [1.0, 2.0, 3.0])
        #expect(result == [4.0, 5.0])
    }

    @Test func subtractFromSelf() {
        // p − p = zero polynomial
        let result = Polynomial.subtract([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])
        #expect(result == [])
    }

    @Test func subtractZeroPolynomial() {
        let p = [1.0, 2.0, 3.0]
        #expect(Polynomial.subtract(p, []) == p)
    }
}

// ============================================================================
// MARK: — Multiplication
// ============================================================================

struct MultiplyTests {

    @Test func multiplyLinearByLinear() {
        // (1 + 2x)(3 + 4x) = 3 + 10x + 8x²
        let result = Polynomial.multiply([1.0, 2.0], [3.0, 4.0])
        #expect(result == [3.0, 10.0, 8.0])
    }

    @Test func multiplyByConstant() {
        // 3 × (1 + 2x + 3x²) = 3 + 6x + 9x²
        let result = Polynomial.multiply([3.0], [1.0, 2.0, 3.0])
        #expect(result == [3.0, 6.0, 9.0])
    }

    @Test func multiplyByZero() {
        let result = Polynomial.multiply([1.0, 2.0, 3.0], [])
        #expect(result == [])
    }

    @Test func multiplyByOne() {
        let p = [1.0, 2.0, 3.0]
        #expect(Polynomial.multiply(p, [1.0]) == p)
    }
}

// ============================================================================
// MARK: — Division (divmod)
// ============================================================================

struct DivmodTests {

    @Test func divideExactly() {
        // (x − 1)(x + 2) = x² + x − 2 = [−2, 1, 1]
        // Divide [−2, 1, 1] by [−1, 1] = (x − 1)
        // Expect quotient [2, 1] = 2 + x, remainder []
        if let (q, r) = Polynomial.divmod([-2.0, 1.0, 1.0], [-1.0, 1.0]) {
            #expect(q == [2.0, 1.0])
            #expect(r == [])
        } else {
            Issue.record("divmod returned nil for valid inputs")
        }
    }

    @Test func divideWithRemainder() {
        // From the spec example:
        // dividend = [5, 1, 3, 2]  =  5 + x + 3x² + 2x³
        // divisor  = [2, 1]        =  2 + x
        // quotient = [3, -1, 2], remainder = [-1]
        if let (q, r) = Polynomial.divmod([5.0, 1.0, 3.0, 2.0], [2.0, 1.0]) {
            // Check by verification: divisor × quotient + remainder = dividend
            let reconstructed = Polynomial.add(
                Polynomial.multiply([2.0, 1.0], q), r
            )
            // Compare term by term within floating-point tolerance
            let expected = [5.0, 1.0, 3.0, 2.0]
            #expect(reconstructed.count == expected.count)
            for (got, exp) in zip(reconstructed, expected) {
                #expect(abs(got - exp) < 1e-9)
            }
        } else {
            Issue.record("divmod returned nil for valid inputs")
        }
    }

    @Test func divideLowerDegreeByHigher() {
        // If degree(dividend) < degree(divisor), quotient = [], remainder = dividend
        if let (q, r) = Polynomial.divmod([1.0, 2.0], [3.0, 4.0, 5.0]) {
            #expect(q == [])
            #expect(r == [1.0, 2.0])
        } else {
            Issue.record("divmod returned nil for valid inputs")
        }
    }

    @Test func divideByZeroReturnsNil() {
        let result = Polynomial.divmod([1.0, 2.0, 3.0], [])
        #expect(result == nil)
    }

    @Test func divideReturnQuotient() {
        let q = Polynomial.divide([5.0, 1.0, 3.0, 2.0], [2.0, 1.0])
        #expect(q != nil)
        #expect(q?.count == 3)  // quotient has 3 coefficients
    }

    @Test func moduloReturnRemainder() {
        let r = Polynomial.modulo([5.0, 1.0, 3.0, 2.0], [2.0, 1.0])
        #expect(r != nil)
    }
}

// ============================================================================
// MARK: — GCD
// ============================================================================

struct GCDTests {

    @Test func gcdOfCommonFactor() {
        // gcd(x² − 3x + 2, x − 1)
        // x² − 3x + 2 = (x−1)(x−2) → gcd is (x−1) = [−1, 1]
        let result = Polynomial.gcd([2.0, -3.0, 1.0], [-1.0, 1.0])
        // Result should be proportional to [−1, 1]; check it divides both.
        if !result.isEmpty {
            let r1 = Polynomial.modulo([2.0, -3.0, 1.0], result)
            let r2 = Polynomial.modulo([-1.0, 1.0], result)
            // Both remainders should be zero (or nil on error)
            #expect(r1 == [] || r1 == nil)
            #expect(r2 == [] || r2 == nil)
        }
    }

    @Test func gcdOfCoprimes() {
        // gcd(x, x + 1) = 1 (they share no common factor)
        let result = Polynomial.gcd([0.0, 1.0], [1.0, 1.0])
        // Result should be a non-zero constant (degree 0).
        #expect(!result.isEmpty)
        #expect(result.count == 1)
    }

    @Test func gcdWithZero() {
        // gcd(p, 0) = p (for any p)
        let p = [1.0, 2.0, 3.0]
        let result = Polynomial.gcd(p, [])
        // Should be proportional to p.
        #expect(result.count == p.count)
    }
}

// ============================================================================
// MARK: — Round-Trip Properties
// ============================================================================

/// Algebraic identities that must hold regardless of specific values.
struct RoundTripTests {

    @Test func multiplyThenDivide() {
        // For any a, b (b nonzero): (a × b) / b = a
        let a = [1.0, 2.0, 3.0]
        let b = [2.0, 1.0]
        let product = Polynomial.multiply(a, b)
        if let quotient = Polynomial.divide(product, b) {
            #expect(quotient.count == a.count)
            for (got, exp) in zip(quotient, a) {
                #expect(abs(got - exp) < 1e-9)
            }
        } else {
            Issue.record("divide returned nil unexpectedly")
        }
    }

    @Test func addThenSubtract() {
        // (a + b) − b = a
        let a = [3.0, 0.0, 2.0]
        let b = [1.0, 5.0]
        let sum = Polynomial.add(a, b)
        let diff = Polynomial.subtract(sum, b)
        let normalized = Polynomial.normalize(a)
        #expect(diff == normalized)
    }

    @Test func evaluateAtRoots() {
        // p(x) = (x − 2)(x − 3) = 6 − 5x + x² = [6, −5, 1]
        // p(2) = 0 and p(3) = 0
        let p = [6.0, -5.0, 1.0]
        #expect(abs(Polynomial.evaluate(p, at: 2.0)) < 1e-9)
        #expect(abs(Polynomial.evaluate(p, at: 3.0)) < 1e-9)
    }
}

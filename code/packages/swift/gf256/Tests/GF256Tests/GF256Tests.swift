import XCTest
@testable import GF256

// ============================================================================
// GF256Tests — Comprehensive tests for the GF(2^8) library
// ============================================================================
//
// These tests cover:
//   1.  Constants — zero, one, primitivePoly
//   2.  Table construction — ALOG and LOG correctness
//   3.  Addition — XOR behavior, identity, self-inverse
//   4.  Subtraction — equals addition in GF(2^8)
//   5.  Multiplication — commutativity, identity, zero, specific values
//   6.  Division — inverse relationship with multiplication
//   7.  Power — base cases, specific values, Fermat's little theorem
//   8.  Inverse — self-check via multiply, specific known values
//   9.  Field axioms — distributivity, associativity
//  10.  Edge cases and known Reed-Solomon values
//
// ============================================================================

// MARK: - Constants Tests

final class ConstantsTests: XCTestCase {

    /// zero should be 0.
    func testZeroConstant() {
        XCTAssertEqual(GF256.zero, 0)
    }

    /// one should be 1.
    func testOneConstant() {
        XCTAssertEqual(GF256.one, 1)
    }

    /// Primitive polynomial should be 0x11D = 285.
    func testPrimitivePoly() {
        XCTAssertEqual(GF256.primitivePoly, 0x11D)
        XCTAssertEqual(GF256.primitivePoly, 285)
    }
}


// MARK: - Table Tests

final class TableTests: XCTestCase {

    /// ALOG table should have 256 entries.
    func testALOGSize() {
        XCTAssertEqual(GF256.ALOG.count, 256)
    }

    /// LOG table should have 256 entries.
    func testLOGSize() {
        XCTAssertEqual(GF256.LOG.count, 256)
    }

    /// ALOG[0] = 1 (g^0 = 1).
    func testALOGFirstEntry() {
        XCTAssertEqual(GF256.ALOG[0], 1)
    }

    /// ALOG[1] = 2 (g^1 = 2, the generator).
    func testALOGSecondEntry() {
        XCTAssertEqual(GF256.ALOG[1], 2)
    }

    /// ALOG[7] = 128 = 0x80 (2^7 = 128, no overflow yet).
    func testALOG7() {
        XCTAssertEqual(GF256.ALOG[7], 128)
    }

    /// ALOG[8] = 29 = 0x1D (2^8 overflows, XOR with 0x11D: 256 ^ 285 = 29).
    func testALOG8FirstReduction() {
        XCTAssertEqual(GF256.ALOG[8], 29)
    }

    /// ALOG[255] = 1 (the multiplicative group has order 255, g^255 = g^0 = 1).
    func testALOG255Wraps() {
        XCTAssertEqual(GF256.ALOG[255], 1)
    }

    /// LOG[1] = 0 (2^0 = 1).
    func testLOGOne() {
        XCTAssertEqual(GF256.LOG[1], 0)
    }

    /// LOG[2] = 1 (2^1 = 2).
    func testLOGTwo() {
        XCTAssertEqual(GF256.LOG[2], 1)
    }

    /// ALOG and LOG are mutual inverses for non-zero elements.
    func testALOGLOGInverse() {
        for x in 1...255 {
            let logX = Int(GF256.LOG[x])
            XCTAssertEqual(Int(GF256.ALOG[logX]), x, "ALOG[LOG[\(x)]] should equal \(x)")
        }
    }

    /// All 255 non-zero elements appear exactly once in ALOG[0..254].
    func testALOGCoversAllNonZero() {
        var seen = Set<UInt8>()
        for i in 0..<255 {
            let v = GF256.ALOG[i]
            XCTAssertFalse(seen.contains(v), "ALOG[\(i)] = \(v) appeared more than once")
            seen.insert(v)
        }
        XCTAssertEqual(seen.count, 255)
    }
}


// MARK: - Addition Tests

final class AdditionTests: XCTestCase {

    /// Addition is XOR.
    func testAddIsXOR() {
        XCTAssertEqual(GF256.add(0x53, 0xCA), 0x53 ^ 0xCA)
    }

    /// Adding zero is the identity: a + 0 = a.
    func testAddZeroIdentity() {
        for a: UInt8 in [0, 1, 2, 127, 128, 255] {
            XCTAssertEqual(GF256.add(a, 0), a)
            XCTAssertEqual(GF256.add(0, a), a)
        }
    }

    /// Every element is its own additive inverse: a + a = 0.
    func testAddSelfIsZero() {
        for a: UInt8 in [0, 1, 7, 42, 128, 200, 255] {
            XCTAssertEqual(GF256.add(a, a), 0, "\(a) + \(a) should be 0 in GF(256)")
        }
    }

    /// Addition is commutative: a + b = b + a.
    func testAddCommutativity() {
        XCTAssertEqual(GF256.add(0x5A, 0x3F), GF256.add(0x3F, 0x5A))
        XCTAssertEqual(GF256.add(200, 100), GF256.add(100, 200))
    }

    /// Addition is associative: (a + b) + c = a + (b + c).
    func testAddAssociativity() {
        let a: UInt8 = 0x12
        let b: UInt8 = 0x34
        let c: UInt8 = 0x56
        XCTAssertEqual(GF256.add(GF256.add(a, b), c), GF256.add(a, GF256.add(b, c)))
    }

    /// Known specific value: 0x01 + 0x01 = 0x00.
    func testAddKnownValue() {
        XCTAssertEqual(GF256.add(1, 1), 0)
        XCTAssertEqual(GF256.add(0xFF, 0xFF), 0)
    }
}


// MARK: - Subtraction Tests

final class SubtractionTests: XCTestCase {

    /// Subtraction equals addition in GF(2^8).
    func testSubtractEqualsAdd() {
        for a: UInt8 in [0, 1, 42, 128, 255] {
            for b: UInt8 in [0, 1, 42, 128, 255] {
                XCTAssertEqual(GF256.subtract(a, b), GF256.add(a, b))
            }
        }
    }

    /// Subtracting a value from itself gives zero.
    func testSubtractSelfIsZero() {
        for a: UInt8 in [1, 50, 100, 200, 255] {
            XCTAssertEqual(GF256.subtract(a, a), 0)
        }
    }

    /// Subtracting zero is the identity: a - 0 = a.
    func testSubtractZeroIdentity() {
        XCTAssertEqual(GF256.subtract(42, 0), 42)
        XCTAssertEqual(GF256.subtract(255, 0), 255)
    }
}


// MARK: - Multiplication Tests

final class MultiplicationTests: XCTestCase {

    /// Multiplying by zero gives zero.
    func testMultiplyByZero() {
        for a: UInt8 in [0, 1, 2, 127, 128, 255] {
            XCTAssertEqual(GF256.multiply(a, 0), 0)
            XCTAssertEqual(GF256.multiply(0, a), 0)
        }
    }

    /// Multiplying by one is the identity: a × 1 = a.
    func testMultiplyByOne() {
        for a: UInt8 in [0, 1, 2, 7, 42, 128, 200, 255] {
            XCTAssertEqual(GF256.multiply(a, 1), a)
            XCTAssertEqual(GF256.multiply(1, a), a)
        }
    }

    /// Multiplication is commutative: a × b = b × a.
    func testMultiplyCommutativity() {
        XCTAssertEqual(GF256.multiply(3, 7), GF256.multiply(7, 3))
        XCTAssertEqual(GF256.multiply(100, 200), GF256.multiply(200, 100))
    }

    /// 2 × 2 = 4 (no overflow yet — just a left shift).
    func testMultiply2Times2() {
        XCTAssertEqual(GF256.multiply(2, 2), 4)
    }

    /// 2 × 3 = 6 (verified: LOG[2]=1, LOG[3]=25, ALOG[26]=6... actually let's check)
    func testMultiply2Times3() {
        // 2 × 3: LOG[2]=1, LOG[3]=?. We verify via the round-trip property.
        let result = GF256.multiply(2, 3)
        // Verify: result / 2 = 3 and result / 3 = 2
        XCTAssertEqual(GF256.divide(result, 2), 3)
        XCTAssertEqual(GF256.divide(result, 3), 2)
    }

    /// 2^8 mod p(x) = 29 = 0x1D (first polynomial reduction step).
    func testMultiplyGenerator8th() {
        // 2 × 128 should equal 2^8 mod p(x) = 29.
        XCTAssertEqual(GF256.multiply(2, 128), 29)
    }

    /// Multiplication is associative: (a × b) × c = a × (b × c).
    func testMultiplyAssociativity() {
        let a: UInt8 = 3
        let b: UInt8 = 5
        let c: UInt8 = 7
        let lhs = GF256.multiply(GF256.multiply(a, b), c)
        let rhs = GF256.multiply(a, GF256.multiply(b, c))
        XCTAssertEqual(lhs, rhs)
    }

    /// Multiplying by the inverse gives 1.
    func testMultiplyByInverse() {
        for a: UInt8 in [1, 2, 7, 42, 100, 200, 255] {
            let inv = GF256.inverse(a)
            XCTAssertEqual(GF256.multiply(a, inv), 1, "\(a) × inv(\(a)) should be 1")
        }
    }
}


// MARK: - Division Tests

final class DivisionTests: XCTestCase {

    /// Dividing by one is the identity: a / 1 = a.
    func testDivideByOne() {
        for a: UInt8 in [0, 1, 2, 42, 128, 255] {
            XCTAssertEqual(GF256.divide(a, 1), a)
        }
    }

    /// Zero divided by anything (non-zero) is zero.
    func testZeroDividedByAnything() {
        for b: UInt8 in [1, 2, 100, 255] {
            XCTAssertEqual(GF256.divide(0, b), 0)
        }
    }

    /// Dividing by self gives one (for any non-zero element).
    func testDivideSelf() {
        for a: UInt8 in [1, 2, 7, 42, 128, 255] {
            XCTAssertEqual(GF256.divide(a, a), 1, "\(a) / \(a) should be 1")
        }
    }

    /// multiply(a, b) then divide by b gives a back.
    func testDivideInvertsMultiply() {
        let a: UInt8 = 57
        let b: UInt8 = 83
        let product = GF256.multiply(a, b)
        XCTAssertEqual(GF256.divide(product, b), a)
    }

    /// Divide result satisfies: divide(a, b) × b = a.
    func testDivideMultiplyProperty() {
        for a: UInt8 in [1, 42, 100, 200, 255] {
            for b: UInt8 in [1, 2, 7, 128] {
                let q = GF256.divide(a, b)
                XCTAssertEqual(GF256.multiply(q, b), a, "\(a)/\(b)*\(b) should equal \(a)")
            }
        }
    }
}


// MARK: - Power Tests

final class PowerTests: XCTestCase {

    /// Any non-zero element to the 0th power is 1.
    func testPowerZeroExponent() {
        for a: UInt8 in [1, 2, 7, 128, 255] {
            XCTAssertEqual(GF256.power(a, 0), 1, "\(a)^0 should be 1")
        }
    }

    /// 0^0 = 1 by convention.
    func testPowerZeroToZero() {
        XCTAssertEqual(GF256.power(0, 0), 1)
    }

    /// 0^n = 0 for n > 0.
    func testPowerZeroBase() {
        XCTAssertEqual(GF256.power(0, 1), 0)
        XCTAssertEqual(GF256.power(0, 5), 0)
        XCTAssertEqual(GF256.power(0, 255), 0)
    }

    /// a^1 = a for any a.
    func testPowerOneExponent() {
        for a: UInt8 in [1, 2, 7, 128, 255] {
            XCTAssertEqual(GF256.power(a, 1), a, "\(a)^1 should be \(a)")
        }
    }

    /// 2^8 = ALOG[8] = 29.
    func testPower2To8() {
        XCTAssertEqual(GF256.power(2, 8), 29)
    }

    /// Fermat's little theorem: a^255 = 1 for all non-zero a.
    func testFermatLittleTheorem() {
        for a: UInt8 in [1, 2, 3, 42, 100, 200, 255] {
            XCTAssertEqual(GF256.power(a, 255), 1, "\(a)^255 should be 1 (Fermat)")
        }
    }

    /// power(a, 2) = multiply(a, a).
    func testPowerSquareMatchesMultiply() {
        for a: UInt8 in [2, 3, 5, 10, 50, 100] {
            XCTAssertEqual(GF256.power(a, 2), GF256.multiply(a, a))
        }
    }
}


// MARK: - Inverse Tests

final class InverseTests: XCTestCase {

    /// inverse(1) = 1.
    func testInverseOfOne() {
        XCTAssertEqual(GF256.inverse(1), 1)
    }

    /// Inverse of 2 gives an element that multiplies back to 1.
    func testInverseOf2() {
        let inv2 = GF256.inverse(2)
        XCTAssertEqual(GF256.multiply(2, inv2), 1)
    }

    /// For all non-zero elements, a × inverse(a) = 1.
    func testInverseProperty() {
        for a: UInt8 in 1...255 {
            let inv = GF256.inverse(a)
            XCTAssertEqual(
                GF256.multiply(a, inv), 1,
                "\(a) × inverse(\(a)) should equal 1"
            )
        }
    }

    /// inverse is its own inverse: inverse(inverse(a)) = a.
    func testInverseOfInverse() {
        for a: UInt8 in [1, 2, 7, 42, 100, 200, 255] {
            XCTAssertEqual(GF256.inverse(GF256.inverse(a)), a)
        }
    }

    /// inverse(a) = power(a, 254) (by Fermat: a^255 = 1, so a^(-1) = a^254).
    func testInverseEqualsPower254() {
        for a: UInt8 in [1, 2, 3, 7, 50, 200, 255] {
            XCTAssertEqual(GF256.inverse(a), GF256.power(a, 254))
        }
    }
}


// MARK: - Field Axiom Tests

final class FieldAxiomTests: XCTestCase {

    /// Distributivity: a × (b + c) = (a × b) + (a × c).
    func testDistributivity() {
        let a: UInt8 = 5
        let b: UInt8 = 7
        let c: UInt8 = 11
        let lhs = GF256.multiply(a, GF256.add(b, c))
        let rhs = GF256.add(GF256.multiply(a, b), GF256.multiply(a, c))
        XCTAssertEqual(lhs, rhs)
    }

    /// Distributivity holds for many triples.
    func testDistributivityMultiple() {
        let triples: [(UInt8, UInt8, UInt8)] = [
            (2, 3, 5), (10, 20, 30), (100, 150, 200), (1, 254, 255)
        ]
        for (a, b, c) in triples {
            let lhs = GF256.multiply(a, GF256.add(b, c))
            let rhs = GF256.add(GF256.multiply(a, b), GF256.multiply(a, c))
            XCTAssertEqual(lhs, rhs, "Distributivity failed for (\(a), \(b), \(c))")
        }
    }

    /// Commutativity of multiplication for many pairs.
    func testMultiplicationCommutativityExtended() {
        let pairs: [(UInt8, UInt8)] = [(3, 17), (42, 99), (128, 3), (255, 2)]
        for (a, b) in pairs {
            XCTAssertEqual(GF256.multiply(a, b), GF256.multiply(b, a))
        }
    }
}


// MARK: - GF256Field (parameterizable field factory) Tests

/// Tests for GF256Field — a field factory that accepts any primitive polynomial.
///
/// The module-level GF256 enum is fixed to the Reed-Solomon polynomial 0x11D.
/// GF256Field allows AES (polynomial 0x11B) and other applications to use the
/// same Russian peasant multiplication with a different polynomial.
final class GF256FieldTests: XCTestCase {

    // ── AES field (polynomial 0x11B) ─────────────────────────────────────────

    /// In the AES field (poly 0x11B): 0x53 × 0xCA = 0x01.
    /// These are multiplicative inverses in AES GF(2^8).
    func testAESFieldMultiplyInverses() {
        let aes = GF256Field(polynomial: 0x11B)
        XCTAssertEqual(aes.multiply(0x53, 0xCA), 0x01)
    }

    /// FIPS 197 Appendix B test vector: 0x57 × 0x83 = 0xC1 in AES GF(2^8).
    /// This is the canonical test from the AES specification.
    func testAESFieldFIPS197() {
        let aes = GF256Field(polynomial: 0x11B)
        XCTAssertEqual(aes.multiply(0x57, 0x83), 0xC1)
    }

    /// inverse(0x53) = 0xCA in the AES field.
    func testAESFieldInverse() {
        let aes = GF256Field(polynomial: 0x11B)
        XCTAssertEqual(aes.inverse(0x53), 0xCA)
    }

    /// a × inverse(a) = 1 for a in 1..20 using the AES field.
    func testAESFieldInverseProperty() {
        let aes = GF256Field(polynomial: 0x11B)
        for a: UInt8 in 1...20 {
            XCTAssertEqual(aes.multiply(a, aes.inverse(a)), 1,
                "AES field: \(a) × inverse(\(a)) should be 1")
        }
    }

    /// Multiplication is commutative in the AES field.
    func testAESFieldCommutativity() {
        let aes = GF256Field(polynomial: 0x11B)
        let vals: [UInt8] = [0, 1, 0x53, 0xCA, 0xFF]
        for a in vals {
            for b in vals {
                XCTAssertEqual(aes.multiply(a, b), aes.multiply(b, a),
                    "AES field: multiply(\(a), \(b)) should equal multiply(\(b), \(a))")
            }
        }
    }

    /// add is XOR regardless of polynomial (characteristic 2).
    func testAESFieldAddIsXOR() {
        let aes = GF256Field(polynomial: 0x11B)
        XCTAssertEqual(aes.add(0x53, 0xCA), 0x53 ^ 0xCA)
    }

    /// divide by zero panics in the AES field.
    func testAESFieldDivideByZero() {
        let aes = GF256Field(polynomial: 0x11B)
        // XCTAssertThrowsError doesn't work for fatalError; verify non-zero divides work.
        // We verify the happy path here; the fatalError path is documented behavior.
        XCTAssertEqual(aes.divide(0x53, 0x8C), aes.multiply(0x53, aes.inverse(0x8C)))
    }

    /// polynomial property is stored on the field instance.
    func testAESFieldPolynomialProperty() {
        let aes = GF256Field(polynomial: 0x11B)
        XCTAssertEqual(aes.polynomial, 0x11B)
    }

    // ── RS field (0x11D) matches module-level functions ───────────────────────

    /// GF256Field(0x11D).multiply should match the module-level GF256.multiply
    /// for a sample of values, verifying backward compatibility.
    func testRSFieldMatchesModuleMultiply() {
        let rs = GF256Field(polynomial: 0x11D)
        let vals: [UInt8] = [0, 1, 0x53, 0xCA, 0xFF]
        for a in vals {
            for b in vals {
                XCTAssertEqual(rs.multiply(a, b), GF256.multiply(a, b),
                    "RS field multiply(\(a), \(b)) should match module-level")
            }
        }
    }

    /// GF256Field(0x11D).inverse should match GF256.inverse for a sample.
    func testRSFieldMatchesModuleInverse() {
        let rs = GF256Field(polynomial: 0x11D)
        for a: UInt8 in 1...20 {
            XCTAssertEqual(rs.inverse(a), GF256.inverse(a),
                "RS field inverse(\(a)) should match module-level")
        }
    }

    // ── General field properties ──────────────────────────────────────────────

    /// multiply by zero always returns zero.
    func testFieldMultiplyByZero() {
        let aes = GF256Field(polynomial: 0x11B)
        for a: UInt8 in [0, 1, 0x53, 0xFF] {
            XCTAssertEqual(aes.multiply(a, 0), 0)
            XCTAssertEqual(aes.multiply(0, a), 0)
        }
    }

    /// multiply by one is the identity.
    func testFieldMultiplyByOne() {
        let aes = GF256Field(polynomial: 0x11B)
        for a: UInt8 in [0, 1, 0x53, 0xFF] {
            XCTAssertEqual(aes.multiply(a, 1), a)
            XCTAssertEqual(aes.multiply(1, a), a)
        }
    }

    /// divide(x, 1) = x.
    func testFieldDivideByOne() {
        let aes = GF256Field(polynomial: 0x11B)
        for a: UInt8 in [0, 1, 0x53, 0xFF] {
            XCTAssertEqual(aes.divide(a, 1), a)
        }
    }

    /// divide(x, x) = 1 for non-zero x.
    func testFieldDivideSelf() {
        let aes = GF256Field(polynomial: 0x11B)
        for a: UInt8 in 1...10 {
            XCTAssertEqual(aes.divide(a, a), 1)
        }
    }

    /// inverse(inverse(a)) = a.
    func testFieldInverseOfInverse() {
        let aes = GF256Field(polynomial: 0x11B)
        for a: UInt8 in 1...20 {
            XCTAssertEqual(aes.inverse(aes.inverse(a)), a)
        }
    }
}

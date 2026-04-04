// ============================================================================
// GF256NativeTests.swift — Tests for the GF256Native Swift wrapper.
// ============================================================================
//
// IMPORTANT: These tests require libgf256_c.a to be compiled and placed at
// Sources/CGF256/libgf256_c.a before running. See the BUILD file for the
// full two-step process:
//
//   Step 1:  cd code/packages/rust/gf256-c && cargo build --release
//   Step 2:  cp target/release/libgf256_c.a \
//               ../../swift/gf256-native/Sources/CGF256/
//   Step 3:  cd ../../swift/gf256-native && swift test
//
// ============================================================================

import Testing
@testable import GF256Native

// ============================================================================
// MARK: — Addition
// ============================================================================

/// GF(256) addition is bitwise XOR. Each bit is an independent GF(2) element,
/// and GF(2) addition is `1 + 1 = 0 (mod 2)` — which is XOR.
struct AdditionTests {

    @Test func addTwoValues() {
        // 0x53 XOR 0xCA = 0x99
        #expect(GF256Native.add(0x53, 0xCA) == 0x99)
    }

    @Test func addingZeroIsIdentity() {
        for x: UInt8 in [0, 1, 2, 127, 255] {
            #expect(GF256Native.add(x, 0) == x)
            #expect(GF256Native.add(0, x) == x)
        }
    }

    @Test func addingToSelfIsZero() {
        // In GF(2^8), every element satisfies x + x = 0
        for x: UInt8 in [1, 7, 42, 128, 255] {
            #expect(GF256Native.add(x, x) == 0)
        }
    }

    @Test func commutativity() {
        // a + b = b + a
        #expect(GF256Native.add(0x12, 0x34) == GF256Native.add(0x34, 0x12))
        #expect(GF256Native.add(255, 1)     == GF256Native.add(1, 255))
    }

    @Test func associativity() {
        // (a + b) + c = a + (b + c)
        let a: UInt8 = 0x12
        let b: UInt8 = 0x34
        let c: UInt8 = 0x56
        let lhs = GF256Native.add(GF256Native.add(a, b), c)
        let rhs = GF256Native.add(a, GF256Native.add(b, c))
        #expect(lhs == rhs)
    }
}

// ============================================================================
// MARK: — Subtraction
// ============================================================================

/// In GF(2^8), subtraction equals addition (both are XOR).
struct SubtractionTests {

    @Test func subtractEqualsAdd() {
        // a - b = a + b in GF(256)
        let pairs: [(UInt8, UInt8)] = [(0x53, 0xCA), (1, 1), (255, 127), (0, 0)]
        for (a, b) in pairs {
            #expect(GF256Native.subtract(a, b) == GF256Native.add(a, b))
        }
    }

    @Test func subtractFromSelfIsZero() {
        for x: UInt8 in [1, 42, 255] {
            #expect(GF256Native.subtract(x, x) == 0)
        }
    }

    @Test func subtractZeroIsIdentity() {
        for x: UInt8 in [0, 1, 128, 255] {
            #expect(GF256Native.subtract(x, 0) == x)
        }
    }
}

// ============================================================================
// MARK: — Multiplication
// ============================================================================

struct MultiplicationTests {

    @Test func multiplyByZero() {
        for x: UInt8 in [0, 1, 42, 255] {
            #expect(GF256Native.multiply(x, 0) == 0)
            #expect(GF256Native.multiply(0, x) == 0)
        }
    }

    @Test func multiplyByOne() {
        // 1 is the multiplicative identity
        for x: UInt8 in [0, 1, 2, 42, 255] {
            #expect(GF256Native.multiply(x, 1) == x)
            #expect(GF256Native.multiply(1, x) == x)
        }
    }

    @Test func multiplyTwiceIsPowerTwo() {
        // Multiplying by 2 is the generator step
        // 2 × 64 = 128 (no overflow)
        #expect(GF256Native.multiply(2, 64) == 128)
        // 2 × 128 = 29 (first reduction: 256 XOR 0x11D = 0x1D = 29)
        #expect(GF256Native.multiply(2, 128) == 29)
    }

    @Test func commutativity() {
        // a × b = b × a
        #expect(GF256Native.multiply(3, 7) == GF256Native.multiply(7, 3))
        #expect(GF256Native.multiply(100, 200) == GF256Native.multiply(200, 100))
    }

    @Test func distributivityOverAddition() {
        // a × (b + c) = (a × b) + (a × c)
        let a: UInt8 = 3
        let b: UInt8 = 17
        let c: UInt8 = 42
        let lhs = GF256Native.multiply(a, GF256Native.add(b, c))
        let rhs = GF256Native.add(
            GF256Native.multiply(a, b),
            GF256Native.multiply(a, c)
        )
        #expect(lhs == rhs)
    }
}

// ============================================================================
// MARK: — Division
// ============================================================================

struct DivisionTests {

    @Test func divideByZeroReturnsNil() {
        #expect(GF256Native.divide(42, 0) == nil)
        #expect(GF256Native.divide(0, 0) == nil)
        #expect(GF256Native.divide(255, 0) == nil)
    }

    @Test func divideZeroByAnythingIsZero() {
        for b: UInt8 in [1, 2, 42, 255] {
            #expect(GF256Native.divide(0, b) == 0)
        }
    }

    @Test func divideByOne() {
        // a / 1 = a
        for a: UInt8 in [1, 2, 42, 128, 255] {
            #expect(GF256Native.divide(a, 1) == a)
        }
    }

    @Test func divideBySelf() {
        // a / a = 1 for all non-zero a
        for a: UInt8 in [1, 2, 42, 128, 255] {
            #expect(GF256Native.divide(a, a) == 1)
        }
    }

    @Test func multiplyInversesDivide() {
        // a * b / b = a
        let a: UInt8 = 42
        let b: UInt8 = 17
        let product = GF256Native.multiply(a, b)
        #expect(GF256Native.divide(product, b) == a)
    }
}

// ============================================================================
// MARK: — Power
// ============================================================================

struct PowerTests {

    @Test func powerZeroIsOne() {
        // Any non-zero base^0 = 1 (convention)
        for base: UInt8 in [1, 2, 42, 255] {
            #expect(GF256Native.power(base, 0) == 1)
        }
    }

    @Test func zeroPowerZeroIsOne() {
        // 0^0 = 1 by convention
        #expect(GF256Native.power(0, 0) == 1)
    }

    @Test func zeroPowerPositive() {
        // 0^n = 0 for n > 0
        #expect(GF256Native.power(0, 1) == 0)
        #expect(GF256Native.power(0, 10) == 0)
        #expect(GF256Native.power(0, 255) == 0)
    }

    @Test func powerOneIsIdentity() {
        // base^1 = base
        for base: UInt8 in [0, 1, 2, 42, 255] {
            #expect(GF256Native.power(base, 1) == base)
        }
    }

    @Test func generatorCycles() {
        // g^255 = 1 (the group wraps around after 255 steps)
        #expect(GF256Native.power(2, 255) == 1)
        // g^8 = 29 (first reduction step)
        #expect(GF256Native.power(2, 8) == 29)
    }

    @Test func powerConsistencyWithMultiply() {
        // a^2 = a * a
        for a: UInt8 in [2, 3, 5, 17, 42] {
            let sq = GF256Native.power(a, 2)
            let prod = GF256Native.multiply(a, a)
            #expect(sq == prod)
        }
    }
}

// ============================================================================
// MARK: — Inverse
// ============================================================================

struct InverseTests {

    @Test func inverseOfZeroIsNil() {
        #expect(GF256Native.inverse(0) == nil)
    }

    @Test func inverseOfOne() {
        // 1 is its own inverse: 1 × 1 = 1
        #expect(GF256Native.inverse(1) == 1)
    }

    @Test func inverseProperty() {
        // a × inverse(a) = 1 for all non-zero a
        for a: UInt8 in [1, 2, 3, 42, 127, 128, 255] {
            if let inv = GF256Native.inverse(a) {
                #expect(GF256Native.multiply(a, inv) == 1)
            } else {
                Issue.record("inverse(\(a)) returned nil unexpectedly")
            }
        }
    }

    @Test func inverseOfInverse() {
        // inverse(inverse(a)) = a
        for a: UInt8 in [2, 17, 42, 200] {
            if let inv = GF256Native.inverse(a),
               let invInv = GF256Native.inverse(inv) {
                #expect(invInv == a)
            }
        }
    }

    @Test func inverseConsistencyWithDivide() {
        // inverse(a) = 1 / a = divide(1, a)
        for a: UInt8 in [1, 2, 42, 255] {
            let inv = GF256Native.inverse(a)
            let div = GF256Native.divide(1, a)
            #expect(inv == div)
        }
    }
}

// ============================================================================
// MARK: — Constants
// ============================================================================

struct ConstantTests {

    @Test func zeroIsAdditiveIdentity() {
        #expect(GF256Native.zero == 0)
        #expect(GF256Native.add(GF256Native.zero, 42) == 42)
    }

    @Test func oneIsMultiplicativeIdentity() {
        #expect(GF256Native.one == 1)
        #expect(GF256Native.multiply(GF256Native.one, 42) == 42)
    }

    @Test func primitivePolynomialValue() {
        // The primitive polynomial is x^8 + x^4 + x^3 + x^2 + 1 = 285 = 0x11D
        #expect(GF256Native.primitivePolynomial == 285)
        #expect(GF256Native.primitivePolynomial == 0x11D)
    }
}

// ============================================================================
// MARK: — Algebraic Properties (Field Axioms)
// ============================================================================

/// Verify that the GF256Native implementation satisfies the field axioms.
/// These are mathematical laws that must hold in any finite field:
///
/// 1. Closure: all operations produce elements in [0, 255]
/// 2. Associativity of addition and multiplication
/// 3. Commutativity of addition and multiplication
/// 4. Distributivity of multiplication over addition
/// 5. Identity elements (0 for add, 1 for multiply)
/// 6. Additive inverses: a + a = 0
/// 7. Multiplicative inverses: a * inverse(a) = 1 (for a != 0)
struct FieldAxiomTests {

    // A small set of representative values to test axioms on.
    let samples: [UInt8] = [0, 1, 2, 7, 42, 128, 200, 255]

    @Test func fermatLittleTheorem() {
        // In GF(256), every non-zero element satisfies a^255 = 1.
        // This is analogous to Fermat's little theorem for prime fields.
        for a in samples where a != 0 {
            #expect(GF256Native.power(a, 255) == 1,
                    "Fermat: \(a)^255 should be 1")
        }
    }

    @Test func generatorGeneratesAllNonZeroElements() {
        // g = 2 generates all 255 non-zero elements of GF(256).
        // Verify that { 2^1, 2^2, ..., 2^255 } has exactly 255 distinct values.
        var seen = Set<UInt8>()
        for exp: UInt32 in 1...255 {
            let val = GF256Native.power(2, exp)
            seen.insert(val)
        }
        // All 255 non-zero elements should appear exactly once.
        #expect(seen.count == 255)
        #expect(!seen.contains(0))  // zero is never a power of the generator
    }
}

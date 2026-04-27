// ============================================================================
// X25519.swift — X25519 Elliptic Curve Diffie-Hellman (RFC 7748)
// ============================================================================
//
// X25519 is the Diffie-Hellman function on Curve25519, one of the most widely
// used key agreement protocols in modern cryptography. It is used in TLS 1.3,
// SSH, Signal, WireGuard, and many other protocols.
//
// ## The Challenge in Swift
//
// Unlike TypeScript (BigInt) and Ruby (native big integers), Swift does not
// have built-in arbitrary-precision integers. We need numbers up to 2^255,
// which far exceeds UInt64's capacity (2^64 - 1).
//
// Our solution: represent field elements as arrays of 5 UInt64 "limbs",
// each holding up to 51 bits of the number. This is the "radix-2^51"
// representation used by many high-performance Curve25519 implementations.
//
// A 255-bit number n is split as:
//   n = limbs[0] + limbs[1]*2^51 + limbs[2]*2^102 + limbs[3]*2^153 + limbs[4]*2^204
//
// Why 51 bits per limb? Because 51*5 = 255, and when we multiply two 51-bit
// numbers, the product fits in 102 bits — which we can handle using UInt64
// multiplication with careful carry propagation (since we use UInt128 for
// intermediate products via the Limb type with overflow detection).
//
// Actually, for simplicity and correctness, we'll use a simpler approach:
// represent numbers as [UInt64] arrays in base 2^64 with standard schoolbook
// arithmetic, then reduce mod p. This is clearer for educational purposes.
//
// ============================================================================

import Foundation

// ============================================================================
// FieldElement — A number in GF(2^255 - 19)
// ============================================================================
//
// We represent field elements as 4 UInt64 limbs in little-endian order:
//   value = limbs[0] + limbs[1]*2^64 + limbs[2]*2^128 + limbs[3]*2^192
//
// This gives us 256 bits of storage, enough for any element of GF(2^255-19).
// All arithmetic reduces mod p after each operation.

/// A field element in GF(2^255 - 19), represented as 4 UInt64 limbs.
///
/// The prime p = 2^255 - 19 defines the field. All arithmetic is performed
/// modulo this prime. The limbs are in little-endian order: limbs[0] is the
/// least significant 64-bit word.
public struct FieldElement: Equatable, Sendable {
    /// The four 64-bit limbs, least significant first.
    var limbs: (UInt64, UInt64, UInt64, UInt64)

    /// Manual Equatable conformance since Swift tuples don't conform to Equatable.
    public static func == (lhs: FieldElement, rhs: FieldElement) -> Bool {
        return lhs.limbs.0 == rhs.limbs.0
            && lhs.limbs.1 == rhs.limbs.1
            && lhs.limbs.2 == rhs.limbs.2
            && lhs.limbs.3 == rhs.limbs.3
    }

    /// The zero element.
    static let zero = FieldElement(limbs: (0, 0, 0, 0))

    /// The one element.
    static let one = FieldElement(limbs: (1, 0, 0, 0))

    /// The prime p = 2^255 - 19.
    ///
    /// In 4 limbs (little-endian):
    ///   2^255 - 19 = 0x7fffffffffffffed 0xffffffffffffffff 0xffffffffffffffff 0x7fffffffffffffff
    ///
    /// Let's verify: 2^255 = 2^192 * 2^63 = 0x8000000000000000 in limb[3].
    /// Subtracting 19 from the full 256-bit number gives us:
    ///   limb[0] = 0 - 19 = -19, which borrows, giving 2^64 - 19 = 0xFFFFFFFFFFFFFFED
    ///   limb[1] = 0xFFFFFFFFFFFFFFFF - 1 (borrow) = 0xFFFFFFFFFFFFFFFE... no.
    ///
    /// Actually: 2^255 in 4 limbs is (0, 0, 0, 0x8000000000000000).
    /// 2^255 - 19 = borrow from limb[0]:
    ///   limb[0] = 0 - 19 wraps to 2^64 - 19 = 0xFFFFFFFFFFFFFFED, borrow 1
    ///   limb[1] = 0 - 1 wraps to 0xFFFFFFFFFFFFFFFF, borrow 1
    ///   limb[2] = 0 - 1 wraps to 0xFFFFFFFFFFFFFFFF, borrow 1
    ///   limb[3] = 0x8000000000000000 - 1 = 0x7FFFFFFFFFFFFFFF
    static let p = FieldElement(limbs: (
        0xFFFF_FFFF_FFFF_FFED,
        0xFFFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x7FFF_FFFF_FFFF_FFFF
    ))

    /// The constant a24 = 121665.
    static let a24 = FieldElement(limbs: (121665, 0, 0, 0))
}

// ============================================================================
// Multi-precision arithmetic helpers
// ============================================================================
//
// These functions implement basic operations on 256-bit numbers represented
// as 4 UInt64 limbs. They form the foundation for field arithmetic.

/// Add two 256-bit numbers, returning (result, carry).
/// The carry is 0 or 1, indicating overflow beyond 256 bits.
private func add256(
    _ a: (UInt64, UInt64, UInt64, UInt64),
    _ b: (UInt64, UInt64, UInt64, UInt64)
) -> ((UInt64, UInt64, UInt64, UInt64), UInt64) {
    var carry: UInt64 = 0
    var r: (UInt64, UInt64, UInt64, UInt64) = (0, 0, 0, 0)

    // Limb 0
    var (sum, overflow) = a.0.addingReportingOverflow(b.0)
    var (sum2, overflow2) = sum.addingReportingOverflow(carry)
    r.0 = sum2
    carry = (overflow ? 1 : 0) + (overflow2 ? 1 : 0)

    // Limb 1
    (sum, overflow) = a.1.addingReportingOverflow(b.1)
    (sum2, overflow2) = sum.addingReportingOverflow(carry)
    r.1 = sum2
    carry = (overflow ? 1 : 0) + (overflow2 ? 1 : 0)

    // Limb 2
    (sum, overflow) = a.2.addingReportingOverflow(b.2)
    (sum2, overflow2) = sum.addingReportingOverflow(carry)
    r.2 = sum2
    carry = (overflow ? 1 : 0) + (overflow2 ? 1 : 0)

    // Limb 3
    (sum, overflow) = a.3.addingReportingOverflow(b.3)
    (sum2, overflow2) = sum.addingReportingOverflow(carry)
    r.3 = sum2
    carry = (overflow ? 1 : 0) + (overflow2 ? 1 : 0)

    return (r, carry)
}

/// Subtract b from a (256-bit), returning (result, borrow).
/// If borrow is 1, the true result is negative (a < b).
private func sub256(
    _ a: (UInt64, UInt64, UInt64, UInt64),
    _ b: (UInt64, UInt64, UInt64, UInt64)
) -> ((UInt64, UInt64, UInt64, UInt64), UInt64) {
    var borrow: UInt64 = 0
    var r: (UInt64, UInt64, UInt64, UInt64) = (0, 0, 0, 0)

    // Limb 0
    var (diff, underflow) = a.0.subtractingReportingOverflow(b.0)
    var (diff2, underflow2) = diff.subtractingReportingOverflow(borrow)
    r.0 = diff2
    borrow = (underflow ? 1 : 0) + (underflow2 ? 1 : 0)

    // Limb 1
    (diff, underflow) = a.1.subtractingReportingOverflow(b.1)
    (diff2, underflow2) = diff.subtractingReportingOverflow(borrow)
    r.1 = diff2
    borrow = (underflow ? 1 : 0) + (underflow2 ? 1 : 0)

    // Limb 2
    (diff, underflow) = a.2.subtractingReportingOverflow(b.2)
    (diff2, underflow2) = diff.subtractingReportingOverflow(borrow)
    r.2 = diff2
    borrow = (underflow ? 1 : 0) + (underflow2 ? 1 : 0)

    // Limb 3
    (diff, underflow) = a.3.subtractingReportingOverflow(b.3)
    (diff2, underflow2) = diff.subtractingReportingOverflow(borrow)
    r.3 = diff2
    borrow = (underflow ? 1 : 0) + (underflow2 ? 1 : 0)

    return (r, borrow)
}

/// Compare two 256-bit numbers.
/// Returns -1 if a < b, 0 if a == b, 1 if a > b.
private func cmp256(
    _ a: (UInt64, UInt64, UInt64, UInt64),
    _ b: (UInt64, UInt64, UInt64, UInt64)
) -> Int {
    if a.3 != b.3 { return a.3 < b.3 ? -1 : 1 }
    if a.2 != b.2 { return a.2 < b.2 ? -1 : 1 }
    if a.1 != b.1 { return a.1 < b.1 ? -1 : 1 }
    if a.0 != b.0 { return a.0 < b.0 ? -1 : 1 }
    return 0
}

/// Multiply two 64-bit numbers and return the 128-bit result as (low, high).
private func mul64(
    _ a: UInt64,
    _ b: UInt64
) -> (low: UInt64, high: UInt64) {
    let result = a.multipliedFullWidth(by: b)
    return (result.low, result.high)
}

/// Multiply a 256-bit number by a 256-bit number, returning a 512-bit result
/// as 8 UInt64 limbs.
///
/// This is the schoolbook multiplication algorithm:
///   for each limb i of a:
///     for each limb j of b:
///       result[i+j] += a[i] * b[j]
///
/// We accumulate into a wider result to avoid overflow.
private func mul256Full(
    _ a: (UInt64, UInt64, UInt64, UInt64),
    _ b: (UInt64, UInt64, UInt64, UInt64)
) -> (UInt64, UInt64, UInt64, UInt64, UInt64, UInt64, UInt64, UInt64) {
    // Use an array for accumulation, then copy to tuple
    let aArr = [a.0, a.1, a.2, a.3]
    let bArr = [b.0, b.1, b.2, b.3]
    var result = [UInt64](repeating: 0, count: 8)

    for i in 0..<4 {
        var carry: UInt64 = 0
        for j in 0..<4 {
            let (lo, hi) = mul64(aArr[i], bArr[j])
            // Add lo + carry + result[i+j]
            let (s1, o1) = result[i + j].addingReportingOverflow(lo)
            let (s2, o2) = s1.addingReportingOverflow(carry)
            result[i + j] = s2
            carry = hi + (o1 ? 1 : 0) + (o2 ? 1 : 0)
        }
        result[i + 4] = carry
    }

    return (result[0], result[1], result[2], result[3],
            result[4], result[5], result[6], result[7])
}

// ============================================================================
// Modular reduction: the key to field arithmetic
// ============================================================================
//
// After multiplication, we have a 512-bit result that needs to be reduced
// mod p = 2^255 - 19. The key insight is:
//
//   2^255 ≡ 19 (mod p)
//
// So any bits above position 255 can be "folded back" by multiplying by 19.
// If we have a number n = nLow + nHigh * 2^255, then:
//   n ≡ nLow + nHigh * 19 (mod p)
//
// This may still be >= p, so we repeat or do a final conditional subtraction.

/// Reduce a 512-bit number mod p = 2^255 - 19.
///
/// Strategy:
/// 1. Split at bit 255: low = bits 0..254, high = bits 255..511
/// 2. Compute low + high * 19 (this is at most ~320 bits)
/// 3. Repeat the fold for any remaining bits above 255
/// 4. Final conditional subtraction if result >= p
private func reduce512(
    _ n: (UInt64, UInt64, UInt64, UInt64, UInt64, UInt64, UInt64, UInt64)
) -> (UInt64, UInt64, UInt64, UInt64) {
    // The 512-bit number in array form for easier manipulation
    var r = [n.0, n.1, n.2, n.3, n.4, n.5, n.6, n.7]

    // We'll do two rounds of reduction to handle the full range.
    // Each round: split at bit 255, multiply high part by 19, add to low part.
    for _ in 0..<2 {
        // Split at bit 255:
        // low = r[0..3] with r[3] masked to 63 bits (bits 0-254)
        // high = r[3] >> 63 | r[4..7] shifted, plus remaining bits

        // The high part starts at bit 255, so:
        // high_bit_of_r3 = r[3] >> 63 (this is bit 255 within the 256-bit low chunk)
        // But we need ALL bits from position 255 upward.

        // Bits 255..511 as a separate number:
        // Shift the entire 512-bit number right by 255 positions.
        // This is: shift right by 3 full limbs (192 bits) then by 63 more bits.

        var high = [UInt64](repeating: 0, count: 5)
        // Shift right by 255 = shift right by 192 then by 63
        // After shifting by 192, we have r[3], r[4], r[5], r[6], r[7]
        // Then shift right by 63:
        high[0] = (r[3] >> 63) | (r[4] << 1)
        high[1] = (r[4] >> 63) | (r[5] << 1)
        high[2] = (r[5] >> 63) | (r[6] << 1)
        high[3] = (r[6] >> 63) | (r[7] << 1)
        high[4] = r[7] >> 63

        // Mask r[3] to keep only bits 0-62 (the low 255 bits use bits 0-62 of limb 3)
        r[3] &= 0x7FFF_FFFF_FFFF_FFFF
        r[4] = 0; r[5] = 0; r[6] = 0; r[7] = 0

        // Multiply high by 19 and add to low
        var carry: UInt64 = 0
        for i in 0..<5 {
            let (lo, hi) = mul64(high[i], 19)
            let (s1, o1) = r[i].addingReportingOverflow(lo)
            let (s2, o2) = s1.addingReportingOverflow(carry)
            r[i] = s2
            carry = hi + (o1 ? 1 : 0) + (o2 ? 1 : 0)
        }
        // Propagate any remaining carry
        for i in 5..<8 {
            if carry == 0 { break }
            let (s, o) = r[i].addingReportingOverflow(carry)
            r[i] = s
            carry = o ? 1 : 0
        }
    }

    var result = (r[0], r[1], r[2], r[3])

    // Final conditional subtraction: if result >= p, subtract p
    while cmp256(result, FieldElement.p.limbs) >= 0 {
        let (sub, _) = sub256(result, FieldElement.p.limbs)
        result = sub
    }

    return result
}

// ============================================================================
// Field arithmetic operations
// ============================================================================

extension FieldElement {
    /// Field addition: (a + b) mod p
    static func + (a: FieldElement, b: FieldElement) -> FieldElement {
        let (sum, carry) = add256(a.limbs, b.limbs)
        var result = sum

        if carry > 0 || cmp256(result, p.limbs) >= 0 {
            // If we overflowed or result >= p, subtract p.
            // If carry is 1, the true value is result + 2^256.
            // 2^256 mod p = 2^256 - p = 2^256 - (2^255 - 19) = 2^255 + 19
            // So: result + 2^256 ≡ result + 2^255 + 19 (mod p)
            // But 2^255 ≡ 19 (mod p), so result + 2^256 ≡ result + 38 (mod p)
            if carry > 0 {
                // Add 38 (= 2 * 19) to account for the 2^256 overflow
                let (r2, c2) = add256(result, (38, 0, 0, 0))
                result = r2
                // If this overflows again (extremely unlikely but possible),
                // add another 38
                if c2 > 0 {
                    let (r3, _) = add256(result, (38, 0, 0, 0))
                    result = r3
                }
            }
            // Final reduction if still >= p
            while cmp256(result, p.limbs) >= 0 {
                let (sub, _) = sub256(result, p.limbs)
                result = sub
            }
        }

        return FieldElement(limbs: result)
    }

    /// Field subtraction: (a - b) mod p
    static func - (a: FieldElement, b: FieldElement) -> FieldElement {
        let (diff, borrow) = sub256(a.limbs, b.limbs)
        var result = diff

        if borrow > 0 {
            // Result was negative, add p to wrap around
            let (r2, _) = add256(result, p.limbs)
            result = r2
        }

        return FieldElement(limbs: result)
    }

    /// Field multiplication: (a * b) mod p
    static func * (a: FieldElement, b: FieldElement) -> FieldElement {
        let full = mul256Full(a.limbs, b.limbs)
        let reduced = reduce512(full)
        return FieldElement(limbs: reduced)
    }

    /// Field squaring: a^2 mod p (calls multiplication)
    func squared() -> FieldElement {
        return self * self
    }

    /// Field inversion: a^(-1) mod p using Fermat's little theorem.
    ///
    /// For prime p: a^(p-2) ≡ a^(-1) (mod p)
    ///
    /// p - 2 = 2^255 - 21
    ///
    /// We use the standard square-and-multiply algorithm.
    func inverse() -> FieldElement {
        // p - 2 = 2^255 - 21
        // In binary, 21 = 10101, so p-2 = 2^255 - 10101_2
        // p-2 in limbs:
        let exp = FieldElement(limbs: (
            0xFFFF_FFFF_FFFF_FFEB,  // p.limbs.0 - 2 = ...FFED - 2 = ...FFEB
            0xFFFF_FFFF_FFFF_FFFF,
            0xFFFF_FFFF_FFFF_FFFF,
            0x7FFF_FFFF_FFFF_FFFF
        ))

        // Square-and-multiply: process bits from MSB to LSB
        var result = FieldElement.one
        var base = self

        // Process each limb from least significant
        let expLimbs = [exp.limbs.0, exp.limbs.1, exp.limbs.2, exp.limbs.3]
        for limbIndex in 0..<4 {
            var limbVal = expLimbs[limbIndex]
            for _ in 0..<64 {
                if limbVal & 1 == 1 {
                    result = result * base
                }
                base = base.squared()
                limbVal >>= 1
            }
        }

        return result
    }
}

// ============================================================================
// X25519 public API
// ============================================================================

/// Errors that can occur during X25519 operations.
public enum X25519Error: Error, CustomStringConvertible, Sendable {
    /// The scalar input was not exactly 32 bytes.
    case invalidScalarLength
    /// The u-coordinate input was not exactly 32 bytes.
    case invalidUCoordinateLength
    /// The result was all zeros, indicating a low-order input point.
    case lowOrderPoint

    public var description: String {
        switch self {
        case .invalidScalarLength:
            return "Scalar must be exactly 32 bytes"
        case .invalidUCoordinateLength:
            return "U-coordinate must be exactly 32 bytes"
        case .lowOrderPoint:
            return "X25519 produced all-zero output — input is a low-order point"
        }
    }
}

/// The X25519 namespace containing all public functions.
public enum X25519 {
    // -------------------------------------------------------------------
    // Byte encoding/decoding
    // -------------------------------------------------------------------

    /// Decode a 32-byte little-endian array into a FieldElement.
    ///
    /// Byte 0 is the least significant. The 32 bytes are packed into 4
    /// UInt64 limbs, 8 bytes each.
    static func decodeLittleEndian(_ bytes: [UInt8]) -> FieldElement {
        var limbs: (UInt64, UInt64, UInt64, UInt64) = (0, 0, 0, 0)
        // Each limb consumes 8 bytes
        for i in 0..<8 { limbs.0 |= UInt64(bytes[i]) << (i * 8) }
        for i in 0..<8 { limbs.1 |= UInt64(bytes[8 + i]) << (i * 8) }
        for i in 0..<8 { limbs.2 |= UInt64(bytes[16 + i]) << (i * 8) }
        for i in 0..<8 { limbs.3 |= UInt64(bytes[24 + i]) << (i * 8) }
        return FieldElement(limbs: limbs)
    }

    /// Encode a FieldElement as a 32-byte little-endian array.
    static func encodeLittleEndian(_ fe: FieldElement) -> [UInt8] {
        var result = [UInt8](repeating: 0, count: 32)
        var val = fe.limbs.0
        for i in 0..<8 { result[i] = UInt8(val & 0xFF); val >>= 8 }
        val = fe.limbs.1
        for i in 0..<8 { result[8 + i] = UInt8(val & 0xFF); val >>= 8 }
        val = fe.limbs.2
        for i in 0..<8 { result[16 + i] = UInt8(val & 0xFF); val >>= 8 }
        val = fe.limbs.3
        for i in 0..<8 { result[24 + i] = UInt8(val & 0xFF); val >>= 8 }
        return result
    }

    /// Decode a u-coordinate from 32 bytes (masks high bit per RFC 7748).
    static func decodeUCoordinate(_ bytes: [UInt8]) -> FieldElement {
        var copy = bytes
        copy[31] &= 0x7F  // Mask bit 255
        return decodeLittleEndian(copy)
    }

    /// Clamp a scalar per RFC 7748:
    ///   k[0] &= 248, k[31] &= 127, k[31] |= 64
    static func clampScalar(_ bytes: [UInt8]) -> FieldElement {
        var clamped = bytes
        clamped[0] &= 248
        clamped[31] &= 127
        clamped[31] |= 64
        return decodeLittleEndian(clamped)
    }

    // -------------------------------------------------------------------
    // Conditional swap
    // -------------------------------------------------------------------

    /// Constant-time conditional swap.
    /// If swap is 1, returns (b, a). If swap is 0, returns (a, b).
    static func cswap(
        _ swap: UInt64,
        _ a: FieldElement,
        _ b: FieldElement
    ) -> (FieldElement, FieldElement) {
        // mask is all 1s if swap=1, all 0s if swap=0
        let mask = UInt64(bitPattern: -Int64(swap))
        let d0 = mask & (a.limbs.0 ^ b.limbs.0)
        let d1 = mask & (a.limbs.1 ^ b.limbs.1)
        let d2 = mask & (a.limbs.2 ^ b.limbs.2)
        let d3 = mask & (a.limbs.3 ^ b.limbs.3)
        return (
            FieldElement(limbs: (a.limbs.0 ^ d0, a.limbs.1 ^ d1, a.limbs.2 ^ d2, a.limbs.3 ^ d3)),
            FieldElement(limbs: (b.limbs.0 ^ d0, b.limbs.1 ^ d1, b.limbs.2 ^ d2, b.limbs.3 ^ d3))
        )
    }

    // -------------------------------------------------------------------
    // Get bit i from a FieldElement
    // -------------------------------------------------------------------

    static func getBit(_ fe: FieldElement, _ i: Int) -> UInt64 {
        let limbIndex = i / 64
        let bitIndex = i % 64
        let limbs = [fe.limbs.0, fe.limbs.1, fe.limbs.2, fe.limbs.3]
        if limbIndex >= 4 { return 0 }
        return (limbs[limbIndex] >> bitIndex) & 1
    }

    // -------------------------------------------------------------------
    // The Montgomery Ladder
    // -------------------------------------------------------------------

    /// Perform X25519 scalar multiplication.
    ///
    /// - Parameters:
    ///   - scalar: 32-byte private scalar (will be clamped)
    ///   - uBytes: 32-byte u-coordinate of the input point
    /// - Returns: 32-byte u-coordinate of the result
    /// - Throws: `X25519Error` if inputs are invalid or result is all zeros
    public static func x25519(scalar: [UInt8], u uBytes: [UInt8]) throws -> [UInt8] {
        guard scalar.count == 32 else { throw X25519Error.invalidScalarLength }
        guard uBytes.count == 32 else { throw X25519Error.invalidUCoordinateLength }

        let k = clampScalar(scalar)
        let u = decodeUCoordinate(uBytes)

        let x1 = u
        var x2 = FieldElement.one
        var z2 = FieldElement.zero
        var x3 = u
        var z3 = FieldElement.one
        var swap: UInt64 = 0

        // Process bits 254 down to 0
        for i in stride(from: 254, through: 0, by: -1) {
            let ki = getBit(k, i)
            swap ^= ki
            (x2, x3) = cswap(swap, x2, x3)
            (z2, z3) = cswap(swap, z2, z3)
            swap = ki

            // Montgomery ladder step
            let A = x2 + z2
            let AA = A.squared()
            let B = x2 - z2
            let BB = B.squared()
            let E = AA - BB

            let C = x3 + z3
            let D = x3 - z3
            let DA = D * A
            let CB = C * B

            x3 = (DA + CB).squared()
            z3 = x1 * (DA - CB).squared()
            x2 = AA * BB
            z2 = E * (AA + FieldElement.a24 * E)
        }

        (x2, x3) = cswap(swap, x2, x3)
        (z2, z3) = cswap(swap, z2, z3)

        let result = x2 * z2.inverse()
        let encoded = encodeLittleEndian(result)

        // Check for all-zero output
        if encoded.allSatisfy({ $0 == 0 }) {
            throw X25519Error.lowOrderPoint
        }

        return encoded
    }

    /// Multiply the scalar by the base point (u = 9).
    ///
    /// This generates a public key from a private key.
    public static func x25519Base(scalar: [UInt8]) throws -> [UInt8] {
        var basePoint = [UInt8](repeating: 0, count: 32)
        basePoint[0] = 9
        return try x25519(scalar: scalar, u: basePoint)
    }

    /// Generate a public key from a private key. Alias for x25519Base.
    public static func generateKeypair(privateKey: [UInt8]) throws -> [UInt8] {
        return try x25519Base(scalar: privateKey)
    }
}

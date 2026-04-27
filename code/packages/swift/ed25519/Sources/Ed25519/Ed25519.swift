// Ed25519.swift
// Part of coding-adventures -- an educational computing stack.
//
// ============================================================================
// Ed25519 Digital Signatures (RFC 8032)
// ============================================================================
//
// Ed25519 is a high-speed, high-security digital signature scheme built on
// the twisted Edwards curve:
//
//   -x^2 + y^2 = 1 + d*x^2*y^2     over GF(2^255 - 19)
//
// Why Twisted Edwards Curves?
// ===========================
// Edwards curves have a "complete" addition formula that works for ALL pairs
// of points -- including doubling and the identity. No special cases! This
// eliminates an entire class of timing side-channel attacks.
//
// The "twisted" variant (coefficient a = -1) enables faster arithmetic
// while preserving completeness.
//
// Swift BigInt Challenge
// ======================
// Swift does not have a built-in arbitrary-precision integer type. For
// Ed25519's 255-bit arithmetic, we implement a custom multi-precision
// integer using arrays of UInt64 "limbs" (base 2^64 digits).
//
// Each big integer is stored as [UInt64], least significant limb first:
//   [low64, mid64, mid-high64, high64]
//
// For a 255-bit prime, we need 4 limbs (4 * 64 = 256 bits).
//
// Dependencies
// ============
// SHA-512 from the SHA512 package in this monorepo.

import Foundation
import SHA512

// ============================================================================
// Multi-Precision Integer Arithmetic
// ============================================================================
//
// We represent big integers as [UInt64] arrays (little-endian limbs).
// Operations work on arbitrary-length arrays and produce correctly-sized
// results. This is "schoolbook" arithmetic -- simple, clear, and correct,
// though not the fastest possible implementation.

/// A big integer represented as little-endian UInt64 limbs.
/// bigint[0] is the least significant 64-bit word.
public typealias BigInt = [UInt64]

// ---- Constants as BigInt arrays ----

/// p = 2^255 - 19: the field prime
/// In hex: 7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffed
private let P: BigInt = [
    0xffffffffffffffed, 0xffffffffffffffff,
    0xffffffffffffffff, 0x7fffffffffffffff
]

/// d = -121665/121666 mod p (the curve parameter)
private let D: BigInt = [
    0x75eb4dca135978a3, 0x00700a4d4141d8ab,
    0x8cc740797779e898, 0x52036cee2b6ffe73
]

/// L = 2^252 + 27742317777372353535851937790883648493 (group order)
private let GROUP_ORDER: BigInt = [
    0x5812631a5cf5d3ed, 0x14def9dea2f79cd6,
    0x0000000000000000, 0x1000000000000000
]

/// SQRT_M1 = 2^((p-1)/4) mod p (square root of -1)
private let SQRT_M1: BigInt = [
    0xc4ee1b274a0ea0b0, 0x2f431806ad2fe478,
    0x2b4d00993dfbd7a7, 0x2b8324804fc1df0b
]

/// Base point x-coordinate (even root)
private let B_X: BigInt = [
    0xc9562d608f25d51a, 0x692cc7609525a7b2,
    0xc0a4e231fdd6dc5c, 0x216936d3cd6e53fe
]

/// Base point y-coordinate = 4/5 mod p
private let B_Y: BigInt = [
    0x6666666666666658, 0x6666666666666666,
    0x6666666666666666, 0x6666666666666666
]

/// Zero
private let ZERO: BigInt = [0, 0, 0, 0]

/// One
private let ONE: BigInt = [1, 0, 0, 0]

// ============================================================================
// Basic BigInt Operations
// ============================================================================

/// Compare two big integers. Returns -1, 0, or 1.
private func bigCmp(_ a: BigInt, _ b: BigInt) -> Int {
    let maxLen = max(a.count, b.count)
    for i in stride(from: maxLen - 1, through: 0, by: -1) {
        let aLimb = i < a.count ? a[i] : 0
        let bLimb = i < b.count ? b[i] : 0
        if aLimb < bLimb { return -1 }
        if aLimb > bLimb { return 1 }
    }
    return 0
}

/// Add two big integers, returning result (may be one limb longer).
private func bigAdd(_ a: BigInt, _ b: BigInt) -> BigInt {
    let maxLen = max(a.count, b.count)
    var result = BigInt(repeating: 0, count: maxLen + 1)
    var carry: UInt64 = 0
    for i in 0..<maxLen {
        let aLimb = i < a.count ? a[i] : 0
        let bLimb = i < b.count ? b[i] : 0
        let (s1, c1) = aLimb.addingReportingOverflow(bLimb)
        let (s2, c2) = s1.addingReportingOverflow(carry)
        result[i] = s2
        carry = (c1 ? 1 : 0) + (c2 ? 1 : 0)
    }
    result[maxLen] = carry
    return bigTrim(result)
}

/// Subtract b from a (assumes a >= b). Returns a - b.
private func bigSub(_ a: BigInt, _ b: BigInt) -> BigInt {
    var result = BigInt(repeating: 0, count: a.count)
    var borrow: UInt64 = 0
    for i in 0..<a.count {
        let bLimb = i < b.count ? b[i] : 0
        let (s1, c1) = a[i].subtractingReportingOverflow(bLimb)
        let (s2, c2) = s1.subtractingReportingOverflow(borrow)
        result[i] = s2
        borrow = (c1 ? 1 : 0) + (c2 ? 1 : 0)
    }
    return bigTrim(result)
}

/// Multiply two big integers (schoolbook algorithm).
private func bigMul(_ a: BigInt, _ b: BigInt) -> BigInt {
    let n = a.count + b.count
    var result = BigInt(repeating: 0, count: n)
    for i in 0..<a.count {
        var carry: UInt64 = 0
        for j in 0..<b.count {
            // Compute a[i] * b[j] + result[i+j] + carry
            let (hi, lo) = a[i].multipliedFullWidth(by: b[j])
            let (s1, c1) = lo.addingReportingOverflow(result[i + j])
            let (s2, c2) = s1.addingReportingOverflow(carry)
            result[i + j] = s2
            carry = hi &+ (c1 ? 1 : 0) &+ (c2 ? 1 : 0)
        }
        result[i + b.count] = carry
    }
    return bigTrim(result)
}

/// Divide a by b, returning (quotient, remainder).
/// Uses long division with UInt64 limbs.
private func bigDivMod(_ a: BigInt, _ b: BigInt) -> (BigInt, BigInt) {
    if bigCmp(a, b) < 0 {
        return (ZERO, a)
    }
    if b.count == 1 && b[0] == 0 {
        fatalError("Division by zero")
    }

    // Simple case: single-limb divisor
    if b.count == 1 {
        var remainder: UInt64 = 0
        var quotient = BigInt(repeating: 0, count: a.count)
        for i in stride(from: a.count - 1, through: 0, by: -1) {
            let (q, r) = b[0].dividingFullWidth((high: remainder, low: a[i]))
            quotient[i] = q
            remainder = r
        }
        return (bigTrim(quotient), bigTrim([remainder]))
    }

    // Multi-limb: use bit-by-bit long division
    var remainder: BigInt = [0]
    let totalBits = a.count * 64
    var quotient = BigInt(repeating: 0, count: a.count)

    for i in stride(from: totalBits - 1, through: 0, by: -1) {
        // Shift remainder left by 1 bit
        remainder = bigShiftLeft1(remainder)
        // Bring down bit i of a
        let limbIdx = i / 64
        let bitIdx = i % 64
        if limbIdx < a.count && (a[limbIdx] >> bitIdx) & 1 == 1 {
            remainder[0] |= 1
        }
        // If remainder >= b, subtract and set quotient bit
        if bigCmp(remainder, b) >= 0 {
            remainder = bigSub(remainder, b)
            let qLimb = i / 64
            let qBit = i % 64
            if qLimb < quotient.count {
                quotient[qLimb] |= (1 << qBit)
            }
        }
    }
    return (bigTrim(quotient), bigTrim(remainder))
}

/// Left shift by 1 bit.
private func bigShiftLeft1(_ a: BigInt) -> BigInt {
    var result = BigInt(repeating: 0, count: a.count + 1)
    var carry: UInt64 = 0
    for i in 0..<a.count {
        result[i] = (a[i] << 1) | carry
        carry = a[i] >> 63
    }
    result[a.count] = carry
    return bigTrim(result)
}

/// Remove leading zero limbs (but keep at least one).
private func bigTrim(_ a: BigInt) -> BigInt {
    var result = a
    while result.count > 1 && result.last == 0 {
        result.removeLast()
    }
    return result
}

/// Check if a big integer is zero.
private func bigIsZero(_ a: BigInt) -> Bool {
    return a.allSatisfy { $0 == 0 }
}

// ============================================================================
// Modular Arithmetic
// ============================================================================

/// Compute a mod m (always non-negative).
private func bigMod(_ a: BigInt, _ m: BigInt) -> BigInt {
    let (_, r) = bigDivMod(a, m)
    return r
}

/// Modular addition: (a + b) mod m
private func modAdd(_ a: BigInt, _ b: BigInt, _ m: BigInt) -> BigInt {
    return bigMod(bigAdd(a, b), m)
}

/// Modular subtraction: (a - b) mod m, always non-negative.
private func modSub(_ a: BigInt, _ b: BigInt, _ m: BigInt) -> BigInt {
    if bigCmp(a, b) >= 0 {
        return bigMod(bigSub(a, b), m)
    } else {
        // a < b: compute m - (b - a) mod m
        let diff = bigSub(b, a)
        let diffMod = bigMod(diff, m)
        if bigIsZero(diffMod) { return diffMod }
        return bigSub(m, diffMod)
    }
}

/// Modular multiplication: (a * b) mod m
private func modMul(_ a: BigInt, _ b: BigInt, _ m: BigInt) -> BigInt {
    return bigMod(bigMul(a, b), m)
}

/// Modular exponentiation: base^exp mod m (square-and-multiply).
private func modPow(_ base: BigInt, _ exp: BigInt, _ m: BigInt) -> BigInt {
    var result: BigInt = [1]
    var b = bigMod(base, m)
    var e = exp

    while !bigIsZero(e) {
        if e[0] & 1 == 1 {
            result = modMul(result, b, m)
        }
        b = modMul(b, b, m)
        // Right shift e by 1
        e = bigShiftRight1(e)
    }
    return result
}

/// Right shift by 1 bit.
private func bigShiftRight1(_ a: BigInt) -> BigInt {
    var result = BigInt(repeating: 0, count: a.count)
    for i in 0..<a.count {
        result[i] = a[i] >> 1
        if i + 1 < a.count {
            result[i] |= a[i + 1] << 63
        }
    }
    return bigTrim(result)
}

/// Modular inverse via Fermat's little theorem: a^(p-2) mod p
private func modInv(_ a: BigInt, _ m: BigInt) -> BigInt {
    let exp = bigSub(m, [2, 0, 0, 0])
    return modPow(a, exp, m)
}

/// Field square root using Atkin algorithm (for p = 5 mod 8).
private func fieldSqrt(_ a: BigInt) -> BigInt? {
    // exp = (P + 3) / 8
    let pPlus3 = bigAdd(P, [3, 0, 0, 0])
    let exp = bigShiftRight1(bigShiftRight1(bigShiftRight1(pPlus3)))
    let candidate = modPow(a, exp, P)
    let check = modMul(candidate, candidate, P)

    let aMod = bigMod(a, P)
    if bigCmp(check, aMod) == 0 {
        return candidate
    }

    // Check if candidate^2 == -a mod p
    let negA = modSub(ZERO, a, P)
    if bigCmp(check, negA) == 0 {
        return modMul(candidate, SQRT_M1, P)
    }

    return nil
}

// ============================================================================
// Conversion Helpers
// ============================================================================

/// Convert a Data (byte array) to a BigInt (little-endian).
private func bytesToBigInt(_ data: Data) -> BigInt {
    let limbCount = (data.count + 7) / 8
    var result = BigInt(repeating: 0, count: max(limbCount, 1))
    for i in 0..<data.count {
        let limbIdx = i / 8
        let bitOffset = (i % 8) * 8
        result[limbIdx] |= UInt64(data[i]) << bitOffset
    }
    return bigTrim(result)
}

/// Convert a BigInt to a fixed-length Data (little-endian).
private func bigIntToBytes(_ n: BigInt, count: Int) -> Data {
    var result = Data(count: count)
    for i in 0..<count {
        let limbIdx = i / 8
        let bitOffset = (i % 8) * 8
        if limbIdx < n.count {
            result[i] = UInt8((n[limbIdx] >> bitOffset) & 0xFF)
        }
    }
    return result
}

/// Convert a hex string to Data.
public func hexToData(_ hex: String) -> Data {
    var data = Data()
    let chars = Array(hex)
    for i in stride(from: 0, to: chars.count - 1, by: 2) {
        let byteStr = String(chars[i]) + String(chars[i + 1])
        if let byte = UInt8(byteStr, radix: 16) {
            data.append(byte)
        }
    }
    return data
}

/// Convert Data to hex string.
public func dataToHex(_ data: Data) -> String {
    return data.map { String(format: "%02x", $0) }.joined()
}

// ============================================================================
// Extended Point Representation
// ============================================================================
//
// Points on the curve are stored as (X, Y, Z, T) where:
//   x = X/Z, y = Y/Z, T = X*Y/Z
//
// This avoids expensive modular inversions during point arithmetic.

private struct ExtendedPoint {
    var X: BigInt
    var Y: BigInt
    var Z: BigInt
    var T: BigInt
}

private let IDENTITY = ExtendedPoint(X: ZERO, Y: ONE, Z: ONE, T: ZERO)

private let BASE_POINT = ExtendedPoint(
    X: B_X,
    Y: B_Y,
    Z: ONE,
    T: modMul(B_X, B_Y, P)
)

// ============================================================================
// Point Arithmetic
// ============================================================================

/// Add two points on the twisted Edwards curve (a = -1).
///
/// Uses the unified Hisil et al. (2008) formula:
///   A = X1*X2, B = Y1*Y2, C = T1*d*T2, D = Z1*Z2
///   E = (X1+Y1)*(X2+Y2) - A - B, F = D - C, G = D + C
///   H = B + A (because a = -1)
///   X3 = E*F, Y3 = G*H, Z3 = F*G, T3 = E*H
private func pointAdd(_ p1: ExtendedPoint, _ p2: ExtendedPoint) -> ExtendedPoint {
    let A = modMul(p1.X, p2.X, P)
    let B = modMul(p1.Y, p2.Y, P)
    let C = modMul(modMul(p1.T, D, P), p2.T, P)
    let DD = modMul(p1.Z, p2.Z, P)
    let sum1 = modAdd(p1.X, p1.Y, P)
    let sum2 = modAdd(p2.X, p2.Y, P)
    let E = modSub(modMul(sum1, sum2, P), modAdd(A, B, P), P)
    let F = modSub(DD, C, P)
    let G = modAdd(DD, C, P)
    let H = modAdd(B, A, P)  // B + A because a = -1

    return ExtendedPoint(
        X: modMul(E, F, P),
        Y: modMul(G, H, P),
        Z: modMul(F, G, P),
        T: modMul(E, H, P)
    )
}

/// Double a point on the twisted Edwards curve.
///
///   A = X1^2, B = Y1^2, C = 2*Z1^2
///   D = -A (because a = -1)
///   E = (X1+Y1)^2 - A - B, G = D + B, F = G - C, H = D - B
///   X3 = E*F, Y3 = G*H, Z3 = F*G, T3 = E*H
private func pointDouble(_ pt: ExtendedPoint) -> ExtendedPoint {
    let A = modMul(pt.X, pt.X, P)
    let B = modMul(pt.Y, pt.Y, P)
    // C = 2 * Z1^2
    let zSq = modMul(pt.Z, pt.Z, P)
    let CC = modAdd(zSq, zSq, P)
    let DD = modSub(ZERO, A, P)  // -A because a = -1
    let sum = modAdd(pt.X, pt.Y, P)
    let E = modSub(modMul(sum, sum, P), modAdd(A, B, P), P)
    let G = modAdd(DD, B, P)
    let F = modSub(G, CC, P)
    let H = modSub(DD, B, P)

    return ExtendedPoint(
        X: modMul(E, F, P),
        Y: modMul(G, H, P),
        Z: modMul(F, G, P),
        T: modMul(E, H, P)
    )
}

/// Scalar multiplication: n * point using double-and-add.
private func scalarMul(_ n: BigInt, _ point: ExtendedPoint) -> ExtendedPoint {
    var scalar = bigMod(n, GROUP_ORDER)
    if bigIsZero(scalar) { return IDENTITY }

    var result = IDENTITY
    var temp = point

    while !bigIsZero(scalar) {
        if scalar[0] & 1 == 1 {
            result = pointAdd(result, temp)
        }
        temp = pointDouble(temp)
        scalar = bigShiftRight1(scalar)
    }
    return result
}

// ============================================================================
// Point Encoding / Decoding
// ============================================================================

/// Encode a point as 32 bytes: y as LE with sign bit of x in high bit.
private func encodePoint(_ pt: ExtendedPoint) -> Data {
    let zInv = modInv(pt.Z, P)
    let x = modMul(pt.X, zInv, P)
    let y = modMul(pt.Y, zInv, P)

    var encoded = bigIntToBytes(y, count: 32)
    // Set high bit of byte 31 to low bit of x
    if x[0] & 1 == 1 {
        encoded[31] |= 0x80
    }
    return encoded
}

/// Decode a 32-byte point encoding. Returns nil if invalid.
private func decodePoint(_ data: Data) -> ExtendedPoint? {
    guard data.count == 32 else { return nil }

    // Extract sign bit
    let sign: UInt64 = UInt64((data[31] >> 7) & 1)

    // Decode y (clear sign bit)
    var yBytes = data
    yBytes[31] &= 0x7F
    let y = bytesToBigInt(yBytes)

    // Reject y >= p
    if bigCmp(y, P) >= 0 { return nil }

    // Compute x^2 = (y^2 - 1) / (d*y^2 + 1) mod p
    let y2 = modMul(y, y, P)
    let num = modSub(y2, ONE, P)
    let den = modAdd(modMul(D, y2, P), ONE, P)
    let x2 = modMul(num, modInv(den, P), P)

    if bigIsZero(x2) {
        if sign != 0 { return nil }
        return ExtendedPoint(X: ZERO, Y: y, Z: ONE, T: ZERO)
    }

    guard var x = fieldSqrt(x2) else { return nil }

    // Correct sign
    if (x[0] & 1) != sign {
        x = modSub(ZERO, x, P)
    }

    return ExtendedPoint(X: x, Y: y, Z: ONE, T: modMul(x, y, P))
}

// ============================================================================
// Key Clamping
// ============================================================================

/// Clamp a 32-byte scalar for Ed25519 key derivation.
///
/// - Clear lowest 3 bits: makes scalar divisible by 8 (cofactor)
/// - Clear bit 255: keeps scalar < 2^255
/// - Set bit 254: ensures fixed bit length
private func clampScalar(_ data: Data) -> Data {
    var clamped = Data(data.prefix(32))
    clamped[0] &= 248
    clamped[31] &= 127
    clamped[31] |= 64
    return clamped
}

// ============================================================================
// Public API
// ============================================================================

/// An Ed25519 keypair.
public struct Ed25519Keypair: Sendable {
    /// 32-byte public key (compressed curve point)
    public let publicKey: Data
    /// 64-byte secret key (seed || publicKey)
    public let secretKey: Data
}

/// Generate an Ed25519 keypair from a 32-byte seed.
///
/// The seed is the true secret. Key derivation is deterministic:
/// the same seed always produces the same keypair.
///
/// - Parameter seed: 32-byte random seed
/// - Returns: Ed25519Keypair with publicKey and secretKey
public func generateKeypair(seed: Data) -> Ed25519Keypair {
    precondition(seed.count == 32, "Seed must be 32 bytes")

    let hash = sha512(seed)
    let clamped = clampScalar(hash)
    let a = bytesToBigInt(clamped)

    let pubPoint = scalarMul(a, BASE_POINT)
    let publicKey = encodePoint(pubPoint)

    var secretKey = Data()
    secretKey.append(seed)
    secretKey.append(publicKey)

    return Ed25519Keypair(publicKey: publicKey, secretKey: secretKey)
}

/// Sign a message with an Ed25519 secret key.
///
/// The signature is deterministic: same message + key always produces
/// the same 64-byte signature.
///
/// - Parameters:
///   - message: The message to sign (any length)
///   - secretKey: 64-byte secret key from generateKeypair
/// - Returns: 64-byte signature (R || S)
public func ed25519Sign(message: Data, secretKey: Data) -> Data {
    precondition(secretKey.count == 64, "Secret key must be 64 bytes")

    let seed = secretKey.prefix(32)
    let publicKey = secretKey.suffix(32)

    let hash = sha512(Data(seed))
    let clamped = clampScalar(hash)
    let a = bytesToBigInt(clamped)
    let prefix = hash.suffix(32)

    // Deterministic nonce: r = SHA-512(prefix || message) mod L
    var rInput = Data()
    rInput.append(prefix)
    rInput.append(message)
    let rHash = sha512(rInput)
    let rScalar = bigMod(bytesToBigInt(rHash), GROUP_ORDER)

    // Commitment: R = r * B
    let R = encodePoint(scalarMul(rScalar, BASE_POINT))

    // Challenge: k = SHA-512(R || A || message) mod L
    var kInput = Data()
    kInput.append(R)
    kInput.append(publicKey)
    kInput.append(message)
    let kHash = sha512(kInput)
    let k = bigMod(bytesToBigInt(kHash), GROUP_ORDER)

    // Response: S = (r + k * a) mod L
    let ka = modMul(k, a, GROUP_ORDER)
    let S = modAdd(rScalar, ka, GROUP_ORDER)

    var signature = Data()
    signature.append(R)
    signature.append(bigIntToBytes(S, count: 32))
    return signature
}

/// Verify an Ed25519 signature.
///
/// Checks: S * B == R + k * A where k = SHA-512(R || A || message) mod L
///
/// - Parameters:
///   - message: The message that was signed
///   - signature: 64-byte signature (R || S)
///   - publicKey: 32-byte public key
/// - Returns: true if the signature is valid
public func ed25519Verify(message: Data, signature: Data, publicKey: Data) -> Bool {
    guard signature.count == 64 else { return false }
    guard publicKey.count == 32 else { return false }

    let rBytes = Data(signature.prefix(32))
    guard let rPoint = decodePoint(rBytes) else { return false }

    let sScalar = bytesToBigInt(Data(signature.suffix(32)))
    guard bigCmp(sScalar, GROUP_ORDER) < 0 else { return false }

    guard let aPoint = decodePoint(publicKey) else { return false }

    // k = SHA-512(R || A || message) mod L
    var kInput = Data()
    kInput.append(rBytes)
    kInput.append(publicKey)
    kInput.append(message)
    let kHash = sha512(kInput)
    let k = bigMod(bytesToBigInt(kHash), GROUP_ORDER)

    // Check: S * B == R + k * A
    let lhs = scalarMul(sScalar, BASE_POINT)
    let rhs = pointAdd(rPoint, scalarMul(k, aPoint))

    return encodePoint(lhs) == encodePoint(rhs)
}

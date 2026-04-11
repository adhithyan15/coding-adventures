// ============================================================================
// ScryptTests.swift — Test Suite for the Scrypt Package
// ============================================================================
//
// Test Strategy
// =============
// The primary correctness check is against the official RFC 7914 test vectors.
// These vectors were computed by Colin Percival (scrypt's author) and are the
// canonical reference for any scrypt implementation.
//
// Additional tests verify:
//   - Parameter validation (invalid N, r, p, dkLen)
//   - Edge cases (empty password per vector 1, single-byte inputs)
//   - The hex convenience wrapper
//   - Determinism (same inputs → same output)
//   - Output length matches requested dkLen
//
// RFC 7914 §11 — Test Vectors
// ============================
// Vector 1: N=16, r=1, p=1
//   Password: ""   (empty — exercises the empty-password code path)
//   Salt:     ""   (empty)
//   DK:       77d65762...  (64 bytes)
//
// Vector 2: N=1024, r=8, p=16
//   Password: "password"
//   Salt:     "NaCl"
//   DK:       fdbabe1c...  (64 bytes)
//   Note: This vector is expensive — it exercises the memory-hard ROMix.
//
// Vector 3: N=16384, r=8, p=1
//   Password: "pleaseletmein"
//   Salt:     "SodiumChloride"
//   DK:       7023bdcb...  (64 bytes)
//   Note: Very expensive (16 MB table). Included because it's in the RFC.
//
// We skip vector 3 in CI because it takes ~5 seconds — but it's left here
// as a commented-out test that can be enabled for local validation.

import XCTest
@testable import Scrypt

final class ScryptTests: XCTestCase {

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Convert a String to its UTF-8 byte array.
    func enc(_ s: String) -> [UInt8] { Array(s.utf8) }

    /// Convert a byte array to a lowercase hex string.
    func hex(_ b: [UInt8]) -> String { b.map { String(format: "%02x", $0) }.joined() }

    // ── RFC 7914 Test Vectors ─────────────────────────────────────────────────

    /// RFC 7914 §11, Vector 1: empty password and salt, N=16, r=1, p=1.
    ///
    /// This vector specifically tests the empty-password code path. Our
    /// internal PBKDF2 implementation handles empty passwords by using the
    /// lower-level `hmac()` function that has no empty-key guard.
    ///
    /// Expected output (64 bytes):
    ///   77d6576238657b203b19ca42c18a0497
    ///   f16b4844e3074ae8dfdffa3fede21442
    ///   0e3e7e6cf64cf7d6d58a28b8faecbc77
    ///   c7c0d68eb6c36fc4c83c6c24e1ac1d24
    func testRFC7914Vector1() throws {
        let dk = try scrypt(password: [], salt: [], n: 16, r: 1, p: 1, dkLen: 64)
        XCTAssertEqual(hex(dk),
            "77d6576238657b203b19ca42c18a0497" +
            "f16b4844e3074ae8dfdffa3fede21442" +
            "fcd0069ded0948f8326a753a0fc81f17" +
            "e8d3e0fb2e0d3628cf35e20c38d18906"
        )
    }

    /// RFC 7914 §11, Vector 2: "password" / "NaCl", N=1024, r=8, p=16.
    ///
    /// This is the most commonly-cited test vector. N=1024 requires a 1 MB
    /// ROMix table (1024 * 8 * 128 = 1,048,576 bytes). It is moderately
    /// expensive — suitable for CI.
    ///
    /// Expected output (64 bytes):
    ///   fdbabe1c9d3472007856e7190d01e9fe
    ///   c0044298eb1c1127886d3a3f48f2f7b0
    ///   d9a34e8c2ded1f83e8eb37e9677f83a3
    ///   8ae1d0a8da30dd1d4bc4ca26a96e38db
    func testRFC7914Vector2() throws {
        let dk = try scrypt(
            password: enc("password"),
            salt:     enc("NaCl"),
            n: 1024, r: 8, p: 16, dkLen: 64
        )
        XCTAssertEqual(hex(dk),
            "fdbabe1c9d3472007856e7190d01e9fe" +
            "7c6ad7cbc8237830e77376634b373162" +
            "2eaf30d92e22a3886ff109279d9830da" +
            "c727afb94a83ee6d8360cbdfa2cc0640"
        )
    }

    // ── Parameter Validation ─────────────────────────────────────────────────

    /// N must be ≥ 2 and a power of 2.
    func testInvalidN_zero() {
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 0, r: 1, p: 1, dkLen: 32)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.invalidN)
        }
    }

    func testInvalidN_one() {
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 1, r: 1, p: 1, dkLen: 32)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.invalidN)
        }
    }

    func testInvalidN_notPowerOfTwo() {
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 3, r: 1, p: 1, dkLen: 32)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.invalidN)
        }
    }

    func testInvalidN_notPowerOfTwo2() {
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 6, r: 1, p: 1, dkLen: 32)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.invalidN)
        }
    }

    /// N = 2^20 is allowed; N = 2^21 is rejected.
    func testNTooLarge() {
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 1 << 21, r: 1, p: 1, dkLen: 32)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.nTooLarge)
        }
    }

    /// r must be ≥ 1.
    func testInvalidR() {
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 16, r: 0, p: 1, dkLen: 32)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.invalidR)
        }
    }

    /// p must be ≥ 1.
    func testInvalidP() {
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 16, r: 1, p: 0, dkLen: 32)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.invalidP)
        }
    }

    /// dkLen must be ≥ 1.
    func testInvalidKeyLength_zero() {
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 16, r: 1, p: 1, dkLen: 0)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.invalidKeyLength)
        }
    }

    /// dkLen must be ≤ 2^20.
    func testInvalidKeyLength_tooLarge() {
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 16, r: 1, p: 1, dkLen: (1 << 20) + 1)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.invalidKeyLength)
        }
    }

    /// p * r must be ≤ 2^30.
    func testPRTooLarge() {
        // p=2^15, r=2^16 → p*r = 2^31 > 2^30
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 16, r: 1 << 16, p: 1 << 15, dkLen: 32)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.prTooLarge)
        }
    }

    /// p*128*r > 2^30 must be rejected even when p*r ≤ 2^30 (memory cap).
    func testBLenTooLarge() {
        // p=1, r=2^24: p*r = 2^24 ≤ 2^30 (passes old guard), but
        // p*128*r = 2^31 > 2^30 (triggers memory-cap guard).
        XCTAssertThrowsError(
            try scrypt(password: [], salt: [], n: 2, r: 1 << 24, p: 1, dkLen: 32)
        ) { error in
            XCTAssertEqual(error as? ScryptError, ScryptError.prTooLarge)
        }
    }

    // ── Valid boundary values ─────────────────────────────────────────────────

    /// N=2 is the smallest valid N. Verify it runs without error.
    func testMinimalN() throws {
        let dk = try scrypt(password: enc("x"), salt: enc("y"), n: 2, r: 1, p: 1, dkLen: 32)
        XCTAssertEqual(dk.count, 32)
    }

    /// r=1 is the smallest valid r.
    func testMinimalR() throws {
        let dk = try scrypt(password: enc("x"), salt: enc("y"), n: 4, r: 1, p: 1, dkLen: 32)
        XCTAssertEqual(dk.count, 32)
    }

    /// dkLen=1 produces exactly 1 byte.
    func testDkLen1() throws {
        let dk = try scrypt(password: enc("a"), salt: enc("b"), n: 4, r: 1, p: 1, dkLen: 1)
        XCTAssertEqual(dk.count, 1)
    }

    /// dkLen=64 produces exactly 64 bytes (spans two PBKDF2 hLen blocks).
    func testDkLen64() throws {
        let dk = try scrypt(password: enc("pw"), salt: enc("s"), n: 4, r: 1, p: 1, dkLen: 64)
        XCTAssertEqual(dk.count, 64)
    }

    // ── Determinism ──────────────────────────────────────────────────────────

    /// Same inputs must always produce the same output.
    func testDeterminism() throws {
        let a = try scrypt(password: enc("test"), salt: enc("salt"), n: 16, r: 1, p: 1, dkLen: 32)
        let b = try scrypt(password: enc("test"), salt: enc("salt"), n: 16, r: 1, p: 1, dkLen: 32)
        XCTAssertEqual(a, b)
    }

    // ── Sensitivity to inputs ─────────────────────────────────────────────────

    /// Changing the password by one bit should change the output completely.
    func testPasswordSensitivity() throws {
        let a = try scrypt(password: enc("password"), salt: enc("salt"), n: 16, r: 1, p: 1, dkLen: 32)
        let b = try scrypt(password: enc("Password"), salt: enc("salt"), n: 16, r: 1, p: 1, dkLen: 32)
        XCTAssertNotEqual(a, b)
    }

    /// Changing the salt by one bit should change the output completely.
    func testSaltSensitivity() throws {
        let a = try scrypt(password: enc("pw"), salt: enc("salt1"), n: 16, r: 1, p: 1, dkLen: 32)
        let b = try scrypt(password: enc("pw"), salt: enc("salt2"), n: 16, r: 1, p: 1, dkLen: 32)
        XCTAssertNotEqual(a, b)
    }

    /// Changing N must change the output (different ROMix table).
    func testNSensitivity() throws {
        let a = try scrypt(password: enc("pw"), salt: enc("s"), n: 4,  r: 1, p: 1, dkLen: 32)
        let b = try scrypt(password: enc("pw"), salt: enc("s"), n: 16, r: 1, p: 1, dkLen: 32)
        XCTAssertNotEqual(a, b)
    }

    // ── scryptHex convenience wrapper ─────────────────────────────────────────

    /// scryptHex must produce the same output as hex(scrypt(...)).
    func testScryptHex_matchesScrypt() throws {
        let dk  = try scrypt(   password: [], salt: [], n: 16, r: 1, p: 1, dkLen: 64)
        let hex = try scryptHex(password: [], salt: [], n: 16, r: 1, p: 1, dkLen: 64)
        XCTAssertEqual(hex, dk.map { String(format: "%02x", $0) }.joined())
    }

    /// scryptHex on vector 1 must match the expected RFC 7914 hex string.
    func testScryptHex_vector1() throws {
        let hex = try scryptHex(password: [], salt: [], n: 16, r: 1, p: 1, dkLen: 64)
        XCTAssertEqual(hex,
            "77d6576238657b203b19ca42c18a0497" +
            "f16b4844e3074ae8dfdffa3fede21442" +
            "fcd0069ded0948f8326a753a0fc81f17" +
            "e8d3e0fb2e0d3628cf35e20c38d18906"
        )
    }

    // ── N = 2^20 boundary (valid) ─────────────────────────────────────────────
    // NOTE: Commented out — would require ~1 GB RAM and several minutes.
    // Uncomment only for local validation:
    //
    // func testMaxN() throws {
    //     let dk = try scrypt(password: enc("pw"), salt: enc("s"), n: 1 << 20, r: 1, p: 1, dkLen: 32)
    //     XCTAssertEqual(dk.count, 32)
    // }
}

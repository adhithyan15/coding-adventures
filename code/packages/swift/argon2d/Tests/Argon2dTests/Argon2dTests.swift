import XCTest
@testable import Argon2d

final class Argon2dTests: XCTestCase {
    // --- helpers ---
    func bytes(_ s: String) -> [UInt8] { return Array(s.utf8) }
    func repeated(_ byte: UInt8, _ n: Int) -> [UInt8] {
        return [UInt8](repeating: byte, count: n)
    }

    // RFC 9106 §5.1 gold-standard Argon2d vector.
    func testRfc9106Section5_1Vector() throws {
        let tag = try Argon2d.argon2dHex(
            password: repeated(0x01, 32),
            salt:     repeated(0x02, 16),
            timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32,
            key: repeated(0x03, 8),
            associatedData: repeated(0x04, 12)
        )
        XCTAssertEqual(tag,
            "512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb")
    }

    func testHexMatchesBinary() throws {
        let raw = try Argon2d.argon2d(
            password: repeated(0x01, 32), salt: repeated(0x02, 16),
            timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32,
            key: repeated(0x03, 8), associatedData: repeated(0x04, 12))
        let hex = try Argon2d.argon2dHex(
            password: repeated(0x01, 32), salt: repeated(0x02, 16),
            timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32,
            key: repeated(0x03, 8), associatedData: repeated(0x04, 12))
        XCTAssertEqual(raw.map { String(format: "%02x", $0) }.joined(), hex)
    }

    // --- Validation rejections ---
    func testRejectsShortSalt() {
        XCTAssertThrowsError(try Argon2d.argon2d(
            password: bytes("pw"), salt: bytes("short"),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32))
    }

    func testRejectsZeroTimeCost() {
        XCTAssertThrowsError(try Argon2d.argon2d(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 0, memoryCost: 8, parallelism: 1, tagLength: 32))
    }

    func testRejectsTagLengthUnder4() {
        XCTAssertThrowsError(try Argon2d.argon2d(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 3))
    }

    func testRejectsMemoryUnder8p() {
        XCTAssertThrowsError(try Argon2d.argon2d(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 7, parallelism: 1, tagLength: 32))
    }

    func testRejectsZeroParallelism() {
        XCTAssertThrowsError(try Argon2d.argon2d(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 0, tagLength: 32))
    }

    func testRejectsUnsupportedVersion() {
        XCTAssertThrowsError(try Argon2d.argon2d(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32,
            version: 0x10))
    }

    // --- Determinism and separation ---
    func testDeterministic() throws {
        let a = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        let b = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        XCTAssertEqual(a, b)
    }

    func testPasswordDifferentiates() throws {
        let a = try Argon2d.argon2dHex(
            password: bytes("pw1"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        let b = try Argon2d.argon2dHex(
            password: bytes("pw2"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        XCTAssertNotEqual(a, b)
    }

    func testSaltDifferentiates() throws {
        let a = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        let b = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x62, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        XCTAssertNotEqual(a, b)
    }

    // --- Key and AD binding ---
    func testKeyBinds() throws {
        let none = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        let k1 = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32,
            key: bytes("k1"))
        let k2 = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32,
            key: bytes("k2"))
        XCTAssertNotEqual(none, k1)
        XCTAssertNotEqual(k1, k2)
    }

    func testAdBinds() throws {
        let none = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        let a1 = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32,
            associatedData: bytes("x"))
        let a2 = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32,
            associatedData: bytes("y"))
        XCTAssertNotEqual(none, a1)
        XCTAssertNotEqual(a1, a2)
    }

    // --- Tag length variants (exercises H' branches) ---
    func testTagLengths() throws {
        for t in [4, 16, 65, 128] {
            let out = try Argon2d.argon2d(
                password: bytes("pw"), salt: repeated(0x61, 8),
                timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: t)
            XCTAssertEqual(out.count, t, "tag length \(t)")
        }
    }

    // --- Parallelism and multi-pass ---
    func testMultiLaneTagSize() throws {
        let out = try Argon2d.argon2d(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 16, parallelism: 2, tagLength: 32)
        XCTAssertEqual(out.count, 32)
    }

    func testMultiPassDiffersFromSinglePass() throws {
        let a = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        let b = try Argon2d.argon2dHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 2, memoryCost: 8, parallelism: 1, tagLength: 32)
        XCTAssertNotEqual(a, b)
    }
}

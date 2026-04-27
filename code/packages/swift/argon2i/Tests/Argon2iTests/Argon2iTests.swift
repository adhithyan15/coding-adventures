import XCTest
@testable import Argon2i

final class Argon2iTests: XCTestCase {
    func bytes(_ s: String) -> [UInt8] { return Array(s.utf8) }
    func repeated(_ byte: UInt8, _ n: Int) -> [UInt8] {
        return [UInt8](repeating: byte, count: n)
    }

    // RFC 9106 §5.2 gold-standard Argon2i vector.
    func testRfc9106Section5_2Vector() throws {
        let tag = try Argon2i.argon2iHex(
            password: repeated(0x01, 32),
            salt:     repeated(0x02, 16),
            timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32,
            key: repeated(0x03, 8),
            associatedData: repeated(0x04, 12)
        )
        XCTAssertEqual(tag,
            "c814d9d1dc7f37aa13f0d77f2494bda1c8de6b016dd388d29952a4c4672b6ce8")
    }

    func testHexMatchesBinary() throws {
        let raw = try Argon2i.argon2i(
            password: repeated(0x01, 32), salt: repeated(0x02, 16),
            timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32,
            key: repeated(0x03, 8), associatedData: repeated(0x04, 12))
        let hex = try Argon2i.argon2iHex(
            password: repeated(0x01, 32), salt: repeated(0x02, 16),
            timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32,
            key: repeated(0x03, 8), associatedData: repeated(0x04, 12))
        XCTAssertEqual(raw.map { String(format: "%02x", $0) }.joined(), hex)
    }

    func testRejectsShortSalt() {
        XCTAssertThrowsError(try Argon2i.argon2i(
            password: bytes("pw"), salt: bytes("short"),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32))
    }

    func testRejectsZeroTimeCost() {
        XCTAssertThrowsError(try Argon2i.argon2i(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 0, memoryCost: 8, parallelism: 1, tagLength: 32))
    }

    func testRejectsTagLengthUnder4() {
        XCTAssertThrowsError(try Argon2i.argon2i(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 3))
    }

    func testRejectsMemoryUnder8p() {
        XCTAssertThrowsError(try Argon2i.argon2i(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 7, parallelism: 1, tagLength: 32))
    }

    func testRejectsZeroParallelism() {
        XCTAssertThrowsError(try Argon2i.argon2i(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 0, tagLength: 32))
    }

    func testRejectsUnsupportedVersion() {
        XCTAssertThrowsError(try Argon2i.argon2i(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32,
            version: 0x10))
    }

    func testDeterministic() throws {
        let a = try Argon2i.argon2iHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        let b = try Argon2i.argon2iHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        XCTAssertEqual(a, b)
    }

    func testPasswordAndSaltDifferentiate() throws {
        let base = try Argon2i.argon2iHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        let diffPw = try Argon2i.argon2iHex(
            password: bytes("pw2"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        let diffSalt = try Argon2i.argon2iHex(
            password: bytes("pw"), salt: repeated(0x62, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        XCTAssertNotEqual(base, diffPw)
        XCTAssertNotEqual(base, diffSalt)
    }

    func testKeyAndAdBind() throws {
        let none = try Argon2i.argon2iHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32)
        let keyed = try Argon2i.argon2iHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32,
            key: bytes("k"))
        let ad = try Argon2i.argon2iHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 32,
            associatedData: bytes("a"))
        XCTAssertNotEqual(none, keyed)
        XCTAssertNotEqual(none, ad)
    }

    func testTagLengths() throws {
        for t in [4, 16, 65, 128] {
            let out = try Argon2i.argon2i(
                password: bytes("pw"), salt: repeated(0x61, 8),
                timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: t)
            XCTAssertEqual(out.count, t)
        }
    }

    func testMultiLaneAndMultiPass() throws {
        let a = try Argon2i.argon2iHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 1, memoryCost: 16, parallelism: 2, tagLength: 32)
        let b = try Argon2i.argon2iHex(
            password: bytes("pw"), salt: repeated(0x61, 8),
            timeCost: 2, memoryCost: 16, parallelism: 2, tagLength: 32)
        XCTAssertNotEqual(a, b)
    }
}

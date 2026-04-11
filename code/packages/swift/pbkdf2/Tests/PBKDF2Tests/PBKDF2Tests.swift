import Testing
import Foundation
@testable import PBKDF2

// Helper: decode a hex string to Data.
private func fromHex(_ s: String) -> Data {
    var data = Data()
    var index = s.startIndex
    while index < s.endIndex {
        let next = s.index(index, offsetBy: 2)
        data.append(UInt8(s[index..<next], radix: 16)!)
        index = next
    }
    return data
}

// ─────────────────────────────────────────────────────────────────────────────
// RFC 6070 — PBKDF2-HMAC-SHA1
// ─────────────────────────────────────────────────────────────────────────────

@Suite("RFC 6070 PBKDF2-HMAC-SHA1")
struct RFC6070Tests {
    @Test func vector1_c1() throws {
        let dk = try pbkdf2HmacSHA1(
            password: Data("password".utf8),
            salt: Data("salt".utf8),
            iterations: 1,
            keyLength: 20
        )
        #expect(dk == fromHex("0c60c80f961f0e71f3a9b524af6012062fe037a6"))
    }

    @Test func vector2_c4096() throws {
        let dk = try pbkdf2HmacSHA1(
            password: Data("password".utf8),
            salt: Data("salt".utf8),
            iterations: 4096,
            keyLength: 20
        )
        #expect(dk == fromHex("4b007901b765489abead49d926f721d065a429c1"))
    }

    @Test func vector3_longPasswordSalt() throws {
        let dk = try pbkdf2HmacSHA1(
            password: Data("passwordPASSWORDpassword".utf8),
            salt: Data("saltSALTsaltSALTsaltSALTsaltSALTsalt".utf8),
            iterations: 4096,
            keyLength: 25
        )
        #expect(dk == fromHex("3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038"))
    }

    @Test func vector4_nullBytes() throws {
        var pw = Data("pass".utf8); pw.append(0x00); pw.append(contentsOf: "word".utf8)
        var s = Data("sa".utf8); s.append(0x00); s.append(contentsOf: "lt".utf8)
        let dk = try pbkdf2HmacSHA1(password: pw, salt: s, iterations: 4096, keyLength: 16)
        #expect(dk == fromHex("56fa6aa75548099dcc37d7f03425e0c3"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RFC 7914 — PBKDF2-HMAC-SHA256
// ─────────────────────────────────────────────────────────────────────────────

@Suite("RFC 7914 PBKDF2-HMAC-SHA256")
struct RFC7914Tests {
    @Test func vector1_c1_64bytes() throws {
        let dk = try pbkdf2HmacSHA256(
            password: Data("passwd".utf8),
            salt: Data("salt".utf8),
            iterations: 1,
            keyLength: 64
        )
        let expected = fromHex(
            "55ac046e56e3089fec1691c22544b605" +
            "f94185216dde0465e68b9d57c20dacbc" +
            "49ca9cccf179b645991664b39d77ef31" +
            "7c71b845b1e30bd509112041d3a19783"
        )
        #expect(dk == expected)
    }

    @Test func outputLength() throws {
        let dk = try pbkdf2HmacSHA256(
            password: Data("key".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 32
        )
        #expect(dk.count == 32)
    }

    @Test func truncationConsistency() throws {
        let short = try pbkdf2HmacSHA256(
            password: Data("key".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 16
        )
        let full = try pbkdf2HmacSHA256(
            password: Data("key".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 32
        )
        #expect(short == full.prefix(16))
    }

    @Test func multiBlock() throws {
        let dk64 = try pbkdf2HmacSHA256(
            password: Data("password".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 64
        )
        let dk32 = try pbkdf2HmacSHA256(
            password: Data("password".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 32
        )
        #expect(dk64.count == 64)
        #expect(dk64.prefix(32) == dk32)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SHA-512 sanity checks
// ─────────────────────────────────────────────────────────────────────────────

@Suite("PBKDF2-HMAC-SHA512")
struct SHA512Tests {
    @Test func outputLength() throws {
        let dk = try pbkdf2HmacSHA512(
            password: Data("secret".utf8), salt: Data("nacl".utf8), iterations: 1, keyLength: 64
        )
        #expect(dk.count == 64)
    }

    @Test func truncation() throws {
        let short = try pbkdf2HmacSHA512(
            password: Data("secret".utf8), salt: Data("nacl".utf8), iterations: 1, keyLength: 32
        )
        let full = try pbkdf2HmacSHA512(
            password: Data("secret".utf8), salt: Data("nacl".utf8), iterations: 1, keyLength: 64
        )
        #expect(short == full.prefix(32))
    }

    @Test func multiBlock128() throws {
        let dk = try pbkdf2HmacSHA512(
            password: Data("key".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 128
        )
        #expect(dk.count == 128)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hex variants
// ─────────────────────────────────────────────────────────────────────────────

@Suite("Hex variants")
struct HexTests {
    @Test func sha1HexRFC6070() throws {
        let h = try pbkdf2HmacSHA1Hex(
            password: Data("password".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 20
        )
        #expect(h == "0c60c80f961f0e71f3a9b524af6012062fe037a6")
    }

    @Test func sha256HexMatchesBytes() throws {
        let dk = try pbkdf2HmacSHA256(
            password: Data("passwd".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 32
        )
        let h = try pbkdf2HmacSHA256Hex(
            password: Data("passwd".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 32
        )
        #expect(h == dk.map { String(format: "%02x", $0) }.joined())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation
// ─────────────────────────────────────────────────────────────────────────────

@Suite("Validation")
struct ValidationTests {
    @Test func emptyPasswordThrows() {
        #expect(throws: PBKDF2Error.emptyPassword) {
            try pbkdf2HmacSHA256(
                password: Data(), salt: Data("salt".utf8), iterations: 1, keyLength: 32
            )
        }
    }

    @Test func zeroIterationsThrows() {
        #expect(throws: PBKDF2Error.invalidIterations) {
            try pbkdf2HmacSHA256(
                password: Data("pw".utf8), salt: Data("salt".utf8), iterations: 0, keyLength: 32
            )
        }
    }

    @Test func zeroKeyLengthThrows() {
        #expect(throws: PBKDF2Error.invalidKeyLength) {
            try pbkdf2HmacSHA256(
                password: Data("pw".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 0
            )
        }
    }

    @Test func emptySaltAllowed() throws {
        let dk = try pbkdf2HmacSHA256(
            password: Data("password".utf8), salt: Data(), iterations: 1, keyLength: 32
        )
        #expect(dk.count == 32)
    }

    @Test func deterministic() throws {
        let a = try pbkdf2HmacSHA256(
            password: Data("secret".utf8), salt: Data("nacl".utf8), iterations: 100, keyLength: 32
        )
        let b = try pbkdf2HmacSHA256(
            password: Data("secret".utf8), salt: Data("nacl".utf8), iterations: 100, keyLength: 32
        )
        #expect(a == b)
    }

    @Test func differentSalts() throws {
        let a = try pbkdf2HmacSHA256(
            password: Data("password".utf8), salt: Data("salt1".utf8), iterations: 1, keyLength: 32
        )
        let b = try pbkdf2HmacSHA256(
            password: Data("password".utf8), salt: Data("salt2".utf8), iterations: 1, keyLength: 32
        )
        #expect(a != b)
    }

    @Test func differentPasswords() throws {
        let a = try pbkdf2HmacSHA256(
            password: Data("password1".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 32
        )
        let b = try pbkdf2HmacSHA256(
            password: Data("password2".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 32
        )
        #expect(a != b)
    }

    @Test func differentIterations() throws {
        let a = try pbkdf2HmacSHA256(
            password: Data("password".utf8), salt: Data("salt".utf8), iterations: 1, keyLength: 32
        )
        let b = try pbkdf2HmacSHA256(
            password: Data("password".utf8), salt: Data("salt".utf8), iterations: 2, keyLength: 32
        )
        #expect(a != b)
    }
}

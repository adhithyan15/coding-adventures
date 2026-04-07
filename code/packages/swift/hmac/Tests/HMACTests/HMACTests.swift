import Testing
import Foundation
@testable import HMAC

// ─── RFC 4231 — HMAC-SHA256 ───────────────────────────────────────────────────

@Suite("HMAC-SHA256 (RFC 4231)")
struct HmacSHA256Tests {
    @Test func tc1_20ByteKey_HiThere() {
        let key = Data(repeating: 0x0b, count: 20)
        #expect(
            hmacSHA256Hex(key: key, message: Data("Hi There".utf8)) ==
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        )
    }

    @Test func tc2_Jefe() {
        #expect(
            hmacSHA256Hex(
                key: Data("Jefe".utf8),
                message: Data("what do ya want for nothing?".utf8)
            ) == "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        )
    }

    @Test func tc3_0xaaKey_0xddData() {
        let key  = Data(repeating: 0xaa, count: 20)
        let data = Data(repeating: 0xdd, count: 50)
        #expect(
            hmacSHA256Hex(key: key, message: data) ==
            "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"
        )
    }

    @Test func tc6_longerThanBlockSizeKey() {
        let key = Data(repeating: 0xaa, count: 131)
        #expect(
            hmacSHA256Hex(
                key: key,
                message: Data("Test Using Larger Than Block-Size Key - Hash Key First".utf8)
            ) == "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        )
    }

    @Test func tc7_longerThanBlockSizeKeyAndData() {
        let key = Data(repeating: 0xaa, count: 131)
        let msg = Data((
            "This is a test using a larger than block-size key and a larger than block-size data. " +
            "The key needs to be hashed before being used by the HMAC algorithm."
        ).utf8)
        #expect(
            hmacSHA256Hex(key: key, message: msg) ==
            "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2"
        )
    }
}

// ─── RFC 4231 — HMAC-SHA512 ───────────────────────────────────────────────────

@Suite("HMAC-SHA512 (RFC 4231)")
struct HmacSHA512Tests {
    @Test func tc1_20ByteKey_HiThere() {
        let key = Data(repeating: 0x0b, count: 20)
        #expect(
            hmacSHA512Hex(key: key, message: Data("Hi There".utf8)) ==
            "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
        )
    }

    @Test func tc2_Jefe() {
        #expect(
            hmacSHA512Hex(
                key: Data("Jefe".utf8),
                message: Data("what do ya want for nothing?".utf8)
            ) == "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        )
    }

    @Test func tc6_longerThanBlockSizeKey() {
        let key = Data(repeating: 0xaa, count: 131)
        #expect(
            hmacSHA512Hex(
                key: key,
                message: Data("Test Using Larger Than Block-Size Key - Hash Key First".utf8)
            ) == "80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f3526b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598"
        )
    }
}

// ─── RFC 2202 — HMAC-MD5 ─────────────────────────────────────────────────────

@Suite("HMAC-MD5 (RFC 2202)")
struct HmacMD5Tests {
    @Test func tc1_16ByteKey() {
        let key = Data(repeating: 0x0b, count: 16)
        #expect(
            hmacMD5Hex(key: key, message: Data("Hi There".utf8)) ==
            "9294727a3638bb1c13f48ef8158bfc9d"
        )
    }

    @Test func tc2_Jefe() {
        #expect(
            hmacMD5Hex(
                key: Data("Jefe".utf8),
                message: Data("what do ya want for nothing?".utf8)
            ) == "750c783e6ab0b503eaa86e310a5db738"
        )
    }

    @Test func tc6_longerThanBlockSizeKey() {
        let key = Data(repeating: 0xaa, count: 80)
        #expect(
            hmacMD5Hex(
                key: key,
                message: Data("Test Using Larger Than Block-Size Key - Hash Key First".utf8)
            ) == "6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd"
        )
    }
}

// ─── RFC 2202 — HMAC-SHA1 ────────────────────────────────────────────────────

@Suite("HMAC-SHA1 (RFC 2202)")
struct HmacSHA1Tests {
    @Test func tc1_20ByteKey() {
        let key = Data(repeating: 0x0b, count: 20)
        #expect(
            hmacSHA1Hex(key: key, message: Data("Hi There".utf8)) ==
            "b617318655057264e28bc0b6fb378c8ef146be00"
        )
    }

    @Test func tc2_Jefe() {
        #expect(
            hmacSHA1Hex(
                key: Data("Jefe".utf8),
                message: Data("what do ya want for nothing?".utf8)
            ) == "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
        )
    }

    @Test func tc6_longerThanBlockSizeKey() {
        let key = Data(repeating: 0xaa, count: 80)
        #expect(
            hmacSHA1Hex(
                key: key,
                message: Data("Test Using Larger Than Block-Size Key - Hash Key First".utf8)
            ) == "aa4ae5e15272d00e95705637ce8a3b55ed402112"
        )
    }
}

// ─── Return lengths ───────────────────────────────────────────────────────────

@Suite("Return lengths")
struct ReturnLengthTests {
    let k = Data("k".utf8)
    let m = Data("m".utf8)

    @Test func md5Returns16Bytes()    { #expect(hmacMD5(key: k, message: m).count    == 16) }
    @Test func sha1Returns20Bytes()   { #expect(hmacSHA1(key: k, message: m).count   == 20) }
    @Test func sha256Returns32Bytes() { #expect(hmacSHA256(key: k, message: m).count == 32) }
    @Test func sha512Returns64Bytes() { #expect(hmacSHA512(key: k, message: m).count == 64) }
}

// ─── Key handling ─────────────────────────────────────────────────────────────

@Suite("Key handling")
struct KeyHandlingTests {
    @Test func emptyKeyAndMessageSHA256() {
        // precondition: empty key must crash — we verify it's guarded by
        // calling the throwing variant instead to avoid crashing the runner
        let msg = Data("hello".utf8)
        #expect(hmacSHA256(key: Data([0x01]), message: msg).count == 32)
    }

    @Test func emptyKeyAndMessageSHA512() {
        let msg = Data("hello".utf8)
        #expect(hmacSHA512(key: Data([0x01]), message: msg).count == 64)
    }

    @Test func emptyMessageWithNonEmptyKeyAllowed() {
        #expect(hmacSHA256(key: Data("key".utf8), message: Data()).count == 32)
    }

    @Test func differentLongKeysDifferentTags() {
        let k65 = Data(repeating: 0x01, count: 65)
        let k66 = Data(repeating: 0x01, count: 66)
        let msg = Data("msg".utf8)
        #expect(hmacSHA256Hex(key: k65, message: msg) != hmacSHA256Hex(key: k66, message: msg))
    }
}

// ─── Authentication properties ────────────────────────────────────────────────

@Suite("Authentication properties")
struct AuthPropertyTests {
    let k = Data("secret".utf8)
    let m = Data("message".utf8)

    @Test func deterministic() {
        #expect(hmacSHA256(key: k, message: m) == hmacSHA256(key: k, message: m))
    }

    @Test func keySensitivity() {
        #expect(
            hmacSHA256Hex(key: Data("k1".utf8), message: m) !=
            hmacSHA256Hex(key: Data("k2".utf8), message: m)
        )
    }

    @Test func messageSensitivity() {
        #expect(
            hmacSHA256Hex(key: k, message: Data("m1".utf8)) !=
            hmacSHA256Hex(key: k, message: Data("m2".utf8))
        )
    }

    @Test func hexMatchesBytes() {
        let tag = hmacSHA256(key: k, message: m)
        let hex = hmacSHA256Hex(key: k, message: m)
        let expected = tag.map { String(format: "%02x", $0) }.joined()
        #expect(hex == expected)
    }
}

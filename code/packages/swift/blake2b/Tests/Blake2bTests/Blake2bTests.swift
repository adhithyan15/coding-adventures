import XCTest
@testable import Blake2b

// All expected values in this file are pre-computed from Python's
// hashlib.blake2b, which wraps the reference implementation.  The same
// KATs are mirrored in every language's BLAKE2b test suite for
// cross-language consistency.
final class Blake2bTests: XCTestCase {
    // --- Helpers ---

    func bytesFromRange(_ start: Int, _ end: Int) -> [UInt8] {
        return (start..<end).map { UInt8($0 & 0xFF) }
    }

    func utf8(_ s: String) -> [UInt8] { return Array(s.utf8) }

    func hex(_ b: [UInt8]) -> String { return Blake2b.Hasher.bytesToHex(b) }

    // --- Canonical ---

    func testEmptyMessageDefault() throws {
        XCTAssertEqual(
            try Blake2b.hashHex([]),
            "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"
        )
    }

    func testAbc() throws {
        XCTAssertEqual(
            try Blake2b.hashHex(utf8("abc")),
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
        )
    }

    func testFox() throws {
        XCTAssertEqual(
            try Blake2b.hashHex(utf8("The quick brown fox jumps over the lazy dog")),
            "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"
        )
    }

    func testTruncatedDigestSize32() throws {
        XCTAssertEqual(
            try Blake2b.hashHex([], options: .init(digestSize: 32)),
            "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"
        )
    }

    func testKeyedLongVector() throws {
        let key = bytesFromRange(1, 65)
        let data = bytesFromRange(0, 256)
        XCTAssertEqual(
            try Blake2b.hashHex(data, options: .init(key: key)),
            "402fa70e35f026c9bfc1202805e931b995647fe479e1701ad8b7203cddad5927ee7950b898a5a8229443d93963e4f6f27136b2b56f6845ab18f59bc130db8bf3"
        )
    }

    // --- Block-boundary sizes ---

    func testBlockBoundarySizes() throws {
        let cases: [(Int, String)] = [
            (0, "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"),
            (1, "4fe4da61bcc756071b226843361d74944c72245d23e8245ea678c13fdcd7fe2ae529cf999ad99cc24f7a73416a18ba53e76c0afef83b16a568b12fbfc1a2674d"),
            (63, "70b2a0e6daecac22c7a2df82c06e3fc0b4c66bd5ef8098e4ed54e723b393d79ef3bceba079a01a14c6ef2ae2ed1171df1662cd14ef38e6f77b01c7f48144dd09"),
            (64, "3db7bb5c40745f0c975ac6bb8578f590e2cd2cc1fc6d13533ef725325c9fddff5cca24e7a591a0f6032a24fad0e09f6df873c4ff314628391f78df7f09cb7ed7"),
            (65, "149c114a3e8c6e06bafee27c9d0de0e39ef28294fa0d9f81876dcceb10bb41101e256593587e46b844819ed7ded90d56c0843df06c95d1695c3de635cd7a888e"),
            (127, "71546bbf9110ad184cc60f2eb120fcfd9b4dbbca7a7f1270045b8a23a6a4f4330f65c1f030dd2f5fabc6c57617242c37cf427bd90407fac5b9deffd3ae888c39"),
            (128, "2d9e329f42afa3601d646692b81c13e87fcaff5bf15972e9813d7373cb6d181f9599f4d513d4af4fd6ebd37497aceb29aba5ee23ed764d8510b552bd088814fb"),
            (129, "47889df9eb4d717afc5019df5c6a83df00a0b8677395e078cd5778ace0f338a618e68b7d9afb065d9e6a01ccd31d109447e7fae771c3ee3e105709194122ba2b"),
            (255, "1a5199ac66a00e8a87ad1c7fbad30b33137dd8312bf6d98602dacf8f40ea2cb623a7fbc63e5a6bfa434d337ae7da5ca1a52502a215a3fe0297a151be85d88789"),
            (256, "91019c558584980249ca43eceed27e19f1c3c24161b93eed1eee2a6a774f60bf8a81b43750870bee1698feac9c5336ae4d5c842e7ead159bf3916387e8ded9ae"),
            (257, "9f1975efca45e7b74b020975d4d2c22802906ed8bfefca51ac497bd23147fc8f303890d8e5471ab6caaa02362e831a9e8d3435279912ccd4842c7806b096c348"),
            (1024, "eddc3f3af9392eff065b359ce5f2b28f71e9f3a3a50e60ec27787b9fa623094d17b046c1dfce89bc5cdfc951b95a9a9c05fb8cc2361c905db01dd237fe56efb3"),
            (4096, "31404c9c7ed64c59112579f300f2afef181ee6283c3918bf026c4ed4bcde0697a7834f3a3410396622ef3d4f432602528a689498141c184cc2063554ba688dc7"),
            (9999, "b4a5808e65d7424b517bde11e04075a09b1343148e3ab2c8b13ff35c542e0a2beff6309ecc54b59ac046f6d65a9e3680c6372a033607709c95d5fd8070be6069"),
        ]
        for (size, want) in cases {
            let data: [UInt8] = (0..<size).map { UInt8(($0 * 7 + 3) & 0xFF) }
            XCTAssertEqual(try Blake2b.hashHex(data), want, "size \(size)")
        }
    }

    // --- Variable digest sizes ---

    func testVariableDigestSizes() throws {
        let data = utf8("The quick brown fox jumps over the lazy dog")
        let cases: [(Int, String)] = [
            (1, "b5"),
            (16, "249df9a49f517ddcd37f5c897620ec73"),
            (20, "3c523ed102ab45a37d54f5610d5a983162fde84f"),
            (32, "01718cec35cd3d796dd00020e0bfecb473ad23457d063b75eff29c0ffa2e58a9"),
            (48, "b7c81b228b6bd912930e8f0b5387989691c1cee1e65aade4da3b86a3c9f678fc8018f6ed9e2906720c8d2a3aeda9c03d"),
            (64, "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"),
        ]
        for (ds, want) in cases {
            let out = try Blake2b.hash(data, options: .init(digestSize: ds))
            XCTAssertEqual(out.count, ds)
            XCTAssertEqual(hex(out), want, "digestSize \(ds)")
        }
    }

    // --- Keyed ---

    func testKeyedVariants() throws {
        let data = utf8("secret message body")
        let cases: [(Int, String)] = [
            (1, "affd4e429aa2fb18da276f6ecff16f7d048769cacefe1a7ac75184448e082422"),
            (16, "5f8510d05dac42e8b6fc542af93f349d41ae4ebaf5cecae4af43fae54c7ca618"),
            (32, "88a78036d5890e91b5e3d70ba4738d2be302b76e0857d8ee029dc56dfa04fe67"),
            (64, "df7eab2ec9135ab8c58f48c288cdc873bac245a7fa46ca9f047cab672bd1eabb"),
        ]
        for (klen, want) in cases {
            let key = bytesFromRange(1, klen + 1)
            XCTAssertEqual(
                try Blake2b.hashHex(data, options: .init(digestSize: 32, key: key)),
                want, "keyLen \(klen)"
            )
        }
    }

    func testSaltAndPersonal() throws {
        let salt = bytesFromRange(0, 16)
        let personal = bytesFromRange(16, 32)
        XCTAssertEqual(
            try Blake2b.hashHex(utf8("parameterized hash"),
                                options: .init(salt: salt, personal: personal)),
            "a2185d648fc63f3d363871a76360330c9b238af5466a20f94bb64d363289b95da0453438eea300cd6f31521274ec001011fa29e91a603fabf00f2b454e30bf3d"
        )
    }

    // --- Streaming ---

    func testStreamingSingleChunkMatchesOneShot() throws {
        var h = try Blake2b.Hasher()
        h.update(utf8("hello world"))
        XCTAssertEqual(hex(h.digest()), try Blake2b.hashHex(utf8("hello world")))
    }

    func testStreamingByteByByteMatchesOneShot() throws {
        let data = bytesFromRange(0, 200)
        var h = try Blake2b.Hasher(options: .init(digestSize: 32))
        for b in data { h.update([b]) }
        XCTAssertEqual(hex(h.digest()),
                       try Blake2b.hashHex(data, options: .init(digestSize: 32)))
    }

    func testStreamingAcrossBlockBoundary() throws {
        let data = bytesFromRange(0, 129)
        var h = try Blake2b.Hasher()
        h.update(Array(data[0..<127]))
        h.update(Array(data[127..<129]))
        XCTAssertEqual(hex(h.digest()), try Blake2b.hashHex(data))
    }

    func testStreamingExactBlockThenMore() throws {
        // Canonical BLAKE2 off-by-one: 128 bytes exact then 4 more.
        let data: [UInt8] = (0..<132).map { UInt8($0 & 0xFF) }
        var h = try Blake2b.Hasher()
        h.update(Array(data[0..<128]))
        h.update(Array(data[128..<132]))
        XCTAssertEqual(hex(h.digest()), try Blake2b.hashHex(data))
    }

    func testDigestIsIdempotent() throws {
        var h = try Blake2b.Hasher()
        h.update(utf8("hello"))
        XCTAssertEqual(h.hexDigest(), h.hexDigest())
    }

    func testUpdateAfterDigestContinuesStream() throws {
        var h = try Blake2b.Hasher(options: .init(digestSize: 32))
        h.update(utf8("hello "))
        _ = h.digest()
        h.update(utf8("world"))
        XCTAssertEqual(
            h.hexDigest(),
            try Blake2b.hashHex(utf8("hello world"), options: .init(digestSize: 32))
        )
    }

    func testCopyIsIndependent() throws {
        var h = try Blake2b.Hasher()
        h.update(utf8("prefix "))
        var c = h.copy()
        h.update(utf8("path A"))
        c.update(utf8("path B"))
        XCTAssertEqual(hex(h.digest()), try Blake2b.hashHex(utf8("prefix path A")))
        XCTAssertEqual(hex(c.digest()), try Blake2b.hashHex(utf8("prefix path B")))
    }

    // --- Validation ---

    func testRejectsDigestSizeZero() {
        XCTAssertThrowsError(try Blake2b.hash([], options: .init(digestSize: 0))) { err in
            XCTAssertEqual(err as? Blake2b.ValidationError, .invalidDigestSize(0))
        }
    }

    func testRejectsDigestSize65() {
        XCTAssertThrowsError(try Blake2b.hash([], options: .init(digestSize: 65))) { err in
            XCTAssertEqual(err as? Blake2b.ValidationError, .invalidDigestSize(65))
        }
    }

    func testRejectsKeyTooLong() {
        let key = [UInt8](repeating: 0, count: 65)
        XCTAssertThrowsError(try Blake2b.hash([], options: .init(key: key))) { err in
            XCTAssertEqual(err as? Blake2b.ValidationError, .keyTooLong(65))
        }
    }

    func testRejectsWrongSaltLength() {
        let salt = [UInt8](repeating: 0, count: 8)
        XCTAssertThrowsError(try Blake2b.hash([], options: .init(salt: salt))) { err in
            XCTAssertEqual(err as? Blake2b.ValidationError, .invalidSaltLength(8))
        }
    }

    func testRejectsWrongPersonalLength() {
        let personal = [UInt8](repeating: 0, count: 20)
        XCTAssertThrowsError(try Blake2b.hash([], options: .init(personal: personal))) { err in
            XCTAssertEqual(err as? Blake2b.ValidationError, .invalidPersonalLength(20))
        }
    }

    func testAcceptsMax64ByteKey() throws {
        let key = [UInt8](repeating: 0x41, count: 64)
        XCTAssertNoThrow(try Blake2b.hash(utf8("x"), options: .init(key: key)))
    }

    func testHexDigestMatchesHexOfDigest() throws {
        var h = try Blake2b.Hasher(options: .init(digestSize: 32))
        h.update(utf8("hex check"))
        XCTAssertEqual(h.hexDigest(), hex(h.digest()))
    }
}

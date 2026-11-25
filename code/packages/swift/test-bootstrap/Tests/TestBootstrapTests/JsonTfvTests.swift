import XCTest
@testable import TestBootstrap

final class JsonTfvTests: XCTestCase {
    private func parse(_ s: String) throws -> Json { try StrictJson.parse(Array(s.utf8)) }
    private func rejects(_ s: String, _ note: String = "", line: UInt = #line) {
        XCTAssertThrowsError(try parse(s), "\(s) \(note)", line: line)
    }

    func testAcceptsAWellFormedDocumentAndKeepsNumbersAsText() throws {
        let j = try parse(" {\"a\": [1, -2.5e+10, \"x\\u00e9\\n\", true, false, null], \"b\": {}} ")
        guard case .array(let items)? = j["a"] else { return XCTFail() }
        guard case .number(let n0) = items[0], case .number(let n1) = items[1] else { return XCTFail() }
        XCTAssertEqual(n0, "1")
        XCTAssertEqual(n1, "-2.5e+10")
        guard case .string(let s) = items[2] else { return XCTFail() }
        XCTAssertEqual(s, "x\u{e9}\n")
        // A 40-digit integer survives as text: Foundation would round it to a Double.
        guard case .number(let big)? = try parse("{\"v\": 123456789012345678901234567890123456789}")["v"] else { return XCTFail() }
        XCTAssertEqual(big, "123456789012345678901234567890123456789")
    }

    func testRefusesDuplicateKeys() {
        rejects("{\"a\":1,\"a\":2}")
        rejects("{\"a\":1,\"b\":{\"c\":1,\"c\":2}}")
        XCTAssertThrowsError(try parse("{\"a\":1,\"a\":2}")) { XCTAssertTrue(($0 as! JsonError).message.contains("duplicate")) }
        XCTAssertNoThrow(try parse("{\"a\":1,\"b\":{\"a\":2}}"))
    }

    func testRefusesLeadingZerosAndMalformedNumbers() {
        for s in ["01", "-01", "00", "1.", ".5", "1e", "1e+", "-", "+1", "0x10", "1.e3"] { rejects(s) }
        XCTAssertNoThrow(try parse("0"))
        XCTAssertNoThrow(try parse("-0"))
        XCTAssertNoThrow(try parse("0.5e-3"))
    }

    func testRefusesTrailingDataAndMalformedStructure() {
        for s in ["{} x", "[1] 2", "{}{}", "[1,]", "{\"a\":1,}", "[1 2]", "{\"a\" 1}", "{a:1}", "[", "{", "\"abc", "nul", "tru", "fals", ""] {
            rejects(s)
        }
    }

    func testRefusesBadStringsAndEscapes() {
        rejects("\"a\u{1}b\"")
        rejects("\"\\x\"")
        rejects("\"\\u12\"")
        rejects("\"\\u12g4\"")
        rejects("\"\\ud800\"", "lone high surrogate")
        rejects("\"\\udc00\"", "lone low surrogate")
        rejects("\"\\ud800\\u0041\"", "bad pair")
        rejects("\"abc\\")
        XCTAssertEqual({ () -> String? in
            if case .string(let s)? = try? parse("\"\\ud83d\\ude00\"") { return s } else { return nil }
        }(), "\u{1F600}")
        XCTAssertNoThrow(try parse("\"\\u0041\\/\\b\\f\\n\\r\\t\\\"\\\\\""))
    }

    func testRefusesInvalidUtf8() {
        let bad: [[UInt8]] = [
            [0x22, 0xff, 0x22],                    // 0xff never appears
            [0x22, 0xc0, 0x80, 0x22],              // overlong NUL
            [0x22, 0xc1, 0xbf, 0x22],              // overlong
            [0x22, 0xe0, 0x80, 0x80, 0x22],        // overlong 3-byte
            [0x22, 0xed, 0xa0, 0x80, 0x22],        // surrogate U+D800
            [0x22, 0xf0, 0x80, 0x80, 0x80, 0x22],  // overlong 4-byte
            [0x22, 0xf4, 0x90, 0x80, 0x80, 0x22],  // above U+10FFFF
            [0x22, 0xf5, 0x80, 0x80, 0x80, 0x22],
            [0x22, 0xc2, 0x22],                    // truncated
            [0x22, 0x80, 0x22],                    // stray continuation
        ]
        for b in bad { XCTAssertThrowsError(try StrictJson.parse(b), "\(b)") }
        XCTAssertNoThrow(try StrictJson.parse([0x22, 0xc3, 0xa9, 0x22]))
        XCTAssertNoThrow(try StrictJson.parse([0x22, 0xf0, 0x9f, 0x98, 0x80, 0x22]))
        XCTAssertNoThrow(try StrictJson.parse([0x22, 0xed, 0x9f, 0xbf, 0x22]))
        XCTAssertNoThrow(try StrictJson.parse([0x22, 0xf4, 0x8f, 0xbf, 0xbf, 0x22]))
    }

    func testDepthCap() {
        XCTAssertNoThrow(try parse(String(repeating: "[", count: 64) + String(repeating: "]", count: 64)))
        rejects(String(repeating: "[", count: 65) + String(repeating: "]", count: 65), "65 nested containers")
        rejects(String(repeating: "{\"a\":", count: 65) + "0" + String(repeating: "}", count: 65), "65 nested objects")
        XCTAssertNoThrow(try parse(String(repeating: "{\"a\":", count: 64) + "0" + String(repeating: "}", count: 64)))
        rejects(String(repeating: "[", count: 100_000))
    }

    func testNodeAndSizeCaps() {
        let ok = "[" + String(repeating: "0,", count: 199_990) + "0]"
        XCTAssertNoThrow(try parse(ok))
        let many = "[" + String(repeating: "0,", count: 200_001) + "0]"
        XCTAssertThrowsError(try parse(many)) { XCTAssertTrue(($0 as! JsonError).message.contains("values")) }
        XCTAssertThrowsError(try StrictJson.parse([UInt8](repeating: 0x20, count: StrictJson.maxBytes + 1)))
    }

    // MARK: TFV -----------------------------------------------------------------

    func testTfvAcceptsTheRealCorpusFile() throws {
        let tfv = Support.fixtureURL().deletingLastPathComponent().appendingPathComponent("cases.tfv")
        XCTAssertNoThrow(try Tfv.validate(String(contentsOf: tfv, encoding: .utf8)))
    }

    func testTfvRejections() {
        let rejects = [
            "bad header\n", "", "tfv 2\n", "TFV 1\n",
            "tfv 1\nr R1 Kdata XFF\n",                  // uppercase hex
            "tfv 1\nr R1 Kdata X0\n",                   // odd hex
            "tfv 1\nr R1 Kdata I01\n", "tfv 1\nr R1 Kdata I-0\n", "tfv 1\nr R1 Kdata I\n", "tfv 1\nr R1 Kdata Ix\n",
            "tfv 1\nr R2 Kdata I1\n",                   // ends early
            "tfv 1\nr R1 Kdata I1 I2\n",                // trailing tokens
            "tfv 1\nr R1 Kdata I1\nr R1 Kdata I1\n",    // duplicate record id
            "tfv 1\nr R2 Kdata I1 Kdata I2\n",          // duplicate field
            "tfv 1\nr R1 KData I1\n",                   // bad field name
            "tfv 1\nr  R1 Kdata I1\n", "tfv 1\n r R1 Kdata I1\n",   // doubled/leading space
            "tfv 1\nr R1 Kdata I1\r\n",                 // carriage return
            "tfv 1\nr R1 Kdata Q1\n",                   // unknown tag
            "tfv 1\nr R1 Kdata T1\n",                   // body on a bool
            "tfv 1\nr R1 Kdata D00\n",                  // short float
            "tfv 1\nr R1 Kdata Sc0\n",                  // invalid UTF-8 text
            "tfv 1\nr R1 Kdata Sc080\n",
            "tfv 1\nr R1 Kdata L2 I1\n",                // list ends early
            "tfv 1\nr R1 Kdata L01\n",                  // leading zero count
            "tfv 1\nr R1 Kdata M2 I1 I1 I1 I2\n",       // duplicate map key
            "tfv 1\nr R01 Kdata I1\n", "tfv 1\nr Rx Kdata I1\n", "tfv 1\nr\n",
            "tfv 1\nr$ R1 Kdata I1\n",
            "tfv 1\nr R1 Kdata L9999999999\n",
        ]
        for text in rejects { XCTAssertFalse(Tfv.isValid(text), text) }
        // Depth cap.
        let deep = "tfv 1\nr R1 Kv " + String(repeating: "L1 ", count: 70) + "N\n"
        XCTAssertFalse(Tfv.isValid(deep))
        XCTAssertFalse(Tfv.isValid("tfv 1\n" + String(repeating: "x", count: Tfv.maxInput)))
        XCTAssertFalse(Tfv.isValid("tfv 1\nr R1 Kv I" + String(repeating: "9", count: 1001) + "\n"))
    }

    func testTfvAcceptances() {
        let accepts = [
            "tfv 1\n", "tfv 1", "tfv 1\n\n# comment\n",
            "tfv 1\nr/a-b.c_d R2 Kx I-5 Ky S68656c6c6f\n",
            "tfv 1\nr R1 Kv L2 T F\n", "tfv 1\nr R1 Kv M1 S6b I1\n",
            "tfv 1\nr R1 Kv D7ff8000000000000\n", "tfv 1\nr R1 Kv X\n", "tfv 1\nr R1 Kv N\n",
            "tfv 1\na R1 Kx I0\nb R1 Kx I0\n",
        ]
        for text in accepts { XCTAssertTrue(Tfv.isValid(text), text) }
        XCTAssertTrue(Tfv.isValid("tfv 1\nr R1 Kv " + String(repeating: "L1 ", count: 63) + "N\n"))
    }

    func testTfvErrorsNameTheLine() {
        XCTAssertThrowsError(try Tfv.validate("tfv 1\nok R1 Kx I1\nbad R1 Kx I01\n")) {
            XCTAssertTrue(($0 as! TfvError).message.contains("line 3"))
        }
    }

    // MARK: Values ----------------------------------------------------------------

    func testValueDecodingIsStrict() throws {
        XCTAssertEqual(try Values.decode(Support.int("-12")), .int(BigInt(-12)))
        XCTAssertEqual(try Values.decode(Support.bytes("00ff")), .bytes([0, 255]))
        XCTAssertEqual(try Values.decode(.array([.bool(true), .string("s")])), .list([.bool(true), .string("s")]))
        let rejected: [Json] = [Support.int("01"), Support.int("-0"), Support.int("1.5"), Support.int(""),
                                Support.bytes("0"), Support.bytes("FF"), .number("5"), .null,
                                .object([("kind", .string("int"))]),
                                .object([("kind", .string("int")), ("value", .string("1")), ("extra", .null)]),
                                .object([("kind", .string("float")), ("value", .string("1"))]),
                                Support.int(String(repeating: "7", count: 301)),
                                .array(Array(repeating: .bool(true), count: 10_001)),
                                Support.bytes(String(repeating: "00", count: 10_001))]
        for r in rejected { XCTAssertThrowsError(try Values.decode(r)) }
        var deep: Json = .bool(true)
        for _ in 0..<70 { deep = .array([deep]) }
        XCTAssertThrowsError(try Values.decode(deep))
    }

    func testValueEqualityIsKindSensitive() {
        XCTAssertNotEqual(Value.int(1), Value.bool(true))
        XCTAssertNotEqual(Value.bytes([1]), Value.list([.int(1)]))
        XCTAssertNotEqual(Value.string("1"), Value.int(1))
        XCTAssertEqual(Value.int(BigInt(10)), Value.int(BigInt(10)))
        XCTAssertTrue(Values.isCanonicalInt("-5"))
        XCTAssertFalse(Values.isCanonicalInt("-"))
        XCTAssertTrue(Values.isLowercaseHex("0aff"))
        XCTAssertFalse(Values.isLowercaseHex("0AFF"))
        XCTAssertEqual(Values.show(nil), "nil")
        XCTAssertLessThanOrEqual(Values.show(String(repeating: "z", count: 5000)).count, Values.maxFragmentLen + 3)
    }
}

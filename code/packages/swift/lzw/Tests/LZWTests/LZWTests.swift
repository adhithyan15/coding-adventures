// LZWTests.swift
// Comprehensive tests for the LZW compression implementation.
//
// Test vectors come from the CMP03 specification. Covers: spec vectors,
// encode properties, decode correctness, the tricky-token edge case,
// round-trip invariants, wire format, compression effectiveness, and
// security (malformed input does not crash).

import XCTest
@testable import LZW

final class LZWTests: XCTestCase {

    // MARK: - Helpers

    /// Encode a string as UTF-8 bytes.
    func enc(_ s: String) -> [UInt8] { Array(s.utf8) }

    /// Decode bytes as a UTF-8 string.
    func str(_ b: [UInt8]) -> String { String(bytes: b, encoding: .utf8) ?? "" }

    /// Round-trip helper for byte arrays.
    func rt(_ b: [UInt8]) -> [UInt8] { decompress(compress(b)) }

    /// Round-trip helper for strings.
    func rtStr(_ s: String) -> String { str(rt(enc(s))) }

    // MARK: - Constants

    func testClearCodeIs256() {
        XCTAssertEqual(clearCode, 256)
    }

    func testStopCodeIs257() {
        XCTAssertEqual(stopCode, 257)
    }

    func testInitialNextCodeIs258() {
        XCTAssertEqual(initialNextCode, 258)
    }

    func testInitialCodeSizeIs9() {
        XCTAssertEqual(initialCodeSize, 9)
    }

    func testMaxCodeSizeIs16() {
        XCTAssertEqual(maxCodeSize, 16)
    }

    // MARK: - Spec Vector 1 — Empty input

    func testEncodeCodesEmpty() {
        // An empty input should still emit CLEAR_CODE and STOP_CODE.
        let (codes, origLen) = encodeCodes([])
        XCTAssertEqual(origLen, 0)
        XCTAssertEqual(codes.first, clearCode)
        XCTAssertEqual(codes.last, stopCode)
    }

    func testCompressEmpty() {
        // Wire format: 4-byte header (origLen=0) + 3 bytes for CLEAR+STOP at 9 bits each.
        // 2 codes × 9 bits = 18 bits → 3 bytes (with 6 zero-padding bits).
        let c = compress([])
        XCTAssertGreaterThanOrEqual(c.count, 4) // at minimum the header
        let origLen = Int(UInt32(c[0]) << 24 | UInt32(c[1]) << 16 | UInt32(c[2]) << 8 | UInt32(c[3]))
        XCTAssertEqual(origLen, 0)
    }

    func testRoundTripEmpty() {
        XCTAssertEqual(rt([]), [])
    }

    // MARK: - Spec Vector 2 — Single byte

    func testEncodeCodesSingleByte() {
        // Input "A" (0x41 = 65) → CLEAR, 65, STOP
        let (codes, origLen) = encodeCodes(enc("A"))
        XCTAssertEqual(origLen, 1)
        XCTAssertEqual(codes, [clearCode, 65, stopCode])
    }

    func testRoundTripSingleByte() {
        XCTAssertEqual(rtStr("A"), "A")
    }

    // MARK: - Spec Vector 3 — Two distinct bytes

    func testEncodeCodesAB() {
        // Input "AB" → CLEAR, 65, 66, STOP
        // dict[258] = "AB" is added but never emitted.
        let (codes, _) = encodeCodes(enc("AB"))
        XCTAssertEqual(codes, [clearCode, 65, 66, stopCode])
    }

    func testRoundTripAB() {
        XCTAssertEqual(rtStr("AB"), "AB")
    }

    // MARK: - Spec Vector 4 — Repeated pair "ABABAB"

    func testEncodeCodesABABAB() {
        // Expected: CLEAR, 65("A"), 66("B"), 258("AB"), 258("AB"), STOP
        // Trace:
        //   b=A: w="A" → in dict
        //   b=B: w="AB" → not in dict → emit 65; add 258="AB"; w="B"
        //   b=A: w="BA" → not in dict → emit 66; add 259="BA"; w="A"
        //   b=B: w="AB" → in dict (258)
        //   b=A: w="ABA" → not in dict → emit 258; add 260="ABA"; w="A"
        //   b=B: w="AB" → in dict (258)
        //   EOF → emit 258; STOP
        let (codes, origLen) = encodeCodes(enc("ABABAB"))
        XCTAssertEqual(origLen, 6)
        XCTAssertEqual(codes, [clearCode, 65, 66, 258, 258, stopCode])
    }

    func testRoundTripABABAB() {
        XCTAssertEqual(rtStr("ABABAB"), "ABABAB")
    }

    // MARK: - Spec Vector 5 — Tricky token ("AAAAAAA")

    func testEncodeCodesAAAAAAARaw() {
        // Expected: CLEAR, 65, 258, 259, 65, STOP
        // Trace:
        //   b=A: w="A" → in dict
        //   b=A: w="AA" → not in dict → emit 65; add 258="AA"; w="A"
        //   b=A: w="AA" → in dict (258)
        //   b=A: w="AAA" → not in dict → emit 258; add 259="AAA"; w="A"
        //   b=A: w="AA" → in dict (258)
        //   b=A: w="AAA" → in dict (259)
        //   b=A: w="AAAA" → not in dict → emit 259; add 260="AAAA"; w="A"
        //   EOF → emit 65; STOP
        let (codes, origLen) = encodeCodes(enc("AAAAAAA"))
        XCTAssertEqual(origLen, 7)
        XCTAssertEqual(codes, [clearCode, 65, 258, 259, 65, stopCode])
    }

    func testDecodeTrickyTokenAAAAAAA() {
        // Verify that decoding the tricky-token code sequence produces "AAAAAAA".
        // The tricky tokens are 258 and 259, both arriving before their entries
        // are fully in the dictionary.
        let codes: [UInt] = [clearCode, 65, 258, 259, 65, stopCode]
        let output = decodeCodes(codes)
        XCTAssertEqual(str(output), "AAAAAAA")
    }

    func testRoundTripAAAAAAAString() {
        XCTAssertEqual(rtStr("AAAAAAA"), "AAAAAAA")
    }

    func testRoundTripAllSameByte() {
        // 100 identical bytes — exercises the tricky-token path repeatedly.
        let data = [UInt8](repeating: 0x42, count: 100)
        XCTAssertEqual(rt(data), data)
    }

    // MARK: - encodeCodes properties

    func testEncodeCodesStartsWithClear() {
        // Every well-formed LZW stream starts with CLEAR_CODE.
        let (codes, _) = encodeCodes(enc("hello world"))
        XCTAssertEqual(codes.first, clearCode)
    }

    func testEncodeCodesEndsWithStop() {
        // Every well-formed LZW stream ends with STOP_CODE.
        let (codes, _) = encodeCodes(enc("hello world"))
        XCTAssertEqual(codes.last, stopCode)
    }

    func testEncodeCodesOriginalLengthPreserved() {
        let data = enc("hello world, hello world!")
        let (_, origLen) = encodeCodes(data)
        XCTAssertEqual(origLen, data.count)
    }

    // MARK: - decodeCodes properties

    func testDecodeCodesEmptyStreamIsEmpty() {
        // CLEAR + STOP with no data codes should produce empty output.
        let output = decodeCodes([clearCode, stopCode])
        XCTAssertEqual(output, [])
    }

    func testDecodeCodesIgnoresCodesAfterStop() {
        // STOP_CODE terminates decoding; extra codes after it are ignored.
        let codes: [UInt] = [clearCode, 65, stopCode, 66, 67]
        let output = decodeCodes(codes)
        XCTAssertEqual(str(output), "A")
    }

    func testDecodeCodesHandlesMidStreamClear() {
        // A CLEAR_CODE mid-stream resets the dictionary. After the reset the
        // decoder starts fresh, so the next code maps to a pre-seeded entry.
        // Encode "AB" twice with a CLEAR in between.
        let codes: [UInt] = [clearCode, 65, 66, clearCode, 65, 66, stopCode]
        let output = decodeCodes(codes)
        XCTAssertEqual(str(output), "ABAB")
    }

    // MARK: - packCodes / unpackCodes round-trip

    func testPackUnpackRoundTrip() {
        let (codes, origLen) = encodeCodes(enc("ABABAB"))
        let packed = packCodes(codes, originalLength: origLen)
        let (unpacked, unpackedLen) = unpackCodes(packed)
        XCTAssertEqual(unpacked, codes)
        XCTAssertEqual(unpackedLen, origLen)
    }

    func testPackUnpackEmptyInput() {
        let (codes, origLen) = encodeCodes([])
        let packed = packCodes(codes, originalLength: origLen)
        let (unpacked, unpackedLen) = unpackCodes(packed)
        XCTAssertEqual(unpacked, codes)
        XCTAssertEqual(unpackedLen, 0)
    }

    // MARK: - Wire format

    func testCompressStoresOriginalLength() {
        let data = enc("hello")
        let c = compress(data)
        let stored = Int(UInt32(c[0]) << 24 | UInt32(c[1]) << 16 | UInt32(c[2]) << 8 | UInt32(c[3]))
        XCTAssertEqual(stored, 5)
    }

    func testCompressHeaderIsBigEndian() {
        // original_length = 0x01020304 → bytes [0x01, 0x02, 0x03, 0x04]
        let data = [UInt8](repeating: 0x41, count: 0x0102_0304)
        // We don't want to actually allocate 16MB in tests, so use a smaller value
        // and manually check the encoding.
        let small = [UInt8](repeating: 0x41, count: 259)
        let c = compress(small)
        let stored = Int(UInt32(c[0]) << 24 | UInt32(c[1]) << 16 | UInt32(c[2]) << 8 | UInt32(c[3]))
        XCTAssertEqual(stored, 259)
        _ = data.count // suppress unused warning
    }

    func testCompressIsDeterministic() {
        let data = enc("hello world test")
        XCTAssertEqual(compress(data), compress(data))
    }

    // MARK: - Round-trip invariants

    func testRoundTripNoRepetition() {
        XCTAssertEqual(rtStr("ABCDE"), "ABCDE")
    }

    func testRoundTripHelloWorld() {
        XCTAssertEqual(rtStr("hello world"), "hello world")
    }

    func testRoundTripHelloHello() {
        // Repeated phrase exercises multi-byte dictionary entries.
        XCTAssertEqual(rtStr("hello hello hello"), "hello hello hello")
    }

    func testRoundTripABCx100() {
        let data = String(repeating: "ABC", count: 100)
        XCTAssertEqual(rtStr(data), data)
    }

    func testRoundTripLongRepeatedPattern() {
        let data = String(repeating: "ABCDEF", count: 500)
        XCTAssertEqual(rtStr(data), data)
    }

    func testRoundTripBinaryNulls() {
        let data: [UInt8] = [0, 0, 0, 255, 255]
        XCTAssertEqual(rt(data), data)
    }

    func testRoundTripFullByteRange() {
        // All 256 possible byte values — covers every pre-seeded dictionary entry.
        let data = [UInt8](0...255)
        XCTAssertEqual(rt(data), data)
    }

    func testRoundTripRepeatedBytePattern() {
        let data = [UInt8]((0..<300).map { UInt8($0 % 3) })
        XCTAssertEqual(rt(data), data)
    }

    func testRoundTripLongAllSameBytes() {
        // 10 000 identical bytes — dictionary fills fast and CLEAR is emitted.
        let data = [UInt8](repeating: 0x42, count: 10_000)
        XCTAssertEqual(rt(data), data)
    }

    func testRoundTripAAAAAAALength() {
        // Verify not just bytes but also length is preserved.
        let data = enc("AAAAAAA")
        let result = rt(data)
        XCTAssertEqual(result.count, 7)
        XCTAssertEqual(result, data)
    }

    // MARK: - Compression effectiveness

    func testRepetitiveDataCompresses() {
        // "ABC" repeated 1000 times is 3000 bytes — LZW should compress it well.
        let data = enc(String(repeating: "ABC", count: 1000))
        let compressed = compress(data)
        XCTAssertLessThan(compressed.count, data.count)
    }

    func testAllSameByteCompressesWell() {
        // 10 000 identical bytes should compress to well under 10 000.
        let data = [UInt8](repeating: 0x42, count: 10_000)
        let compressed = compress(data)
        XCTAssertLessThan(compressed.count, data.count / 2)
    }

    func testCompressedIsSmallForHighlyRepetitive() {
        let data = enc(String(repeating: "ABABABABAB", count: 200))
        let compressed = compress(data)
        XCTAssertLessThan(compressed.count, data.count)
    }

    // MARK: - Security / malformed input

    func testDecompressTruncatedHeaderDoesNotCrash() {
        // Input shorter than 4 bytes (header) — must not crash.
        for n in 0..<4 {
            let bad = [UInt8](repeating: 0, count: n)
            let result = decompress(bad)
            XCTAssertNotNil(result) // just checking it returns something
        }
    }

    func testDecompressEmptyByteArrayDoesNotCrash() {
        let result = decompress([])
        XCTAssertEqual(result, [])
    }

    func testDecompressCraftedLargeOriginalLength() {
        // Header claims very large original_length but payload is tiny.
        // Must not panic or allocate out-of-bounds memory.
        let bad: [UInt8] = [0xFF, 0xFF, 0xFF, 0xFF,  // origLen = 4294967295
                            0x00, 0x10, 0x00]         // minimal bit stream
        let result = decompress(bad)
        XCTAssertNotNil(result)
    }

    func testDecompressRandomBytesDoesNotCrash() {
        // Random garbage input — must not crash or produce unexpected side effects.
        let random: [UInt8] = [0xDE, 0xAD, 0xBE, 0xEF,
                               0x12, 0x34, 0x56, 0x78,
                               0xAB, 0xCD, 0xEF, 0x01]
        let result = decompress(random)
        XCTAssertNotNil(result)
    }

    func testDecompressInvalidCodeSkippedGracefully() {
        // Feed a code stream that contains an invalid out-of-range code (not
        // a tricky token) — the decoder should skip it and not crash.
        let codes: [UInt] = [clearCode, 65, 9999, 66, stopCode] // 9999 is invalid
        let output = decodeCodes(codes)
        // Should at least produce the valid parts (A and B).
        XCTAssertTrue(output.contains(65))
        XCTAssertTrue(output.contains(66))
    }
}

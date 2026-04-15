// LZSSTests.swift
// Comprehensive tests for the LZSS compression implementation.
//
// Test vectors come from the CMP02 specification. Covers: spec vectors,
// encode properties, decode correctness, round-trip invariants, wire format,
// and compression effectiveness.

import XCTest
@testable import LZSS

final class LZSSTests: XCTestCase {

    // MARK: - Helpers

    func enc(_ s: String) -> [UInt8] { Array(s.utf8) }
    func dec(_ b: [UInt8]) -> String { String(bytes: b, encoding: .utf8) ?? "" }
    func rt(_ s: String) -> String { dec(decompress(compress(enc(s)))) }
    func rtBytes(_ b: [UInt8]) -> [UInt8] { decompress(compress(b)) }

    // MARK: - Spec vectors

    func testEncodeEmpty() {
        XCTAssertEqual(encode([]).count, 0)
    }

    func testEncodeSingleByte() {
        let tokens = encode([65])
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0], .literal(65))
    }

    func testEncodeNoRepetition() {
        let tokens = encode(enc("ABCDE"))
        XCTAssertEqual(tokens.count, 5)
        for t in tokens {
            if case .literal = t { } else { XCTFail("Expected literal") }
        }
    }

    func testEncodeAABCBBABC() {
        let tokens = encode(enc("AABCBBABC"))
        XCTAssertEqual(tokens.count, 7)
        if case .match(let off, let len) = tokens[6] {
            XCTAssertEqual(off, 5)
            XCTAssertEqual(len, 3)
        } else {
            XCTFail("Last token should be match(5, 3)")
        }
    }

    func testEncodeABABAB() {
        let tokens = encode(enc("ABABAB"))
        XCTAssertEqual(tokens.count, 3)
        XCTAssertEqual(tokens[0], .literal(65))
        XCTAssertEqual(tokens[1], .literal(66))
        XCTAssertEqual(tokens[2], .match(offset: 2, length: 4))
    }

    func testEncodeAAAAAAA() {
        let tokens = encode(enc("AAAAAAA"))
        XCTAssertEqual(tokens.count, 2)
        XCTAssertEqual(tokens[0], .literal(65))
        XCTAssertEqual(tokens[1], .match(offset: 1, length: 6))
    }

    // MARK: - Encode properties

    func testMatchOffsetAtLeastOne() {
        let tokens = encode(enc("ABABABAB"))
        for t in tokens {
            if case .match(let off, _) = t {
                XCTAssertGreaterThanOrEqual(off, 1)
            }
        }
    }

    func testMatchLengthAtLeastMinMatch() {
        let tokens = encode(enc("ABABABABABAB"))
        for t in tokens {
            if case .match(_, let len) = t {
                XCTAssertGreaterThanOrEqual(len, 3)
            }
        }
    }

    func testLargeMinMatchForcesAllLiterals() {
        let tokens = encode(enc("ABABAB"), minMatch: 100)
        for t in tokens {
            if case .literal = t { } else { XCTFail("Expected literal") }
        }
    }

    // MARK: - Decode

    func testDecodeEmpty() {
        XCTAssertEqual(decode([]), [])
    }

    func testDecodeSingleLiteral() {
        XCTAssertEqual(decode([.literal(65)], originalLength: 1), [65])
    }

    func testDecodeOverlappingMatchAAAAAAA() {
        let tokens: [Token] = [.literal(65), .match(offset: 1, length: 6)]
        XCTAssertEqual(dec(decode(tokens, originalLength: 7)), "AAAAAAA")
    }

    func testDecodeABABAB() {
        let tokens: [Token] = [.literal(65), .literal(66), .match(offset: 2, length: 4)]
        XCTAssertEqual(dec(decode(tokens, originalLength: 6)), "ABABAB")
    }

    // MARK: - Round-trip

    func testRoundTripEmpty()         { XCTAssertEqual(rt(""), "") }
    func testRoundTripSingle()        { XCTAssertEqual(rt("A"), "A") }
    func testRoundTripNoRepetition()  { XCTAssertEqual(rt("ABCDE"), "ABCDE") }
    func testRoundTripAllIdentical()  { XCTAssertEqual(rt("AAAAAAA"), "AAAAAAA") }
    func testRoundTripABABAB()        { XCTAssertEqual(rt("ABABAB"), "ABABAB") }
    func testRoundTripAABCBBABC()     { XCTAssertEqual(rt("AABCBBABC"), "AABCBBABC") }
    func testRoundTripHelloWorld()    { XCTAssertEqual(rt("hello world"), "hello world") }

    func testRoundTripABCx100() {
        let data = String(repeating: "ABC", count: 100)
        XCTAssertEqual(rt(data), data)
    }

    func testRoundTripBinaryNulls() {
        let data: [UInt8] = [0, 0, 0, 255, 255]
        XCTAssertEqual(rtBytes(data), data)
    }

    func testRoundTripFullByteRange() {
        let data = [UInt8](0...255)
        XCTAssertEqual(rtBytes(data), data)
    }

    func testRoundTripRepeatedPattern() {
        let data = [UInt8]((0..<300).map { UInt8($0 % 3) })
        XCTAssertEqual(rtBytes(data), data)
    }

    func testRoundTripLongABCDEF() {
        let data = String(repeating: "ABCDEF", count: 500)
        XCTAssertEqual(rt(data), data)
    }

    // MARK: - Wire format

    func testCompressStoresOriginalLength() {
        let compressed = compress(enc("hello"))
        let origLen = Int(UInt32(compressed[0]) << 24 | UInt32(compressed[1]) << 16
                         | UInt32(compressed[2]) << 8 | UInt32(compressed[3]))
        XCTAssertEqual(origLen, 5)
    }

    func testCompressEmpty8ByteHeader() {
        let c = compress([])
        XCTAssertEqual(c.count, 8)
        let origLen = Int(UInt32(c[0]) << 24 | UInt32(c[1]) << 16 | UInt32(c[2]) << 8 | UInt32(c[3]))
        let blockCount = Int(UInt32(c[4]) << 24 | UInt32(c[5]) << 16 | UInt32(c[6]) << 8 | UInt32(c[7]))
        XCTAssertEqual(origLen, 0)
        XCTAssertEqual(blockCount, 0)
    }

    func testCompressIsDeterministic() {
        let data = enc("hello world test")
        XCTAssertEqual(compress(data), compress(data))
    }

    func testCraftedLargeBlockCountIsSafe() {
        // Craft a header claiming 0x40000000 blocks but minimal payload.
        let bad: [UInt8] = [0, 0, 0, 4,   // originalLength = 4
                            0x40, 0, 0, 0, // blockCount = huge
                            0, 65, 66, 67, 68]
        let result = decompress(bad)
        XCTAssertNotNil(result)
    }

    // MARK: - Compression effectiveness

    func testRepetitiveDataCompresses() {
        let data = enc(String(repeating: "ABC", count: 1000))
        XCTAssertLessThan(compress(data).count, data.count)
    }

    func testAllSameByteCompresses() {
        let data = [UInt8](repeating: 0x42, count: 10000)
        let compressed = compress(data)
        XCTAssertLessThan(compressed.count, data.count)
        XCTAssertEqual(decompress(compressed), data)
    }
}

import XCTest
@testable import LZ78

final class LZ78Tests: XCTestCase {

    // ─── Helpers ──────────────────────────────────────────────────────────────

    func bytes(_ s: String) -> [UInt8] { [UInt8](s.utf8) }

    func rt(_ data: [UInt8]) -> [UInt8] { decompress(compress(data)) }
    func rtStr(_ s: String) -> [UInt8]  { rt(bytes(s)) }

    // ─── TrieCursor ───────────────────────────────────────────────────────────

    func testTrieCursorNewAtRoot() {
        var c = TrieCursor()
        XCTAssertTrue(c.atRoot)
        XCTAssertEqual(c.dictID, 0)
    }

    func testTrieCursorStepMissOnEmpty() {
        var c = TrieCursor()
        XCTAssertFalse(c.step(65))
    }

    func testTrieCursorInsertThenStep() {
        var c = TrieCursor()
        c.insert(65, dictID: 1)
        XCTAssertTrue(c.atRoot, "insert should not advance cursor")
        XCTAssertTrue(c.step(65))
        XCTAssertEqual(c.dictID, 1)
        XCTAssertFalse(c.atRoot)
    }

    func testTrieCursorReset() {
        var c = TrieCursor()
        c.insert(65, dictID: 1)
        _ = c.step(65)
        c.reset()
        XCTAssertTrue(c.atRoot)
    }

    func testTrieCursorStepMissUnknownByte() {
        var c = TrieCursor()
        c.insert(65, dictID: 1)
        XCTAssertFalse(c.step(66))
    }

    func testTrieCursorLZ78Simulation() {
        // Simulate encoding "AABCBBABC" and verify the token sequence
        var cursor  = TrieCursor()
        var nextID  = UInt16(1)
        var got     = [(UInt16, UInt8)]()
        for byte: UInt8 in [65, 65, 66, 67, 66, 66, 65, 66, 67] {
            if !cursor.step(byte) {
                got.append((cursor.dictID, byte))
                cursor.insert(byte, dictID: nextID)
                nextID += 1
                cursor.reset()
            }
        }
        let want: [(UInt16, UInt8)] = [(0,65),(1,66),(0,67),(0,66),(4,65),(4,67)]
        XCTAssertEqual(got.map(\.0), want.map(\.0))
        XCTAssertEqual(got.map(\.1), want.map(\.1))
    }

    // ─── encode ───────────────────────────────────────────────────────────────

    func testEncodeEmpty() {
        XCTAssertEqual(encode([]), [])
    }

    func testEncodeSingleByte() {
        let tokens = encode([65])
        XCTAssertEqual(tokens, [Token(dictIndex: 0, nextChar: 65)])
    }

    func testEncodeNoRepetition() {
        let tokens = encode(bytes("ABCDE"))
        XCTAssertEqual(tokens.count, 5)
        XCTAssertTrue(tokens.allSatisfy { $0.dictIndex == 0 })
    }

    func testEncodeAABCBBABC() {
        let want = [
            Token(dictIndex: 0, nextChar: 65),
            Token(dictIndex: 1, nextChar: 66),
            Token(dictIndex: 0, nextChar: 67),
            Token(dictIndex: 0, nextChar: 66),
            Token(dictIndex: 4, nextChar: 65),
            Token(dictIndex: 4, nextChar: 67),
        ]
        XCTAssertEqual(encode(bytes("AABCBBABC")), want)
    }

    func testEncodeABABABFlush() {
        let want = [
            Token(dictIndex: 0, nextChar: 65),
            Token(dictIndex: 0, nextChar: 66),
            Token(dictIndex: 1, nextChar: 66),
            Token(dictIndex: 3, nextChar: 0),
        ]
        XCTAssertEqual(encode(bytes("ABABAB")), want)
    }

    func testEncodeAllIdentical() {
        XCTAssertEqual(encode(bytes("AAAAAAA")).count, 4)
    }

    // ─── decode ───────────────────────────────────────────────────────────────

    func testDecodeEmpty() {
        XCTAssertEqual(decode([], originalLength: 0), [])
    }

    func testDecodeSingleLiteral() {
        XCTAssertEqual(decode([Token(dictIndex: 0, nextChar: 65)], originalLength: 1), [65])
    }

    func testDecodeAABCBBABC() {
        let tokens = encode(bytes("AABCBBABC"))
        XCTAssertEqual(decode(tokens, originalLength: 9), bytes("AABCBBABC"))
    }

    func testDecodeABABAB() {
        let tokens = encode(bytes("ABABAB"))
        XCTAssertEqual(decode(tokens, originalLength: 6), bytes("ABABAB"))
    }

    // ─── Round-trip ───────────────────────────────────────────────────────────

    func testRoundTripEmpty()         { XCTAssertEqual(rtStr(""), bytes("")) }
    func testRoundTripSingle()        { XCTAssertEqual(rtStr("A"), bytes("A")) }
    func testRoundTripNoRepetition()  { XCTAssertEqual(rtStr("ABCDE"), bytes("ABCDE")) }
    func testRoundTripAllIdentical()  { XCTAssertEqual(rtStr("AAAAAAA"), bytes("AAAAAAA")) }
    func testRoundTripAABCBBABC()     { XCTAssertEqual(rtStr("AABCBBABC"), bytes("AABCBBABC")) }
    func testRoundTripABABAB()        { XCTAssertEqual(rtStr("ABABAB"), bytes("ABABAB")) }
    func testRoundTripHelloWorld()    { XCTAssertEqual(rtStr("hello world"), bytes("hello world")) }
    func testRoundTripRepeated()      { XCTAssertEqual(rtStr(String(repeating: "ABC", count: 100)), bytes(String(repeating: "ABC", count: 100))) }

    func testRoundTripBinaryNulls() {
        let data: [UInt8] = [0, 0, 0, 255, 255]
        XCTAssertEqual(rt(data), data)
    }

    func testRoundTripFullByteRange() {
        let data = [UInt8](0...255)
        XCTAssertEqual(rt(data), data)
    }

    func testRoundTripRepeatedPattern() {
        let data = [UInt8](Array(repeating: [0, 1, 2], count: 100).flatMap { $0 })
        XCTAssertEqual(rt(data), data)
    }

    // ─── Parameters ───────────────────────────────────────────────────────────

    func testMaxDictSizeRespected() {
        let tokens = encode(bytes("ABCABCABCABCABC"), maxDictSize: 10)
        XCTAssertTrue(tokens.allSatisfy { Int($0.dictIndex) < 10 })
    }

    func testMaxDictSize1() {
        let tokens = encode(bytes("AAAA"), maxDictSize: 1)
        XCTAssertTrue(tokens.allSatisfy { $0.dictIndex == 0 })
    }

    // ─── Wire format ──────────────────────────────────────────────────────────

    func testCompressFormatSize() {
        let data = bytes("AB")
        let compressed = compress(data)
        let tokens = encode(data)
        XCTAssertEqual(compressed.count, 8 + tokens.count * 4)
    }

    func testCompressDeterministic() {
        let data = bytes("hello world test")
        XCTAssertEqual(compress(data), compress(data))
    }

    // ─── Compression effectiveness ────────────────────────────────────────────

    func testRepetitiveDataCompresses() {
        let data = [UInt8](Array(repeating: bytes("ABC"), count: 1000).flatMap { $0 })
        XCTAssertLessThan(compress(data).count, data.count)
    }

    func testAllSameByteCompresses() {
        let data = [UInt8](repeating: 65, count: 10000)
        let compressed = compress(data)
        XCTAssertLessThan(compressed.count, data.count)
        XCTAssertEqual(decompress(compressed), data)
    }
}

import XCTest
@testable import LZW

final class LZWTests: XCTestCase {

    // ---- Constants -------------------------------------------------------

    func testConstants() {
        XCTAssertEqual(clearCode,       256)
        XCTAssertEqual(stopCode,        257)
        XCTAssertEqual(initialNextCode, 258)
        XCTAssertEqual(initialCodeSize, 9)
        XCTAssertEqual(maxCodeSize,     16)
    }

    // ---- encodeCodes ------------------------------------------------------

    func testEncodeEmpty() {
        let (codes, orig) = encodeCodes([])
        XCTAssertEqual(orig, 0)
        XCTAssertEqual(codes.first, clearCode)
        XCTAssertEqual(codes.last,  stopCode)
        XCTAssertEqual(codes.count, 2)
    }

    func testEncodeSingleByte() {
        let (codes, orig) = encodeCodes([65])
        XCTAssertEqual(orig, 1)
        XCTAssertEqual(codes.first, clearCode)
        XCTAssertEqual(codes.last,  stopCode)
        XCTAssertTrue(codes.contains(65))
    }

    func testEncodeTwoDistinct() {
        let (codes, _) = encodeCodes(Array("AB".utf8))
        XCTAssertEqual(codes, [clearCode, 65, 66, stopCode])
    }

    func testEncodeRepeatedPair() {
        let (codes, _) = encodeCodes(Array("ABABAB".utf8))
        XCTAssertEqual(codes, [clearCode, 65, 66, 258, 258, stopCode])
    }

    func testEncodeAllSame() {
        let (codes, _) = encodeCodes(Array("AAAAAAA".utf8))
        XCTAssertEqual(codes, [clearCode, 65, 258, 259, 65, stopCode])
    }

    // ---- decodeCodes ------------------------------------------------------

    func testDecodeEmptyStream() {
        XCTAssertEqual(decodeCodes([clearCode, stopCode]), [])
    }

    func testDecodeSingleByte() {
        XCTAssertEqual(decodeCodes([clearCode, 65, stopCode]), Array("A".utf8))
    }

    func testDecodeTwoDistinct() {
        XCTAssertEqual(decodeCodes([clearCode, 65, 66, stopCode]), Array("AB".utf8))
    }

    func testDecodeRepeatedPair() {
        let result = decodeCodes([clearCode, 65, 66, 258, 258, stopCode])
        XCTAssertEqual(result, Array("ABABAB".utf8))
    }

    func testDecodeTrickyToken() {
        let result = decodeCodes([clearCode, 65, 258, 259, 65, stopCode])
        XCTAssertEqual(result, Array("AAAAAAA".utf8))
    }

    func testDecodeClearMidStream() {
        let result = decodeCodes([clearCode, 65, clearCode, 66, stopCode])
        XCTAssertEqual(result, Array("AB".utf8))
    }

    func testDecodeInvalidCodeSkipped() {
        let result = decodeCodes([clearCode, 9999, 65, stopCode])
        XCTAssertEqual(result, Array("A".utf8))
    }

    // ---- packCodes / unpackCodes ------------------------------------------

    func testHeaderStoresOriginalLength() {
        let packed = packCodes([clearCode, stopCode], originalLength: 42)
        XCTAssertGreaterThanOrEqual(packed.count, 4)
        let stored = Int(UInt32(packed[0]) << 24 | UInt32(packed[1]) << 16 |
                        UInt32(packed[2]) << 8  | UInt32(packed[3]))
        XCTAssertEqual(stored, 42)
    }

    func testPackUnpackABABAB() {
        let codes: [UInt32] = [clearCode, 65, 66, 258, 258, stopCode]
        let packed = packCodes(codes, originalLength: 6)
        let (unpacked, orig) = unpackCodes(packed)
        XCTAssertEqual(orig, 6)
        XCTAssertEqual(unpacked, codes)
    }

    func testPackUnpackAllSame() {
        let codes: [UInt32] = [clearCode, 65, 258, 259, 65, stopCode]
        let packed = packCodes(codes, originalLength: 7)
        let (unpacked, orig) = unpackCodes(packed)
        XCTAssertEqual(orig, 7)
        XCTAssertEqual(unpacked, codes)
    }

    func testUnpackTruncated() {
        // Just verify that a short/truncated input doesn't crash. No assertion
        // beyond "we got here" — the return values are consumed to silence warnings.
        let (codes, orig) = unpackCodes([0x00, 0x00])
        _ = codes
        _ = orig
    }

    // ---- compress / decompress -------------------------------------------

    func rt(_ data: [UInt8]) -> [UInt8] {
        return decompress(compress(data))
    }

    func testCompressEmpty()       { XCTAssertEqual(rt([]), []) }
    func testCompressSingleByte()  { XCTAssertEqual(rt(Array("A".utf8)), Array("A".utf8)) }
    func testCompressTwoDistinct() { XCTAssertEqual(rt(Array("AB".utf8)), Array("AB".utf8)) }
    func testCompressABABAB()      { XCTAssertEqual(rt(Array("ABABAB".utf8)), Array("ABABAB".utf8)) }
    func testCompressAAAAAAATrickyToken() {
        XCTAssertEqual(rt(Array("AAAAAAA".utf8)), Array("AAAAAAA".utf8))
    }
    func testCompressAABABC()      { XCTAssertEqual(rt(Array("AABABC".utf8)), Array("AABABC".utf8)) }

    func testCompressLongString() {
        let text = String(repeating: "the quick brown fox jumps over the lazy dog ", count: 20)
        XCTAssertEqual(rt(Array(text.utf8)), Array(text.utf8))
    }

    func testCompressBinaryData() {
        let data = (0..<512).map { UInt8($0 % 256) }
        XCTAssertEqual(rt(data), data)
    }

    func testCompressAllZeros() {
        let data = [UInt8](repeating: 0x00, count: 100)
        XCTAssertEqual(rt(data), data)
    }

    func testCompressAllFF() {
        let data = [UInt8](repeating: 0xFF, count: 100)
        XCTAssertEqual(rt(data), data)
    }

    func testCompressRepetitiveData() {
        let data = Array(String(repeating: "ABCABC", count: 100).utf8)
        let compressed = compress(data)
        XCTAssertLessThan(compressed.count, data.count, "expected compression")
    }

    func testHeaderContainsOriginalLength() {
        let data = Array("hello world".utf8)
        let compressed = compress(data)
        XCTAssertGreaterThanOrEqual(compressed.count, 4)
        let stored = Int(UInt32(compressed[0]) << 24 | UInt32(compressed[1]) << 16 |
                        UInt32(compressed[2]) << 8  | UInt32(compressed[3]))
        XCTAssertEqual(stored, data.count)
    }
}

import XCTest
@testable import WasmLeb128

/// WasmLeb128Tests — unit tests for the LEB128 encoding/decoding module.
final class WasmLeb128Tests: XCTestCase {

    // MARK: - Unsigned decoding

    func testDecodeUnsignedSingleByte() throws {
        let (val, n) = try decodeLEB128Unsigned32([0x01], offset: 0)
        XCTAssertEqual(val, 1)
        XCTAssertEqual(n, 1)
    }

    func testDecodeUnsignedZero() throws {
        let (val, n) = try decodeLEB128Unsigned32([0x00], offset: 0)
        XCTAssertEqual(val, 0)
        XCTAssertEqual(n, 1)
    }

    func testDecodeUnsigned128TwoBytes() throws {
        let (val, n) = try decodeLEB128Unsigned32([0x80, 0x01], offset: 0)
        XCTAssertEqual(val, 128)
        XCTAssertEqual(n, 2)
    }

    func testDecodeUnsigned300() throws {
        let (val, n) = try decodeLEB128Unsigned32([0xAC, 0x02], offset: 0)
        XCTAssertEqual(val, 300)
        XCTAssertEqual(n, 2)
    }

    // MARK: - Signed decoding

    func testDecodeSignedPositive() throws {
        let (val, n) = try decodeLEB128Signed32([0x3F], offset: 0)
        XCTAssertEqual(val, 63)
        XCTAssertEqual(n, 1)
    }

    func testDecodeSignedNegativeOne() throws {
        let (val, n) = try decodeLEB128Signed32([0x7F], offset: 0)
        XCTAssertEqual(val, -1)
        XCTAssertEqual(n, 1)
    }

    func testDecodeSignedNegative128() throws {
        let (val, n) = try decodeLEB128Signed32([0x80, 0x7F], offset: 0)
        XCTAssertEqual(val, -128)
        XCTAssertEqual(n, 2)
    }

    // MARK: - LEB128Decoder struct

    func testDecoderDecodeU32() throws {
        let decoder = LEB128Decoder(bytes: [0x80, 0x01], offset: 0)
        let val = try decoder.decodeU32()
        XCTAssertEqual(val, 128)
    }

    func testDecoderDecodeI32Negative() throws {
        let decoder = LEB128Decoder(bytes: [0x7F], offset: 0)
        let val = try decoder.decodeI32()
        XCTAssertEqual(val, -1)
    }

    // MARK: - LEB128Encoder

    func testEncodeUnsignedSingleByte() {
        let bytes = LEB128Encoder.encodeUnsigned(1)
        XCTAssertEqual(bytes, [0x01])
    }

    func testEncodeUnsigned128() {
        let bytes = LEB128Encoder.encodeUnsigned(128)
        XCTAssertEqual(bytes, [0x80, 0x01])
    }

    func testEncodeSignedNegativeOne() {
        let bytes = LEB128Encoder.encodeSigned(-1)
        XCTAssertEqual(bytes, [0x7F])
    }

    func testRoundTripUnsigned() throws {
        let original: UInt32 = 624485
        let encoded = LEB128Encoder.encodeUnsigned(UInt64(original))
        let (decoded, _) = try decodeLEB128Unsigned32(encoded, offset: 0)
        XCTAssertEqual(decoded, original)
    }

    func testRoundTripSigned() throws {
        let original: Int32 = -123456
        let encoded = LEB128Encoder.encodeSigned(Int64(original))
        let (decoded, _) = try decodeLEB128Signed32(encoded, offset: 0)
        XCTAssertEqual(decoded, original)
    }

    // MARK: - 64-bit

    func testDecodeUnsigned64() throws {
        let (val, n) = try decodeLEB128Unsigned64([0x80, 0x80, 0x04], offset: 0)
        XCTAssertEqual(val, 65536)
        XCTAssertEqual(n, 3)
    }

    func testDecodeSigned64Negative() throws {
        let (val, n) = try decodeLEB128Signed64([0x7F], offset: 0)
        XCTAssertEqual(val, -1)
        XCTAssertEqual(n, 1)
    }
}

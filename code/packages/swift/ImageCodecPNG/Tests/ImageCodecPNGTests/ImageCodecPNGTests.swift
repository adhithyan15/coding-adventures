import PixelContainer
import XCTest
import Zip

@testable import ImageCodecPNG

final class ImageCodecPNGTests: XCTestCase {
  func testImageCodecContract() throws {
    let codec: any ImageCodec = PngCodec()
    XCTAssertEqual(codec.mimeType, "image/png")

    var pixels = PixelContainer(width: 1, height: 1)
    pixels.data = [1, 2, 3, 4]
    let encoded = codec.encode(pixels)
    XCTAssertEqual(try codec.decode(encoded).data, pixels.data)
    XCTAssertEqual(try PngCodec(maxPixels: 1).decode(encoded).data, pixels.data)
  }

  func testCallerPixelLimitsRejectNumericCoercion() {
    let invalid: [Double] = [
      0,
      -1,
      1.5,
      Double(pngDefaultMaxPixels + 1),
      .nan,
      .infinity,
      -.infinity,
    ]
    for value in invalid {
      expectPngError("invalid-max-pixels") {
        _ = try PngCodec(maxPixels: value)
      }
      expectPngError("invalid-max-pixels") {
        _ = try decodePng([], maxPixels: value)
      }
    }
  }

  func testEncoderValidatesMutablePixelContainerState() {
    expectPngError("invalid-image-dimensions") {
      _ = try encodePng(PixelContainer(width: 0, height: 1))
    }
    expectPngError("invalid-image-dimensions") {
      _ = try encodePng(PixelContainer(width: UInt32(pngMaxDimension + 1), height: 1))
    }

    var malformed = PixelContainer(width: 1, height: 1)
    malformed.data = [1, 2, 3]
    expectPngError("invalid-pixel-data-length") {
      _ = try encodePng(malformed)
    }
  }

  func testEncoderShapeChecksProductBeforeAllocation() {
    expectPngError("invalid-image-dimensions") {
      _ = try validateEncodeShape(width: 16_384, height: 2_049, dataCount: 0)
    }
    expectPngError("invalid-image-dimensions") {
      _ = try validateEncodeShape(width: UInt32.max, height: 1, dataCount: 0)
    }
  }

  func testPayloadBlindErrorTaxonomy() {
    XCTAssertEqual(pngErrorCodes.count, 29)
    XCTAssertEqual(Set(pngErrorCodes).count, 29)
    let error = PngError("invalid-filter")
    XCTAssertEqual(error.code, "invalid-filter")
    XCTAssertEqual(error.description, "invalid-filter")
  }

  func testAPNGPreservesCRCAndFirstIHDRPrecedence() throws {
    let encoded = try encodePng(PixelContainer(width: 1, height: 1))
    let valid = chunk("acTL", payload: [UInt8](repeating: 0, count: 8))
    expectPngError("unsupported-feature") {
      _ = try decodePng(insert(valid, into: encoded, at: 33))
    }

    var corrupt = valid
    corrupt[corrupt.count - 1] ^= 1
    expectPngError("chunk-crc-mismatch") {
      _ = try decodePng(insert(corrupt, into: encoded, at: 33))
    }
    expectPngError("chunk-before-ihdr") {
      _ = try decodePng(insert(valid, into: encoded, at: 8))
    }
  }

  func testAdler32PublishedVectorsAndReductionBoundary() {
    XCTAssertEqual(adler32(Array("Wikipedia".utf8)), 0x11E6_0398)
    let boundary = (0..<5_553).map { UInt8(truncatingIfNeeded: $0) }
    XCTAssertEqual(adler32(boundary), 0x2CCA_B2EF)
  }

  private func expectPngError(
    _ expected: String,
    _ operation: () throws -> Void,
    file: StaticString = #filePath,
    line: UInt = #line
  ) {
    do {
      try operation()
      XCTFail("expected PngError(\(expected))", file: file, line: line)
    } catch let error as PngError {
      XCTAssertEqual(error.code, expected, file: file, line: line)
      XCTAssertEqual(error.description, expected, file: file, line: line)
    } catch {
      XCTFail("wrong error type: \(error)", file: file, line: line)
    }
  }

  private func chunk(_ type: String, payload: [UInt8]) -> [UInt8] {
    let typeBytes = Array(type.utf8)
    var checksum = crc32(typeBytes)
    checksum = crc32(payload, initial: checksum)
    return u32(payload.count) + typeBytes + payload + u32(Int(checksum))
  }

  private func insert(_ inserted: [UInt8], into original: [UInt8], at offset: Int) -> [UInt8] {
    Array(original[..<offset]) + inserted + Array(original[offset...])
  }

  private func u32(_ value: Int) -> [UInt8] {
    let unsigned = UInt32(truncatingIfNeeded: value)
    return [
      UInt8((unsigned >> 24) & 0xFF),
      UInt8((unsigned >> 16) & 0xFF),
      UInt8((unsigned >> 8) & 0xFF),
      UInt8(unsigned & 0xFF),
    ]
  }
}

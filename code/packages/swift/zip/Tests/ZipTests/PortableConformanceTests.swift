import Foundation
import XCTest

@testable import Zip

final class PortableConformanceTests: XCTestCase {
  private static let expectedErrorCodes = [
    "invalid-output-limit",
    "unexpected-eof",
    "reserved-block-type",
    "stored-length-mismatch",
    "huffman-oversubscribed",
    "incomplete-code-length-tree",
    "incomplete-literal-length-tree",
    "incomplete-distance-tree",
    "repeat-without-previous",
    "repeat-overrun",
    "invalid-literal-length-symbol",
    "reserved-distance-symbol",
    "invalid-back-reference",
    "output-limit-exceeded",
  ]

  func testPortableCorpus() throws {
    let document = try Self.loadFixture()
    XCTAssertEqual((document["schema_version"] as? NSNumber)?.intValue, 1)
    XCTAssertEqual(document["profile"] as? String, "zip-owned-raw-rfc1951-v1")

    let limits = try XCTUnwrap(document["limits"] as? [String: Any])
    XCTAssertEqual((limits["default_max_output"] as? NSNumber)?.intValue, 268_435_456)
    XCTAssertEqual((limits["hard_max_output"] as? NSNumber)?.intValue, 268_435_456)
    XCTAssertEqual(rawInflateMaxOutput, 268_435_456)

    let errorCodes = try XCTUnwrap(document["error_ids"] as? [String])
    XCTAssertEqual(errorCodes, Self.expectedErrorCodes)
    XCTAssertEqual(rawInflateErrorCodes, Self.expectedErrorCodes)

    let cases = try XCTUnwrap(document["cases"] as? [[String: Any]])
    XCTAssertEqual(cases.count, 34)

    for testCase in cases {
      let id = try XCTUnwrap(testCase["id"] as? String)
      let operation = try XCTUnwrap(testCase["operation"] as? String)
      switch operation {
      case "inflate":
        let input = try Self.bytes(fromHex: try XCTUnwrap(testCase["input_hex"] as? String))
        let expected = try XCTUnwrap(testCase["expected"] as? [String: Any])
        let outputSpec = try XCTUnwrap(expected["output"] as? [String: Any])
        let expectedOutput = try Self.materialize(outputSpec)
        let expectedConsumed = try XCTUnwrap(
          (expected["bytes_consumed"] as? NSNumber)?.intValue
        )
        let maxOutput =
          (testCase["max_output"] as? NSNumber)?.intValue
          ?? rawInflateMaxOutput

        let result = try rawInflateCounted(input, maxOutput: maxOutput)
        XCTAssertEqual(result.output, expectedOutput, id)
        XCTAssertEqual(result.bytesConsumed, expectedConsumed, id)
        XCTAssertEqual(try rawInflate(input, maxOutput: maxOutput), expectedOutput, id)

      case "inflate-error":
        let input = try Self.bytes(fromHex: try XCTUnwrap(testCase["input_hex"] as? String))
        let expected = try XCTUnwrap(testCase["expected"] as? [String: Any])
        let expectedCode = try XCTUnwrap(expected["error_id"] as? String)
        let maxOutput =
          (testCase["max_output"] as? NSNumber)?.intValue
          ?? rawInflateMaxOutput

        do {
          _ = try rawInflateCounted(input, maxOutput: maxOutput)
          XCTFail("\(id): expected \(expectedCode)")
        } catch let error as RawInflateError {
          XCTAssertEqual(error.code, expectedCode, id)
          XCTAssertEqual(error.description, expectedCode, id)
        } catch {
          XCTFail("\(id): wrong error type: \(error)")
        }

      case "deflate-interoperability":
        let input = try Self.bytes(fromHex: try XCTUnwrap(testCase["input_hex"] as? String))
        let expected = try XCTUnwrap(testCase["expected"] as? [String: Any])
        let outputSpec = try XCTUnwrap(expected["output"] as? [String: Any])
        let expectedOutput = try Self.materialize(outputSpec)
        let encoded = rawDeflate(input)
        XCTAssertEqual(try Self.pythonRawDeflate("decompress", input: encoded), expectedOutput, id)

      case "crc32":
        let chunks = try XCTUnwrap(testCase["chunks_hex"] as? [String])
        let initialHex = testCase["initial_crc32_hex"] as? String
        var checksum = initialHex.flatMap { UInt32($0, radix: 16) } ?? 0
        for chunk in chunks {
          checksum = crc32(try Self.bytes(fromHex: chunk), initial: checksum)
        }
        let expected = try XCTUnwrap(testCase["expected"] as? [String: Any])
        let expectedHex = try XCTUnwrap(expected["crc32_hex"] as? String)
        XCTAssertEqual(String(format: "%08x", checksum), expectedHex, id)

      default:
        XCTFail("\(id): unsupported fixture operation \(operation)")
      }
    }
  }

  func testDynamicZipAndStrictPayloadBoundaries() throws {
    let fixture = try Self.loadFixture()
    let cases = try XCTUnwrap(fixture["cases"] as? [[String: Any]])
    let dynamic = try XCTUnwrap(
      cases.first {
        ($0["id"] as? String) == "zip-raw-v1-inflate-dynamic-foreign"
      })
    let compressed = try Self.bytes(fromHex: try XCTUnwrap(dynamic["input_hex"] as? String))
    let expected = try XCTUnwrap(dynamic["expected"] as? [String: Any])
    let plain = try Self.materialize(try XCTUnwrap(expected["output"] as? [String: Any]))

    let archive = Self.rawZip(name: "dynamic.bin", compressed: compressed, plain: plain)
    XCTAssertEqual(try ZipReader(archive).readByName("dynamic.bin"), plain)

    let cavityArchive = Self.rawZip(
      name: "dynamic.bin",
      compressed: compressed + [0xDE, 0xAD],
      plain: plain
    )
    XCTAssertThrowsError(try ZipReader(cavityArchive).readByName("dynamic.bin")) { error in
      guard case ZipError.malformed(let message) = error else {
        return XCTFail("wrong suffix-cavity error: \(error)")
      }
      XCTAssertEqual(message, "zip: compressed payload contains trailing bytes")
    }

    let sizeArchive = Self.rawZip(
      name: "dynamic.bin",
      compressed: compressed,
      plain: plain,
      declaredSize: plain.count + 1
    )
    XCTAssertThrowsError(try ZipReader(sizeArchive).readByName("dynamic.bin")) { error in
      guard case ZipError.malformed(let message) = error else {
        return XCTFail("wrong declared-size error: \(error)")
      }
      XCTAssertEqual(message, "zip: uncompressed size does not match the directory")
    }
  }

  func testForeignFullWindowAndHistoricalWrappers() throws {
    let prefix = (0..<32_768).map { index in
      UInt8(truncatingIfNeeded: (index * 73) + (index / 251))
    }
    let expected = prefix + prefix
    let foreign = try Self.pythonRawDeflate("compress", input: expected)
    XCTAssertEqual(try rawInflate(foreign, maxOutput: expected.count), expected)

    let historical = Array("historical wrapper compatibility".utf8)
    XCTAssertEqual(try deflateDecompress(deflateCompress(historical)), historical)
  }

  func testDirectOutputLimitValidation() {
    for invalid in [-1, rawInflateMaxOutput + 1] {
      XCTAssertThrowsError(
        try rawInflateCounted([0x01, 0x00, 0x00, 0xFF, 0xFF], maxOutput: invalid)
      ) {
        error in
        XCTAssertEqual((error as? RawInflateError)?.code, "invalid-output-limit")
      }
    }
  }

  private static func loadFixture() throws -> [String: Any] {
    var codeRoot = URL(fileURLWithPath: #filePath)
    for _ in 0..<6 {
      codeRoot.deleteLastPathComponent()
    }
    let fixtureURL =
      codeRoot
      .appendingPathComponent("specs")
      .appendingPathComponent("fixtures")
      .appendingPathComponent("zip-raw-rfc1951-v1")
      .appendingPathComponent("cases.json")
    let object = try JSONSerialization.jsonObject(with: Data(contentsOf: fixtureURL))
    return try XCTUnwrap(object as? [String: Any])
  }

  private static func materialize(_ specification: [String: Any]) throws -> [UInt8] {
    if let hex = specification["hex"] as? String {
      return try bytes(fromHex: hex)
    }
    let byte = try bytes(fromHex: try XCTUnwrap(specification["repeat_hex"] as? String))
    let count = try XCTUnwrap((specification["count"] as? NSNumber)?.intValue)
    return [UInt8](repeating: try XCTUnwrap(byte.first), count: count)
  }

  private static func bytes(fromHex hex: String) throws -> [UInt8] {
    guard hex.count.isMultiple(of: 2) else {
      throw FixtureError.invalidHex
    }
    var result: [UInt8] = []
    result.reserveCapacity(hex.count / 2)
    var index = hex.startIndex
    while index < hex.endIndex {
      let next = hex.index(index, offsetBy: 2)
      guard let byte = UInt8(hex[index..<next], radix: 16) else {
        throw FixtureError.invalidHex
      }
      result.append(byte)
      index = next
    }
    return result
  }

  private static func pythonRawDeflate(_ mode: String, input: [UInt8]) throws -> [UInt8] {
    let process = Process()
    let launch = try pythonLaunch()
    process.executableURL = launch.executable
    process.arguments =
      launch.prefixArguments + [
        "-c",
        """
        import sys, zlib
        mode = sys.argv[1]
        data = sys.stdin.buffer.read()
        if mode == "compress":
            codec = zlib.compressobj(level=9, wbits=-15)
            result = codec.compress(data) + codec.flush()
        else:
            result = zlib.decompress(data, -15)
        sys.stdout.buffer.write(result)
        """,
        mode,
      ]

    let inputPipe = Pipe()
    let outputPipe = Pipe()
    let errorPipe = Pipe()
    process.standardInput = inputPipe
    process.standardOutput = outputPipe
    process.standardError = errorPipe
    try process.run()
    inputPipe.fileHandleForWriting.write(Data(input))
    try inputPipe.fileHandleForWriting.close()
    let output = outputPipe.fileHandleForReading.readDataToEndOfFile()
    let diagnostic = errorPipe.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    guard process.terminationStatus == 0 else {
      throw FixtureError.pythonFailed(String(decoding: diagnostic, as: UTF8.self))
    }
    return [UInt8](output)
  }

  private static func pythonLaunch() throws -> (executable: URL, prefixArguments: [String]) {
    #if os(Windows)
      let environment = ProcessInfo.processInfo.environment
      if let location = environment["pythonLocation"] {
        let candidate = URL(fileURLWithPath: location).appendingPathComponent("python.exe")
        if FileManager.default.fileExists(atPath: candidate.path) {
          return (candidate, [])
        }
      }
      if let local = environment["LOCALAPPDATA"] {
        let launcher = URL(fileURLWithPath: local)
          .appendingPathComponent("Programs")
          .appendingPathComponent("Python")
          .appendingPathComponent("Launcher")
          .appendingPathComponent("py.exe")
        if FileManager.default.fileExists(atPath: launcher.path) {
          return (launcher, ["-3"])
        }
      }
      throw FixtureError.pythonUnavailable
    #else
      return (URL(fileURLWithPath: "/usr/bin/env"), ["python3"])
    #endif
  }

  private static func rawZip(
    name: String,
    compressed: [UInt8],
    plain: [UInt8],
    declaredSize: Int? = nil
  ) -> [UInt8] {
    let nameBytes = Array(name.utf8)
    let size = UInt32(declaredSize ?? plain.count)
    let compressedSize = UInt32(compressed.count)
    let checksum = crc32(plain)
    var local: [UInt8] = []
    local += littleEndian32(0x0403_4B50)
    local += littleEndian16(20)
    local += littleEndian16(0x0800)
    local += littleEndian16(8)
    local += littleEndian16(0)
    local += littleEndian16(0)
    local += littleEndian32(checksum)
    local += littleEndian32(compressedSize)
    local += littleEndian32(size)
    local += littleEndian16(UInt16(nameBytes.count))
    local += littleEndian16(0)
    local += nameBytes
    local += compressed

    var central: [UInt8] = []
    central += littleEndian32(0x0201_4B50)
    central += littleEndian16(0x031E)
    central += littleEndian16(20)
    central += littleEndian16(0x0800)
    central += littleEndian16(8)
    central += littleEndian16(0)
    central += littleEndian16(0)
    central += littleEndian32(checksum)
    central += littleEndian32(compressedSize)
    central += littleEndian32(size)
    central += littleEndian16(UInt16(nameBytes.count))
    central += littleEndian16(0)
    central += littleEndian16(0)
    central += littleEndian16(0)
    central += littleEndian16(0)
    central += littleEndian32(0)
    central += littleEndian32(0)
    central += nameBytes

    var eocd: [UInt8] = []
    eocd += littleEndian32(0x0605_4B50)
    eocd += littleEndian16(0)
    eocd += littleEndian16(0)
    eocd += littleEndian16(1)
    eocd += littleEndian16(1)
    eocd += littleEndian32(UInt32(central.count))
    eocd += littleEndian32(UInt32(local.count))
    eocd += littleEndian16(0)
    return local + central + eocd
  }

  private static func littleEndian16(_ value: UInt16) -> [UInt8] {
    [UInt8(value & 0xFF), UInt8(value >> 8)]
  }

  private static func littleEndian32(_ value: UInt32) -> [UInt8] {
    [
      UInt8(value & 0xFF),
      UInt8((value >> 8) & 0xFF),
      UInt8((value >> 16) & 0xFF),
      UInt8((value >> 24) & 0xFF),
    ]
  }

  private enum FixtureError: Error {
    case invalidHex
    case pythonUnavailable
    case pythonFailed(String)
  }
}

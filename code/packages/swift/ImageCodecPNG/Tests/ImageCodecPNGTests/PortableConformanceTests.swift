import Foundation
import PixelContainer
import XCTest

@testable import ImageCodecPNG

#if canImport(CoreGraphics) && canImport(ImageIO)
  import CoreGraphics
  import ImageIO
#endif

final class PortableConformanceTests: XCTestCase {
  private static let expectedErrorCodes = [
    "invalid-max-pixels",
    "invalid-image-dimensions",
    "invalid-pixel-data-length",
    "file-too-short",
    "invalid-signature",
    "truncated-chunk",
    "invalid-chunk-type",
    "chunk-crc-mismatch",
    "chunk-before-ihdr",
    "duplicate-ihdr",
    "invalid-ihdr-length",
    "invalid-dimensions",
    "dimension-limit",
    "pixel-limit",
    "unsupported-feature",
    "invalid-plte",
    "invalid-trns",
    "nonconsecutive-idat",
    "invalid-iend",
    "trailing-data",
    "unknown-critical-chunk",
    "missing-required-chunk",
    "invalid-zlib-header",
    "preset-dictionary",
    "inflate-failed",
    "inflated-length-mismatch",
    "idat-cavity",
    "adler-mismatch",
    "invalid-filter",
  ]

  func testPortableCorpusThroughPublicAPIs() throws {
    let document = try Self.loadFixture()
    XCTAssertEqual((document["schema_version"] as? NSNumber)?.intValue, 1)
    XCTAssertEqual(document["profile"] as? String, "image-codec-png-v1")

    let limits = try XCTUnwrap(document["limits"] as? [String: Any])
    XCTAssertEqual((limits["max_dimension"] as? NSNumber)?.intValue, pngMaxDimension)
    XCTAssertEqual(
      (limits["default_max_pixels"] as? NSNumber)?.intValue,
      pngDefaultMaxPixels
    )
    XCTAssertEqual(try XCTUnwrap(document["error_ids"] as? [String]), Self.expectedErrorCodes)
    XCTAssertEqual(pngErrorCodes, Self.expectedErrorCodes)

    let cases = try XCTUnwrap(document["cases"] as? [[String: Any]])
    XCTAssertEqual(cases.count, 85)

    var consumed = 0
    for fixture in cases {
      let id = try XCTUnwrap(fixture["id"] as? String)
      let operation = try XCTUnwrap(fixture["operation"] as? String)
      switch operation {
      case "decode":
        try Self.assertDecode(fixture, id: id)
      case "decode-error":
        try Self.assertError(fixture, id: id) {
          _ = try Self.decodeFixture(fixture)
        }
      case "encode":
        try Self.assertEncode(fixture, id: id)
      case "encode-error":
        let input = try XCTUnwrap(fixture["input"] as? [String: Any])
        try Self.assertError(fixture, id: id) {
          _ = try Self.encodeFixture(input)
        }
      case "adler32":
        let input = try Self.bytes(fromHex: try XCTUnwrap(fixture["input_hex"] as? String))
        let expected = try XCTUnwrap(fixture["expected"] as? [String: Any])
        let expectedHex = try XCTUnwrap(expected["adler32_hex"] as? String)
        XCTAssertEqual(String(format: "%08x", adler32(input)), expectedHex, id)
      default:
        XCTFail("\(id): unsupported fixture operation \(operation)")
      }
      consumed += 1
    }
    XCTAssertEqual(consumed, 85)
  }

  private static func assertDecode(_ fixture: [String: Any], id: String) throws {
    let actual = try decodeFixture(fixture)
    let expected = try XCTUnwrap(fixture["expected"] as? [String: Any])
    XCTAssertEqual(actual.width, UInt32(try exactInteger(expected["width"])), id)
    XCTAssertEqual(actual.height, UInt32(try exactInteger(expected["height"])), id)
    XCTAssertEqual(
      actual.data, try bytes(fromHex: try XCTUnwrap(expected["rgba_hex"] as? String)), id)
  }

  private static func decodeFixture(_ fixture: [String: Any]) throws -> PixelContainer {
    let png = try bytes(fromHex: try XCTUnwrap(fixture["png_hex"] as? String))
    let options = fixture["options"] as? [String: Any]
    let maxPixels = (options?["max_pixels"] as? NSNumber)?.doubleValue
    return try decodePng(png, maxPixels: maxPixels)
  }

  private static func assertEncode(_ fixture: [String: Any], id: String) throws {
    let input = try XCTUnwrap(fixture["input"] as? [String: Any])
    let encoded = try encodeFixture(input)
    XCTAssertEqual(encoded, try encodeFixture(input), "\(id): encoding must be deterministic")
    let expected = try XCTUnwrap(fixture["expected"] as? [String: Any])
    let chunks = try parseChunks(encoded)
    XCTAssertEqual(chunks.map(\.type), try XCTUnwrap(expected["chunk_types"] as? [String]), id)
    XCTAssertEqual(encoded[24], UInt8(try exactInteger(expected["bit_depth"])), id)
    XCTAssertEqual(encoded[25], UInt8(try exactInteger(expected["colour_type"])), id)
    XCTAssertEqual(encoded[28], UInt8(try exactInteger(expected["interlace"])), id)

    let idat = chunks.filter { $0.type == "IDAT" }.flatMap(\.data)
    let filtered = try runPython(script: Self.inflateScript, input: idat)
    let width = try exactInteger(input["width"])
    let height = try exactInteger(input["height"])
    let rowSize = width * 4 + 1
    let filters = (0..<height).map { Int(filtered[$0 * rowSize]) }
    let expectedFilters = try XCTUnwrap(expected["filter_types"] as? [NSNumber]).map(\.intValue)
    XCTAssertEqual(filters, expectedFilters, id)

    let expectedRGBA = try bytes(fromHex: try XCTUnwrap(input["rgba_hex"] as? String))
    let roundTrip = try decodePng(encoded)
    XCTAssertEqual(Int(roundTrip.width), width, id)
    XCTAssertEqual(Int(roundTrip.height), height, id)
    XCTAssertEqual(roundTrip.data, expectedRGBA, id)

    let foreign = try runPython(script: Self.fullPngDecodeScript, input: encoded)
    XCTAssertGreaterThanOrEqual(foreign.count, 8, id)
    XCTAssertEqual(Int(readU32(foreign, at: 0)), width, id)
    XCTAssertEqual(Int(readU32(foreign, at: 4)), height, id)
    XCTAssertEqual(Array(foreign.dropFirst(8)), expectedRGBA, id)

    try assertRealImageToolAccepts(encoded, width: width, height: height, id: id)
  }

  private static func encodeFixture(_ input: [String: Any]) throws -> [UInt8] {
    let width = try exactInteger(input["width"])
    let height = try exactInteger(input["height"])
    guard width >= 0, height >= 0, width <= Int(UInt32.max), height <= Int(UInt32.max) else {
      throw PngError("invalid-image-dimensions")
    }
    let rgba = try bytes(fromHex: try XCTUnwrap(input["rgba_hex"] as? String))
    var pixels = PixelContainer(width: UInt32(width), height: UInt32(height))
    pixels.data = rgba
    return try encodePng(pixels)
  }

  private static func exactInteger(_ raw: Any?) throws -> Int {
    guard let number = raw as? NSNumber else {
      throw PngError("invalid-image-dimensions")
    }
    // `NSNumber(1) is Bool` also evaluates true on some Swift Foundation
    // implementations. JSON booleans retain Objective-C's `c` type code;
    // checking it avoids rejecting ordinary integer 0/1 fixture values.
    if String(cString: number.objCType) == "c" {
      throw PngError("invalid-image-dimensions")
    }
    let value = number.doubleValue
    guard value.isFinite, value.rounded(.towardZero) == value, value >= 0,
      let exact = Int(exactly: value)
    else {
      throw PngError("invalid-image-dimensions")
    }
    return exact
  }

  private static func assertError(
    _ fixture: [String: Any],
    id: String,
    operation: () throws -> Void
  ) throws {
    let expected = try XCTUnwrap(fixture["expected"] as? [String: Any])
    let expectedCode = try XCTUnwrap(expected["error_id"] as? String)
    do {
      try operation()
      XCTFail("\(id): expected PngError(\(expectedCode))")
    } catch let error as PngError {
      XCTAssertEqual(error.code, expectedCode, id)
      XCTAssertEqual(error.description, expectedCode, id)
    } catch {
      XCTFail("\(id): wrong error type: \(error)")
    }
  }

  private static func loadFixture() throws -> [String: Any] {
    var codeRoot = URL(fileURLWithPath: #filePath)
    for _ in 0..<6 {
      codeRoot.deleteLastPathComponent()
    }
    let fixture =
      codeRoot
      .appendingPathComponent("specs")
      .appendingPathComponent("fixtures")
      .appendingPathComponent("image-codec-png-v1")
      .appendingPathComponent("cases.json")
    let object = try JSONSerialization.jsonObject(with: Data(contentsOf: fixture))
    return try XCTUnwrap(object as? [String: Any])
  }

  private static func bytes(fromHex hex: String) throws -> [UInt8] {
    guard hex.count.isMultiple(of: 2) else { throw FixtureError.invalidHex }
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

  private static func parseChunks(_ png: [UInt8]) throws -> [Chunk] {
    guard png.count >= 8 else { throw FixtureError.invalidPng }
    var chunks: [Chunk] = []
    var offset = 8
    while offset < png.count {
      guard offset <= png.count - 12 else { throw FixtureError.invalidPng }
      let length = Int(readU32(png, at: offset))
      guard length <= png.count - offset - 12 else { throw FixtureError.invalidPng }
      let typeBytes = Array(png[(offset + 4)..<(offset + 8)])
      guard let type = String(bytes: typeBytes, encoding: .utf8) else {
        throw FixtureError.invalidPng
      }
      chunks.append(Chunk(type: type, data: Array(png[(offset + 8)..<(offset + 8 + length)])))
      offset += 12 + length
    }
    return chunks
  }

  private static func readU32(_ data: [UInt8], at offset: Int) -> UInt32 {
    (UInt32(data[offset]) << 24)
      | (UInt32(data[offset + 1]) << 16)
      | (UInt32(data[offset + 2]) << 8)
      | UInt32(data[offset + 3])
  }

  private static func runPython(script: String, input: [UInt8]) throws -> [UInt8] {
    let launch = try pythonLaunch()
    let process = Process()
    process.executableURL = launch.executable
    process.arguments = launch.prefixArguments + ["-c", script]
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

  private static func assertRealImageToolAccepts(
    _ png: [UInt8], width: Int, height: Int, id: String
  ) throws {
    #if canImport(CoreGraphics) && canImport(ImageIO)
      let source = try XCTUnwrap(
        CGImageSourceCreateWithData(Data(png) as CFData, nil),
        "\(id): ImageIO rejected the encoded PNG"
      )
      let image = try XCTUnwrap(
        CGImageSourceCreateImageAtIndex(source, 0, nil),
        "\(id): ImageIO could not decode the encoded PNG"
      )
      XCTAssertEqual(image.width, width, id)
      XCTAssertEqual(image.height, height, id)
    #elseif os(Windows)
      let systemRoot = ProcessInfo.processInfo.environment["SystemRoot"] ?? "C:\\Windows"
      let powershell = URL(fileURLWithPath: systemRoot)
        .appendingPathComponent("System32")
        .appendingPathComponent("WindowsPowerShell")
        .appendingPathComponent("v1.0")
        .appendingPathComponent("powershell.exe")
      let script = """
        $ErrorActionPreference = 'Stop'
        Add-Type -AssemblyName PresentationCore
        $stream = [Console]::OpenStandardInput()
        $memory = [System.IO.MemoryStream]::new()
        $stream.CopyTo($memory)
        $memory.Position = 0
        $decoder = [Windows.Media.Imaging.PngBitmapDecoder]::new(
          $memory,
          [Windows.Media.Imaging.BitmapCreateOptions]::PreservePixelFormat,
          [Windows.Media.Imaging.BitmapCacheOption]::OnLoad)
        $frame = $decoder.Frames[0]
        [Console]::Out.Write(("{0}x{1}" -f $frame.PixelWidth, $frame.PixelHeight))
        """
      let dimensions = try runProcess(
        executable: powershell,
        arguments: ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script],
        input: png
      )
      XCTAssertEqual(String(decoding: dimensions, as: UTF8.self), "\(width)x\(height)", id)
    #else
      // The macOS and Windows lanes exercise real platform PNG decoders. The
      // portable Python parser above remains the independent Linux oracle.
    #endif
  }

  private static func runProcess(
    executable: URL, arguments: [String], input: [UInt8]
  ) throws -> [UInt8] {
    let process = Process()
    process.executableURL = executable
    process.arguments = arguments
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
      throw FixtureError.realImageToolFailed(String(decoding: diagnostic, as: UTF8.self))
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

  private static let inflateScript = """
    import sys, zlib
    sys.stdout.buffer.write(zlib.decompress(sys.stdin.buffer.read()))
    """

  private static let fullPngDecodeScript = """
    import binascii, struct, sys, zlib
    data = sys.stdin.buffer.read()
    assert data[:8] == b'\\x89PNG\\r\\n\\x1a\\n'
    pos, width, height, idat = 8, None, None, bytearray()
    while pos < len(data):
        length = struct.unpack('>I', data[pos:pos + 4])[0]
        kind = data[pos + 4:pos + 8]
        payload = data[pos + 8:pos + 8 + length]
        crc = struct.unpack('>I', data[pos + 8 + length:pos + 12 + length])[0]
        assert (binascii.crc32(kind + payload) & 0xffffffff) == crc
        if kind == b'IHDR':
            width, height, depth, colour, compression, filtering, interlace = struct.unpack('>IIBBBBB', payload)
            assert (depth, colour, compression, filtering, interlace) == (8, 6, 0, 0, 0)
        elif kind == b'IDAT':
            idat.extend(payload)
        elif kind == b'IEND':
            break
        pos += 12 + length
    raw = zlib.decompress(bytes(idat))
    stride, prior, rgba, offset = width * 4, bytearray(width * 4), bytearray(), 0
    def paeth(a, b, c):
        p = a + b - c
        pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
        return a if pa <= pb and pa <= pc else (b if pb <= pc else c)
    for _ in range(height):
        mode, row = raw[offset], bytearray(raw[offset + 1:offset + 1 + stride])
        offset += stride + 1
        for i in range(stride):
            a = row[i - 4] if i >= 4 else 0
            b = prior[i]
            c = prior[i - 4] if i >= 4 else 0
            prediction = (0, a, b, (a + b) // 2, paeth(a, b, c))[mode]
            row[i] = (row[i] + prediction) & 0xff
        rgba.extend(row)
        prior = row
    assert offset == len(raw)
    sys.stdout.buffer.write(struct.pack('>II', width, height) + rgba)
    """

  private struct Chunk {
    let type: String
    let data: [UInt8]
  }

  private enum FixtureError: Error {
    case invalidHex
    case invalidPng
    case pythonUnavailable
    case pythonFailed(String)
    case realImageToolFailed(String)
  }
}

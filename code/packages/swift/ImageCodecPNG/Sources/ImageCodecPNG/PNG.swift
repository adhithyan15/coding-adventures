import PixelContainer
import Zip

/// Largest width or height accepted by the portable PNG profile.
public let pngMaxDimension = 16_384

/// Default and hard pixel-count ceiling.
public let pngDefaultMaxPixels = 32 * 1_024 * 1_024

/// Closed IC18 failure taxonomy in normative order.
public let pngErrorCodes = [
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

/// A stable, payload-blind portable PNG failure.
public struct PngError: Error, Equatable, CustomStringConvertible {
  public let code: String

  public init(_ code: String) {
    self.code = code
  }

  public var description: String { code }
}

@inline(__always)
private func fail(_ code: String) throws -> Never {
  throw PngError(code)
}

private func validatedMaxPixels(_ requested: Double?) throws -> Int {
  guard let requested else { return pngDefaultMaxPixels }
  guard requested.isFinite,
    requested.rounded(.towardZero) == requested,
    requested > 0,
    requested <= Double(pngDefaultMaxPixels)
  else {
    try fail("invalid-max-pixels")
  }
  return Int(requested)
}

/// Stateful `ImageCodec` adapter with an eagerly validated pixel ceiling.
public struct PngCodec: ImageCodec {
  private let maximumPixels: Int

  public init() {
    maximumPixels = pngDefaultMaxPixels
  }

  public init(maxPixels: Double) throws {
    maximumPixels = try validatedMaxPixels(maxPixels)
  }

  public var mimeType: String { "image/png" }

  /// Compatibility witness for IC00's historical nonthrowing encoder.
  ///
  /// IC00 treats every `PixelContainer` as valid. Because its bytes remain
  /// publicly mutable, callers that need typed validation must call
  /// `encodePng(_:)` directly.
  public func encode(_ pixels: PixelContainer) -> [UInt8] {
    guard let encoded = try? encodePng(pixels) else {
      preconditionFailure(
        "PngCodec.encode requires a valid PixelContainer; use encodePng for errors")
    }
    return encoded
  }

  public func decode(_ bytes: [UInt8]) throws -> PixelContainer {
    try decodePngWithLimit(bytes, maximumPixels: maximumPixels)
  }
}

private let signature: [UInt8] = [
  0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
]

private let adlerModulus: UInt64 = 65_521

/// Compute the RFC 1950 Adler-32 checksum used by PNG's zlib wrapper.
public func adler32(_ data: [UInt8]) -> UInt32 {
  var a: UInt64 = 1
  var b: UInt64 = 0
  var start = 0
  while start < data.count {
    let end = min(start + 5_552, data.count)
    for index in start..<end {
      a += UInt64(data[index])
      b += a
    }
    a %= adlerModulus
    b %= adlerModulus
    start = end
  }
  return UInt32((b << 16) | a)
}

private func paeth(_ a: Int, _ b: Int, _ c: Int) -> Int {
  let prediction = a + b - c
  let distanceA = abs(prediction - a)
  let distanceB = abs(prediction - b)
  let distanceC = abs(prediction - c)
  if distanceA <= distanceB, distanceA <= distanceC { return a }
  if distanceB <= distanceC { return b }
  return c
}

private func applyFilter(
  _ filter: Int,
  raw: [UInt8],
  prior: [UInt8],
  bytesPerPixel: Int,
  output: inout [UInt8]
) {
  for index in raw.indices {
    let left = index >= bytesPerPixel ? Int(raw[index - bytesPerPixel]) : 0
    let above = Int(prior[index])
    let aboveLeft = index >= bytesPerPixel ? Int(prior[index - bytesPerPixel]) : 0
    let predicted: Int
    switch filter {
    case 1: predicted = left
    case 2: predicted = above
    case 3: predicted = (left + above) / 2
    case 4: predicted = paeth(left, above, aboveLeft)
    default: predicted = 0
    }
    output[index] = UInt8(truncatingIfNeeded: Int(raw[index]) - predicted)
  }
}

private func chooseFilter(
  raw: [UInt8],
  prior: [UInt8],
  bytesPerPixel: Int,
  scratch: inout [UInt8],
  best: inout [UInt8]
) -> UInt8 {
  var bestFilter = 0
  var bestScore = Int.max
  for filter in 0...4 {
    applyFilter(
      filter,
      raw: raw,
      prior: prior,
      bytesPerPixel: bytesPerPixel,
      output: &scratch
    )
    var score = 0
    for value in scratch {
      let unsigned = Int(value)
      score += unsigned < 128 ? unsigned : 256 - unsigned
    }
    if score < bestScore {
      bestScore = score
      bestFilter = filter
      best = scratch
    }
  }
  return UInt8(bestFilter)
}

private func undoFilter(
  _ filter: UInt8,
  row: inout [UInt8],
  prior: [UInt8],
  bytesPerPixel: Int
) throws {
  switch filter {
  case 0:
    return
  case 1:
    for index in bytesPerPixel..<row.count {
      row[index] = UInt8(truncatingIfNeeded: Int(row[index]) + Int(row[index - bytesPerPixel]))
    }
  case 2:
    for index in row.indices {
      row[index] = UInt8(truncatingIfNeeded: Int(row[index]) + Int(prior[index]))
    }
  case 3:
    for index in row.indices {
      let left = index >= bytesPerPixel ? Int(row[index - bytesPerPixel]) : 0
      let predicted = (left + Int(prior[index])) / 2
      row[index] = UInt8(truncatingIfNeeded: Int(row[index]) + predicted)
    }
  case 4:
    for index in row.indices {
      let left = index >= bytesPerPixel ? Int(row[index - bytesPerPixel]) : 0
      let aboveLeft = index >= bytesPerPixel ? Int(prior[index - bytesPerPixel]) : 0
      let predicted = paeth(left, Int(prior[index]), aboveLeft)
      row[index] = UInt8(truncatingIfNeeded: Int(row[index]) + predicted)
    }
  default:
    try fail("invalid-filter")
  }
}

private func bigEndian32(_ value: UInt32) -> [UInt8] {
  [
    UInt8((value >> 24) & 0xFF),
    UInt8((value >> 16) & 0xFF),
    UInt8((value >> 8) & 0xFF),
    UInt8(value & 0xFF),
  ]
}

private func readBigEndian32(_ data: [UInt8], at offset: Int) -> UInt32 {
  (UInt32(data[offset]) << 24)
    | (UInt32(data[offset + 1]) << 16)
    | (UInt32(data[offset + 2]) << 8)
    | UInt32(data[offset + 3])
}

private func readBigEndian16(_ data: [UInt8], at offset: Int) -> UInt16 {
  (UInt16(data[offset]) << 8) | UInt16(data[offset + 1])
}

private func addChunk(_ output: inout [UInt8], type: String, data: [UInt8]) {
  let typeBytes = Array(type.utf8)
  var checksum = crc32(typeBytes)
  checksum = crc32(data, initial: checksum)
  output += bigEndian32(UInt32(data.count))
  output += typeBytes
  output += data
  output += bigEndian32(checksum)
}

/// Validate encoder shape arithmetic without allocating a `PixelContainer`.
@discardableResult
func validateEncodeShape(width: UInt32, height: UInt32, dataCount: Int) throws
  -> (width: Int, height: Int, pixelCount: Int)
{
  guard width > 0, height > 0,
    width <= UInt32(pngMaxDimension), height <= UInt32(pngMaxDimension)
  else {
    try fail("invalid-image-dimensions")
  }
  let integerWidth = Int(width)
  let integerHeight = Int(height)
  let pixelCount = integerWidth * integerHeight
  guard pixelCount <= pngDefaultMaxPixels else {
    try fail("invalid-image-dimensions")
  }
  guard dataCount == pixelCount * 4 else {
    try fail("invalid-pixel-data-length")
  }
  return (integerWidth, integerHeight, pixelCount)
}

/// Encode RGBA8 pixels as deterministic colour-type-6 portable PNG.
public func encodePng(_ pixels: PixelContainer) throws -> [UInt8] {
  let shape = try validateEncodeShape(
    width: pixels.width,
    height: pixels.height,
    dataCount: pixels.data.count
  )
  let width = shape.width
  let height = shape.height

  var output = signature
  var ihdr = bigEndian32(pixels.width) + bigEndian32(pixels.height)
  ihdr += [8, 6, 0, 0, 0]
  addChunk(&output, type: "IHDR", data: ihdr)

  let stride = width * 4
  var filtered = [UInt8](repeating: 0, count: height * (stride + 1))
  var prior = [UInt8](repeating: 0, count: stride)
  var scratch = [UInt8](repeating: 0, count: stride)
  var best = [UInt8](repeating: 0, count: stride)
  for rowIndex in 0..<height {
    let raw = Array(pixels.data[(rowIndex * stride)..<((rowIndex + 1) * stride)])
    let destination = rowIndex * (stride + 1)
    filtered[destination] = chooseFilter(
      raw: raw,
      prior: prior,
      bytesPerPixel: 4,
      scratch: &scratch,
      best: &best
    )
    filtered.replaceSubrange((destination + 1)..<(destination + 1 + stride), with: best)
    prior = raw
  }

  let deflated = rawDeflate(filtered)
  let idat = [UInt8](arrayLiteral: 0x78, 0x9C) + deflated + bigEndian32(adler32(filtered))
  addChunk(&output, type: "IDAT", data: idat)
  addChunk(&output, type: "IEND", data: [])
  return output
}

private func validChunkType(_ type: [UInt8]) -> Bool {
  guard type.count == 4, type[2] & 0x20 == 0 else { return false }
  return type.allSatisfy { byte in
    (0x41...0x5A).contains(byte) || (0x61...0x7A).contains(byte)
  }
}

/// Decode the bounded, non-interlaced, 8-bit IC18 portable PNG profile.
public func decodePng(_ data: [UInt8], maxPixels: Double? = nil) throws -> PixelContainer {
  try decodePngWithLimit(data, maximumPixels: validatedMaxPixels(maxPixels))
}

private func decodePngWithLimit(_ data: [UInt8], maximumPixels: Int) throws -> PixelContainer {
  guard data.count >= signature.count else { try fail("file-too-short") }
  guard Array(data[..<signature.count]) == signature else { try fail("invalid-signature") }

  var width = 0
  var height = 0
  var colourType: UInt8 = 0
  var sawIHDR = false
  var sawIEND = false
  var sawPLTE = false
  var sawTRNS = false
  var inIDAT = false
  var idatEnded = false
  var transparentGrey: UInt8?
  var transparentRGB: [UInt8]?
  var idatParts: [[UInt8]] = []

  var position = signature.count
  while position < data.count {
    let remaining = data.count - position
    guard remaining >= 8 else { try fail("truncated-chunk") }
    let length32 = readBigEndian32(data, at: position)
    guard remaining >= 12, UInt64(length32) <= UInt64(remaining - 12) else {
      try fail("truncated-chunk")
    }
    let length = Int(length32)
    let typeStart = position + 4
    let dataStart = position + 8
    let dataEnd = dataStart + length
    let typeBytes = Array(data[typeStart..<dataStart])
    guard validChunkType(typeBytes) else { try fail("invalid-chunk-type") }
    let chunkData = Array(data[dataStart..<dataEnd])
    let declaredCRC = readBigEndian32(data, at: dataEnd)
    var actualCRC = crc32(typeBytes)
    actualCRC = crc32(chunkData, initial: actualCRC)
    guard actualCRC == declaredCRC else { try fail("chunk-crc-mismatch") }
    let type = String(decoding: typeBytes, as: UTF8.self)
    guard sawIHDR || type == "IHDR" else { try fail("chunk-before-ihdr") }

    switch type {
    case "IHDR":
      guard !sawIHDR else { try fail("duplicate-ihdr") }
      guard length == 13 else { try fail("invalid-ihdr-length") }
      let width32 = readBigEndian32(chunkData, at: 0)
      let height32 = readBigEndian32(chunkData, at: 4)
      guard width32 > 0, height32 > 0 else { try fail("invalid-dimensions") }
      guard width32 <= UInt32(pngMaxDimension), height32 <= UInt32(pngMaxDimension) else {
        try fail("dimension-limit")
      }
      width = Int(width32)
      height = Int(height32)
      guard width * height <= maximumPixels else { try fail("pixel-limit") }
      let bitDepth = chunkData[8]
      colourType = chunkData[9]
      guard chunkData[10] == 0, chunkData[11] == 0, chunkData[12] == 0 else {
        try fail("unsupported-feature")
      }
      guard bitDepth == 8, colourType != 3, [0, 2, 4, 6].contains(colourType) else {
        try fail("unsupported-feature")
      }
      sawIHDR = true

    case "PLTE":
      guard !sawPLTE, idatParts.isEmpty, !sawTRNS,
        colourType == 2 || colourType == 6,
        length >= 3, length <= 768, length.isMultiple(of: 3)
      else {
        try fail("invalid-plte")
      }
      sawPLTE = true

    case "tRNS":
      guard !sawTRNS, idatParts.isEmpty else { try fail("invalid-trns") }
      if colourType == 0 {
        guard length == 2 else { try fail("invalid-trns") }
        let sample = readBigEndian16(chunkData, at: 0)
        guard sample <= 255 else { try fail("invalid-trns") }
        transparentGrey = UInt8(sample)
      } else if colourType == 2 {
        guard length == 6 else { try fail("invalid-trns") }
        var values: [UInt8] = []
        for index in 0..<3 {
          let sample = readBigEndian16(chunkData, at: index * 2)
          guard sample <= 255 else { try fail("invalid-trns") }
          values.append(UInt8(sample))
        }
        transparentRGB = values
      } else {
        try fail("invalid-trns")
      }
      sawTRNS = true

    case "IDAT":
      guard !idatEnded else { try fail("nonconsecutive-idat") }
      idatParts.append(chunkData)
      inIDAT = true

    case "IEND":
      guard length == 0 else { try fail("invalid-iend") }
      guard dataEnd + 4 == data.count else { try fail("trailing-data") }
      sawIEND = true
      position = dataEnd + 4
      continue

    case "acTL", "fcTL", "fdAT":
      try fail("unsupported-feature")

    default:
      guard typeBytes[0] & 0x20 != 0 else { try fail("unknown-critical-chunk") }
    }

    if type != "IDAT", inIDAT {
      inIDAT = false
      idatEnded = true
    }
    position = dataEnd + 4
  }

  guard sawIHDR, sawIEND, !idatParts.isEmpty else { try fail("missing-required-chunk") }
  var zlibLength = 0
  for part in idatParts {
    guard part.count <= data.count - zlibLength else { try fail("truncated-chunk") }
    zlibLength += part.count
  }
  var zlibData: [UInt8] = []
  zlibData.reserveCapacity(zlibLength)
  for part in idatParts { zlibData += part }
  guard zlibData.count >= 6 else { try fail("invalid-zlib-header") }
  let cmf = zlibData[0]
  let flg = zlibData[1]
  let header = Int(cmf) << 8 | Int(flg)
  guard cmf & 0x0F == 8, cmf >> 4 <= 7, header.isMultiple(of: 31) else {
    try fail("invalid-zlib-header")
  }
  guard flg & 0x20 == 0 else { try fail("preset-dictionary") }

  let channels: Int
  switch colourType {
  case 0: channels = 1
  case 2: channels = 3
  case 4: channels = 2
  default: channels = 4
  }
  let stride = width * channels
  let expected = height * (stride + 1)
  let deflateData = Array(zlibData[2..<(zlibData.count - 4)])
  let inflated: RawInflateResult
  do {
    inflated = try rawInflateCounted(deflateData, maxOutput: expected)
  } catch let error as RawInflateError {
    if error.code == "output-limit-exceeded" {
      try fail("inflated-length-mismatch")
    }
    try fail("inflate-failed")
  } catch {
    throw error
  }
  guard inflated.output.count == expected else { try fail("inflated-length-mismatch") }
  guard inflated.bytesConsumed == deflateData.count else { try fail("idat-cavity") }
  let declaredAdler = readBigEndian32(zlibData, at: zlibData.count - 4)
  guard adler32(inflated.output) == declaredAdler else { try fail("adler-mismatch") }

  let rowSize = stride + 1
  for rowIndex in 0..<height {
    guard inflated.output[rowIndex * rowSize] <= 4 else { try fail("invalid-filter") }
  }

  var container = PixelContainer(width: UInt32(width), height: UInt32(height))
  var prior = [UInt8](repeating: 0, count: stride)
  for rowIndex in 0..<height {
    let sourceOffset = rowIndex * rowSize
    var row = Array(inflated.output[(sourceOffset + 1)..<(sourceOffset + rowSize)])
    try undoFilter(
      inflated.output[sourceOffset],
      row: &row,
      prior: prior,
      bytesPerPixel: channels
    )
    let destinationRow = rowIndex * width * 4
    for column in 0..<width {
      let source = column * channels
      let destination = destinationRow + column * 4
      switch channels {
      case 1:
        let value = row[source]
        container.data[destination] = value
        container.data[destination + 1] = value
        container.data[destination + 2] = value
        container.data[destination + 3] = transparentGrey == value ? 0 : 255
      case 2:
        let value = row[source]
        container.data[destination] = value
        container.data[destination + 1] = value
        container.data[destination + 2] = value
        container.data[destination + 3] = row[source + 1]
      case 3:
        let red = row[source]
        let green = row[source + 1]
        let blue = row[source + 2]
        let transparent = transparentRGB == [red, green, blue]
        container.data[destination] = red
        container.data[destination + 1] = green
        container.data[destination + 2] = blue
        container.data[destination + 3] = transparent ? 0 : 255
      default:
        container.data.replaceSubrange(
          destination..<(destination + 4), with: row[source..<(source + 4)])
      }
    }
    prior = row
  }
  return container
}

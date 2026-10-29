// The public namespace is deliberately uninhabited: RESP owns no sockets,
// files, clocks, callbacks, or mutable global state. It only transforms owned
// byte values under explicit limits.

public enum RESP {
  /// Encode one typed value after a complete size and syntax preflight.
  public static func encode(
    _ value: RespValue, limits: RespLimits = .default
  ) throws -> [UInt8] {
    let encodedSize = try measure(value, limits: limits)
    var output: [UInt8] = []
    output.reserveCapacity(encodedSize)
    var pending: [(RespValue, Int)] = [(value, 1)]

    while let (current, depth) = pending.popLast() {
      switch current {
      case .simpleString(let text):
        output.append(43)
        output.append(contentsOf: text.utf8)
        output.append(contentsOf: [13, 10])
      case .error(let text):
        output.append(45)
        output.append(contentsOf: text.utf8)
        output.append(contentsOf: [13, 10])
      case .integer(let number):
        output.append(58)
        output.append(contentsOf: String(number).utf8)
        output.append(contentsOf: [13, 10])
      case .bulkString(let bytes):
        output.append(36)
        output.append(contentsOf: String(bytes.count).utf8)
        output.append(contentsOf: [13, 10])
        output.append(contentsOf: bytes)
        output.append(contentsOf: [13, 10])
      case .nullBulkString:
        output.append(contentsOf: [36, 45, 49, 13, 10])
      case .array(let values):
        output.append(42)
        output.append(contentsOf: String(values.count).utf8)
        output.append(contentsOf: [13, 10])
        for child in values.reversed() {
          pending.append((child, depth + 1))
        }
      case .nullArray:
        output.append(contentsOf: [42, 45, 49, 13, 10])
      }
    }
    return output
  }

  /// Attempt one failure-atomic decode from the start of an owned byte value.
  public static func decode(
    _ bytes: [UInt8], limits: RespLimits = .default
  ) throws -> RespDecodeResult {
    try requireBuffered(bytes.count, limits: limits)
    return try parse(bytes, at: 0, limits: limits)
  }

  /// Decode every complete frame prefix, retaining an incomplete final tail.
  /// A malformed frame throws without publishing a partial batch.
  public static func decodeAll(
    _ bytes: [UInt8], limits: RespLimits = .default
  ) throws -> RespDecodeBatch {
    try requireBuffered(bytes.count, limits: limits)
    var values: [RespValue] = []
    var cursor = 0
    while cursor < bytes.count {
      switch try parse(bytes, at: cursor, limits: limits) {
      case .incomplete:
        return RespDecodeBatch(values: values, consumed: cursor)
      case .complete(let value, let consumed):
        values.append(value)
        cursor += consumed
      }
    }
    return RespDecodeBatch(values: values, consumed: cursor)
  }

  private struct ArrayFrame {
    var remaining: Int
    var values: [RespValue]
  }

  private static func parse(
    _ bytes: [UInt8], at start: Int, limits: RespLimits
  ) throws -> RespDecodeResult {
    var cursor = start
    var frames: [ArrayFrame] = []
    var valueCount = 0

    while true {
      guard cursor < bytes.count else { return .incomplete }
      let depth = frames.count + 1
      guard depth <= limits.maximumNestingDepth else {
        throw RespProtocolError.nestingDepthLimitExceeded(
          maximum: limits.maximumNestingDepth)
      }
      valueCount += 1
      guard valueCount <= limits.maximumValues else {
        throw RespProtocolError.valueLimitExceeded(maximum: limits.maximumValues)
      }

      let prefix = bytes[cursor]
      cursor += 1
      try requireFrame(cursor - start, limits: limits)
      var completed: RespValue

      switch prefix {
      case 43, 45:  // '+' simple string, '-' error
        guard let line = try readLine(bytes, from: cursor, start: start, limits: limits)
        else { return .incomplete }
        cursor = line.next
        let text = String(decoding: line.bytes, as: UTF8.self)
        guard Array(text.utf8) == line.bytes else {
          throw RespProtocolError.invalidUTF8
        }
        completed = prefix == 43 ? .simpleString(text) : .error(text)

      case 58:  // ':' signed integer
        guard let line = try readLine(bytes, from: cursor, start: start, limits: limits)
        else { return .incomplete }
        try requireHeader(line.bytes.count, limits: limits)
        cursor = line.next
        completed = .integer(try parseInteger(line.bytes))

      case 36:  // '$' bulk string
        guard let line = try readLine(bytes, from: cursor, start: start, limits: limits)
        else { return .incomplete }
        try requireHeader(line.bytes.count, limits: limits)
        cursor = line.next
        let length = try parseLength(line.bytes)
        if length == -1 {
          completed = .nullBulkString
          break
        }
        guard length >= 0 else {
          throw RespProtocolError.invalidNegativeBulkLength
        }
        guard length <= Int64(limits.maximumBulkBytes) else {
          throw RespProtocolError.bulkByteLimitExceeded(maximum: limits.maximumBulkBytes)
        }
        let payloadLength = Int(length)
        let (payloadEnd, overflow1) = cursor.addingReportingOverflow(payloadLength)
        let (requiredEnd, overflow2) = payloadEnd.addingReportingOverflow(2)
        guard !overflow1 && !overflow2 else {
          throw RespProtocolError.frameByteLimitExceeded(maximum: limits.maximumFrameBytes)
        }
        try requireFrame(requiredEnd - start, limits: limits)
        guard requiredEnd <= bytes.count else { return .incomplete }
        guard bytes[payloadEnd] == 13 && bytes[payloadEnd + 1] == 10 else {
          throw RespProtocolError.missingBulkTerminator
        }
        completed = .bulkString(Array(bytes[cursor..<payloadEnd]))
        cursor = requiredEnd

      case 42:  // '*' array
        guard let line = try readLine(bytes, from: cursor, start: start, limits: limits)
        else { return .incomplete }
        try requireHeader(line.bytes.count, limits: limits)
        cursor = line.next
        let count = try parseLength(line.bytes)
        if count == -1 {
          completed = .nullArray
          break
        }
        guard count >= 0 else {
          throw RespProtocolError.invalidNegativeArrayLength
        }
        guard count <= Int64(limits.maximumArrayElements) else {
          throw RespProtocolError.arrayElementLimitExceeded(
            maximum: limits.maximumArrayElements)
        }
        let elementCount = Int(count)
        guard elementCount <= limits.maximumValues - valueCount else {
          throw RespProtocolError.valueLimitExceeded(maximum: limits.maximumValues)
        }
        if elementCount == 0 {
          completed = .array([])
          break
        }
        var values: [RespValue] = []
        values.reserveCapacity(elementCount)
        frames.append(ArrayFrame(remaining: elementCount, values: values))
        continue

      default:
        throw RespProtocolError.unknownType(prefix)
      }

      while true {
        guard var frame = frames.popLast() else {
          try requireFrame(cursor - start, limits: limits)
          return .complete(value: completed, consumed: cursor - start)
        }
        frame.values.append(completed)
        frame.remaining -= 1
        if frame.remaining > 0 {
          frames.append(frame)
          break
        }
        completed = .array(frame.values)
      }
    }
  }

  private static func measure(
    _ value: RespValue, limits: RespLimits
  ) throws -> Int {
    var pending: [(RespValue, Int)] = [(value, 1)]
    var count = 0
    var total = 0
    while let (current, depth) = pending.popLast() {
      guard depth <= limits.maximumNestingDepth else {
        throw RespProtocolError.nestingDepthLimitExceeded(
          maximum: limits.maximumNestingDepth)
      }
      count += 1
      guard count <= limits.maximumValues else {
        throw RespProtocolError.valueLimitExceeded(maximum: limits.maximumValues)
      }
      switch current {
      case .simpleString(let text), .error(let text):
        let bytes = Array(text.utf8)
        guard !bytes.contains(13) && !bytes.contains(10) else {
          throw RespProtocolError.invalidLineTerminator
        }
        guard bytes.count <= limits.maximumLineBytes else {
          throw RespProtocolError.lineByteLimitExceeded(maximum: limits.maximumLineBytes)
        }
        try addSize(1 + bytes.count + 2, to: &total, limits: limits)
      case .integer(let number):
        try addSize(1 + String(number).utf8.count + 2, to: &total, limits: limits)
      case .bulkString(let bytes):
        guard bytes.count <= limits.maximumBulkBytes else {
          throw RespProtocolError.bulkByteLimitExceeded(maximum: limits.maximumBulkBytes)
        }
        try addSize(
          1 + String(bytes.count).utf8.count + 2 + bytes.count + 2,
          to: &total,
          limits: limits)
      case .nullBulkString, .nullArray:
        try addSize(5, to: &total, limits: limits)
      case .array(let values):
        guard values.count <= limits.maximumArrayElements else {
          throw RespProtocolError.arrayElementLimitExceeded(
            maximum: limits.maximumArrayElements)
        }
        try addSize(1 + String(values.count).utf8.count + 2, to: &total, limits: limits)
        for child in values.reversed() {
          pending.append((child, depth + 1))
        }
      }
    }
    return total
  }

  private static func addSize(
    _ amount: Int, to total: inout Int, limits: RespLimits
  ) throws {
    let (next, overflow) = total.addingReportingOverflow(amount)
    guard !overflow && next <= limits.maximumOutputBytes else {
      throw RespProtocolError.outputByteLimitExceeded(maximum: limits.maximumOutputBytes)
    }
    total = next
  }

  private static func readLine(
    _ bytes: [UInt8], from start: Int, start frameStart: Int, limits: RespLimits
  ) throws -> (bytes: [UInt8], next: Int)? {
    var cursor = start
    while cursor < bytes.count {
      let length = cursor - start
      guard length <= limits.maximumLineBytes else {
        throw RespProtocolError.lineByteLimitExceeded(maximum: limits.maximumLineBytes)
      }
      try requireFrame(cursor - frameStart + 1, limits: limits)
      if bytes[cursor] == 10 {
        throw RespProtocolError.invalidLineTerminator
      }
      if bytes[cursor] == 13 {
        guard cursor + 1 < bytes.count else { return nil }
        guard bytes[cursor + 1] == 10 else {
          throw RespProtocolError.invalidLineTerminator
        }
        try requireFrame(cursor + 2 - frameStart, limits: limits)
        return (Array(bytes[start..<cursor]), cursor + 2)
      }
      cursor += 1
    }
    if cursor - start > limits.maximumLineBytes {
      throw RespProtocolError.lineByteLimitExceeded(maximum: limits.maximumLineBytes)
    }
    return nil
  }

  private static func parseInteger(_ bytes: [UInt8]) throws -> Int64 {
    guard validNumber(bytes, allowNegativeOneOnly: false) else {
      throw RespProtocolError.invalidInteger
    }
    guard let value = Int64(String(decoding: bytes, as: UTF8.self)) else {
      throw RespProtocolError.integerOutOfRange
    }
    return value
  }

  private static func parseLength(_ bytes: [UInt8]) throws -> Int64 {
    guard validNumber(bytes, allowNegativeOneOnly: false) else {
      throw RespProtocolError.invalidLength
    }
    guard let value = Int64(String(decoding: bytes, as: UTF8.self)) else {
      throw RespProtocolError.invalidLength
    }
    return value
  }

  private static func validNumber(
    _ bytes: [UInt8], allowNegativeOneOnly: Bool
  ) -> Bool {
    guard !bytes.isEmpty else { return false }
    var start = 0
    if bytes[0] == 45 {
      guard bytes.count > 1 else { return false }
      if allowNegativeOneOnly { return bytes == [45, 49] }
      start = 1
    }
    let digits = bytes[start...]
    guard digits.allSatisfy({ (48...57).contains($0) }) else { return false }
    if digits.count > 1 && digits.first == 48 { return false }
    if start == 1 && digits.count == 1 && digits.first == 48 { return false }
    return true
  }

  private static func requireHeader(_ count: Int, limits: RespLimits) throws {
    guard count <= limits.maximumHeaderBytes else {
      throw RespProtocolError.headerByteLimitExceeded(maximum: limits.maximumHeaderBytes)
    }
  }

  private static func requireFrame(_ count: Int, limits: RespLimits) throws {
    guard count <= limits.maximumFrameBytes else {
      throw RespProtocolError.frameByteLimitExceeded(maximum: limits.maximumFrameBytes)
    }
  }

  private static func requireBuffered(_ count: Int, limits: RespLimits) throws {
    guard count <= limits.maximumBufferedBytes else {
      throw RespProtocolError.bufferedByteLimitExceeded(maximum: limits.maximumBufferedBytes)
    }
  }
}

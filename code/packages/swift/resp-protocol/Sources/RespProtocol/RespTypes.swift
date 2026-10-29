// RESP is a tagged byte protocol.  Keeping every wire kind distinct prevents
// the two null forms from collapsing into Swift's single Optional.none value.

/// A fully owned RESP2 value.
///
/// Arrays and byte buffers have Swift copy-on-write value semantics. Strings
/// are compared by their exact UTF-8 bytes rather than Swift's canonically
/// equivalent Unicode equality, matching the wire identity.
public indirect enum RespValue: Sendable, Equatable {
  case simpleString(String)
  case error(String)
  case integer(Int64)
  case bulkString([UInt8])
  case nullBulkString
  case array([RespValue])
  case nullArray

  public static func == (left: RespValue, right: RespValue) -> Bool {
    var pending: [(RespValue, RespValue)] = [(left, right)]
    while let (lhs, rhs) = pending.popLast() {
      switch (lhs, rhs) {
      case (.simpleString(let a), .simpleString(let b)),
        (.error(let a), .error(let b)):
        guard Array(a.utf8) == Array(b.utf8) else { return false }
      case (.integer(let a), .integer(let b)):
        guard a == b else { return false }
      case (.bulkString(let a), .bulkString(let b)):
        guard a == b else { return false }
      case (.nullBulkString, .nullBulkString), (.nullArray, .nullArray):
        continue
      case (.array(let a), .array(let b)):
        guard a.count == b.count else { return false }
        for index in a.indices.reversed() {
          pending.append((a[index], b[index]))
        }
      default:
        return false
      }
    }
    return true
  }
}

/// Which configurable ceiling was invalid.
public enum RespLimitKind: String, Sendable, Equatable {
  case lineBytes
  case headerBytes
  case bulkBytes
  case arrayElements
  case nestingDepth
  case values
  case frameBytes
  case outputBytes
  case bufferedBytes
  case fragmentedWork
}

/// Stable failures returned by encoding and decoding.
public enum RespProtocolError: Error, Sendable, Equatable {
  case invalidLimit(RespLimitKind)
  case lineByteLimitExceeded(maximum: Int)
  case headerByteLimitExceeded(maximum: Int)
  case bulkByteLimitExceeded(maximum: Int)
  case arrayElementLimitExceeded(maximum: Int)
  case nestingDepthLimitExceeded(maximum: Int)
  case valueLimitExceeded(maximum: Int)
  case frameByteLimitExceeded(maximum: Int)
  case outputByteLimitExceeded(maximum: Int)
  case bufferedByteLimitExceeded(maximum: Int)
  case fragmentedWorkLimitExceeded(maximum: Int64)
  case invalidLineTerminator
  case invalidUTF8
  case invalidInteger
  case integerOutOfRange
  case invalidLength
  case invalidNegativeBulkLength
  case invalidNegativeArrayLength
  case missingBulkTerminator
  case unknownType(UInt8)
  case incompleteFrame
}

/// Tighten-only resource ceilings for untrusted wire data.
public struct RespLimits: Sendable, Equatable {
  public static let maximumLineByteCeiling = 1_024
  public static let maximumHeaderByteCeiling = 20
  public static let maximumBulkByteCeiling = 8_000
  public static let maximumArrayElementCeiling = 1_024
  public static let maximumNestingDepthCeiling = 64
  public static let maximumValueCeiling = 4_096
  public static let maximumFrameByteCeiling = 8_192
  public static let maximumOutputByteCeiling = 8_192
  public static let maximumBufferedByteCeiling = 8_192
  public static let maximumFragmentedWorkCeiling: Int64 = 34_000_000

  public static let `default` = RespLimits(unchecked: ())

  public let maximumLineBytes: Int
  public let maximumHeaderBytes: Int
  public let maximumBulkBytes: Int
  public let maximumArrayElements: Int
  public let maximumNestingDepth: Int
  public let maximumValues: Int
  public let maximumFrameBytes: Int
  public let maximumOutputBytes: Int
  public let maximumBufferedBytes: Int
  public let maximumFragmentedWork: Int64

  public init(
    maximumLineBytes: Int = RespLimits.maximumLineByteCeiling,
    maximumHeaderBytes: Int = RespLimits.maximumHeaderByteCeiling,
    maximumBulkBytes: Int = RespLimits.maximumBulkByteCeiling,
    maximumArrayElements: Int = RespLimits.maximumArrayElementCeiling,
    maximumNestingDepth: Int = RespLimits.maximumNestingDepthCeiling,
    maximumValues: Int = RespLimits.maximumValueCeiling,
    maximumFrameBytes: Int = RespLimits.maximumFrameByteCeiling,
    maximumOutputBytes: Int = RespLimits.maximumOutputByteCeiling,
    maximumBufferedBytes: Int = RespLimits.maximumBufferedByteCeiling,
    maximumFragmentedWork: Int64 = RespLimits.maximumFragmentedWorkCeiling
  ) throws {
    try Self.require(
      maximumLineBytes, in: 1...Self.maximumLineByteCeiling, .lineBytes)
    try Self.require(
      maximumHeaderBytes, in: 1...Self.maximumHeaderByteCeiling, .headerBytes)
    try Self.require(
      maximumBulkBytes, in: 0...Self.maximumBulkByteCeiling, .bulkBytes)
    try Self.require(
      maximumArrayElements, in: 0...Self.maximumArrayElementCeiling, .arrayElements)
    try Self.require(
      maximumNestingDepth, in: 1...Self.maximumNestingDepthCeiling, .nestingDepth)
    try Self.require(maximumValues, in: 1...Self.maximumValueCeiling, .values)
    try Self.require(
      maximumFrameBytes, in: 1...Self.maximumFrameByteCeiling, .frameBytes)
    try Self.require(
      maximumOutputBytes, in: 1...Self.maximumOutputByteCeiling, .outputBytes)
    try Self.require(
      maximumBufferedBytes, in: 1...Self.maximumBufferedByteCeiling, .bufferedBytes)
    guard (1...Self.maximumFragmentedWorkCeiling).contains(maximumFragmentedWork) else {
      throw RespProtocolError.invalidLimit(.fragmentedWork)
    }
    guard maximumHeaderBytes <= maximumLineBytes else {
      throw RespProtocolError.invalidLimit(.headerBytes)
    }
    guard maximumBulkBytes <= maximumFrameBytes else {
      throw RespProtocolError.invalidLimit(.bulkBytes)
    }
    guard maximumOutputBytes <= maximumFrameBytes else {
      throw RespProtocolError.invalidLimit(.outputBytes)
    }
    guard maximumFrameBytes <= maximumBufferedBytes else {
      throw RespProtocolError.invalidLimit(.frameBytes)
    }

    self.maximumLineBytes = maximumLineBytes
    self.maximumHeaderBytes = maximumHeaderBytes
    self.maximumBulkBytes = maximumBulkBytes
    self.maximumArrayElements = maximumArrayElements
    self.maximumNestingDepth = maximumNestingDepth
    self.maximumValues = maximumValues
    self.maximumFrameBytes = maximumFrameBytes
    self.maximumOutputBytes = maximumOutputBytes
    self.maximumBufferedBytes = maximumBufferedBytes
    self.maximumFragmentedWork = maximumFragmentedWork
  }

  private init(unchecked: Void) {
    maximumLineBytes = Self.maximumLineByteCeiling
    maximumHeaderBytes = Self.maximumHeaderByteCeiling
    maximumBulkBytes = Self.maximumBulkByteCeiling
    maximumArrayElements = Self.maximumArrayElementCeiling
    maximumNestingDepth = Self.maximumNestingDepthCeiling
    maximumValues = Self.maximumValueCeiling
    maximumFrameBytes = Self.maximumFrameByteCeiling
    maximumOutputBytes = Self.maximumOutputByteCeiling
    maximumBufferedBytes = Self.maximumBufferedByteCeiling
    maximumFragmentedWork = Self.maximumFragmentedWorkCeiling
  }

  private static func require(
    _ value: Int, in range: ClosedRange<Int>, _ kind: RespLimitKind
  ) throws {
    guard range.contains(value) else { throw RespProtocolError.invalidLimit(kind) }
  }
}

/// A complete value or a failure-atomic request for more bytes.
public enum RespDecodeResult: Sendable, Equatable {
  case complete(value: RespValue, consumed: Int)
  case incomplete
}

/// Complete values decoded from a prefix, plus the exact prefix length.
public struct RespDecodeBatch: Sendable, Equatable {
  public let values: [RespValue]
  public let consumed: Int

  public init(values: [RespValue], consumed: Int) {
    self.values = values
    self.consumed = consumed
  }
}

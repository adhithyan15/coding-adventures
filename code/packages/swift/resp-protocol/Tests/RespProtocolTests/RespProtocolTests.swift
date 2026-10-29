import Foundation
import Testing

@testable import RespProtocol

private struct FixtureDocument: Decodable {
  let version: Int
  let cases: [FixtureCase]
}

private struct FixtureCase: Decodable {
  let id: String
  let wireHex: String
  let outcome: String
  let kind: String?
  let error: String?

  enum CodingKeys: String, CodingKey {
    case id
    case wireHex = "wire_hex"
    case outcome
    case kind
    case error
  }
}

private let fixtureIDs = [
  "simple-ok", "error-value", "integer-zero", "integer-min", "bulk-empty",
  "bulk-binary", "bulk-null", "array-empty", "array-null", "array-two-integers",
  "invalid-utf8-line", "invalid-integer-leading-zero", "invalid-integer-negative-zero",
  "invalid-bulk-negative", "invalid-array-negative", "invalid-bulk-terminator",
  "invalid-line-ending", "invalid-type", "incomplete-bulk",
]

private func loadFixtures() throws -> FixtureDocument {
  let url = try #require(Bundle.module.url(forResource: "dt23-cases", withExtension: "json"))
  return try JSONDecoder().decode(FixtureDocument.self, from: Data(contentsOf: url))
}

private func bytes(hex: String) throws -> [UInt8] {
  guard hex.count.isMultiple(of: 2) else { throw FixtureFailure.invalidHex }
  var result: [UInt8] = []
  result.reserveCapacity(hex.count / 2)
  var cursor = hex.startIndex
  while cursor < hex.endIndex {
    let end = hex.index(cursor, offsetBy: 2)
    guard let value = UInt8(hex[cursor..<end], radix: 16) else {
      throw FixtureFailure.invalidHex
    }
    result.append(value)
    cursor = end
  }
  return result
}

private enum FixtureFailure: Error { case invalidHex }

private func kind(of value: RespValue) -> String {
  switch value {
  case .simpleString: "simple_string"
  case .error: "error"
  case .integer: "integer"
  case .bulkString: "bulk_string"
  case .nullBulkString: "null_bulk"
  case .array: "array"
  case .nullArray: "null_array"
  }
}

private func errorCode(_ error: RespProtocolError) -> String {
  switch error {
  case .invalidUTF8: "invalid_utf8"
  case .invalidInteger: "invalid_integer"
  case .invalidNegativeBulkLength: "negative_bulk"
  case .invalidNegativeArrayLength: "negative_array"
  case .missingBulkTerminator: "bulk_terminator"
  case .invalidLineTerminator: "line_terminator"
  case .unknownType: "unknown_type"
  default: "other"
  }
}

@Test func exactDT23FixtureRosterRunsThroughProductionCodec() throws {
  let document = try loadFixtures()
  #expect(document.version == 1)
  #expect(document.cases.map(\.id) == fixtureIDs)

  for fixture in document.cases {
    let wire = try bytes(hex: fixture.wireHex)
    switch fixture.outcome {
    case "value":
      let decoded: RespDecodeResult
      do {
        decoded = try RESP.decode(wire)
      } catch {
        Issue.record("\(fixture.id): unexpected decode error \(error)")
        continue
      }
      guard case .complete(let value, let consumed) = decoded else {
        Issue.record("\(fixture.id): expected a complete value")
        continue
      }
      #expect(consumed == wire.count, "\(fixture.id)")
      #expect(kind(of: value) == fixture.kind, "\(fixture.id)")
      #expect(try RESP.encode(value) == wire, "\(fixture.id)")
    case "incomplete":
      #expect(try RESP.decode(wire) == .incomplete, "\(fixture.id)")
    case "error":
      do {
        _ = try RESP.decode(wire)
        Issue.record("\(fixture.id): expected an error")
      } catch let error as RespProtocolError {
        #expect(errorCode(error) == fixture.error, "\(fixture.id)")
      }
    default:
      Issue.record("\(fixture.id): unknown fixture outcome")
    }
  }
}

@Test func typedValuesPreserveWireIdentityAndNullKinds() {
  #expect(RespValue.nullBulkString != .nullArray)
  #expect(RespValue.bulkString([]) != .nullBulkString)
  #expect(RespValue.array([]) != .nullArray)
  #expect(RespValue.simpleString("é") != .simpleString("e\u{301}"))
  #expect(RespValue.error("ERR x") != .simpleString("ERR x"))

  var source: [UInt8] = [0, 255]
  let value = RespValue.bulkString(source)
  source[0] = 1
  #expect(value == .bulkString([0, 255]))
}

@Test func encoderHandlesEveryKindAndRejectsLineInjection() throws {
  let nested = RespValue.array([
    .simpleString("OK"), .error("ERR boom"), .integer(-1),
    .bulkString([0, 13, 10, 255]), .nullBulkString, .array([]), .nullArray,
  ])
  let wire = try RESP.encode(nested)
  #expect(try RESP.decode(wire) == .complete(value: nested, consumed: wire.count))
  #expect(throws: RespProtocolError.invalidLineTerminator) {
    try RESP.encode(.simpleString("bad\rline"))
  }
  #expect(throws: RespProtocolError.invalidLineTerminator) {
    try RESP.encode(.error("bad\nline"))
  }
}

@Test func everyFragmentBoundaryIsFailureAtomic() throws {
  let value = RespValue.array([
    .bulkString(Array("SET".utf8)), .bulkString([0, 255]), .array([.integer(7)]),
  ])
  let wire = try RESP.encode(value)
  for split in 0..<wire.count {
    #expect(try RESP.decode(Array(wire[..<split])) == .incomplete, "split \(split)")
  }
  #expect(try RESP.decode(wire) == .complete(value: value, consumed: wire.count))
}

@Test func decodeAllUsesCorrectOffsetsAndRetainsPartialTail() throws {
  let first = Array("*2\r\n:1\r\n:2\r\n".utf8)
  #expect(first.count == 12)
  let second = Array("+OK\r\n".utf8)
  let tail = Array("$3\r\nf".utf8)
  let batch = try RESP.decodeAll(first + second + tail)
  #expect(batch.values == [.array([.integer(1), .integer(2)]), .simpleString("OK")])
  #expect(batch.consumed == 17)
}

@Test func streamDecoderHandlesOneByteFragmentsAndOwnsItsTail() throws {
  let expected: [RespValue] = [
    .array([.bulkString(Array("PING".utf8))]), .integer(42), .error("ERR bad"),
  ]
  let wire = try expected.flatMap { try RESP.encode($0) }
  var decoder = RespDecoder()
  var actual: [RespValue] = []
  for byte in wire {
    actual.append(contentsOf: try decoder.feed([byte]))
  }
  #expect(actual == expected)
  #expect(decoder.pendingBytes.isEmpty)
  try decoder.finish()

  var partial: [UInt8] = [36, 51, 13, 10, 102]
  _ = try decoder.feed(partial)
  partial[4] = 120
  #expect(decoder.pendingBytes == [36, 51, 13, 10, 102])
  #expect(throws: RespProtocolError.incompleteFrame) { try decoder.finish() }
  decoder.reset()
  #expect(decoder.pendingByteCount == 0)
}

@Test func streamDecoderDoesNotCommitMalformedCandidate() throws {
  var decoder = RespDecoder()
  _ = try decoder.feed(Array("$3\r\nf".utf8))
  let before = decoder.pendingBytes
  #expect(throws: RespProtocolError.missingBulkTerminator) {
    try decoder.feed(Array("ooXX".utf8))
  }
  #expect(decoder.pendingBytes == before)
}

@Test func binaryBulkRoundTripsAllByteValues() throws {
  let payload = (0...255).map(UInt8.init)
  let wire = try RESP.encode(.bulkString(payload))
  #expect(try RESP.decode(wire) == .complete(value: .bulkString(payload), consumed: wire.count))
}

@Test func limitsRejectBeforeOversizedAllocationOrOutput() throws {
  let limits = try RespLimits(
    maximumLineBytes: 8,
    maximumHeaderBytes: 4,
    maximumBulkBytes: 4,
    maximumArrayElements: 2,
    maximumNestingDepth: 2,
    maximumValues: 3,
    maximumFrameBytes: 16,
    maximumOutputBytes: 16,
    maximumBufferedBytes: 16,
    maximumFragmentedWork: 64)

  #expect(throws: RespProtocolError.bulkByteLimitExceeded(maximum: 4)) {
    try RESP.decode(Array("$5\r\n".utf8), limits: limits)
  }
  #expect(throws: RespProtocolError.arrayElementLimitExceeded(maximum: 2)) {
    try RESP.decode(Array("*3\r\n".utf8), limits: limits)
  }
  #expect(throws: RespProtocolError.nestingDepthLimitExceeded(maximum: 2)) {
    try RESP.decode(Array("*1\r\n*1\r\n*0\r\n".utf8), limits: limits)
  }
  #expect(throws: RespProtocolError.bulkByteLimitExceeded(maximum: 4)) {
    try RESP.encode(.bulkString([0, 1, 2, 3, 4]), limits: limits)
  }
  #expect(throws: RespProtocolError.outputByteLimitExceeded(maximum: 16)) {
    try RESP.encode(.array([.bulkString([1, 2, 3, 4]), .bulkString([5, 6])]), limits: limits)
  }
  #expect(throws: RespProtocolError.lineByteLimitExceeded(maximum: 8)) {
    try RESP.encode(.simpleString(String(repeating: "é", count: 5)), limits: limits)
  }

  let headerFrame = try RespLimits(
    maximumLineBytes: 2, maximumHeaderBytes: 2, maximumBulkBytes: 0,
    maximumArrayElements: 1, maximumFrameBytes: 3, maximumOutputBytes: 3,
    maximumBufferedBytes: 8)
  #expect(throws: RespProtocolError.frameByteLimitExceeded(maximum: 3)) {
    try RESP.decode(Array("*1\r\n".utf8), limits: headerFrame)
  }
}

@Test func numericHeadersEnforceTheirBoundBeforeTerminationOrAllocation() throws {
  let limits = try RespLimits(maximumLineBytes: 8, maximumHeaderBytes: 2)

  for prefix: UInt8 in [36, 42, 58] {  // '$', '*', ':'
    #expect(throws: RespProtocolError.headerByteLimitExceeded(maximum: 2)) {
      try RESP.decode([prefix] + Array("123\r\n".utf8), limits: limits)
    }
    #expect(throws: RespProtocolError.headerByteLimitExceeded(maximum: 2)) {
      try RESP.decode([prefix] + Array("123".utf8), limits: limits)
    }
  }

  let competingFrameLimit = try RespLimits(
    maximumLineBytes: 2, maximumHeaderBytes: 2, maximumBulkBytes: 0,
    maximumFrameBytes: 3, maximumOutputBytes: 3, maximumBufferedBytes: 8)
  #expect(throws: RespProtocolError.headerByteLimitExceeded(maximum: 2)) {
    try RESP.decode(Array("$123".utf8), limits: competingFrameLimit)
  }
}

@Test func encoderEnforcesNumericHeaderBoundsBeforeOutputAllocation() throws {
  let limits = try RespLimits(maximumHeaderBytes: 1)

  let oversizedHeaders: [RespValue] = [
    .integer(42),
    .bulkString(Array(repeating: 0, count: 10)),
    .nullBulkString,
    .array(Array(repeating: .integer(0), count: 10)),
    .nullArray,
  ]
  for value in oversizedHeaders {
    #expect(throws: RespProtocolError.headerByteLimitExceeded(maximum: 1)) {
      try RESP.encode(value, limits: limits)
    }
  }

  #expect(try RESP.encode(.integer(1), limits: limits) == Array(":1\r\n".utf8))
  #expect(try RESP.encode(.bulkString([]), limits: limits) == Array("$0\r\n\r\n".utf8))
  #expect(try RESP.encode(.array([]), limits: limits) == Array("*0\r\n".utf8))
}

@Test func limitConfigurationIsTightenOnlyAndCoherent() {
  #expect(throws: RespProtocolError.invalidLimit(.lineBytes)) {
    try RespLimits(maximumLineBytes: 0)
  }
  #expect(throws: RespProtocolError.invalidLimit(.headerBytes)) {
    try RespLimits(maximumLineBytes: 2, maximumHeaderBytes: 3)
  }
  #expect(throws: RespProtocolError.invalidLimit(.bulkBytes)) {
    try RespLimits(maximumBulkBytes: 10, maximumFrameBytes: 9)
  }
  #expect(throws: RespProtocolError.invalidLimit(.frameBytes)) {
    try RespLimits(
      maximumBulkBytes: 4, maximumFrameBytes: 8, maximumOutputBytes: 8,
      maximumBufferedBytes: 7)
  }
  #expect(throws: RespProtocolError.invalidLimit(.fragmentedWork)) {
    try RespLimits(maximumFragmentedWork: 0)
  }
}

@Test func parserRejectsIntegerOverflowAndResourceEdges() throws {
  #expect(throws: RespProtocolError.integerOutOfRange) {
    try RESP.decode(Array(":9223372036854775808\r\n".utf8))
  }
  #expect(throws: RespProtocolError.invalidLength) {
    try RESP.decode(Array("$+1\r\na\r\n".utf8))
  }

  let shortLine = try RespLimits(maximumLineBytes: 2, maximumHeaderBytes: 2)
  #expect(throws: RespProtocolError.lineByteLimitExceeded(maximum: 2)) {
    try RESP.decode(Array("+abc".utf8), limits: shortLine)
  }

  let tinyBuffer = try RespLimits(
    maximumBulkBytes: 1, maximumFrameBytes: 8, maximumOutputBytes: 8,
    maximumBufferedBytes: 8, maximumFragmentedWork: 8)
  var decoder = RespDecoder(limits: tinyBuffer)
  _ = try decoder.feed([36])
  _ = try decoder.feed([49])
  _ = try decoder.feed([13])
  #expect(throws: RespProtocolError.fragmentedWorkLimitExceeded(maximum: 8)) {
    try decoder.feed([10])
  }
}

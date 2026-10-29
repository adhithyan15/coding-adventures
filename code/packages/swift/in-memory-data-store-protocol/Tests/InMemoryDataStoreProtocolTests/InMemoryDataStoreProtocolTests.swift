import Testing

@testable import InMemoryDataStoreProtocol

private func limits(
  commandBytes: Int = ProtocolValidationLimits.default.maximumCommandBytes,
  argumentCount: Int = ProtocolValidationLimits.default.maximumArgumentCount,
  argumentBytes: Int = ProtocolValidationLimits.default.maximumArgumentBytes,
  frameBytes: Int = ProtocolValidationLimits.default.maximumFrameBytes,
  responseDepth: Int = ProtocolValidationLimits.default.maximumResponseDepth,
  responseElements: Int = ProtocolValidationLimits.default.maximumResponseElements,
  responsePayloadBytes: Int = ProtocolValidationLimits.default.maximumResponsePayloadBytes
) throws -> ProtocolValidationLimits {
  try ProtocolValidationLimits(
    maximumCommandBytes: commandBytes,
    maximumArgumentCount: argumentCount,
    maximumArgumentBytes: argumentBytes,
    maximumFrameBytes: frameBytes,
    maximumResponseDepth: responseDepth,
    maximumResponseElements: responseElements,
    maximumResponsePayloadBytes: responsePayloadBytes)
}

@Test func commandNormalizesVisibleASCIIButPreservesBinaryArguments() throws {
  var parts: [[UInt8]] = [Array("sEt-2!".utf8), [0x00, 0xFF, 0x80], []]
  let frame = try CommandFrame(parts: parts)
  parts[1][0] = 0x41

  #expect(frame.command == "SET-2!")
  #expect(frame.arguments == [[0x00, 0xFF, 0x80], []])
  #expect(frame.parts == [Array("SET-2!".utf8), [0x00, 0xFF, 0x80], []])

  var exported = frame.parts
  exported[1][0] = 0x42
  #expect(frame.arguments[0][0] == 0x00)
  #expect(try CommandFrame(command: "get", arguments: [[0x6B]]).command == "GET")
}

@Test func commandRejectsEmptyAndNonVisibleBytesPrecisely() {
  #expect(throws: InMemoryDataStoreProtocolError.emptyCommandFrame) {
    try CommandFrame(parts: [])
  }
  #expect(throws: InMemoryDataStoreProtocolError.emptyCommand) {
    try CommandFrame(parts: [[]])
  }

  for invalid: UInt8 in [0x00, 0x09, 0x20, 0x7F, 0x80, 0xFF] {
    #expect(
      throws: InMemoryDataStoreProtocolError.invalidCommandByte(index: 1, byte: invalid)
    ) {
      try CommandFrame(parts: [[0x41, invalid]])
    }
  }
}

@Test func commandLimitsAcceptBoundariesAndRejectTheNextValue() throws {
  let commandLimits = try limits(
    commandBytes: 3, argumentCount: 2, argumentBytes: 2, frameBytes: 7)
  #expect(
    try CommandFrame(command: "abc", arguments: [[1, 2], [3, 4]], limits: commandLimits).parts.count
      == 3)
  #expect(throws: InMemoryDataStoreProtocolError.commandByteLimitExceeded(maximum: 3)) {
    try CommandFrame(command: "abcd", limits: commandLimits)
  }
  #expect(throws: InMemoryDataStoreProtocolError.commandByteLimitExceeded(maximum: 3)) {
    try CommandFrame(parts: [Array("abcd".utf8)], limits: commandLimits)
  }
  #expect(throws: InMemoryDataStoreProtocolError.argumentCountLimitExceeded(maximum: 2)) {
    try CommandFrame(command: "a", arguments: [[], [], []], limits: commandLimits)
  }
  #expect(throws: InMemoryDataStoreProtocolError.argumentCountLimitExceeded(maximum: 2)) {
    try CommandFrame(parts: [[0x61], [], [], []], limits: commandLimits)
  }
  #expect(
    throws: InMemoryDataStoreProtocolError.argumentByteLimitExceeded(index: 0, maximum: 2)
  ) {
    try CommandFrame(command: "a", arguments: [[1, 2, 3]], limits: commandLimits)
  }

  let frameLimits = try limits(
    commandBytes: 3, argumentCount: 2, argumentBytes: 3, frameBytes: 5)
  #expect(
    try CommandFrame(command: "abc", arguments: [[1, 2]], limits: frameLimits).parts.count == 2)
  #expect(throws: InMemoryDataStoreProtocolError.frameByteLimitExceeded(maximum: 5)) {
    try CommandFrame(command: "abc", arguments: [[1, 2, 3]], limits: frameLimits)
  }
}

@Test func customLimitsAreTightenOnlyAndInternallyCoherent() {
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.commandBytes)) {
    try limits(commandBytes: 0)
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.commandBytes)) {
    try limits(commandBytes: 257)
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.argumentCount)) {
    try limits(argumentCount: -1)
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.argumentCount)) {
    try limits(argumentCount: 1_025)
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.argumentBytes)) {
    try limits(argumentBytes: -1)
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.frameBytes)) {
    try limits(frameBytes: 0)
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.commandBytes)) {
    try limits(commandBytes: 2, argumentBytes: 0, frameBytes: 1)
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.argumentBytes)) {
    try limits(commandBytes: 1, argumentBytes: 2, frameBytes: 1)
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.responseDepth)) {
    try limits(responseDepth: 0)
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.responseElements)) {
    try limits(responseElements: 0)
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidLimit(.responsePayloadBytes)) {
    try limits(responsePayloadBytes: -1)
  }
}

@Test func responseCasesHelpersAndExactUTF8IdentityAreStable() {
  #expect(EngineResponse.ok == .simpleString("OK"))
  #expect(EngineResponse.null == .bulkString(nil))
  #expect(EngineResponse.zero == .integer(0))
  #expect(EngineResponse.one == .integer(1))
  #expect(EngineResponse.bulkString(nil) != .bulkString([]))
  #expect(EngineResponse.array(nil) != .array([]))
  #expect(EngineResponse.bulkString(nil) != .array(nil))
  #expect(EngineResponse.simpleString("\u{00E9}") != .simpleString("e\u{0301}"))
  #expect(EngineResponse.error("x") != .simpleString("x"))
  #expect(EngineResponse.integer(Int64.min) != .integer(Int64.max))
  #expect(EngineResponse.bulkString([0]) != .bulkString([1]))
  #expect(EngineResponse.array(nil) == .array(nil))
  #expect(EngineResponse.array([]) == .array([]))
  #expect(EngineResponse.array([.one, .null]) == .array([.one, .null]))
  #expect(EngineResponse.array([.one]) != .array([.one, .zero]))
  #expect(EngineResponse.array([.one, .zero]) != .array([.zero, .one]))
}

@Test func validationReturnsMetricsForMixedOrderedResponses() throws {
  let response = EngineResponse.array([
    .simpleString("é"),
    .integer(-1),
    .bulkString([0x00, 0xFF]),
    .array([.error("x")]),
  ])
  let metrics = try response.validate()
  #expect(metrics == EngineResponseMetrics(elementCount: 6, maximumDepth: 3, payloadByteCount: 13))

  guard case .array(.some(let values)) = response else {
    Issue.record("expected an array response")
    return
  }
  #expect(values[0] == .simpleString("é"))
  #expect(values[1] == .integer(-1))
}

@Test func validationRejectsLineBreaksAndPayloadOverruns() throws {
  #expect(throws: InMemoryDataStoreProtocolError.invalidResponseLineBreak) {
    try EngineResponse.simpleString("a\nb").validate()
  }
  #expect(throws: InMemoryDataStoreProtocolError.invalidResponseLineBreak) {
    try EngineResponse.error("a\rb").validate()
  }

  let twoBytes = try limits(responsePayloadBytes: 2)
  #expect(try EngineResponse.simpleString("é").validate(limits: twoBytes).payloadByteCount == 2)
  #expect(
    throws: InMemoryDataStoreProtocolError.responsePayloadByteLimitExceeded(maximum: 2)
  ) {
    try EngineResponse.simpleString("abc").validate(limits: twoBytes)
  }
  #expect(
    throws: InMemoryDataStoreProtocolError.responsePayloadByteLimitExceeded(maximum: 2)
  ) {
    try EngineResponse.bulkString([1, 2, 3]).validate(limits: twoBytes)
  }

  let integerBudget = try limits(responsePayloadBytes: 8)
  #expect(try EngineResponse.integer(1).validate(limits: integerBudget).payloadByteCount == 8)
  let shortIntegerBudget = try limits(responsePayloadBytes: 7)
  #expect(
    throws: InMemoryDataStoreProtocolError.responsePayloadByteLimitExceeded(maximum: 7)
  ) {
    try EngineResponse.integer(1).validate(limits: shortIntegerBudget)
  }
}

@Test func responseDepthBoundaryIsIterativeAndRootIsDepthOne() throws {
  var response = EngineResponse.integer(1)
  for _ in 1..<ProtocolValidationLimits.maximumResponseDepth {
    response = .array([response])
  }
  #expect(try response.validate().maximumDepth == 128)

  response = .array([response])
  #expect(
    throws: InMemoryDataStoreProtocolError.responseDepthLimitExceeded(maximum: 128)
  ) {
    try response.validate()
  }
}

@Test func responseElementBoundaryHandlesWideArraysWithoutWidthSizedTraversalState() throws {
  let children = Array(
    repeating: EngineResponse.bulkString(nil),
    count: ProtocolValidationLimits.maximumResponseElements - 1)
  #expect(try EngineResponse.array(children).validate().elementCount == 65_536)

  let smallLimit = try limits(responseElements: 3)
  #expect(try EngineResponse.array([.zero, .one]).validate(limits: smallLimit).elementCount == 3)
  #expect(
    throws: InMemoryDataStoreProtocolError.responseElementLimitExceeded(maximum: 3)
  ) {
    try EngineResponse.array([.zero, .one, .null]).validate(limits: smallLimit)
  }
}

@Test func nullAndEmptyValuesHaveNoPhantomPayloadOrChildren() throws {
  let zeroPayload = try limits(responsePayloadBytes: 0)
  for response: EngineResponse in [
    .simpleString(""), .error(""), .bulkString(nil), .bulkString([]), .array(nil), .array([]),
  ] {
    #expect(
      try response.validate(limits: zeroPayload)
        == EngineResponseMetrics(elementCount: 1, maximumDepth: 1, payloadByteCount: 0))
  }
}

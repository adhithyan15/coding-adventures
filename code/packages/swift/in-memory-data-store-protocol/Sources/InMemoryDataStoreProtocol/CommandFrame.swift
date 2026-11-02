/// The bounded field associated with a protocol-validation failure.
public enum ProtocolLimit: String, Equatable, Sendable {
  case commandBytes
  case argumentCount
  case argumentBytes
  case frameBytes
  case responseDepth
  case responseElements
  case responsePayloadBytes
}

/// Stable failures at the command/response intermediate-representation boundary.
public enum InMemoryDataStoreProtocolError: Error, Equatable, Sendable {
  case invalidLimit(ProtocolLimit)
  case emptyCommandFrame
  case emptyCommand
  case invalidCommandByte(index: Int, byte: UInt8)
  case commandByteLimitExceeded(maximum: Int)
  case argumentCountLimitExceeded(maximum: Int)
  case argumentByteLimitExceeded(index: Int, maximum: Int)
  case frameByteLimitExceeded(maximum: Int)
  case invalidResponseLineBreak
  case responseDepthLimitExceeded(maximum: Int)
  case responseElementLimitExceeded(maximum: Int)
  case responsePayloadByteLimitExceeded(maximum: Int)
}

/// Tighten-only budgets for command frames and engine responses.
public struct ProtocolValidationLimits: Equatable, Sendable {
  public static let maximumCommandBytes = 256
  public static let maximumArgumentCount = 1_024
  public static let maximumArgumentBytes = 16_777_216
  public static let maximumFrameBytes = 67_108_864
  public static let maximumResponseDepth = 128
  public static let maximumResponseElements = 65_536
  public static let maximumResponsePayloadBytes = 67_108_864

  public static let `default` = ProtocolValidationLimits(uncheckedDefaults: ())

  public let maximumCommandBytes: Int
  public let maximumArgumentCount: Int
  public let maximumArgumentBytes: Int
  public let maximumFrameBytes: Int
  public let maximumResponseDepth: Int
  public let maximumResponseElements: Int
  public let maximumResponsePayloadBytes: Int

  public init(
    maximumCommandBytes: Int = Self.maximumCommandBytes,
    maximumArgumentCount: Int = Self.maximumArgumentCount,
    maximumArgumentBytes: Int = Self.maximumArgumentBytes,
    maximumFrameBytes: Int = Self.maximumFrameBytes,
    maximumResponseDepth: Int = Self.maximumResponseDepth,
    maximumResponseElements: Int = Self.maximumResponseElements,
    maximumResponsePayloadBytes: Int = Self.maximumResponsePayloadBytes
  ) throws {
    try Self.require(
      maximumCommandBytes >= 1 && maximumCommandBytes <= Self.maximumCommandBytes,
      .commandBytes)
    try Self.require(
      maximumArgumentCount >= 0 && maximumArgumentCount <= Self.maximumArgumentCount,
      .argumentCount)
    try Self.require(
      maximumArgumentBytes >= 0 && maximumArgumentBytes <= Self.maximumArgumentBytes,
      .argumentBytes)
    try Self.require(
      maximumFrameBytes >= 1 && maximumFrameBytes <= Self.maximumFrameBytes,
      .frameBytes)
    try Self.require(maximumCommandBytes <= maximumFrameBytes, .commandBytes)
    try Self.require(maximumArgumentBytes <= maximumFrameBytes, .argumentBytes)
    try Self.require(
      maximumResponseDepth >= 1 && maximumResponseDepth <= Self.maximumResponseDepth,
      .responseDepth)
    try Self.require(
      maximumResponseElements >= 1
        && maximumResponseElements <= Self.maximumResponseElements,
      .responseElements)
    try Self.require(
      maximumResponsePayloadBytes >= 0
        && maximumResponsePayloadBytes <= Self.maximumResponsePayloadBytes,
      .responsePayloadBytes)

    self.maximumCommandBytes = maximumCommandBytes
    self.maximumArgumentCount = maximumArgumentCount
    self.maximumArgumentBytes = maximumArgumentBytes
    self.maximumFrameBytes = maximumFrameBytes
    self.maximumResponseDepth = maximumResponseDepth
    self.maximumResponseElements = maximumResponseElements
    self.maximumResponsePayloadBytes = maximumResponsePayloadBytes
  }

  private init(uncheckedDefaults: Void) {
    maximumCommandBytes = Self.maximumCommandBytes
    maximumArgumentCount = Self.maximumArgumentCount
    maximumArgumentBytes = Self.maximumArgumentBytes
    maximumFrameBytes = Self.maximumFrameBytes
    maximumResponseDepth = Self.maximumResponseDepth
    maximumResponseElements = Self.maximumResponseElements
    maximumResponsePayloadBytes = Self.maximumResponsePayloadBytes
  }

  private static func require(_ condition: Bool, _ limit: ProtocolLimit) throws {
    guard condition else {
      throw InMemoryDataStoreProtocolError.invalidLimit(limit)
    }
  }
}

/// A normalized command plus its ordered, binary-safe arguments.
public struct CommandFrame: Equatable, Sendable {
  public let command: String
  public let arguments: [[UInt8]]

  public init(
    parts: [[UInt8]],
    limits: ProtocolValidationLimits = .default
  ) throws {
    guard let commandBytes = parts.first else {
      throw InMemoryDataStoreProtocolError.emptyCommandFrame
    }
    guard parts.count - 1 <= limits.maximumArgumentCount else {
      throw InMemoryDataStoreProtocolError.argumentCountLimitExceeded(
        maximum: limits.maximumArgumentCount)
    }
    try self.init(
      commandBytes: commandBytes,
      arguments: Array(parts.dropFirst()),
      limits: limits)
  }

  public init(
    command: String,
    arguments: [[UInt8]] = [],
    limits: ProtocolValidationLimits = .default
  ) throws {
    var commandBytes: [UInt8] = []
    commandBytes.reserveCapacity(limits.maximumCommandBytes)
    for byte in command.utf8 {
      guard commandBytes.count < limits.maximumCommandBytes else {
        throw InMemoryDataStoreProtocolError.commandByteLimitExceeded(
          maximum: limits.maximumCommandBytes)
      }
      commandBytes.append(byte)
    }
    try self.init(commandBytes: commandBytes, arguments: arguments, limits: limits)
  }

  /// Recreate command-first value collections without exposing internal storage.
  public var parts: [[UInt8]] {
    [Array(command.utf8)] + arguments
  }

  private init(
    commandBytes: [UInt8],
    arguments: [[UInt8]],
    limits: ProtocolValidationLimits
  ) throws {
    guard !commandBytes.isEmpty else {
      throw InMemoryDataStoreProtocolError.emptyCommand
    }
    guard commandBytes.count <= limits.maximumCommandBytes else {
      throw InMemoryDataStoreProtocolError.commandByteLimitExceeded(
        maximum: limits.maximumCommandBytes)
    }
    guard arguments.count <= limits.maximumArgumentCount else {
      throw InMemoryDataStoreProtocolError.argumentCountLimitExceeded(
        maximum: limits.maximumArgumentCount)
    }

    var frameBytes = commandBytes.count
    for (index, argument) in arguments.enumerated() {
      guard argument.count <= limits.maximumArgumentBytes else {
        throw InMemoryDataStoreProtocolError.argumentByteLimitExceeded(
          index: index,
          maximum: limits.maximumArgumentBytes)
      }
      guard argument.count <= limits.maximumFrameBytes - frameBytes else {
        throw InMemoryDataStoreProtocolError.frameByteLimitExceeded(
          maximum: limits.maximumFrameBytes)
      }
      frameBytes += argument.count
    }

    var normalized = commandBytes
    for (index, byte) in normalized.enumerated() {
      guard (0x21...0x7E).contains(byte) else {
        throw InMemoryDataStoreProtocolError.invalidCommandByte(index: index, byte: byte)
      }
      if (0x61...0x7A).contains(byte) {
        normalized[index] = byte - 0x20
      }
    }

    command = String(decoding: normalized, as: UTF8.self)
    self.arguments = arguments
  }
}

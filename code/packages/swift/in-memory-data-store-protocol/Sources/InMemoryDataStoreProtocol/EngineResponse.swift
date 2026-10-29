/// Measurements observed while validating one complete engine response.
public struct EngineResponseMetrics: Equatable, Sendable {
  public let elementCount: Int
  public let maximumDepth: Int
  public let payloadByteCount: Int
}

/// The closed response algebra shared by an engine and its wire facade.
public indirect enum EngineResponse: Sendable {
  case simpleString(String)
  case error(String)
  case integer(Int64)
  case bulkString([UInt8]?)
  case array([EngineResponse]?)

  public static let ok: Self = .simpleString("OK")
  public static let null: Self = .bulkString(nil)
  public static let zero: Self = .integer(0)
  public static let one: Self = .integer(1)

  /// Validate the complete response with O(nesting depth) auxiliary storage.
  @discardableResult
  public func validate(
    limits: ProtocolValidationLimits = .default
  ) throws -> EngineResponseMetrics {
    typealias Cursor = (children: [EngineResponse], nextIndex: Int, depth: Int)

    var cursors: [Cursor] = []
    var current: (response: EngineResponse, depth: Int)? = (self, 1)
    var elementCount = 0
    var maximumDepth = 0
    var payloadByteCount = 0

    while let node = current {
      guard node.depth <= limits.maximumResponseDepth else {
        throw InMemoryDataStoreProtocolError.responseDepthLimitExceeded(
          maximum: limits.maximumResponseDepth)
      }
      guard elementCount < limits.maximumResponseElements else {
        throw InMemoryDataStoreProtocolError.responseElementLimitExceeded(
          maximum: limits.maximumResponseElements)
      }
      elementCount += 1
      maximumDepth = max(maximumDepth, node.depth)

      switch node.response {
      case .simpleString(let value), .error(let value):
        payloadByteCount = try countStringPayload(
          value,
          accumulated: payloadByteCount,
          maximum: limits.maximumResponsePayloadBytes)
      case .integer:
        payloadByteCount = try addPayload(
          8,
          to: payloadByteCount,
          maximum: limits.maximumResponsePayloadBytes)
      case .bulkString(.some(let value)):
        payloadByteCount = try addPayload(
          value.count,
          to: payloadByteCount,
          maximum: limits.maximumResponsePayloadBytes)
      case .array(.some(let children)) where !children.isEmpty:
        let childDepth = node.depth + 1
        cursors.append((children: children, nextIndex: 1, depth: childDepth))
        current = (children[0], childDepth)
        continue
      case .bulkString(nil), .array(nil), .array(.some):
        break
      }

      current = nil
      while var cursor = cursors.popLast() {
        guard cursor.nextIndex < cursor.children.count else {
          continue
        }
        let next = cursor.children[cursor.nextIndex]
        cursor.nextIndex += 1
        cursors.append(cursor)
        current = (next, cursor.depth)
        break
      }
    }

    return EngineResponseMetrics(
      elementCount: elementCount,
      maximumDepth: maximumDepth,
      payloadByteCount: payloadByteCount)
  }
}

extension EngineResponse: Equatable {
  public static func == (left: Self, right: Self) -> Bool {
    typealias Cursor = (
      left: [EngineResponse], right: [EngineResponse], nextIndex: Int
    )

    var cursors: [Cursor] = []
    var current: (left: EngineResponse, right: EngineResponse)? = (left, right)

    while let pair = current {
      switch (pair.left, pair.right) {
      case (.simpleString(let left), .simpleString(let right)),
        (.error(let left), .error(let right)):
        guard left.utf8.elementsEqual(right.utf8) else { return false }
      case (.integer(let left), .integer(let right)):
        guard left == right else { return false }
      case (.bulkString(let left), .bulkString(let right)):
        guard left == right else { return false }
      case (.array(nil), .array(nil)):
        break
      case (.array(.some(let left)), .array(.some(let right))):
        guard left.count == right.count else { return false }
        guard !left.isEmpty else { break }
        cursors.append((left: left, right: right, nextIndex: 1))
        current = (left[0], right[0])
        continue
      default:
        return false
      }

      current = nil
      while var cursor = cursors.popLast() {
        guard cursor.nextIndex < cursor.left.count else { continue }
        let index = cursor.nextIndex
        cursor.nextIndex += 1
        cursors.append(cursor)
        current = (cursor.left[index], cursor.right[index])
        break
      }
    }

    return true
  }
}

private func countStringPayload(
  _ value: String,
  accumulated: Int,
  maximum: Int
) throws -> Int {
  var result = accumulated
  for byte in value.utf8 {
    guard byte != 0x0A, byte != 0x0D else {
      throw InMemoryDataStoreProtocolError.invalidResponseLineBreak
    }
    guard result < maximum else {
      throw InMemoryDataStoreProtocolError.responsePayloadByteLimitExceeded(maximum: maximum)
    }
    result += 1
  }
  return result
}

private func addPayload(_ count: Int, to accumulated: Int, maximum: Int) throws -> Int {
  guard count <= maximum - accumulated else {
    throw InMemoryDataStoreProtocolError.responsePayloadByteLimitExceeded(maximum: maximum)
  }
  return accumulated + count
}

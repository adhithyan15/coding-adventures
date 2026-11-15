// ============================================================================
// Types.swift -- Canonical CBOR Value Model & Types
// ============================================================================

public struct CborError: Error, Equatable, Sendable, CustomStringConvertible {
  public let id: CborErrorId
  public let message: String

  public init(_ id: CborErrorId) {
    self.id = id
    self.message = Messages.message(for: id)
  }

  public var description: String {
    Messages.formatCborError(id: id, message: message)
  }
}

public struct Span: Equatable, Sendable, CustomStringConvertible {
  public let start: UInt64
  public let finish: UInt64

  public init?(start: UInt64, finish: UInt64) {
    guard start <= finish else { return nil }
    self.start = start
    self.finish = finish
  }

  // Internal initializer used when start <= finish is guaranteed by parsing
  init(uncheckedStart start: UInt64, finish: UInt64) {
    self.start = start
    self.finish = finish
  }

  public var length: UInt64 {
    finish - start
  }

  public var description: String {
    "[\(start), \(finish))"
  }
}

public struct CborMapEntry: Equatable, Sendable, CustomStringConvertible {
  public let key: CborValue
  public let value: CborValue

  public init(key: CborValue, value: CborValue) {
    self.key = key
    self.value = value
  }

  public var description: String {
    "\(key): \(value)"
  }
}

public struct SpannedMapEntry: Equatable, Sendable, CustomStringConvertible,
  CustomDebugStringConvertible, CustomReflectable
{
  public let key: CborValue
  public let value: CborValue
  public let valueSpan: Span

  public init(key: CborValue, value: CborValue, valueSpan: Span) {
    self.key = key
    self.value = value
    self.valueSpan = valueSpan
  }

  // Custom string representation strictly redacting key and value payloads (Standing Rule #20 / CVE-5)
  public var description: String {
    Messages.formatSpannedMapEntry(valueSpan: valueSpan)
  }

  public var debugDescription: String {
    Messages.formatSpannedMapEntry(valueSpan: valueSpan)
  }

  // Custom Mirror strictly redacting key and value child elements to prevent reflection-based data leakage
  public var customMirror: Mirror {
    let children: [Mirror.Child] = [
      (label: Messages.keyLabel, value: Messages.redactedPlaceholder),
      (label: Messages.valueLabel, value: Messages.redactedPlaceholder),
      (label: Messages.valueSpanLabel, value: valueSpan),
    ]
    return Mirror(
      self,
      children: children,
      displayStyle: .struct
    )
  }
}

public enum DecodeMapSpansResult: Equatable, Sendable {
  case map([SpannedMapEntry])
  case nonMap
}

public indirect enum CborValue: Equatable, Sendable, CustomStringConvertible {
  case unsigned(UInt64)
  case negative(UInt64)
  case bytes([UInt8])
  case text(String)
  case array([CborValue])
  case map([CborMapEntry])
  case tag(UInt64, CborValue)
  case boolean(Bool)
  case null

  public var description: String {
    switch self {
    case .unsigned(let u):
      return "\(u)"
    case .negative(let n):
      return "-1-\(n)"
    case .bytes(let b):
      return "bytes(\(b.count))"
    case .text(let s):
      return "\"\(s)\""
    case .array(let arr):
      let items = arr.map { $0.description }.joined(separator: ", ")
      return "[\(items)]"
    case .map(let entries):
      let items = entries.map { $0.description }.joined(separator: ", ")
      return "{\(items)}"
    case .tag(let tagVal, let inner):
      return "tag(\(tagVal), \(inner.description))"
    case .boolean(let b):
      return "\(b)"
    case .null:
      return "null"
    }
  }
}

// ============================================================================
// Messages.swift -- Canonical CBOR Error Messages & Placeholders (Standing Rule #20)
// ============================================================================

public enum CborErrorId: String, CaseIterable, Equatable, Sendable {
  case unexpectedEof = "unexpected-eof"
  case trailingBytes = "trailing-bytes"
  case reserved = "reserved"
  case indefinite = "indefinite"
  case nonMinimalInteger = "non-minimal-integer"
  case invalidUtf8 = "invalid-utf8"
  case nonCanonicalMapOrder = "non-canonical-map-order"
  case unsupportedSimple = "unsupported-simple"
  case floatNotSupported = "float-not-supported"
  case tooDeep = "too-deep"
  case lengthTooLarge = "length-too-large"
  case duplicateMapKey = "duplicate-map-key"
  case encodeTooDeep = "encode-too-deep"
  case encodeTooLarge = "encode-too-large"
  case unknown = "unknown"
}

public enum Messages {
  public static let redactedPlaceholder: String = "<redacted>"
  public static let invalidPlaceholder: String = "<invalid>"
  public static let unknownErrorIdMessage: String = "canonical-cbor: unknown error id"

  public static let keyLabel: String = "key"
  public static let valueLabel: String = "value"
  public static let valueSpanLabel: String = "value_span"

  public static func message(for id: CborErrorId) -> String {
    switch id {
    case .unexpectedEof:
      return "canonical-cbor: unexpected end of input"
    case .trailingBytes:
      return "canonical-cbor: trailing bytes after decoded item"
    case .reserved:
      return "canonical-cbor: reserved additional-info value"
    case .indefinite:
      return "canonical-cbor: indefinite item rejected"
    case .nonMinimalInteger:
      return "canonical-cbor: argument is not in smallest form"
    case .invalidUtf8:
      return "canonical-cbor: text is not valid UTF-8"
    case .nonCanonicalMapOrder:
      return "canonical-cbor: map key order is not canonical"
    case .unsupportedSimple:
      return "canonical-cbor: unsupported simple value"
    case .floatNotSupported:
      return "canonical-cbor: floats are not supported"
    case .tooDeep:
      return "canonical-cbor: nesting depth limit exceeded"
    case .lengthTooLarge:
      return "canonical-cbor: length exceeds remaining input"
    case .duplicateMapKey:
      return "canonical-cbor: duplicate map key"
    case .encodeTooDeep:
      return "canonical-cbor: nesting depth limit exceeded during encoding"
    case .encodeTooLarge:
      return "canonical-cbor: encoded output exceeds maximum size"
    case .unknown:
      return unknownErrorIdMessage
    }
  }

  public static func formatCborError(id: CborErrorId, message: String) -> String {
    "CborError(id: \(id.rawValue), message: \(message))"
  }

  public static func formatSpannedMapEntry(valueSpan: Span) -> String {
    "SpannedMapEntry(key: \(redactedPlaceholder), value: \(redactedPlaceholder), value_span: \(valueSpan))"
  }

  public static let allCborErrorIds: [CborErrorId] = [
    .unexpectedEof,
    .trailingBytes,
    .reserved,
    .indefinite,
    .nonMinimalInteger,
    .invalidUtf8,
    .nonCanonicalMapOrder,
    .unsupportedSimple,
    .floatNotSupported,
    .tooDeep,
    .lengthTooLarge,
    .duplicateMapKey,
    .encodeTooDeep,
    .encodeTooLarge,
  ]

  public static var allDiagnosticMessages: [String] {
    allCborErrorIds.map { message(for: $0) }
  }
}

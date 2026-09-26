import DerTlv

/// Bounded typed ASN.1 DER values over payload-blind DER framing.
public enum DerAsn1 {
  public typealias Asn1Limits = Limits
  public typealias Asn1ErrorKind = ErrorKind
  public typealias Asn1Error = ValueError
  public typealias Asn1Element = Element
  public typealias Asn1Decoder = Decoder
  public typealias Asn1Cursor = Cursor

  public static let defaultMaxDepth = 32
  public static let defaultMaxTotalElements: UInt64 = 16_384
  public static let defaultMaxOidArcs = 128

  public struct Limits: Sendable {
    public let der: DerTlv.Limits
    public let maxDepth: Int
    public let maxTotalElements: UInt64
    public let maxOidArcs: Int

    public init(
      der: DerTlv.Limits = .defaults,
      maxDepth: Int = DerAsn1.defaultMaxDepth,
      maxTotalElements: UInt64 = DerAsn1.defaultMaxTotalElements,
      maxOidArcs: Int = DerAsn1.defaultMaxOidArcs
    ) {
      precondition(maxDepth >= 0 && maxOidArcs >= 0, "ASN.1 limits must be non-negative")
      self.der = der
      self.maxDepth = maxDepth
      self.maxTotalElements = maxTotalElements
      self.maxOidArcs = maxOidArcs
    }

    fileprivate func matches(_ other: Limits) -> Bool {
      maxDepth == other.maxDepth && maxTotalElements == other.maxTotalElements
        && maxOidArcs == other.maxOidArcs
        && der.maxInputLength == other.der.maxInputLength
        && der.maxValueLength == other.der.maxValueLength
        && der.maxElements == other.der.maxElements
        && der.maxTagNumber == other.der.maxTagNumber
    }
  }

  public enum ErrorKind: String, CaseIterable, Sendable {
    case framing
    case unexpectedTag = "unexpected-tag"
    case decoderLimitMismatch = "decoder-limit-mismatch"
    case depthLimitExceeded = "depth-limit-exceeded"
    case elementLimitExceeded = "element-limit-exceeded"
    case invalidBooleanLength = "invalid-boolean-length"
    case invalidBooleanValue = "invalid-boolean-value"
    case emptyInteger = "empty-integer"
    case nonMinimalInteger = "non-minimal-integer"
    case negativeInteger = "negative-integer"
    case integerOverflow = "integer-overflow"
    case missingUnusedBitCount = "missing-unused-bit-count"
    case invalidUnusedBitCount = "invalid-unused-bit-count"
    case nonZeroBitPadding = "non-zero-bit-padding"
    case bitLengthOverflow = "bit-length-overflow"
    case nonEmptyNull = "non-empty-null"
    case nonAsciiIA5String = "non-ascii-ia5-string"
    case emptyObjectIdentifier = "empty-object-identifier"
    case unterminatedObjectIdentifier = "unterminated-object-identifier"
    case nonMinimalObjectIdentifier = "non-minimal-object-identifier"
    case objectIdentifierOverflow = "object-identifier-overflow"
    case oidArcLimitExceeded = "oid-arc-limit-exceeded"
  }

  public struct ValueError: Error, Equatable, CustomStringConvertible, Sendable {
    public let kind: ErrorKind
    public let offset: Int
    public let framingKind: String?

    public init(kind: ErrorKind, offset: Int, framingKind: String? = nil) {
      self.kind = kind
      self.offset = offset
      self.framingKind = framingKind
    }

    public var description: String {
      "ASN.1 DER value error \(kind.rawValue) at byte \(offset)"
    }
  }

  public struct Element: Sendable {
    public let tag: DerTlv.Tag
    public let depth: Int
    public let header: [UInt8]
    public let value: [UInt8]
    public let encoded: [UInt8]

    fileprivate init(_ element: DerTlv.Element, depth: Int) {
      tag = element.tag
      self.depth = depth
      header = Array(element.header)
      value = Array(element.value)
      encoded = Array(element.encoded)
    }

    fileprivate var valueOffset: Int { header.count }
  }

  public final class Decoder {
    public let limits: Limits
    public private(set) var elementsRead: UInt64 = 0

    public init(limits: Limits = Limits()) { self.limits = limits }

    public func decodeExact(_ input: [UInt8]) throws -> Element {
      guard limits.maxDepth > 0 else { throw failure(.depthLimitExceeded, 0) }
      try requireCapacity(offset: 0)
      do {
        let element = try DerTlv.decodeExact(input, limits: limits.der)
        elementsRead += 1
        return Element(element, depth: 0)
      } catch let error as DerTlv.FramingError {
        throw framing(error)
      }
    }

    public func sequence(_ element: Element) throws -> Cursor {
      try constructed(element, tagClass: "universal", number: 16)
    }

    public func set(_ element: Element) throws -> Cursor {
      try constructed(element, tagClass: "universal", number: 17)
    }

    public func explicit(_ element: Element, tagNumber: UInt64) throws -> Element {
      try expectTag(element, tagClass: "context-specific", constructed: true, number: tagNumber)
      let depth = try childDepth(element)
      try requireCapacity(offset: element.valueOffset)
      do {
        let child = try DerTlv.decodeExact(element.value, limits: limits.der)
        elementsRead += 1
        return Element(child, depth: depth)
      } catch let error as DerTlv.FramingError {
        throw framing(error)
      }
    }

    private func constructed(_ element: Element, tagClass: String, number: UInt64) throws -> Cursor
    {
      try expectTag(element, tagClass: tagClass, constructed: true, number: number)
      let depth = try childDepth(element)
      do {
        return Cursor(
          cursor: try DerTlv.Cursor(element.value, limits: limits.der),
          childDepth: depth,
          limits: limits,
          owner: self
        )
      } catch let error as DerTlv.FramingError {
        throw framing(error)
      }
    }

    private func childDepth(_ element: Element) throws -> Int {
      let depth = element.depth + 1
      guard depth < limits.maxDepth else { throw failure(.depthLimitExceeded, 0) }
      return depth
    }

    fileprivate func requireCapacity(offset: Int) throws {
      guard elementsRead < limits.maxTotalElements else {
        throw failure(.elementLimitExceeded, offset)
      }
    }

    fileprivate func countElement() { elementsRead += 1 }
  }

  public final class Cursor {
    private let cursor: DerTlv.Cursor
    private let childDepth: Int
    private let limits: Limits
    private let owner: Decoder

    fileprivate init(cursor: DerTlv.Cursor, childDepth: Int, limits: Limits, owner: Decoder) {
      self.cursor = cursor
      self.childDepth = childDepth
      self.limits = limits
      self.owner = owner
    }

    public var remaining: [UInt8] { Array(cursor.remaining) }

    public func read(_ decoder: Decoder) throws -> Element? {
      if cursor.remaining.isEmpty { return nil }
      guard decoder === owner && decoder.limits.matches(limits) else {
        throw failure(.decoderLimitMismatch, 0)
      }
      try decoder.requireCapacity(offset: 0)
      do {
        guard let element = try cursor.read() else { return nil }
        decoder.countElement()
        return Element(element, depth: childDepth)
      } catch let error as DerTlv.FramingError {
        throw framing(error)
      }
    }

    public func finish() throws {
      do { try cursor.finish() } catch let error as DerTlv.FramingError { throw framing(error) }
    }
  }

  public struct Integer: Sendable {
    public let signedBytes: [UInt8]
    private let valueOffset: Int
    public var isNegative: Bool { signedBytes[0] & 0x80 != 0 }

    fileprivate init(_ bytes: [UInt8], valueOffset: Int) {
      signedBytes = bytes
      self.valueOffset = valueOffset
    }

    public func toUInt64() throws -> UInt64 {
      guard !isNegative else { throw failure(.negativeInteger, valueOffset) }
      let start = signedBytes[0] == 0 ? 1 : 0
      guard signedBytes.count - start <= 8 else { throw failure(.integerOverflow, valueOffset) }
      var result: UInt64 = 0
      for byte in signedBytes[start...] { result = result * 256 + UInt64(byte) }
      return result
    }
  }

  public struct BitString: Sendable {
    public let bytes: [UInt8]
    public let unusedBits: Int
    public let bitLength: UInt64
  }

  public struct ObjectIdentifier: Sendable {
    public let encoded: [UInt8]
    public let arcs: [UInt64]
    public var arcCount: Int { arcs.count }
    public func equals(arcs expected: [UInt64]) -> Bool { arcs == expected }
  }

  public static func decodeBoolean(_ element: Element) throws -> Bool {
    try expectUniversalPrimitive(element, number: 1)
    guard element.value.count == 1 else {
      throw failure(.invalidBooleanLength, element.valueOffset)
    }
    switch element.value[0] {
    case 0: return false
    case 0xff: return true
    default: throw failure(.invalidBooleanValue, element.valueOffset)
    }
  }

  public static func decodeInteger(_ element: Element) throws -> Integer {
    try expectUniversalPrimitive(element, number: 2)
    guard !element.value.isEmpty else { throw failure(.emptyInteger, element.valueOffset) }
    if element.value.count > 1 {
      let first = element.value[0]
      let second = element.value[1]
      if (first == 0 && second & 0x80 == 0) || (first == 0xff && second & 0x80 != 0) {
        throw failure(.nonMinimalInteger, element.valueOffset)
      }
    }
    return Integer(element.value, valueOffset: element.valueOffset)
  }

  public static func decodeBitString(_ element: Element) throws -> BitString {
    try expectUniversalPrimitive(element, number: 3)
    guard let first = element.value.first else {
      throw failure(.missingUnusedBitCount, element.valueOffset)
    }
    let unused = Int(first)
    let payload = Array(element.value.dropFirst())
    guard unused <= 7 && (!payload.isEmpty || unused == 0) else {
      throw failure(.invalidUnusedBitCount, element.valueOffset)
    }
    if unused != 0, let last = payload.last, last & UInt8((1 << unused) - 1) != 0 {
      throw failure(.nonZeroBitPadding, element.valueOffset + element.value.count - 1)
    }
    let (bits, overflow) = UInt64(payload.count).multipliedReportingOverflow(by: 8)
    guard !overflow else { throw failure(.bitLengthOverflow, element.valueOffset) }
    return BitString(bytes: payload, unusedBits: unused, bitLength: bits - UInt64(unused))
  }

  public static func decodeOctetString(_ element: Element) throws -> [UInt8] {
    try expectUniversalPrimitive(element, number: 4)
    return element.value
  }

  public static func decodeImplicitOctetString(_ element: Element, tagNumber: UInt64) throws
    -> [UInt8]
  {
    try expectContextPrimitive(element, number: tagNumber)
    return element.value
  }

  public static func decodeIA5String(_ element: Element) throws -> String {
    try expectUniversalPrimitive(element, number: 22)
    return try decodeAscii(element)
  }

  public static func decodeImplicitIA5String(_ element: Element, tagNumber: UInt64) throws -> String
  {
    try expectContextPrimitive(element, number: tagNumber)
    return try decodeAscii(element)
  }

  public static func decodeNull(_ element: Element) throws {
    try expectUniversalPrimitive(element, number: 5)
    guard element.value.isEmpty else { throw failure(.nonEmptyNull, element.valueOffset) }
  }

  public static func decodeObjectIdentifier(
    _ element: Element, limits: Limits = Limits()
  ) throws -> ObjectIdentifier {
    try expectUniversalPrimitive(element, number: 6)
    return try decodeOid(element, limits: limits)
  }

  public static func decodeImplicitObjectIdentifier(
    _ element: Element, tagNumber: UInt64, limits: Limits = Limits()
  ) throws -> ObjectIdentifier {
    try expectContextPrimitive(element, number: tagNumber)
    return try decodeOid(element, limits: limits)
  }

  private static func decodeAscii(_ element: Element) throws -> String {
    for (index, byte) in element.value.enumerated() where byte > 0x7f {
      throw failure(.nonAsciiIA5String, element.valueOffset + index)
    }
    return String(decoding: element.value, as: UTF8.self)
  }

  private static func decodeOid(_ element: Element, limits: Limits) throws -> ObjectIdentifier {
    let encoded = element.value
    guard !encoded.isEmpty else { throw failure(.emptyObjectIdentifier, element.valueOffset) }
    let first = try parseArc(encoded, start: 0, valueOffset: element.valueOffset)
    var arcs: [UInt64]
    if first.value < 40 {
      arcs = [0, first.value]
    } else if first.value < 80 {
      arcs = [1, first.value - 40]
    } else {
      arcs = [2, first.value - 80]
    }
    guard arcs.count <= limits.maxOidArcs else {
      throw failure(.oidArcLimitExceeded, element.valueOffset)
    }
    var offset = first.nextOffset
    while offset < encoded.count {
      let start = offset
      let parsed = try parseArc(encoded, start: offset, valueOffset: element.valueOffset)
      arcs.append(parsed.value)
      guard arcs.count <= limits.maxOidArcs else {
        throw failure(.oidArcLimitExceeded, element.valueOffset + start)
      }
      offset = parsed.nextOffset
    }
    return ObjectIdentifier(encoded: encoded, arcs: arcs)
  }

  private static func parseArc(
    _ encoded: [UInt8], start: Int, valueOffset: Int
  ) throws -> (value: UInt64, nextOffset: Int) {
    guard encoded[start] != 0x80 else {
      throw failure(.nonMinimalObjectIdentifier, valueOffset + start)
    }
    var value: UInt64 = 0
    var offset = start
    while true {
      guard offset < encoded.count else {
        throw failure(.unterminatedObjectIdentifier, valueOffset + offset)
      }
      let octet = encoded[offset]
      let payload = UInt64(octet & 0x7f)
      guard value <= (UInt64.max - payload) / 128 else {
        throw failure(.objectIdentifierOverflow, valueOffset + offset)
      }
      value = value * 128 + payload
      offset += 1
      if octet & 0x80 == 0 { return (value, offset) }
    }
  }

  private static func expectUniversalPrimitive(_ element: Element, number: UInt64) throws {
    try expectTag(element, tagClass: "universal", constructed: false, number: number)
  }

  private static func expectContextPrimitive(_ element: Element, number: UInt64) throws {
    try expectTag(element, tagClass: "context-specific", constructed: false, number: number)
  }

  private static func expectTag(
    _ element: Element, tagClass: String, constructed: Bool, number: UInt64
  ) throws {
    guard
      element.tag.tagClass == tagClass && element.tag.constructed == constructed
        && element.tag.number == number
    else { throw failure(.unexpectedTag, 0) }
  }

  private static func failure(_ kind: ErrorKind, _ offset: Int) -> ValueError {
    ValueError(kind: kind, offset: offset)
  }

  private static func framing(_ error: DerTlv.FramingError) -> ValueError {
    ValueError(kind: .framing, offset: error.offset, framingKind: error.kind)
  }
}

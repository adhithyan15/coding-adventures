import DerAsn1

public enum X509Extension {
  public typealias Asn1Decoder = DerAsn1.Decoder
  public typealias Asn1Element = DerAsn1.Element

  public enum ErrorKind: String, CaseIterable, Sendable {
    case structure
    case missingExtensionId = "missing-extension-id"
    case invalidExtensionId = "invalid-extension-id"
    case invalidCritical = "invalid-critical"
    case encodedDefaultCritical = "encoded-default-critical"
    case missingExtensionValue = "missing-extension-value"
    case invalidExtensionValue = "invalid-extension-value"
    case trailingElement = "trailing-element"
  }

  public struct DecodeError: Error, Equatable, CustomStringConvertible, Sendable {
    public let kind: ErrorKind
    public let offset: Int
    public let asn1Kind: DerAsn1.ErrorKind?
    public let framingKind: String?

    fileprivate init(
      kind: ErrorKind,
      offset: Int,
      asn1Kind: DerAsn1.ErrorKind? = nil,
      framingKind: String? = nil
    ) {
      self.kind = kind
      self.offset = offset
      self.asn1Kind = asn1Kind
      self.framingKind = framingKind
    }

    public var description: String {
      "X.509 extension error \(kind.rawValue) at byte \(offset)"
    }
  }

  public struct Value: Sendable {
    public let extensionId: DerAsn1.ObjectIdentifier
    public let critical: Bool
    public let extensionValue: [UInt8]

    fileprivate init(
      extensionId: DerAsn1.ObjectIdentifier,
      critical: Bool,
      extensionValue: [UInt8]
    ) {
      self.extensionId = extensionId
      self.critical = critical
      self.extensionValue = extensionValue
    }
  }

  private static func structure(
    _ error: DerAsn1.ValueError,
    valueOffset: Int? = nil,
    childOffset: Int? = nil
  ) -> DecodeError {
    var offset = error.offset
    if let valueOffset, let childOffset {
      offset += error.kind == .framing ? valueOffset : childOffset
    }
    return DecodeError(
      kind: .structure,
      offset: offset,
      asn1Kind: error.kind,
      framingKind: error.framingKind
    )
  }

  private static func semantic(
    _ kind: ErrorKind,
    _ error: DerAsn1.ValueError,
    childOffset: Int
  ) -> DecodeError {
    DecodeError(
      kind: kind,
      offset: childOffset + error.offset,
      asn1Kind: error.kind,
      framingKind: error.framingKind
    )
  }

  private static func childOffset(
    valueOffset: Int,
    valueLength: Int,
    remainingLength: Int
  ) -> Int {
    valueOffset + valueLength - remainingLength
  }

  public static func decodeX509Extension(
    _ decoder: Asn1Decoder,
    _ element: Asn1Element
  ) throws -> Value {
    let valueOffset = element.header.count
    let valueLength = element.value.count
    let fields: DerAsn1.Cursor
    do { fields = try decoder.sequence(element) } catch let error as DerAsn1.ValueError {
      throw structure(error)
    }

    func read(_ offset: Int) throws -> DerAsn1.Element? {
      do { return try fields.read(decoder) } catch let error as DerAsn1.ValueError {
        throw structure(error, valueOffset: valueOffset, childOffset: offset)
      }
    }

    let idOffset = childOffset(
      valueOffset: valueOffset,
      valueLength: valueLength,
      remainingLength: fields.remaining.count
    )
    guard let idElement = try read(idOffset) else {
      throw DecodeError(kind: .missingExtensionId, offset: idOffset)
    }
    let extensionId: DerAsn1.ObjectIdentifier
    do {
      extensionId = try DerAsn1.decodeObjectIdentifier(idElement, limits: decoder.limits)
    } catch let error as DerAsn1.ValueError {
      throw semantic(.invalidExtensionId, error, childOffset: idOffset)
    }

    let secondOffset = childOffset(
      valueOffset: valueOffset,
      valueLength: valueLength,
      remainingLength: fields.remaining.count
    )
    guard let second = try read(secondOffset) else {
      throw DecodeError(kind: .missingExtensionValue, offset: secondOffset)
    }

    var critical = false
    var valueElement = second
    var valueElementOffset = secondOffset
    if second.tag.number == 1 {
      do { critical = try DerAsn1.decodeBoolean(second) } catch let error as DerAsn1.ValueError {
        throw semantic(.invalidCritical, error, childOffset: secondOffset)
      }
      guard critical else {
        throw DecodeError(kind: .encodedDefaultCritical, offset: secondOffset)
      }
      valueElementOffset = childOffset(
        valueOffset: valueOffset,
        valueLength: valueLength,
        remainingLength: fields.remaining.count
      )
      guard let third = try read(valueElementOffset) else {
        throw DecodeError(kind: .missingExtensionValue, offset: valueElementOffset)
      }
      valueElement = third
    }

    let extensionValue: [UInt8]
    do {
      extensionValue = try DerAsn1.decodeOctetString(valueElement)
    } catch let error as DerAsn1.ValueError {
      throw semantic(.invalidExtensionValue, error, childOffset: valueElementOffset)
    }

    let trailingOffset = childOffset(
      valueOffset: valueOffset,
      valueLength: valueLength,
      remainingLength: fields.remaining.count
    )
    if try read(trailingOffset) != nil {
      throw DecodeError(kind: .trailingElement, offset: trailingOffset)
    }
    return Value(
      extensionId: extensionId,
      critical: critical,
      extensionValue: extensionValue
    )
  }
}

/// Bounded, payload-blind DER tag-length-value framing.
public enum DerTlv {
  public struct Limits: Sendable {
    public let maxInputLength: UInt64
    public let maxValueLength: UInt64
    public let maxElements: UInt64
    public let maxTagNumber: UInt64

    public init(
      maxInputLength: UInt64 = 1_048_576,
      maxValueLength: UInt64 = 1_048_576,
      maxElements: UInt64 = 4_096,
      maxTagNumber: UInt64 = 0xffff_ffff
    ) throws {
      guard maxTagNumber <= 0xffff_ffff else {
        throw LimitError.invalidTagLimit
      }
      self.maxInputLength = maxInputLength
      self.maxValueLength = maxValueLength
      self.maxElements = maxElements
      self.maxTagNumber = maxTagNumber
    }

    public static var defaults: Limits { try! Limits() }
  }

  public enum LimitError: Error, Equatable {
    case invalidTagLimit
  }

  public struct FramingError: Error, Equatable, CustomStringConvertible {
    public let kind: String
    public let offset: Int

    public var description: String {
      "DER framing error \(kind) at byte \(offset)"
    }
  }

  public struct Tag: Equatable, Sendable {
    public let tagClass: String
    public let constructed: Bool
    public let number: UInt64
  }

  public struct Element: Sendable {
    public let tag: Tag
    private let input: [UInt8]
    private let start: Int
    private let headerLength: Int
    private let encodedLength: Int

    fileprivate init(tag: Tag, input: [UInt8], start: Int, headerLength: Int, encodedLength: Int) {
      self.tag = tag
      self.input = input
      self.start = start
      self.headerLength = headerLength
      self.encodedLength = encodedLength
    }

    public var header: ArraySlice<UInt8> {
      input[start..<start + headerLength]
    }

    public var value: ArraySlice<UInt8> {
      input[start + headerLength..<start + encodedLength]
    }

    public var encoded: ArraySlice<UInt8> {
      input[start..<start + encodedLength]
    }
  }

  public struct Decoded: Sendable {
    public let element: Element
    public let remainder: ArraySlice<UInt8>
  }

  public static func decodeOne(_ input: [UInt8], limits: Limits = .defaults) throws -> Decoded {
    let result = try decodeAt(input, start: 0, available: input.count, limits: limits)
    return Decoded(element: result.element, remainder: input[result.nextOffset...])
  }

  public static func decodeExact(_ input: [UInt8], limits: Limits = .defaults) throws -> Element {
    let result = try decodeAt(input, start: 0, available: input.count, limits: limits)
    guard result.nextOffset == input.count else {
      throw failure("trailing-data", result.nextOffset)
    }
    return result.element
  }

  public final class Cursor {
    private let input: [UInt8]
    private let limits: Limits
    private var offset = 0
    public private(set) var elementsRead: UInt64 = 0

    public init(_ input: [UInt8], limits: Limits = .defaults) throws {
      guard UInt64(input.count) <= limits.maxInputLength else {
        throw failure("input-limit-exceeded", 0)
      }
      self.input = input
      self.limits = limits
    }

    public var remaining: ArraySlice<UInt8> { input[offset...] }

    public func read() throws -> Element? {
      if offset == input.count { return nil }
      guard elementsRead < limits.maxElements else {
        throw failure("element-limit-exceeded", offset)
      }
      let result = try decodeAt(
        input, start: offset, available: input.count - offset, limits: limits)
      offset = result.nextOffset
      elementsRead += 1
      return result.element
    }

    public func finish() throws {
      guard offset == input.count else { throw failure("trailing-data", offset) }
    }
  }

  private struct DecodeResult {
    let element: Element
    let nextOffset: Int
  }

  private static let tagClasses = ["universal", "application", "context-specific", "private"]

  private static func decodeAt(_ input: [UInt8], start: Int, available: Int, limits: Limits) throws
    -> DecodeResult
  {
    guard UInt64(available) <= limits.maxInputLength else {
      throw failure("input-limit-exceeded", start)
    }
    guard available > 0 else { throw failure("empty-input", start) }

    let first = input[start]
    let tagClass = tagClasses[Int(first >> 6)]
    let constructed = first & 0x20 != 0
    let low = first & 0x1f
    let number: UInt64
    let identifierLength: Int
    if low != 0x1f {
      number = UInt64(low)
      identifierLength = 1
      guard number <= limits.maxTagNumber else { throw failure("tag-limit-exceeded", start) }
    } else {
      (number, identifierLength) = try decodeHighTag(
        input, start: start, available: available, limits: limits)
    }
    guard tagClass != "universal" || number != 0 else {
      throw failure("end-of-contents", start)
    }

    let length = try decodeLength(
      input, start: start, available: available, identifierLength: identifierLength)
    guard length.value <= limits.maxValueLength else {
      throw failure("value-limit-exceeded", length.offset)
    }
    let headerLength = identifierLength + length.octets
    guard length.value <= UInt64(Int.max - headerLength) else {
      throw failure("length-host-overflow", length.offset)
    }
    let encodedLength = headerLength + Int(length.value)
    guard encodedLength <= available else { throw failure("truncated-value", start + available) }
    let tag = Tag(tagClass: tagClass, constructed: constructed, number: number)
    return DecodeResult(
      element: Element(
        tag: tag, input: input, start: start, headerLength: headerLength,
        encodedLength: encodedLength),
      nextOffset: start + encodedLength
    )
  }

  private static func decodeHighTag(
    _ input: [UInt8], start: Int, available: Int, limits: Limits
  ) throws -> (UInt64, Int) {
    var number: UInt64 = 0
    var index = 1
    while true {
      guard index < available else { throw failure("truncated-high-tag", start + index) }
      let octet = input[start + index]
      let payload = UInt64(octet & 0x7f)
      guard index != 1 || payload != 0 else { throw failure("non-minimal-tag", start + index) }
      guard number <= (0xffff_ffff - payload) / 128 else {
        throw failure("tag-overflow", start + index)
      }
      number = number * 128 + payload
      guard number <= limits.maxTagNumber else {
        throw failure("tag-limit-exceeded", start + index)
      }
      index += 1
      if octet & 0x80 == 0 { break }
    }
    guard number >= 31 else { throw failure("non-minimal-tag", start) }
    return (number, index)
  }

  private struct Length {
    let value: UInt64
    let octets: Int
    let offset: Int
  }

  private static func decodeLength(
    _ input: [UInt8], start: Int, available: Int, identifierLength: Int
  ) throws -> Length {
    let lengthOffset = start + identifierLength
    guard identifierLength < available else { throw failure("truncated-length", lengthOffset) }
    let first = input[lengthOffset]
    if first < 0x80 { return Length(value: UInt64(first), octets: 1, offset: lengthOffset) }
    guard first != 0x80 else { throw failure("indefinite-length", lengthOffset) }
    guard first != 0xff else { throw failure("reserved-length", lengthOffset) }
    let count = Int(first & 0x7f)
    guard count <= 8 else { throw failure("length-too-wide", lengthOffset) }
    guard identifierLength + 1 + count <= available else {
      throw failure("truncated-length", start + available)
    }
    guard input[lengthOffset + 1] != 0 else {
      throw failure("non-minimal-length", lengthOffset + 1)
    }
    var value: UInt64 = 0
    for index in 0..<count {
      value = value * 256 + UInt64(input[lengthOffset + 1 + index])
    }
    guard value >= 128 else { throw failure("non-minimal-length", lengthOffset) }
    guard value <= UInt64(Int.max) else { throw failure("length-host-overflow", lengthOffset) }
    return Length(value: value, octets: count + 1, offset: lengthOffset)
  }

  private static func failure(_ kind: String, _ offset: Int) -> FramingError {
    FramingError(kind: kind, offset: offset)
  }
}

import DerAsn1
import DerTlv
import Foundation
import XCTest

final class DerAsn1Tests: XCTestCase {
  private static let fixture = loadFixture("der-asn1-v1")
  private static let upstream = loadFixture("der-tlv-v1")

  func testPortableConformance() throws {
    let cases = try XCTUnwrap(Self.fixture["cases"] as? [[String: Any]])
    let errorIds = try XCTUnwrap(Self.fixture["error_ids"] as? [String])
    XCTAssertEqual(cases.count, 122)
    XCTAssertEqual(errorIds.count, 22)
    let references = cases.compactMap { $0["der_tlv_case_id"] as? String }
    XCTAssertEqual(references.count, 46)
    XCTAssertEqual(Set(references).count, 46)
    for testCase in cases {
      let actual = try runCase(testCase)
      XCTAssertEqual(
        try canonical(actual), try canonical(try XCTUnwrap(testCase["expected"])),
        testCase["id"] as? String ?? ""
      )
      if let hostile = testCase["redacted_input_hex"] as? String {
        XCTAssertFalse(try canonical(actual).contains(hostile))
      }
    }
  }

  func testNativeContractsAndSnapshots() throws {
    XCTAssertEqual(DerAsn1.ErrorKind.allCases.count, 22)
    XCTAssertEqual(DerAsn1.Limits().maxDepth, 32)
    XCTAssertEqual(DerAsn1.Limits().maxTotalElements, 16_384)
    XCTAssertEqual(DerAsn1.Limits().maxOidArcs, 128)

    var input = fromHex("060b2a81ffffffffffffffff7f")
    let element = try DerAsn1.Decoder().decodeExact(input)
    let oid = try DerAsn1.decodeObjectIdentifier(element)
    XCTAssertEqual(oid.arcs, [1, 2, UInt64.max])
    XCTAssertTrue(oid.equals(arcs: oid.arcs))
    XCTAssertFalse(oid.equals(arcs: [1, 2]))
    input[input.count - 1] = 1
    XCTAssertEqual(element.value.last, 0x7f)

    let integer = try DerAsn1.decodeInteger(
      DerAsn1.Decoder().decodeExact(fromHex("020101")))
    XCTAssertEqual(try integer.toUInt64(), 1)
    XCTAssertFalse(integer.isNegative)

    let bits = try DerAsn1.decodeBitString(
      DerAsn1.Decoder().decodeExact(fromHex("03020180")))
    XCTAssertEqual(bits.bitLength, 7)
    XCTAssertEqual(bits.unusedBits, 1)

    let hostile = try DerAsn1.Decoder().decodeExact(fromHex("160261ff"))
    XCTAssertThrowsError(try DerAsn1.decodeIA5String(hostile)) { error in
      XCTAssertFalse(String(describing: error).contains("61ff"))
      XCTAssertFalse(String(describing: error).contains("255"))
    }
  }

  func testTypedHelpersLimitsAndErrorPrecedence() throws {
    let oid = try DerAsn1.decodeObjectIdentifier(
      DerAsn1.Decoder().decodeExact(fromHex("06032a0304")))
    XCTAssertEqual(oid.arcs, [1, 2, 3, 4])
    XCTAssertEqual(hex(oid.encoded), "2a0304")

    let negative = try DerAsn1.decodeInteger(
      DerAsn1.Decoder().decodeExact(fromHex("0201ff")))
    XCTAssertTrue(negative.isNegative)
    XCTAssertThrowsError(try negative.toUInt64()) { self.assertKind($0, .negativeInteger) }
    let tooWide = try DerAsn1.decodeInteger(
      DerAsn1.Decoder().decodeExact(fromHex("0209010102030405060708")))
    XCTAssertThrowsError(try tooWide.toUInt64()) { self.assertKind($0, .integerOverflow) }

    XCTAssertEqual(
      try DerAsn1.decodeIA5String(DerAsn1.Decoder().decodeExact(fromHex("16026869"))), "hi")
    XCTAssertEqual(
      try DerAsn1.decodeImplicitIA5String(
        DerAsn1.Decoder().decodeExact(fromHex("80026869")), tagNumber: 0), "hi")
    XCTAssertEqual(
      try DerAsn1.decodeOctetString(DerAsn1.Decoder().decodeExact(fromHex("04012a"))), [0x2a])
    XCTAssertEqual(
      try DerAsn1.decodeImplicitOctetString(
        DerAsn1.Decoder().decodeExact(fromHex("80012a")), tagNumber: 0), [0x2a])

    try assertFailure("010101", .invalidBooleanValue) { try DerAsn1.decodeBoolean($0) }
    try assertFailure("02020001", .nonMinimalInteger) { try DerAsn1.decodeInteger($0) }
    try assertFailure("030108", .invalidUnusedBitCount) { try DerAsn1.decodeBitString($0) }
    try assertFailure("03020181", .nonZeroBitPadding) { try DerAsn1.decodeBitString($0) }
    try assertFailure("050100", .nonEmptyNull) { try DerAsn1.decodeNull($0) }
    try assertFailure("0600", .emptyObjectIdentifier) {
      try DerAsn1.decodeObjectIdentifier($0)
    }
    try assertFailure("0500", .unexpectedTag) { try DerAsn1.decodeBoolean($0) }
  }

  func testCursorLimitComparisonAndMalformedFinish() throws {
    let decoder = DerAsn1.Decoder()
    let cursor = try decoder.sequence(decoder.decodeExact(fromHex("30020500")))
    XCTAssertThrowsError(try cursor.read(DerAsn1.Decoder())) {
      self.assertKind($0, .decoderLimitMismatch)
    }
    XCTAssertNotNil(try cursor.read(decoder))
    XCTAssertTrue(cursor.remaining.isEmpty)

    let variants = [
      DerAsn1.Limits(maxDepth: 31),
      DerAsn1.Limits(maxTotalElements: 16_383),
      DerAsn1.Limits(maxOidArcs: 127),
      DerAsn1.Limits(der: try DerTlv.Limits(maxInputLength: 1_048_575)),
      DerAsn1.Limits(der: try DerTlv.Limits(maxValueLength: 1_048_575)),
      DerAsn1.Limits(der: try DerTlv.Limits(maxElements: 4_095)),
      DerAsn1.Limits(der: try DerTlv.Limits(maxTagNumber: 0xffff_fffe)),
    ]
    for variant in variants {
      let owner = DerAsn1.Decoder()
      let mismatched = try owner.sequence(owner.decodeExact(fromHex("30020500")))
      XCTAssertThrowsError(try mismatched.read(DerAsn1.Decoder(limits: variant))) {
        self.assertKind($0, .decoderLimitMismatch)
      }
    }

    let malformedOwner = DerAsn1.Decoder()
    let malformed = try malformedOwner.sequence(
      malformedOwner.decodeExact(fromHex("300101")))
    XCTAssertThrowsError(try malformed.finish()) { error in
      self.assertKind(error, .framing)
      XCTAssertEqual((error as? DerAsn1.ValueError)?.framingKind, "trailing-data")
    }
  }

  private func runCase(_ testCase: [String: Any]) throws -> Any {
    if let id = testCase["der_tlv_case_id"] as? String { return try verifyUpstream(id) }
    let configured = try limits(testCase)
    let decoder = DerAsn1.Decoder(limits: configured)
    let operation = try XCTUnwrap(testCase["operation"] as? String)
    do {
      let root = try decoder.decodeExact(
        materialize(try XCTUnwrap(testCase["input"] as? [[String: Any]])))
      switch operation {
      case "decode-exact":
        return [
          "outcome": "value", "tag": tag(root), "header_hex": hex(root.header),
          "value_hex": hex(root.value), "encoded_hex": hex(root.encoded), "depth": root.depth,
          "elements_read": decoder.elementsRead,
        ]
      case "cursor-script": return try cursorResult(testCase, decoder: decoder, root: root)
      case "sequence", "set":
        let cursor = try operation == "sequence" ? decoder.sequence(root) : decoder.set(root)
        return [
          "outcome": "value", "elements_read": decoder.elementsRead,
          "remaining_offset": root.value.count - cursor.remaining.count,
        ]
      case "explicit":
        let child = try decoder.explicit(root, tagNumber: number(testCase["tag_number"]))
        return [
          "outcome": "value", "tag": tag(child), "value_hex": hex(child.value),
          "depth": child.depth, "elements_read": decoder.elementsRead,
        ]
      default:
        return try primitive(
          operation, element: root, configured: configured,
          tagNumber: number(testCase["tag_number"]))
      }
    } catch let error as DerAsn1.ValueError {
      let scope =
        operation == "explicit" && error.kind == .framing
        ? "container-value" : "operation-input"
      return failure(error, scope: scope)
    }
  }

  private func verifyUpstream(_ id: String) throws -> Any {
    let cases = try XCTUnwrap(Self.upstream["cases"] as? [[String: Any]])
    let referenced = try XCTUnwrap(cases.first { $0["id"] as? String == id })
    let defaults = try XCTUnwrap(Self.upstream["defaults"] as? [String: Any])
    let der = try derLimits(defaults, referenced["limits"] as? [String: Any] ?? [:])
    let decoder = DerAsn1.Decoder(limits: DerAsn1.Limits(der: der))
    let expected = try XCTUnwrap(referenced["expected"] as? [String: Any])
    let input = materialize(try XCTUnwrap(referenced["input"] as? [[String: Any]]))
    do {
      let element = try decoder.decodeExact(input)
      XCTAssertEqual(expected["outcome"] as? String, "element", id)
      let expectedTag = try XCTUnwrap(expected["tag"] as? [String: Any])
      XCTAssertEqual(expectedTag["class"] as? String, element.tag.tagClass, id)
      XCTAssertEqual(expectedTag["constructed"] as? Bool, element.tag.constructed, id)
      XCTAssertEqual(number(expectedTag["number"]), element.tag.number, id)
      XCTAssertEqual(number(expected["header_len"]), UInt64(element.header.count), id)
      XCTAssertEqual(number(expected["encoded_len"]), UInt64(element.encoded.count), id)
    } catch let error as DerAsn1.ValueError {
      XCTAssertEqual(expected["outcome"] as? String, "error", id)
      XCTAssertEqual(error.kind, .framing, id)
      XCTAssertEqual(error.framingKind, expected["error_id"] as? String, id)
      XCTAssertEqual(error.offset, (expected["offset"] as? NSNumber)?.intValue, id)
      if let hostile = referenced["redacted_input_hex"] as? String {
        XCTAssertFalse(String(describing: error).contains(hostile))
      }
    }
    return ["outcome": "upstream"]
  }

  private func primitive(
    _ operation: String, element: DerAsn1.Element, configured: DerAsn1.Limits,
    tagNumber: UInt64
  ) throws -> Any {
    switch operation {
    case "decode-boolean":
      return [
        "outcome": "value", "boolean": try DerAsn1.decodeBoolean(element), "elements_read": 1,
      ]
    case "decode-integer", "integer-to-u64":
      let integer = try DerAsn1.decodeInteger(element)
      var result: [String: Any] = [
        "outcome": "value", "signed_hex": hex(integer.signedBytes),
        "negative": integer.isNegative,
      ]
      if operation == "integer-to-u64" {
        result["u64_decimal"] = try integer.toUInt64().description
      }
      return result
    case "decode-bit-string":
      let bits = try DerAsn1.decodeBitString(element)
      return [
        "outcome": "value", "bytes_hex": hex(bits.bytes), "unused_bits": bits.unusedBits,
        "bit_length": bits.bitLength,
      ]
    case "decode-octet-string":
      return ["outcome": "value", "bytes_hex": hex(try DerAsn1.decodeOctetString(element))]
    case "decode-implicit-octet-string":
      return [
        "outcome": "value",
        "bytes_hex": hex(try DerAsn1.decodeImplicitOctetString(element, tagNumber: tagNumber)),
      ]
    case "decode-ia5-string":
      return ["outcome": "value", "text": try DerAsn1.decodeIA5String(element)]
    case "decode-implicit-ia5-string":
      return [
        "outcome": "value",
        "text": try DerAsn1.decodeImplicitIA5String(element, tagNumber: tagNumber),
      ]
    case "decode-null":
      try DerAsn1.decodeNull(element)
      return ["outcome": "value"]
    case "decode-object-identifier", "decode-implicit-object-identifier":
      let oid =
        try operation == "decode-object-identifier"
        ? DerAsn1.decodeObjectIdentifier(element, limits: configured)
        : DerAsn1.decodeImplicitObjectIdentifier(
          element, tagNumber: tagNumber, limits: configured)
      return [
        "outcome": "value", "bytes_hex": hex(oid.encoded),
        "arcs_decimal": oid.arcs.map(\.description), "arc_count": oid.arcCount,
      ]
    default: throw TestFailure.unsupportedOperation(operation)
    }
  }

  private func cursorResult(
    _ testCase: [String: Any], decoder: DerAsn1.Decoder, root: DerAsn1.Element
  ) throws -> Any {
    let cursor = try decoder.sequence(root)
    let total = cursor.remaining.count
    var events: [[String: Any]] = []
    for action in try XCTUnwrap(testCase["actions"] as? [String]) {
      if action == "finish" {
        do {
          try cursor.finish()
          events.append(["outcome": "finished"])
        } catch let error as DerAsn1.ValueError {
          events.append(failure(error, scope: "container-value"))
        }
        continue
      }
      let active: DerAsn1.Decoder
      if action == "read-with-different-limits" {
        active = DerAsn1.Decoder(
          limits: DerAsn1.Limits(
            der: decoder.limits.der, maxDepth: decoder.limits.maxDepth,
            maxTotalElements: decoder.limits.maxTotalElements + 1,
            maxOidArcs: decoder.limits.maxOidArcs))
      } else {
        active = decoder
      }
      do {
        let child = try cursor.read(active)
        if action == "read-nested-sequence" {
          let nested = try decoder.sequence(try XCTUnwrap(child))
          let grandchild = try XCTUnwrap(nested.read(decoder))
          try nested.finish()
          events.append(["outcome": "value", "tag": tag(grandchild), "depth": grandchild.depth])
        } else if let child {
          events.append(["outcome": "value", "tag": tag(child), "depth": child.depth])
        } else {
          events.append(["outcome": "end"])
        }
      } catch let error as DerAsn1.ValueError {
        events.append(failure(error, scope: "container-value"))
      }
    }
    return [
      "outcome": "value", "elements_read": decoder.elementsRead,
      "remaining_offset": total - cursor.remaining.count, "events": events,
    ]
  }

  private func limits(_ testCase: [String: Any]) throws -> DerAsn1.Limits {
    let defaults = try XCTUnwrap(Self.fixture["defaults"] as? [String: Any])
    let overrides = testCase["limits"] as? [String: Any] ?? [:]
    return DerAsn1.Limits(
      der: try derLimits(
        try XCTUnwrap(defaults["der"] as? [String: Any]),
        overrides["der"] as? [String: Any] ?? [:]),
      maxDepth: Int(limit(defaults, overrides, "max_depth")),
      maxTotalElements: limit(defaults, overrides, "max_total_elements"),
      maxOidArcs: Int(limit(defaults, overrides, "max_oid_arcs"))
    )
  }

  private func derLimits(_ defaults: [String: Any], _ overrides: [String: Any]) throws
    -> DerTlv.Limits
  {
    try DerTlv.Limits(
      maxInputLength: limit(defaults, overrides, "max_input_len"),
      maxValueLength: limit(defaults, overrides, "max_value_len"),
      maxElements: limit(defaults, overrides, "max_elements"),
      maxTagNumber: limit(defaults, overrides, "max_tag_number"))
  }

  private func limit(_ defaults: [String: Any], _ overrides: [String: Any], _ name: String)
    -> UInt64
  {
    let value = overrides[name] ?? defaults[name]
    return value as? String == "host-max" ? UInt64(Int.max) : number(value)
  }

  private func tag(_ element: DerAsn1.Element) -> [String: Any] {
    [
      "class": element.tag.tagClass, "constructed": element.tag.constructed,
      "number": element.tag.number,
    ]
  }

  private func failure(_ error: DerAsn1.ValueError, scope: String) -> [String: Any] {
    var result: [String: Any] = [
      "outcome": "error", "error_id": error.kind.rawValue, "offset": error.offset,
      "offset_scope": scope,
    ]
    if let framing = error.framingKind { result["framing_error_id"] = framing }
    return result
  }

  private func assertKind(_ error: Error, _ kind: DerAsn1.ErrorKind) {
    XCTAssertEqual((error as? DerAsn1.ValueError)?.kind, kind)
  }

  private func assertFailure(
    _ encoded: String, _ kind: DerAsn1.ErrorKind,
    _ body: (DerAsn1.Element) throws -> Any
  ) throws {
    let element = try DerAsn1.Decoder().decodeExact(fromHex(encoded))
    XCTAssertThrowsError(try body(element)) { self.assertKind($0, kind) }
  }

  private static func loadFixture(_ name: String) -> [String: Any] {
    let path = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
      .appendingPathComponent("../../../specs/fixtures/\(name)/cases.json")
    let data = try! Data(contentsOf: path)
    return try! JSONSerialization.jsonObject(with: data) as! [String: Any]
  }

  private func materialize(_ segments: [[String: Any]]) -> [UInt8] {
    var result: [UInt8] = []
    for segment in segments {
      let bytes = fromHex((segment["hex"] ?? segment["repeat_hex"]) as! String)
      let count = (segment["count"] as? NSNumber)?.intValue ?? 1
      for _ in 0..<count { result.append(contentsOf: bytes) }
    }
    return result
  }

  private func number(_ value: Any?) -> UInt64 { (value as? NSNumber)?.uint64Value ?? 0 }
  private func fromHex(_ value: String) -> [UInt8] {
    stride(from: 0, to: value.count, by: 2).map { offset in
      let start = value.index(value.startIndex, offsetBy: offset)
      let end = value.index(start, offsetBy: 2)
      return UInt8(value[start..<end], radix: 16)!
    }
  }
  private func hex(_ value: [UInt8]) -> String {
    value.map { String(format: "%02x", $0) }.joined()
  }
  private func canonical(_ value: Any) throws -> String {
    let data = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    return try XCTUnwrap(String(data: data, encoding: .utf8))
  }

  private enum TestFailure: Error { case unsupportedOperation(String) }
}

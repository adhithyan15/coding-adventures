import DerAsn1
import DerTlv
import Foundation
import X509Extension
import XCTest

final class X509ExtensionTests: XCTestCase {
  private static let fixture = loadFixture("x509-extension-v1")
  private static let upstream = loadFixture("der-asn1-v1")

  func testPortableConformance() throws {
    let cases = try XCTUnwrap(Self.fixture["cases"] as? [[String: Any]])
    XCTAssertEqual(cases.count, 48)
    for testCase in cases {
      let decoder = DerAsn1.Decoder(limits: try limits(testCase))
      let root = try decoder.decodeExact(
        materialize(try XCTUnwrap(testCase["input"] as? [[String: Any]])))
      let actual: Any
      if testCase["operation"] as? String == "extension-script" {
        let actions = try XCTUnwrap(testCase["actions"] as? [String])
        actual = ["outcome": "script", "events": actions.map { _ in attempt(decoder, root) }]
      } else {
        actual = attempt(decoder, root)
      }
      XCTAssertEqual(
        try canonical(actual),
        try canonical(try XCTUnwrap(testCase["expected"])),
        testCase["id"] as? String ?? ""
      )
      if let hostile = testCase["redacted_input_hex"] as? String {
        XCTAssertFalse(try canonical(actual).contains(hostile))
      }
    }
  }

  func testValueSemantics() throws {
    let decoder = DerAsn1.Decoder()
    let value = try X509Extension.decodeX509Extension(
      decoder, decoder.decodeExact(fromHex("30090603551d1104023000")))
    var opaque = value.extensionValue
    opaque[0] = 0xff
    var encoded = value.extensionId.encoded
    encoded[0] = 0xff
    var arcs = value.extensionId.arcs
    arcs[0] = 0
    XCTAssertEqual(hex(value.extensionValue), "3000")
    XCTAssertEqual(value.extensionId.arcs, [2, 5, 29, 17])
    XCTAssertEqual(value.extensionId.encoded, [0x55, 0x1d, 0x11])
  }

  func testErrorDescriptionIsPayloadBlind() throws {
    let decoder = DerAsn1.Decoder()
    let root = try decoder.decodeExact(fromHex("30080601800403deadbe"))
    do {
      _ = try X509Extension.decodeX509Extension(decoder, root)
      XCTFail("expected invalid extension identifier")
    } catch let error as X509Extension.DecodeError {
      XCTAssertFalse(error.description.lowercased().contains("deadbe"))
      XCTAssertTrue(error.description.contains("byte 4"))
    }
  }

  private func attempt(_ decoder: DerAsn1.Decoder, _ root: DerAsn1.Element) -> [String: Any] {
    do {
      let value = try X509Extension.decodeX509Extension(decoder, root)
      return [
        "outcome": "value",
        "extension_id_arcs_decimal": value.extensionId.arcs.map(\.description),
        "critical": value.critical,
        "extension_value_hex": hex(value.extensionValue),
        "elements_read": decoder.elementsRead,
      ]
    } catch let error as X509Extension.DecodeError {
      var result: [String: Any] = [
        "outcome": "error",
        "error_id": error.kind.rawValue,
        "offset": error.offset,
        "offset_scope": "extension-element",
        "elements_read": decoder.elementsRead,
      ]
      if let kind = error.asn1Kind { result["asn1_error_id"] = kind.rawValue }
      if let kind = error.framingKind { result["framing_error_id"] = kind }
      return result
    } catch {
      XCTFail("unexpected error: \(error)")
      return [:]
    }
  }

  private func limits(_ testCase: [String: Any]) throws -> DerAsn1.Limits {
    let defaults = try XCTUnwrap(Self.upstream["defaults"] as? [String: Any])
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
}

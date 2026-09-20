import Foundation
import XCTest

@testable import DerTlv

final class DerTlvTests: XCTestCase {
  private static let fixture: [String: Any] = {
    let path = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
      .appendingPathComponent("../../../specs/fixtures/der-tlv-v1/cases.json")
    let data = try! Data(contentsOf: path)
    return try! JSONSerialization.jsonObject(with: data) as! [String: Any]
  }()

  func testPortableConformance() throws {
    let cases = try XCTUnwrap(Self.fixture["cases"] as? [[String: Any]])
    XCTAssertEqual(cases.count, 54)
    for testCase in cases {
      let input = materialize(try XCTUnwrap(testCase["input"] as? [[String: Any]]))
      let operation = try XCTUnwrap(testCase["operation"] as? String)
      let actual =
        operation == "cursor"
        ? try runCursor(testCase, input)
        : runDecode(testCase, input)
      let expected = try XCTUnwrap(testCase["expected"])
      XCTAssertEqual(
        try canonical(actual), try canonical(expected), testCase["id"] as? String ?? "")
      if let hostile = testCase["redacted_input_hex"] as? String {
        XCTAssertFalse(try canonical(actual).contains(hostile))
      }
    }
  }

  func testSlicesAndDefaults() throws {
    let element = try DerTlv.decodeExact([4, 1, 42])
    XCTAssertEqual(Array(element.header), [4, 1])
    XCTAssertEqual(Array(element.value), [42])
    XCTAssertEqual(Array(element.encoded), [4, 1, 42])
    XCTAssertEqual(DerTlv.Limits.defaults.maxElements, 4_096)
    XCTAssertEqual(try DerTlv.Cursor([5, 0]).remaining.count, 2)
  }

  func testLimitsAndErrorDescription() throws {
    XCTAssertThrowsError(try DerTlv.Limits(maxTagNumber: 0x1_0000_0000))
    let error = DerTlv.FramingError(kind: "truncated-value", offset: 2)
    XCTAssertEqual(error.description, "DER framing error truncated-value at byte 2")
  }

  private func runDecode(_ testCase: [String: Any], _ input: [UInt8]) -> [String: Any] {
    do {
      if testCase["operation"] as? String == "decode-one" {
        let decoded = try DerTlv.decodeOne(input, limits: try limits(testCase))
        let result = projection(decoded.element, 0)
        XCTAssertEqual(result["remainder_offset"] as? Int, input.count - decoded.remainder.count)
        return result
      }
      return projection(try DerTlv.decodeExact(input, limits: try limits(testCase)), 0)
    } catch let error as DerTlv.FramingError {
      return errorProjection(error)
    } catch {
      XCTFail("unexpected error: \(error)")
      return [:]
    }
  }

  private func runCursor(_ testCase: [String: Any], _ input: [UInt8]) throws -> [String: Any] {
    let cursor = try DerTlv.Cursor(input, limits: limits(testCase))
    let actions = try XCTUnwrap(testCase["actions"] as? [String])
    var events: [[String: Any]] = []
    for action in actions {
      if action == "finish" {
        do {
          try cursor.finish()
          events.append(["outcome": "finished"])
        } catch let error as DerTlv.FramingError {
          events.append(errorProjection(error))
        }
      } else {
        let offset = input.count - cursor.remaining.count
        do {
          if let element = try cursor.read() {
            events.append(projection(element, offset))
          } else {
            events.append(["outcome": "end"])
          }
        } catch let error as DerTlv.FramingError {
          events.append(errorProjection(error))
        }
      }
    }
    return [
      "events": events,
      "elements_read": cursor.elementsRead,
      "remaining_offset": input.count - cursor.remaining.count,
    ]
  }

  private func projection(_ element: DerTlv.Element, _ offset: Int) -> [String: Any] {
    [
      "outcome": "element",
      "element_offset": offset,
      "tag": [
        "class": element.tag.tagClass,
        "constructed": element.tag.constructed,
        "number": element.tag.number,
      ],
      "header_len": element.header.count,
      "encoded_len": element.encoded.count,
      "remainder_offset": offset + element.encoded.count,
    ]
  }

  private func errorProjection(_ error: DerTlv.FramingError) -> [String: Any] {
    ["outcome": "error", "error_id": error.kind, "offset": error.offset]
  }

  private func limits(_ testCase: [String: Any]) throws -> DerTlv.Limits {
    var values = try XCTUnwrap(Self.fixture["defaults"] as? [String: Any])
    if let overrides = testCase["limits"] as? [String: Any] {
      values.merge(overrides) { _, replacement in replacement }
    }
    func value(_ name: String) throws -> UInt64 {
      if values[name] as? String == "host-max" { return UInt64(Int.max) }
      return try XCTUnwrap(values[name] as? NSNumber).uint64Value
    }
    return try DerTlv.Limits(
      maxInputLength: value("max_input_len"),
      maxValueLength: value("max_value_len"),
      maxElements: value("max_elements"),
      maxTagNumber: value("max_tag_number")
    )
  }

  private func materialize(_ segments: [[String: Any]]) -> [UInt8] {
    var result: [UInt8] = []
    for segment in segments {
      let source = (segment["hex"] ?? segment["repeat_hex"]) as! String
      let bytes = stride(from: 0, to: source.count, by: 2).map { offset -> UInt8 in
        let start = source.index(source.startIndex, offsetBy: offset)
        let end = source.index(start, offsetBy: 2)
        return UInt8(source[start..<end], radix: 16)!
      }
      let count = (segment["count"] as? NSNumber)?.intValue ?? 1
      result.reserveCapacity(result.count + bytes.count * count)
      for _ in 0..<count { result.append(contentsOf: bytes) }
    }
    return result
  }

  private func canonical(_ value: Any) throws -> String {
    let data = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    return try XCTUnwrap(String(data: data, encoding: .utf8))
  }
}

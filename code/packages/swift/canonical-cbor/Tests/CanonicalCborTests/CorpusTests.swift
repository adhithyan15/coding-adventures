// ============================================================================
// CorpusTests.swift -- Language-Neutral Conformance Corpus (55 Cases)
// ============================================================================

import CanonicalCbor
import XCTest

final class CorpusTests: XCTestCase {
  func testAllCorpusCases() throws {
    let fixture = try FixtureLoader.loadCorpus()
    XCTAssertEqual(fixture.schemaVersion, 1)
    XCTAssertEqual(fixture.profile, "rfc8949-section-4.2.3-length-first")
    XCTAssertEqual(fixture.cases.count, 55, "Expected 55 test cases in canonical-cbor-v1")

    for tc in fixture.cases {
      switch tc.operation {
      case "round-trip":
        let inBytes = FixtureLoader.hexToBytes(tc.input)
        let decoded = try decode(inBytes)
        let encoded = try encodeChecked(decoded)
        let hexOut = FixtureLoader.bytesToHex(encoded)
        XCTAssertEqual(hexOut, tc.expected, "Case \(tc.id) hex mismatch")
        let reDecoded = try decode(encoded)
        XCTAssertEqual(reDecoded, decoded, "Case \(tc.id) round-trip value mismatch")

      case "encode-map":
        let pairs = tc.input.split(separator: ";")
        var entries: [CborMapEntry] = []
        for p in pairs {
          let parts = p.split(separator: "=")
          XCTAssertEqual(parts.count, 2, "Case \(tc.id) invalid pair: \(p)")
          let kHex = String(parts[0])
          let vHex = String(parts[1])
          let kBytes = FixtureLoader.hexToBytes(kHex)
          let vBytes = FixtureLoader.hexToBytes(vHex)
          let kVal = try decode(kBytes)
          let vVal = try decode(vBytes)
          entries.append(CborMapEntry(key: kVal, value: vVal))
        }
        let mapVal = CborValue.map(entries)
        let encoded = try encodeChecked(mapVal)
        let hexOut = FixtureLoader.bytesToHex(encoded)
        XCTAssertEqual(hexOut, tc.expected, "Case \(tc.id) map hex mismatch")

      case "generated-round-trip":
        let val = FixtureLoader.buildGenerated(tc.input)
        let expectedWire = FixtureLoader.buildExpectedWire(tc.expected)
        let encoded = try encodeChecked(val)
        XCTAssertEqual(encoded, expectedWire, "Case \(tc.id) wire mismatch")
        let decoded = try decode(expectedWire)
        XCTAssertEqual(decoded, val, "Case \(tc.id) value mismatch")

      case "decode-error":
        let wire: [UInt8]
        if tc.input == "nested-array-wire:129" {
          var w = [UInt8](repeating: 0x81, count: 129)
          w.append(0xF6)
          wire = w
        } else {
          wire = FixtureLoader.hexToBytes(tc.input)
        }

        do {
          _ = try decode(wire)
          XCTFail("Case \(tc.id) expected decode error \(tc.expected) but succeeded")
        } catch let err as CborError {
          XCTAssertEqual(
            err.id.rawValue, tc.expected,
            "Case \(tc.id) expected \(tc.expected) got \(err.id.rawValue)")
          XCTAssertTrue(
            err.message.hasPrefix("canonical-cbor: "), "Case \(tc.id) message prefix check")
        } catch {
          XCTFail("Case \(tc.id) unexpected error type: \(error)")
        }

      case "encode-error":
        let val: CborValue
        switch tc.input {
        case "duplicate-map-key":
          val = CborValue.map([
            CborMapEntry(key: .text("dup"), value: .unsigned(1)),
            CborMapEntry(key: .text("dup"), value: .unsigned(2)),
          ])
        case "nested-array:129":
          val = FixtureLoader.buildGenerated("nested-array:129")
        case "bytes-repeat:1048572:00":
          val = CborValue.bytes([UInt8](repeating: 0, count: 1_048_572))
        default:
          XCTFail("Unknown encode-error input: \(tc.input)")
          return
        }

        do {
          _ = try encodeChecked(val)
          XCTFail("Case \(tc.id) expected encode error \(tc.expected) but succeeded")
        } catch let err as CborError {
          XCTAssertEqual(
            err.id.rawValue, tc.expected,
            "Case \(tc.id) expected \(tc.expected) got \(err.id.rawValue)")
          XCTAssertTrue(
            err.message.hasPrefix("canonical-cbor: "), "Case \(tc.id) message prefix check")
        } catch {
          XCTFail("Case \(tc.id) unexpected error type: \(error)")
        }

      default:
        XCTFail("Case \(tc.id) unknown operation: \(tc.operation)")
      }
    }
  }

  func testEmptyArrayAtMaxDepth() throws {
    var val = CborValue.array([])
    for _ in 0..<128 {
      val = .array([val])
    }
    let encoded = try encodeChecked(val)
    var expectedWire = [UInt8](repeating: 0x81, count: 128)
    expectedWire.append(0x80)
    XCTAssertEqual(encoded, expectedWire)
    let decoded = try decode(expectedWire)
    XCTAssertEqual(decoded, val)
  }

  func testEmptyMapAtMaxDepth() throws {
    var val = CborValue.map([])
    for _ in 0..<128 {
      val = .array([val])
    }
    let encoded = try encodeChecked(val)
    var expectedWire = [UInt8](repeating: 0x81, count: 128)
    expectedWire.append(0xA0)
    XCTAssertEqual(encoded, expectedWire)
    let decoded = try decode(expectedWire)
    XCTAssertEqual(decoded, val)
  }
}

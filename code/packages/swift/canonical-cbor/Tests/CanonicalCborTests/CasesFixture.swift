// ============================================================================
// CasesFixture.swift -- Conformance Corpus Fixture Loader & Helpers
// ============================================================================

import CanonicalCbor
import Foundation

public struct TestCase: Decodable {
  public let id: String
  public let operation: String
  public let input: String
  public let expected: String
}

public struct CorpusFixture: Decodable {
  public let schemaVersion: Int
  public let profile: String
  public let cases: [TestCase]

  private enum CodingKeys: String, CodingKey {
    case schemaVersion = "schema_version"
    case profile
    case cases
  }
}

public enum FixtureLoader {
  public static func hexToBytes(_ hex: String) -> [UInt8] {
    var bytes: [UInt8] = []
    var chars = Array(hex)
    var i = 0
    while i < chars.count {
      if i + 1 < chars.count {
        let pair = String(chars[i...i + 1])
        if let b = UInt8(pair, radix: 16) {
          bytes.append(b)
        }
        i += 2
      } else {
        break
      }
    }
    return bytes
  }

  public static func bytesToHex(_ bytes: [UInt8]) -> String {
    bytes.map { String(format: "%02x", $0) }.joined()
  }

  public static func locateCasesFile() -> URL {
    // Strategy 1: Traverse up from #filePath
    var url = URL(fileURLWithPath: #filePath)
    // Tests/CanonicalCborTests/CasesFixture.swift -> 6 steps up is code/
    for _ in 0..<6 {
      url.deleteLastPathComponent()
    }
    let candidate1 = url.appendingPathComponent("specs/fixtures/canonical-cbor-v1/cases.json")
    if FileManager.default.fileExists(atPath: candidate1.path) {
      return candidate1
    }

    // Strategy 2: Check relative to current working directory
    let cwd = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
    let candidate2 = cwd.appendingPathComponent("code/specs/fixtures/canonical-cbor-v1/cases.json")
    if FileManager.default.fileExists(atPath: candidate2.path) {
      return candidate2
    }

    // Strategy 3: Check relative fallback paths
    let relativePaths = [
      "specs/fixtures/canonical-cbor-v1/cases.json",
      "../../specs/fixtures/canonical-cbor-v1/cases.json",
      "../../../specs/fixtures/canonical-cbor-v1/cases.json",
      "../../../../specs/fixtures/canonical-cbor-v1/cases.json",
    ]
    for rel in relativePaths {
      let cand = cwd.appendingPathComponent(rel)
      if FileManager.default.fileExists(atPath: cand.path) {
        return cand
      }
    }

    return candidate1
  }

  public static func loadCorpus() throws -> CorpusFixture {
    let fileUrl = locateCasesFile()
    let data = try Data(contentsOf: fileUrl)
    let decoder = JSONDecoder()
    return try decoder.decode(CorpusFixture.self, from: data)
  }

  public static func buildGenerated(_ spec: String) -> CborValue {
    if spec.hasPrefix("nested-array:") {
      let depthStr = String(spec.dropFirst("nested-array:".count))
      guard let depth = Int(depthStr) else {
        fatalError("Invalid depth in \(spec)")
      }
      var value = CborValue.null
      for _ in 0..<depth {
        value = .array([value])
      }
      return value
    }

    if spec.hasPrefix("bytes-repeat:") {
      let rest = String(spec.dropFirst("bytes-repeat:".count))
      let parts = rest.split(separator: ":")
      guard parts.count == 2,
        let length = Int(parts[0]),
        let byteVal = UInt8(parts[1], radix: 16)
      else {
        fatalError("Invalid bytes-repeat in \(spec)")
      }
      return .bytes([UInt8](repeating: byteVal, count: length))
    }

    fatalError("Unknown generated spec: \(spec)")
  }

  public static func buildExpectedWire(_ spec: String) -> [UInt8] {
    if spec.hasPrefix("wire:nested-array:") {
      let depthStr = String(spec.dropFirst("wire:nested-array:".count))
      guard let depth = Int(depthStr) else {
        fatalError("Invalid depth in \(spec)")
      }
      var wire = [UInt8](repeating: 0x81, count: depth)
      wire.append(0xF6)
      return wire
    }

    if spec.hasPrefix("wire:bytes-repeat:") {
      let rest = String(spec.dropFirst("wire:bytes-repeat:".count))
      let parts = rest.split(separator: ":")
      guard parts.count == 2,
        let length = Int(parts[0]),
        let byteVal = UInt8(parts[1], radix: 16)
      else {
        fatalError("Invalid wire:bytes-repeat in \(spec)")
      }

      var wire: [UInt8] = []
      if length <= 23 {
        wire.append(0x40 | UInt8(length))
      } else if length <= 0xFF {
        wire.append(contentsOf: [0x58, UInt8(length)])
      } else if length <= 0xFFFF {
        wire.append(contentsOf: [0x59, UInt8((length >> 8) & 0xFF), UInt8(length & 0xFF)])
      } else {
        wire.append(contentsOf: [
          0x5A,
          UInt8((length >> 24) & 0xFF),
          UInt8((length >> 16) & 0xFF),
          UInt8((length >> 8) & 0xFF),
          UInt8(length & 0xFF),
        ])
      }
      wire.append(contentsOf: [UInt8](repeating: byteVal, count: length))
      return wire
    }

    fatalError("Unknown wire spec: \(spec)")
  }
}

import Foundation
import GrammarTools
import Lexer
import XCTest

@testable import JavaLexer

private enum PureSha256 {
  private static let initialH: (UInt32, UInt32, UInt32, UInt32, UInt32, UInt32, UInt32, UInt32) = (
    0x6A09_E667, 0xBB67_AE85, 0x3C6E_F372, 0xA54F_F53A,
    0x510E_527F, 0x9B05_688C, 0x1F83_D9AB, 0x5BE0_CD19
  )

  private static let roundConstants: [UInt32] = [
    0x428A_2F98, 0x7137_4491, 0xB5C0_FBCF, 0xE9B5_DBA5,
    0x3956_C25B, 0x59F1_11F1, 0x923F_82A4, 0xAB1C_5ED5,
    0xD807_AA98, 0x1283_5B01, 0x2431_85BE, 0x550C_7DC3,
    0x72BE_5D74, 0x80DE_B1FE, 0x9BDC_06A7, 0xC19B_F174,
    0xE49B_69C1, 0xEFBE_4786, 0x0FC1_9DC6, 0x240C_A1CC,
    0x2DE9_2C6F, 0x4A74_84AA, 0x5CB0_A9DC, 0x76F9_88DA,
    0x983E_5152, 0xA831_C66D, 0xB003_27C8, 0xBF59_7FC7,
    0xC6E0_0BF3, 0xD5A7_9147, 0x06CA_6351, 0x1429_2967,
    0x27B7_0A85, 0x2E1B_2138, 0x4D2C_6DFC, 0x5338_0D13,
    0x650A_7354, 0x766A_0ABB, 0x81C2_C92E, 0x9272_2C85,
    0xA2BF_E8A1, 0xA81A_664B, 0xC24B_8B70, 0xC76C_51A3,
    0xD192_E819, 0xD699_0624, 0xF40E_3585, 0x106A_A070,
    0x19A4_C116, 0x1E37_6C08, 0x2748_774C, 0x34B0_BCB5,
    0x391C_0CB3, 0x4ED8_AA4A, 0x5B9C_CA4F, 0x682E_6FF3,
    0x748F_82EE, 0x78A5_636F, 0x84C8_7814, 0x8CC7_0208,
    0x90BE_FFFA, 0xA450_6CEB, 0xBEF9_A3F7, 0xC671_78F2,
  ]

  @inline(__always)
  private static func rotr(_ x: UInt32, _ n: UInt32) -> UInt32 {
    (x >> n) | (x << (32 - n))
  }

  @inline(__always)
  private static func ch(_ x: UInt32, _ y: UInt32, _ z: UInt32) -> UInt32 {
    (x & y) ^ (~x & z)
  }

  @inline(__always)
  private static func maj(_ x: UInt32, _ y: UInt32, _ z: UInt32) -> UInt32 {
    (x & y) ^ (x & z) ^ (y & z)
  }

  @inline(__always)
  private static func bigSigma0(_ x: UInt32) -> UInt32 {
    rotr(x, 2) ^ rotr(x, 13) ^ rotr(x, 22)
  }

  @inline(__always)
  private static func bigSigma1(_ x: UInt32) -> UInt32 {
    rotr(x, 6) ^ rotr(x, 11) ^ rotr(x, 25)
  }

  @inline(__always)
  private static func smallSigma0(_ x: UInt32) -> UInt32 {
    rotr(x, 7) ^ rotr(x, 18) ^ (x >> 3)
  }

  @inline(__always)
  private static func smallSigma1(_ x: UInt32) -> UInt32 {
    rotr(x, 17) ^ rotr(x, 19) ^ (x >> 10)
  }

  static func hashHex(data: Data) -> String {
    let bytes = [UInt8](data)
    let bitLen = UInt64(bytes.count) * 8
    let afterBit = (bytes.count + 1) % 64
    let zeroCount = afterBit <= 56 ? 56 - afterBit : 64 + 56 - afterBit

    var padded = [UInt8](repeating: 0, count: bytes.count + 1 + zeroCount + 8)
    for i in 0..<bytes.count {
      padded[i] = bytes[i]
    }
    padded[bytes.count] = 0x80

    let lengthOffset = padded.count - 8
    padded[lengthOffset] = UInt8((bitLen >> 56) & 0xFF)
    padded[lengthOffset + 1] = UInt8((bitLen >> 48) & 0xFF)
    padded[lengthOffset + 2] = UInt8((bitLen >> 40) & 0xFF)
    padded[lengthOffset + 3] = UInt8((bitLen >> 32) & 0xFF)
    padded[lengthOffset + 4] = UInt8((bitLen >> 24) & 0xFF)
    padded[lengthOffset + 5] = UInt8((bitLen >> 16) & 0xFF)
    padded[lengthOffset + 6] = UInt8((bitLen >> 8) & 0xFF)
    padded[lengthOffset + 7] = UInt8(bitLen & 0xFF)

    var h0 = initialH.0
    var h1 = initialH.1
    var h2 = initialH.2
    var h3 = initialH.3
    var h4 = initialH.4
    var h5 = initialH.5
    var h6 = initialH.6
    var h7 = initialH.7

    var offset = 0
    var messageSchedule = [UInt32](repeating: 0, count: 64)
    while offset < padded.count {
      for i in 0..<16 {
        let base = offset + i * 4
        messageSchedule[i] =
          (UInt32(padded[base]) << 24)
          | (UInt32(padded[base + 1]) << 16)
          | (UInt32(padded[base + 2]) << 8)
          | UInt32(padded[base + 3])
      }
      for i in 16..<64 {
        messageSchedule[i] =
          smallSigma1(messageSchedule[i - 2]) &+ messageSchedule[i - 7]
          &+ smallSigma0(messageSchedule[i - 15]) &+ messageSchedule[i - 16]
      }

      var a = h0
      var b = h1
      var c = h2
      var d = h3
      var e = h4
      var f = h5
      var g = h6
      var h = h7

      for t in 0..<64 {
        let t1 = h &+ bigSigma1(e) &+ ch(e, f, g) &+ roundConstants[t] &+ messageSchedule[t]
        let t2 = bigSigma0(a) &+ maj(a, b, c)

        h = g
        g = f
        f = e
        e = d &+ t1
        d = c
        c = b
        b = a
        a = t1 &+ t2
      }

      h0 = h0 &+ a
      h1 = h1 &+ b
      h2 = h2 &+ c
      h3 = h3 &+ d
      h4 = h4 &+ e
      h5 = h5 &+ f
      h6 = h6 &+ g
      h7 = h7 &+ h

      offset += 64
    }

    let resultWords = [h0, h1, h2, h3, h4, h5, h6, h7]
    var hex = ""
    hex.reserveCapacity(64)
    for word in resultWords {
      hex += String(format: "%08x", word)
    }
    return hex
  }
}

final class JavaLexerConformanceTests: XCTestCase {
  private let maxFixtureBytes = 2_097_152

  private func locateRepoRoot() -> URL {
    var url = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
    while url.pathComponents.count > 1 {
      let candidate = url.appendingPathComponent(
        "code/specs/fixtures/language-frontends-v1/cases.json")
      if FileManager.default.fileExists(atPath: candidate.path) {
        return url
      }
      url = url.deletingLastPathComponent()
    }
    fatalError("Could not locate repository root")
  }

  private func loadBoundedData(fileURL: URL) throws -> Data {
    let handle = try FileHandle(forReadingFrom: fileURL)
    defer { try? handle.close() }

    guard let data = try handle.read(upToCount: maxFixtureBytes + 1) else {
      throw NSError(domain: "Test", code: 1, userInfo: [NSLocalizedDescriptionKey: "Empty file"])
    }
    XCTAssertLessThanOrEqual(
      data.count, maxFixtureBytes, "Fixture file exceeds byte limit: \(data.count)")
    return data
  }

  private func loadBoundedJSON(relativePath: String) throws -> [String: Any] {
    let repoRoot = locateRepoRoot()
    let fileURL = repoRoot.appendingPathComponent(relativePath)
    let data = try loadBoundedData(fileURL: fileURL)

    let json = try JSONSerialization.jsonObject(with: data, options: [])
    guard let dict = json as? [String: Any] else {
      fatalError("Expected top-level dictionary")
    }
    return dict
  }

  func testCanonicalCasesReplayFromLanguageFrontendsV1() throws {
    let casesData = try loadBoundedJSON(
      relativePath: "code/specs/fixtures/language-frontends-v1/cases.json")
    guard let casesList = casesData["cases"] as? [[String: Any]] else {
      XCTFail("Missing cases list")
      return
    }

    let javaCases = casesList.filter { ($0["family"] as? String) == "java" }
    XCTAssertFalse(javaCases.isEmpty, "Expected at least one canonical java case")

    for testCase in javaCases {
      guard let caseId = testCase["id"] as? String,
        let version = testCase["version"] as? String,
        let source = testCase["source"] as? String,
        let outcome = testCase["outcome"] as? String
      else {
        continue
      }

      if outcome == "ok" {
        let tokens = try tokenizeJava(source, version: version)
        guard let expectedTokens = testCase["tokens"] as? [[String: Any]] else {
          XCTFail("Missing expected tokens in \(caseId)")
          continue
        }

        XCTAssertEqual(tokens.count, expectedTokens.count, "Token count mismatch in \(caseId)")
        for (i, expected) in expectedTokens.enumerated() {
          let actual = tokens[i]
          XCTAssertEqual(
            actual.type, expected["type"] as? String, "Type mismatch at \(i) in \(caseId)")
          XCTAssertEqual(
            actual.value, expected["value"] as? String, "Value mismatch at \(i) in \(caseId)")
          XCTAssertEqual(
            actual.line, expected["line"] as? Int, "Line mismatch at \(i) in \(caseId)")
          XCTAssertEqual(
            actual.column, expected["column"] as? Int, "Column mismatch at \(i) in \(caseId)")
        }
      } else if outcome == "error" {
        XCTAssertThrowsError(
          try tokenizeJava(source, version: version),
          "Expected lexical error in \(caseId)"
        ) { error in
          guard let lexerError = error as? JavaLexerError else {
            XCTFail("Expected JavaLexerError in \(caseId), got \(error)")
            return
          }
          XCTAssertEqual(lexerError.code, .lexicalError)
        }
      } else {
        XCTFail("Unsupported outcome: \(outcome) in \(caseId)")
      }
    }
  }

  func testAllTenJavaTokensFilesMatchSha256InGrammarsJson() throws {
    let grammarsData = try loadBoundedJSON(
      relativePath: "code/specs/fixtures/language-frontends-v1/grammars.json")
    guard let grammarsList = grammarsData["grammars"] as? [[String: Any]] else {
      XCTFail("Missing grammars list")
      return
    }

    let javaTokens = grammarsList.filter {
      ($0["family"] as? String) == "java" && ($0["kind"] as? String) == "tokens"
    }
    XCTAssertEqual(javaTokens.count, 10, "Expected 10 java token grammars")

    let repoRoot = locateRepoRoot()
    for entry in javaTokens {
      guard let relPath = entry["path"] as? String,
        let expectedSha = entry["sha256"] as? String
      else {
        continue
      }
      let fileURL = repoRoot.appendingPathComponent(relPath)
      let data = try loadBoundedData(fileURL: fileURL)
      let actualSha = PureSha256.hashHex(data: data)
      XCTAssertEqual(actualSha, expectedSha, "SHA-256 mismatch for \(relPath)")
    }
  }

  func testEditionSpecificKeywordsAcrossAllTenEditions() throws {
    // 1.0: base keywords
    let tokensV10 = try tokenizeJava("int x = 1;", version: "1.0")
    XCTAssertTrue(tokensV10.contains { $0.type == "KEYWORD" && $0.value == "int" })

    // 1.1: inner classes / transient
    let tokensV11 = try tokenizeJava("transient int y = 2;", version: "1.1")
    XCTAssertTrue(tokensV11.contains { $0.type == "KEYWORD" && $0.value == "transient" })

    // 1.4: assert is keyword in 1.4, identifier in 1.1
    let tokensV14 = try tokenizeJava("assert condition;", version: "1.4")
    XCTAssertTrue(tokensV14.contains { $0.type == "KEYWORD" && $0.value == "assert" })
    let tokensV11Assert = try tokenizeJava("int assert = 5;", version: "1.1")
    XCTAssertTrue(tokensV11Assert.contains { $0.type == "NAME" && $0.value == "assert" })

    // 5: enum and annotations (@)
    let tokensV5 = try tokenizeJava("@Override enum Color { RED }", version: "5")
    XCTAssertTrue(tokensV5.contains { $0.type == "KEYWORD" && $0.value == "enum" })
    XCTAssertTrue(tokensV5.contains { $0.type == "AT" })
    let tokensV14Enum = try tokenizeJava("int enum = 5;", version: "1.4")
    XCTAssertTrue(tokensV14Enum.contains { $0.type == "NAME" && $0.value == "enum" })

    // 7: binary literals and underscores
    let tokensV7 = try tokenizeJava("int b = 0b1010_0101;", version: "7")
    XCTAssertTrue(tokensV7.contains { $0.type == "NUMBER" })

    // 8: lambdas (->) and double-colon (::)
    let tokensV8 = try tokenizeJava("x -> x::toString", version: "8")
    XCTAssertTrue(tokensV8.contains { $0.type == "ARROW" })
    XCTAssertTrue(tokensV8.contains { $0.type == "DOUBLE_COLON" })

    // 10: var is restricted identifier (lexed as NAME per JLS)
    let tokensV10Var = try tokenizeJava("var x = \"test\";", version: "10")
    XCTAssertTrue(tokensV10Var.contains { $0.type == "NAME" && $0.value == "var" })

    // 14: yield in switch expressions
    let tokensV14Yield = try tokenizeJava("yield 42;", version: "14")
    XCTAssertTrue(tokensV14Yield.contains { $0.type == "KEYWORD" && $0.value == "yield" })
    let tokensV10Yield = try tokenizeJava("int yield = 10;", version: "10")
    XCTAssertTrue(tokensV10Yield.contains { $0.type == "NAME" && $0.value == "yield" })

    // 17: sealed and permits (lexed as NAME per JLS)
    let tokensV17 = try tokenizeJava("sealed class A permits B {}", version: "17")
    XCTAssertTrue(tokensV17.contains { $0.type == "NAME" && $0.value == "sealed" })
    XCTAssertTrue(tokensV17.contains { $0.type == "NAME" && $0.value == "permits" })

    // 21: default edition features
    let tokensV21 = try tokenizeJava("record Point(int x, int y) {}")
    XCTAssertTrue(tokensV21.contains { $0.type == "NAME" && $0.value == "record" })
    XCTAssertFalse(tokensV21.contains { $0.type == "KEYWORD" && $0.value == "class" })
  }
}

import CryptoKit
import Foundation
import GrammarTools
import Lexer
import XCTest

@testable import JavaLexer

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
      let digest = SHA256.hash(data: data)
      let actualSha = digest.map { String(format: "%02x", $0) }.joined()
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

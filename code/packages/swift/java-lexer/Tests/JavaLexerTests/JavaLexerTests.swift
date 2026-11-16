import Foundation
import GrammarTools
import Lexer
import XCTest

@testable import JavaLexer

/// Linear congruential generator for deterministic property and fuzz testing.
private struct SimplePRNG {
  private var state: UInt64

  init(seed: UInt64) {
    self.state = seed
  }

  mutating func next() -> UInt64 {
    state = state &* 6_364_136_223_846_793_005 &+ 1_442_695_040_888_963_407
    return state
  }

  mutating func nextInt(upperBound: Int) -> Int {
    Int(next() % UInt64(upperBound))
  }
}

final class JavaLexerTests: XCTestCase {

  func testClass1OversizedSourceCeilingAndExactBoundary() throws {
    // Exact boundary: 1,048,576 bytes of whitespace is accepted
    let atBoundary = String(repeating: " ", count: JavaLexerConstants.maxSourceLength)
    let tokensAtBoundary = try tokenizeJava(atBoundary)
    XCTAssertEqual(tokensAtBoundary.count, 1)
    XCTAssertEqual(tokensAtBoundary.first?.type, "EOF")

    // Exact boundary + 1: 1,048,577 bytes is rejected
    let oversized = String(repeating: " ", count: JavaLexerConstants.maxSourceLength + 1)
    XCTAssertThrowsError(try tokenizeJava(oversized)) { error in
      guard let lexerError = error as? JavaLexerError else {
        XCTFail("Expected JavaLexerError, got \(error)")
        return
      }
      XCTAssertEqual(lexerError.code, .invalidSource)
      XCTAssertEqual(lexerError.errorId, "invalid-source")
      XCTAssertEqual(lexerError.message, JavaLexerMessages.messageSourceTooLarge)
    }
  }

  func testClass2BacktrackingImmunityOnPathologicalInputs() throws {
    // Pathological huge numeric literal with underscores / digits
    let pathologicalNum = "1" + String(repeating: "0", count: 500) + "L;"
    let tokens = try tokenizeJava(pathologicalNum)
    XCTAssertFalse(tokens.isEmpty)
    XCTAssertEqual(tokens.first?.type, "NUMBER")

    // Pathological comment blocks
    let pathologicalComment = "/* " + String(repeating: "* ", count: 1000) + "*/ int z = 1;"
    let commentTokens = try tokenizeJava(pathologicalComment)
    XCTAssertFalse(commentTokens.isEmpty)
    XCTAssertTrue(commentTokens.contains { $0.value == "int" })
  }

  func testClass3RefusalOfInvalidCharacters() {
    // Unrecognized character '#' throws payload-blind lexical error
    XCTAssertThrowsError(try tokenizeJava("int x = #;")) { error in
      guard let lexerError = error as? JavaLexerError else {
        XCTFail("Expected JavaLexerError, got \(error)")
        return
      }
      XCTAssertEqual(lexerError.code, .lexicalError)
      XCTAssertEqual(lexerError.errorId, "lexical-error")
      XCTAssertFalse(lexerError.message.contains("#"))
    }
  }

  func testClass4EditionDowngradeKeywordConfusion() throws {
    // Unknown version throws invalidVersion
    XCTAssertThrowsError(try tokenizeJava("class B {}", version: "invalid-99")) { error in
      guard let lexerError = error as? JavaLexerError else {
        XCTFail("Expected JavaLexerError, got \(error)")
        return
      }
      XCTAssertEqual(lexerError.code, .invalidVersion)
      XCTAssertEqual(lexerError.errorId, "invalid-version")
    }

    // 'yield' is a keyword in 14, identifier in 10
    let t14Yield = try tokenizeJava("yield 42;", version: "14")
    XCTAssertTrue(t14Yield.contains { $0.type == "KEYWORD" && $0.value == "yield" })

    let t10Yield = try tokenizeJava("int yield = 42;", version: "10")
    XCTAssertTrue(t10Yield.contains { $0.type == "NAME" && $0.value == "yield" })
  }

  func testClass5PayloadBlindSecurityAndZeroSensitiveLeakage() {
    let secretPayload = "super_secret_pin_987654"
    XCTAssertThrowsError(try tokenizeJava("int \(secretPayload) = ###;", version: "21")) { error in
      guard let lexerError = error as? JavaLexerError else {
        XCTFail("Expected JavaLexerError, got \(error)")
        return
      }
      XCTAssertFalse(lexerError.message.contains(secretPayload))
      XCTAssertFalse(lexerError.description.contains(secretPayload))
      XCTAssertEqual(lexerError.errorId, "lexical-error")
    }

    let sensitiveVersion = "v_confidential_999"
    XCTAssertThrowsError(try tokenizeJava("int a = 1;", version: sensitiveVersion)) { error in
      guard let lexerError = error as? JavaLexerError else {
        XCTFail("Expected JavaLexerError, got \(error)")
        return
      }
      XCTAssertFalse(lexerError.message.contains(sensitiveVersion))
      XCTAssertFalse(lexerError.description.contains(sensitiveVersion))
      XCTAssertEqual(lexerError.errorId, "invalid-version")
    }
  }

  func testNonAsciiCeilingCountsUtf16CodeUnits() throws {
    // "é" is 1 UTF-16 code unit but 2 UTF-8 bytes. A line comment of
    // (maxSourceLength - 2) "é" characters is ~2 MB of UTF-8 yet exactly
    // maxSourceLength UTF-16 code units, so it must be accepted (Dart parity).
    let prefix = "//"
    let atBoundary =
      prefix
      + String(repeating: "é", count: JavaLexerConstants.maxSourceLength - prefix.utf16.count)
    XCTAssertEqual(atBoundary.utf16.count, JavaLexerConstants.maxSourceLength)
    XCTAssertNoThrow(try JavaLexer(source: atBoundary))

    let overBoundary = atBoundary + "é"
    XCTAssertThrowsError(try JavaLexer(source: overBoundary)) { error in
      guard let lexerError = error as? JavaLexerError else {
        XCTFail("Expected JavaLexerError, got \(error)")
        return
      }
      XCTAssertEqual(lexerError.code, .invalidSource)
      XCTAssertEqual(lexerError.errorId, "invalid-source")
    }
  }

  func testValidationPrecedenceSourceOverVersion() {
    let oversized = String(repeating: " ", count: JavaLexerConstants.maxSourceLength + 1)
    // Source error takes precedence over invalid version error
    XCTAssertThrowsError(try tokenizeJava(oversized, version: "invalid-version-string")) { error in
      guard let lexerError = error as? JavaLexerError else {
        XCTFail("Expected JavaLexerError, got \(error)")
        return
      }
      XCTAssertEqual(lexerError.code, .invalidSource)
    }
  }

  func testVersionAliasesAndCanonicalProperty() throws {
    let lexerV10 = try createJavaLexer("int x = 1;", version: "java1_0")
    XCTAssertEqual(lexerV10.version, "1.0")

    let lexerV14 = try createJavaLexer("int x = 1;", version: "java1.4")
    XCTAssertEqual(lexerV14.version, "1.4")

    let lexer8 = try createJavaLexer("int x = 1;", version: "java8")
    XCTAssertEqual(lexer8.version, "8")

    let lexer8Alias = try createJavaLexer("int x = 1;", version: "1.8")
    XCTAssertEqual(lexer8Alias.version, "8")

    let lexer21 = try createJavaLexer("int x = 1;", version: "java21")
    XCTAssertEqual(lexer21.version, "21")

    let lexerDefault = try createJavaLexer("int x = 1;")
    XCTAssertEqual(lexerDefault.version, "21")
  }

  func testErrorFormattingAndMessages() {
    let errWithLoc = JavaLexerError(code: .lexicalError, line: 3, column: 15)
    XCTAssertEqual(errWithLoc.line, 3)
    XCTAssertEqual(errWithLoc.column, 15)
    XCTAssertTrue(errWithLoc.description.contains("(3:15)"))

    let errWithoutLoc = JavaLexerError(code: .invalidVersion)
    XCTAssertNil(errWithoutLoc.line)
    XCTAssertNil(errWithoutLoc.column)
    XCTAssertFalse(errWithoutLoc.description.contains("("))

    XCTAssertEqual(JavaLexerMessages.redactedPlaceholder, "<redacted>")
    XCTAssertEqual(JavaLexerMessages.invalidPlaceholder, "<invalid>")
    XCTAssertEqual(JavaLexerErrorCode.invalidVersion.errorId, "invalid-version")
    XCTAssertEqual(JavaLexerErrorCode.invalidSource.errorId, "invalid-source")
    XCTAssertEqual(JavaLexerErrorCode.lexicalError.errorId, "lexical-error")
  }

  func testConvenienceHelpersAndDirectGrammarAccess() throws {
    let grammar8 = try loadJavaTokenGrammar(version: "8")
    XCTAssertFalse(grammar8.definitions.isEmpty)

    let lexer = try createJavaLexer("int val = 9;", version: "8")
    let tokens = try lexer.tokenize()
    XCTAssertFalse(tokens.isEmpty)
    XCTAssertEqual(tokens.first?.type, "KEYWORD")
    XCTAssertEqual(tokens.first?.value, "int")
  }

  func testPRNGRoundTripProperty() throws {
    var rng = SimplePRNG(seed: 0xCAFE_BABE)
    let keywords = ["public", "class", "static", "void", "int", "final"]
    let identifiers = ["foo", "bar", "baz", "qux", "alpha", "beta"]
    let numbers = ["0", "42", "1000", "0xFF", "314"]

    for _ in 0..<200 {
      let tokenCount = 3 + rng.nextInt(upperBound: 10)
      var parts: [String] = []
      for _ in 0..<tokenCount {
        let choice = rng.nextInt(upperBound: 3)
        if choice == 0 {
          parts.append(keywords[rng.nextInt(upperBound: keywords.count)])
        } else if choice == 1 {
          parts.append(identifiers[rng.nextInt(upperBound: identifiers.count)])
        } else {
          parts.append(numbers[rng.nextInt(upperBound: numbers.count)])
        }
      }
      let source = parts.joined(separator: " ")
      let tokens = try tokenizeJava(source, version: "21")
      let nonEof = tokens.filter { $0.type != "EOF" }
      XCTAssertEqual(nonEof.count, parts.count, "Token count mismatch for source: \(source)")
      for (i, part) in parts.enumerated() {
        XCTAssertEqual(nonEof[i].value, part)
      }
    }
  }

  func testTenThousandIterationMutationFuzzLoop() throws {
    var rng = SimplePRNG(seed: 0x1337_D00D)
    let seedTokens = [
      "public", "class", "Test", "{", "int", "x", "=", "42", ";",
      "assert", "true", ";", "}",
    ]

    for _ in 0..<10_000 {
      let numTokens = 1 + rng.nextInt(upperBound: 8)
      var tokensText: [String] = []
      for _ in 0..<numTokens {
        var token = seedTokens[rng.nextInt(upperBound: seedTokens.count)]
        if rng.nextInt(upperBound: 100) < 25 {
          let mutationType = rng.nextInt(upperBound: 3)
          if mutationType == 0 {
            // Insert '#'
            let insertPos = rng.nextInt(upperBound: token.count + 1)
            let idx = token.index(token.startIndex, offsetBy: insertPos)
            token.insert("#", at: idx)
          } else if mutationType == 1 {
            // Mutate char
            if !token.isEmpty {
              let mutPos = rng.nextInt(upperBound: token.count)
              let idx = token.index(token.startIndex, offsetBy: mutPos)
              token.remove(at: idx)
              token.insert("@", at: idx)
            }
          } else {
            // Truncate char
            if token.count > 1 {
              let delPos = rng.nextInt(upperBound: token.count)
              let idx = token.index(token.startIndex, offsetBy: delPos)
              token.remove(at: idx)
            }
          }
        }
        tokensText.append(token)
      }

      let source = tokensText.joined(separator: " ")
      do {
        _ = try tokenizeJava(source)
      } catch let error as JavaLexerError {
        XCTAssertFalse(error.errorId.isEmpty)
      } catch {
        XCTFail("Unexpected non-JavaLexerError thrown: \(error)")
      }
    }
  }
}

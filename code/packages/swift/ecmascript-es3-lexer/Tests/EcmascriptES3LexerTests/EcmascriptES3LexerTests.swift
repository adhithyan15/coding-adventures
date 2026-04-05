// ============================================================================
// EcmascriptES3LexerTests.swift -- Tests for ECMAScript 3 (1999) Lexer
// ============================================================================

import XCTest
@testable import EcmascriptES3Lexer
import Lexer

final class EcmascriptES3LexerTests: XCTestCase {

    func tokenTypes(_ source: String) throws -> [String] {
        let tokens = try EcmascriptES3Lexer.tokenize(source)
        return tokens.filter { $0.type != "EOF" }.map(\.type)
    }

    // Module surface
    func testVersion() { XCTAssertFalse(EcmascriptES3Lexer.version.isEmpty) }

    func testLoadGrammar() throws {
        let grammar = try EcmascriptES3Lexer.loadGrammar()
        XCTAssertFalse(grammar.definitions.isEmpty)
    }

    // Empty
    func testEmptyStringProducesOnlyEOF() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("")
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0].type, "EOF")
    }

    func testWhitespaceOnly() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("   \t\n  ")
        XCTAssertEqual(tokens.count, 1)
    }

    // ES1 keywords retained
    func testKeywordVar() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("var")
        XCTAssertEqual(tokens[0].type, "VAR")
    }

    func testKeywordFunction() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("function")
        XCTAssertEqual(tokens[0].type, "FUNCTION")
    }

    func testKeywordTrueFalseNull() throws {
        let types = try tokenTypes("true false null")
        XCTAssertEqual(types, ["TRUE", "FALSE", "NULL"])
    }

    // New ES3 keywords
    func testKeywordTry() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("try")
        XCTAssertEqual(tokens[0].type, "TRY")
    }

    func testKeywordCatch() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("catch")
        XCTAssertEqual(tokens[0].type, "CATCH")
    }

    func testKeywordFinally() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("finally")
        XCTAssertEqual(tokens[0].type, "FINALLY")
    }

    func testKeywordThrow() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("throw")
        XCTAssertEqual(tokens[0].type, "THROW")
    }

    func testKeywordInstanceof() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("instanceof")
        XCTAssertEqual(tokens[0].type, "INSTANCEOF")
    }

    // Strict equality (new in ES3)
    func testStrictEquals() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("===")
        XCTAssertEqual(tokens[0].type, "STRICT_EQUALS")
    }

    func testStrictNotEquals() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("!==")
        XCTAssertEqual(tokens[0].type, "STRICT_NOT_EQUALS")
    }

    func testLooseEquals() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("==")
        XCTAssertEqual(tokens[0].type, "EQUALS_EQUALS")
    }

    // Identifiers and literals
    func testIdentifier() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("myVar")
        XCTAssertEqual(tokens[0].type, "NAME")
    }

    func testNumber() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("42")
        XCTAssertEqual(tokens[0].type, "NUMBER")
    }

    func testString() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("\"hello\"")
        XCTAssertEqual(tokens[0].type, "STRING")
    }

    // Operators
    func testUnsignedRightShift() throws {
        let tokens = try EcmascriptES3Lexer.tokenize(">>>")
        XCTAssertEqual(tokens[0].type, "UNSIGNED_RIGHT_SHIFT")
    }

    // Composite
    func testVarDeclaration() throws {
        let types = try tokenTypes("var x = 1;")
        XCTAssertEqual(types, ["VAR", "NAME", "EQUALS", "NUMBER", "SEMICOLON"])
    }

    func testInstanceofExpression() throws {
        let types = try tokenTypes("x instanceof Foo")
        XCTAssertEqual(types, ["NAME", "INSTANCEOF", "NAME"])
    }

    func testStrictEquality() throws {
        let types = try tokenTypes("a === b")
        XCTAssertEqual(types, ["NAME", "STRICT_EQUALS", "NAME"])
    }

    // Position
    func testColumnTracking() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("var x = 1;")
        XCTAssertEqual(tokens[0].column, 1)
        XCTAssertEqual(tokens[1].column, 5)
    }

    // EOF
    func testEOFIsLast() throws {
        let tokens = try EcmascriptES3Lexer.tokenize("1")
        XCTAssertEqual(tokens.last?.type, "EOF")
    }

    // Error
    func testUnexpectedCharacterThrows() throws {
        XCTAssertThrowsError(try EcmascriptES3Lexer.tokenize("#"))
    }
}

// ============================================================================
// EcmascriptES5LexerTests.swift -- Tests for ECMAScript 5 (2009) Lexer
// ============================================================================

import XCTest
@testable import EcmascriptES5Lexer
import Lexer

final class EcmascriptES5LexerTests: XCTestCase {

    func tokenTypes(_ source: String) throws -> [String] {
        let tokens = try EcmascriptES5Lexer.tokenize(source)
        return tokens.filter { $0.type != "EOF" }.map(\.type)
    }

    // Module surface
    func testVersion() { XCTAssertFalse(EcmascriptES5Lexer.version.isEmpty) }

    func testLoadGrammar() throws {
        let grammar = try EcmascriptES5Lexer.loadGrammar()
        XCTAssertFalse(grammar.definitions.isEmpty)
    }

    // Empty
    func testEmptyStringProducesOnlyEOF() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("")
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0].type, "EOF")
    }

    // ES5-specific: debugger keyword
    func testKeywordDebugger() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("debugger")
        XCTAssertEqual(tokens[0].type, "DEBUGGER")
        XCTAssertEqual(tokens[0].value, "debugger")
    }

    func testDebuggerStatement() throws {
        let types = try tokenTypes("debugger;")
        XCTAssertEqual(types, ["DEBUGGER", "SEMICOLON"])
    }

    // ES3 keywords retained
    func testKeywordVar() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("var")
        XCTAssertEqual(tokens[0].type, "VAR")
    }

    func testKeywordTry() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("try")
        XCTAssertEqual(tokens[0].type, "TRY")
    }

    func testKeywordCatch() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("catch")
        XCTAssertEqual(tokens[0].type, "CATCH")
    }

    func testKeywordInstanceof() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("instanceof")
        XCTAssertEqual(tokens[0].type, "INSTANCEOF")
    }

    func testKeywordTrueFalseNull() throws {
        let types = try tokenTypes("true false null")
        XCTAssertEqual(types, ["TRUE", "FALSE", "NULL"])
    }

    // Strict equality
    func testStrictEquals() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("===")
        XCTAssertEqual(tokens[0].type, "STRICT_EQUALS")
    }

    func testStrictNotEquals() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("!==")
        XCTAssertEqual(tokens[0].type, "STRICT_NOT_EQUALS")
    }

    // Identifiers and literals
    func testIdentifier() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("myVar")
        XCTAssertEqual(tokens[0].type, "NAME")
    }

    func testNumber() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("42")
        XCTAssertEqual(tokens[0].type, "NUMBER")
    }

    func testString() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("\"hello\"")
        XCTAssertEqual(tokens[0].type, "STRING")
    }

    // Composite
    func testVarDeclaration() throws {
        let types = try tokenTypes("var x = 1;")
        XCTAssertEqual(types, ["VAR", "NAME", "EQUALS", "NUMBER", "SEMICOLON"])
    }

    func testStrictEquality() throws {
        let types = try tokenTypes("a === b")
        XCTAssertEqual(types, ["NAME", "STRICT_EQUALS", "NAME"])
    }

    // Position
    func testColumnTracking() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("var x = 1;")
        XCTAssertEqual(tokens[0].column, 1)
        XCTAssertEqual(tokens[1].column, 5)
    }

    // EOF
    func testEOFIsLast() throws {
        let tokens = try EcmascriptES5Lexer.tokenize("1")
        XCTAssertEqual(tokens.last?.type, "EOF")
    }

    // Error
    func testUnexpectedCharacterThrows() throws {
        XCTAssertThrowsError(try EcmascriptES5Lexer.tokenize("#"))
    }
}

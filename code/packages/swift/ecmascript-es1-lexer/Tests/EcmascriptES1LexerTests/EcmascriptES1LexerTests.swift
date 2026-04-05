// ============================================================================
// EcmascriptES1LexerTests.swift -- Tests for ECMAScript 1 (1997) Lexer
// ============================================================================

import XCTest
@testable import EcmascriptES1Lexer
import Lexer

final class EcmascriptES1LexerTests: XCTestCase {

    // ---- Helpers ----

    /// Tokenize and return only non-EOF token types.
    func tokenTypes(_ source: String) throws -> [String] {
        let tokens = try EcmascriptES1Lexer.tokenize(source)
        return tokens.filter { $0.type != "EOF" }.map(\.type)
    }

    // =========================================================================
    // Module surface
    // =========================================================================

    func testVersion() {
        XCTAssertFalse(EcmascriptES1Lexer.version.isEmpty)
    }

    func testLoadGrammar() throws {
        let grammar = try EcmascriptES1Lexer.loadGrammar()
        XCTAssertFalse(grammar.definitions.isEmpty)
    }

    // =========================================================================
    // Empty and trivial inputs
    // =========================================================================

    func testEmptyStringProducesOnlyEOF() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("")
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0].type, "EOF")
    }

    func testWhitespaceOnlyProducesOnlyEOF() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("   \t\r\n  ")
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0].type, "EOF")
    }

    func testLineCommentConsumedSilently() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("// comment")
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0].type, "EOF")
    }

    func testBlockCommentConsumedSilently() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("/* block */")
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0].type, "EOF")
    }

    // =========================================================================
    // Keywords
    // =========================================================================

    func testKeywordVar() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("var")
        XCTAssertEqual(tokens[0].type, "VAR")
        XCTAssertEqual(tokens[0].value, "var")
    }

    func testKeywordFunction() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("function")
        XCTAssertEqual(tokens[0].type, "FUNCTION")
    }

    func testKeywordReturn() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("return")
        XCTAssertEqual(tokens[0].type, "RETURN")
    }

    func testKeywordIfElse() throws {
        let types = try tokenTypes("if else")
        XCTAssertEqual(types, ["IF", "ELSE"])
    }

    func testKeywordForWhile() throws {
        let types = try tokenTypes("for while")
        XCTAssertEqual(types, ["FOR", "WHILE"])
    }

    func testKeywordBreak() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("break")
        XCTAssertEqual(tokens[0].type, "BREAK")
    }

    func testKeywordSwitch() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("switch")
        XCTAssertEqual(tokens[0].type, "SWITCH")
    }

    func testKeywordNew() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("new")
        XCTAssertEqual(tokens[0].type, "NEW")
    }

    func testKeywordTypeof() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("typeof")
        XCTAssertEqual(tokens[0].type, "TYPEOF")
    }

    func testKeywordThis() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("this")
        XCTAssertEqual(tokens[0].type, "THIS")
    }

    func testKeywordTrueFalseNull() throws {
        let types = try tokenTypes("true false null")
        XCTAssertEqual(types, ["TRUE", "FALSE", "NULL"])
    }

    func testKeywordDelete() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("delete")
        XCTAssertEqual(tokens[0].type, "DELETE")
    }

    func testKeywordVoid() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("void")
        XCTAssertEqual(tokens[0].type, "VOID")
    }

    // =========================================================================
    // Identifiers
    // =========================================================================

    func testSimpleIdentifier() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("myVar")
        XCTAssertEqual(tokens[0].type, "NAME")
        XCTAssertEqual(tokens[0].value, "myVar")
    }

    func testDollarPrefixedIdentifier() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("$el")
        XCTAssertEqual(tokens[0].type, "NAME")
    }

    // =========================================================================
    // Numbers
    // =========================================================================

    func testInteger() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("42")
        XCTAssertEqual(tokens[0].type, "NUMBER")
        XCTAssertEqual(tokens[0].value, "42")
    }

    func testHexNumber() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("0xFF")
        XCTAssertEqual(tokens[0].type, "NUMBER")
    }

    func testFloat() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("3.14")
        XCTAssertEqual(tokens[0].type, "NUMBER")
    }

    // =========================================================================
    // Strings
    // =========================================================================

    func testDoubleQuotedString() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("\"hello\"")
        XCTAssertEqual(tokens[0].type, "STRING")
    }

    func testSingleQuotedString() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("'world'")
        XCTAssertEqual(tokens[0].type, "STRING")
    }

    // =========================================================================
    // Operators
    // =========================================================================

    func testEqualsEquals() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("==")
        XCTAssertEqual(tokens[0].type, "EQUALS_EQUALS")
    }

    func testNotEquals() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("!=")
        XCTAssertEqual(tokens[0].type, "NOT_EQUALS")
    }

    func testLogicalAnd() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("&&")
        XCTAssertEqual(tokens[0].type, "AND_AND")
    }

    func testUnsignedRightShift() throws {
        let tokens = try EcmascriptES1Lexer.tokenize(">>>")
        XCTAssertEqual(tokens[0].type, "UNSIGNED_RIGHT_SHIFT")
    }

    func testPlus() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("+")
        XCTAssertEqual(tokens[0].type, "PLUS")
    }

    func testEquals() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("=")
        XCTAssertEqual(tokens[0].type, "EQUALS")
    }

    // =========================================================================
    // Delimiters
    // =========================================================================

    func testParentheses() throws {
        let types = try tokenTypes("()")
        XCTAssertEqual(types, ["LPAREN", "RPAREN"])
    }

    func testBraces() throws {
        let types = try tokenTypes("{}")
        XCTAssertEqual(types, ["LBRACE", "RBRACE"])
    }

    func testSemicolon() throws {
        let tokens = try EcmascriptES1Lexer.tokenize(";")
        XCTAssertEqual(tokens[0].type, "SEMICOLON")
    }

    // =========================================================================
    // Composite expressions
    // =========================================================================

    func testVarDeclaration() throws {
        let types = try tokenTypes("var x = 1;")
        XCTAssertEqual(types, ["VAR", "NAME", "EQUALS", "NUMBER", "SEMICOLON"])
    }

    func testFunctionDeclaration() throws {
        let types = try tokenTypes("function add(a, b) { return a + b; }")
        XCTAssertEqual(types[0], "FUNCTION")
        XCTAssertEqual(types.last, "RBRACE")
    }

    // =========================================================================
    // Position tracking
    // =========================================================================

    func testColumnTracking() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("var x = 1;")
        XCTAssertEqual(tokens[0].column, 1)  // var
        XCTAssertEqual(tokens[1].column, 5)  // x
    }

    func testAllTokensOnLine1() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("var x = 1;")
        for token in tokens {
            XCTAssertEqual(token.line, 1)
        }
    }

    // =========================================================================
    // EOF
    // =========================================================================

    func testEOFIsAlwaysLast() throws {
        let tokens = try EcmascriptES1Lexer.tokenize("1")
        XCTAssertEqual(tokens.last?.type, "EOF")
    }

    // =========================================================================
    // Error handling
    // =========================================================================

    func testUnexpectedCharacterThrows() throws {
        XCTAssertThrowsError(try EcmascriptES1Lexer.tokenize("#"))
    }
}

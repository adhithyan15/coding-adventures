// JsonLexerTests.swift
// ============================================================================
// Unit tests for the JsonLexer tokenizer.
// ============================================================================
//
// We test every token type, all string escape sequences, all number formats,
// whitespace handling, and error cases. The goal is exhaustive coverage of
// the RFC 8259 lexical grammar.
// ============================================================================
import Testing
@testable import JsonLexer

// Helper to extract just the token kinds from a token array
private func kinds(_ tokens: [Token]) -> [TokenKind] {
    tokens.map { $0.kind }
}

// ============================================================================
// Structural punctuation
// ============================================================================
@Suite("Structural tokens")
struct StructuralTests {

    @Test("All structural characters")
    func testAllStructural() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("{}[]:,")
        #expect(kinds(tokens) == [
            .leftBrace, .rightBrace,
            .leftBracket, .rightBracket,
            .colon, .comma,
        ])
    }

    @Test("Token offsets are correct")
    func testOffsets() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("{}")
        #expect(tokens[0].offset == 0)  // {
        #expect(tokens[1].offset == 1)  // }
    }

    @Test("Empty input produces no tokens")
    func testEmptyInput() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("")
        #expect(tokens.isEmpty)
    }

    @Test("Whitespace-only input produces no tokens")
    func testWhitespaceOnly() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("   \t\n\r  ")
        #expect(tokens.isEmpty)
    }
}

// ============================================================================
// Keyword literals: true, false, null
// ============================================================================
@Suite("Keyword literals")
struct KeywordTests {

    @Test("true keyword")
    func testTrue() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("true")
        #expect(kinds(tokens) == [.trueLit])
    }

    @Test("false keyword")
    func testFalse() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("false")
        #expect(kinds(tokens) == [.falseLit])
    }

    @Test("null keyword")
    func testNull() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("null")
        #expect(kinds(tokens) == [.nullLit])
    }

    @Test("All three keywords in sequence")
    func testAllKeywords() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("true false null")
        #expect(kinds(tokens) == [.trueLit, .falseLit, .nullLit])
    }

    @Test("Invalid true keyword throws")
    func testInvalidTrue() throws {
        let lexer = JsonLexer()
        #expect(throws: (any Error).self) {
            try lexer.tokenize("tru")
        }
    }

    @Test("Invalid false keyword throws")
    func testInvalidFalse() throws {
        let lexer = JsonLexer()
        #expect(throws: (any Error).self) {
            try lexer.tokenize("fals")
        }
    }

    @Test("Invalid null keyword throws")
    func testInvalidNull() throws {
        let lexer = JsonLexer()
        #expect(throws: (any Error).self) {
            try lexer.tokenize("nul")
        }
    }
}

// ============================================================================
// String literals
// ============================================================================
@Suite("String literals")
struct StringTests {

    @Test("Simple string")
    func testSimpleString() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"hello\"")
        #expect(kinds(tokens) == [.stringLit("hello")])
    }

    @Test("Empty string")
    func testEmptyString() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"\"")
        #expect(kinds(tokens) == [.stringLit("")])
    }

    @Test("String with escaped quote")
    func testEscapedQuote() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"say \\\"hello\\\"\"")
        #expect(kinds(tokens) == [.stringLit("say \"hello\"")])
    }

    @Test("String with escaped backslash")
    func testEscapedBackslash() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"a\\\\b\"")
        #expect(kinds(tokens) == [.stringLit("a\\b")])
    }

    @Test("String with escaped solidus")
    func testEscapedSolidus() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"\\/\"")
        #expect(kinds(tokens) == [.stringLit("/")])
    }

    @Test("String with \\b (backspace)")
    func testEscapedBackspace() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"\\b\"")
        #expect(kinds(tokens) == [.stringLit("\u{08}")])
    }

    @Test("String with \\f (form feed)")
    func testEscapedFormFeed() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"\\f\"")
        #expect(kinds(tokens) == [.stringLit("\u{0C}")])
    }

    @Test("String with \\n (newline)")
    func testEscapedNewline() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"line1\\nline2\"")
        #expect(kinds(tokens) == [.stringLit("line1\nline2")])
    }

    @Test("String with \\r (carriage return)")
    func testEscapedCarriageReturn() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"\\r\"")
        #expect(kinds(tokens) == [.stringLit("\r")])
    }

    @Test("String with \\t (tab)")
    func testEscapedTab() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"col1\\tcol2\"")
        #expect(kinds(tokens) == [.stringLit("col1\tcol2")])
    }

    @Test("String with \\uXXXX Unicode escape")
    func testUnicodeEscape() throws {
        let lexer = JsonLexer()
        // \u0041 is 'A', \u03B1 is 'α' (Greek small letter alpha)
        let tokens = try lexer.tokenize("\"\\u0041\\u03B1\"")
        #expect(kinds(tokens) == [.stringLit("Aα")])
    }

    @Test("String with surrogate pair (emoji)")
    func testSurrogatePair() throws {
        let lexer = JsonLexer()
        // U+1F600 (😀) encoded as surrogate pair \uD83D\uDE00
        let tokens = try lexer.tokenize("\"\\uD83D\\uDE00\"")
        #expect(kinds(tokens) == [.stringLit("😀")])
    }

    @Test("Unicode BMP character A")
    func testUnicodeBMP() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("\"\\u0041\"")
        if case .stringLit(let s) = tokens[0].kind {
            #expect(s == "A")
        }
    }

    @Test("Unterminated string throws")
    func testUnterminatedString() throws {
        let lexer = JsonLexer()
        #expect(throws: (any Error).self) {
            try lexer.tokenize("\"hello")
        }
    }

    @Test("Invalid escape sequence throws")
    func testInvalidEscape() throws {
        let lexer = JsonLexer()
        #expect(throws: (any Error).self) {
            try lexer.tokenize("\"\\q\"")
        }
    }

    @Test("Raw control character in string throws")
    func testRawControlChar() throws {
        let lexer = JsonLexer()
        // A raw newline inside a string is not valid JSON
        #expect(throws: (any Error).self) {
            try lexer.tokenize("\"line1\nline2\"")
        }
    }
}

// ============================================================================
// Number literals
// ============================================================================
@Suite("Number literals")
struct NumberTests {

    @Test("Integer zero")
    func testZero() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("0")
        #expect(kinds(tokens) == [.numberLit(0)])
    }

    @Test("Positive integer")
    func testPositiveInt() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("42")
        #expect(kinds(tokens) == [.numberLit(42)])
    }

    @Test("Negative integer")
    func testNegativeInt() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("-7")
        #expect(kinds(tokens) == [.numberLit(-7)])
    }

    @Test("Floating point")
    func testFloat() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("3.14")
        if case .numberLit(let n) = tokens[0].kind {
            #expect(abs(n - 3.14) < 1e-10)
        }
    }

    @Test("Negative float")
    func testNegativeFloat() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("-0.5")
        if case .numberLit(let n) = tokens[0].kind {
            #expect(n == -0.5)
        }
    }

    @Test("Scientific notation — e")
    func testScientificLowerE() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("1e3")
        #expect(kinds(tokens) == [.numberLit(1000)])
    }

    @Test("Scientific notation — E")
    func testScientificUpperE() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("2.5E2")
        #expect(kinds(tokens) == [.numberLit(250)])
    }

    @Test("Scientific notation with explicit positive exponent")
    func testScientificPlusExp() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("1e+2")
        #expect(kinds(tokens) == [.numberLit(100)])
    }

    @Test("Scientific notation with negative exponent")
    func testScientificNegExp() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("1e-2")
        if case .numberLit(let n) = tokens[0].kind {
            #expect(abs(n - 0.01) < 1e-15)
        }
    }

    @Test("Leading zeros are forbidden")
    func testLeadingZeroForbidden() throws {
        let lexer = JsonLexer()
        #expect(throws: (any Error).self) {
            try lexer.tokenize("007")
        }
    }

    @Test("Trailing decimal point is invalid")
    func testTrailingDecimalForbidden() throws {
        let lexer = JsonLexer()
        #expect(throws: (any Error).self) {
            try lexer.tokenize("1.")
        }
    }

    @Test("Bare minus is invalid")
    func testBareMinus() throws {
        let lexer = JsonLexer()
        #expect(throws: (any Error).self) {
            try lexer.tokenize("-")
        }
    }
}

// ============================================================================
// Complex inputs
// ============================================================================
@Suite("Complex inputs")
struct ComplexTests {

    @Test("Simple object")
    func testSimpleObject() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("{\"a\":1}")
        #expect(kinds(tokens) == [
            .leftBrace,
            .stringLit("a"),
            .colon,
            .numberLit(1),
            .rightBrace,
        ])
    }

    @Test("Simple array")
    func testSimpleArray() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("[1,2,3]")
        #expect(kinds(tokens) == [
            .leftBracket,
            .numberLit(1), .comma,
            .numberLit(2), .comma,
            .numberLit(3),
            .rightBracket,
        ])
    }

    @Test("Nested structure")
    func testNested() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("{\"x\":[true,null]}")
        #expect(kinds(tokens) == [
            .leftBrace,
            .stringLit("x"),
            .colon,
            .leftBracket,
            .trueLit, .comma, .nullLit,
            .rightBracket,
            .rightBrace,
        ])
    }

    @Test("Whitespace is ignored between tokens")
    func testWhitespaceSeparated() throws {
        let lexer = JsonLexer()
        let tokens = try lexer.tokenize("  {  \"k\"  :  42  }  ")
        #expect(kinds(tokens) == [
            .leftBrace,
            .stringLit("k"),
            .colon,
            .numberLit(42),
            .rightBrace,
        ])
    }

    @Test("Unexpected character throws")
    func testUnexpectedChar() throws {
        let lexer = JsonLexer()
        #expect(throws: (any Error).self) {
            try lexer.tokenize("@bad")
        }
    }
}

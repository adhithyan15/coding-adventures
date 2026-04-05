import XCTest
@testable import MosaicLexer

// ============================================================================
// MosaicLexerTests
// ============================================================================
//
// Tests cover:
//   - Keywords (COMPONENT, SLOT, WHEN, EACH, etc.)
//   - Type keywords (KEYWORD: text, number, bool, ...)
//   - Identifiers with hyphens
//   - Hex colors (#rgb, #rrggbb, #rrggbbaa)
//   - Dimensions (16dp, 50%, 1.5sp)
//   - Numbers (positive, negative, decimal)
//   - Strings
//   - All punctuation tokens
//   - Whitespace and comment skipping
//   - A full mini-component round-trip
//   - Error cases

final class MosaicLexerTests: XCTestCase {

    // -------------------------------------------------------------------------
    // Helper
    // -------------------------------------------------------------------------

    func lex(_ src: String) throws -> [Token] {
        try tokenize(src)
    }

    func types(_ src: String) throws -> [String] {
        try lex(src).map(\.type)
    }

    func values(_ src: String) throws -> [String] {
        try lex(src).map(\.value)
    }

    // -------------------------------------------------------------------------
    // 1. Structural keywords
    // -------------------------------------------------------------------------

    func testComponentKeyword() throws {
        let toks = try lex("component")
        XCTAssertEqual(toks.count, 1)
        XCTAssertEqual(toks[0].type, "COMPONENT")
        XCTAssertEqual(toks[0].value, "component")
    }

    func testSlotKeyword() throws {
        XCTAssertEqual(try types("slot"), ["SLOT"])
    }

    func testWhenKeyword() throws {
        XCTAssertEqual(try types("when"), ["WHEN"])
    }

    func testEachKeyword() throws {
        XCTAssertEqual(try types("each"), ["EACH"])
    }

    func testImportFromAs() throws {
        XCTAssertEqual(try types("import from as"), ["IMPORT", "FROM", "AS"])
    }

    func testTrueFalse() throws {
        XCTAssertEqual(try types("true false"), ["TRUE", "FALSE"])
    }

    // -------------------------------------------------------------------------
    // 2. Type keywords → KEYWORD
    // -------------------------------------------------------------------------

    func testTypeKeywords() throws {
        let src = "text number bool image color node list"
        let toks = try lex(src)
        XCTAssertTrue(toks.allSatisfy { $0.type == "KEYWORD" })
        XCTAssertEqual(toks.map(\.value), ["text", "number", "bool", "image", "color", "node", "list"])
    }

    // -------------------------------------------------------------------------
    // 3. NAME (identifiers with hyphens)
    // -------------------------------------------------------------------------

    func testSimpleName() throws {
        XCTAssertEqual(try types("ProfileCard"), ["NAME"])
        XCTAssertEqual(try values("ProfileCard"), ["ProfileCard"])
    }

    func testHyphenatedName() throws {
        let toks = try lex("corner-radius")
        XCTAssertEqual(toks.count, 1)
        XCTAssertEqual(toks[0].type, "NAME")
        XCTAssertEqual(toks[0].value, "corner-radius")
    }

    func testUnderscoreName() throws {
        XCTAssertEqual(try values("_private"), ["_private"])
    }

    // -------------------------------------------------------------------------
    // 4. Hex colors
    // -------------------------------------------------------------------------

    func testHexColor3() throws {
        let toks = try lex("#fff")
        XCTAssertEqual(toks.count, 1)
        XCTAssertEqual(toks[0].type, "HEX_COLOR")
        XCTAssertEqual(toks[0].value, "#fff")
    }

    func testHexColor6() throws {
        let toks = try lex("#2563eb")
        XCTAssertEqual(toks[0].type, "HEX_COLOR")
        XCTAssertEqual(toks[0].value, "#2563eb")
    }

    func testHexColor8() throws {
        let toks = try lex("#2563ebFF")
        XCTAssertEqual(toks[0].type, "HEX_COLOR")
        XCTAssertEqual(toks[0].value, "#2563ebFF")
    }

    // -------------------------------------------------------------------------
    // 5. Dimensions
    // -------------------------------------------------------------------------

    func testDimensionDp() throws {
        XCTAssertEqual(try types("16dp"), ["DIMENSION"])
        XCTAssertEqual(try values("16dp"), ["16dp"])
    }

    func testDimensionPercent() throws {
        XCTAssertEqual(try types("50%"), ["DIMENSION"])
        XCTAssertEqual(try values("50%"), ["50%"])
    }

    func testDimensionSp() throws {
        XCTAssertEqual(try values("1.5sp"), ["1.5sp"])
        XCTAssertEqual(try types("1.5sp"), ["DIMENSION"])
    }

    // -------------------------------------------------------------------------
    // 6. Numbers
    // -------------------------------------------------------------------------

    func testPositiveInt() throws {
        XCTAssertEqual(try types("42"), ["NUMBER"])
        XCTAssertEqual(try values("42"), ["42"])
    }

    func testNegativeNumber() throws {
        XCTAssertEqual(try types("-3.14"), ["NUMBER"])
    }

    func testDecimalNumber() throws {
        XCTAssertEqual(try values("0.5"), ["0.5"])
    }

    // -------------------------------------------------------------------------
    // 7. Strings
    // -------------------------------------------------------------------------

    func testStringLiteral() throws {
        let toks = try lex("\"hello world\"")
        XCTAssertEqual(toks.count, 1)
        XCTAssertEqual(toks[0].type, "STRING")
        XCTAssertEqual(toks[0].value, "\"hello world\"")
    }

    func testStringWithEscape() throws {
        let toks = try lex("\"say \\\"hi\\\"\"")
        XCTAssertEqual(toks[0].type, "STRING")
    }

    // -------------------------------------------------------------------------
    // 8. Punctuation
    // -------------------------------------------------------------------------

    func testPunctuation() throws {
        let toks = try lex("{ } : ; @ < > , . =")
        let expected = ["LBRACE","RBRACE","COLON","SEMICOLON","AT","LANGLE","RANGLE","COMMA","DOT","EQUALS"]
        XCTAssertEqual(toks.map(\.type), expected)
    }

    // -------------------------------------------------------------------------
    // 9. Whitespace and comments are skipped
    // -------------------------------------------------------------------------

    func testWhitespaceSkipped() throws {
        XCTAssertEqual(try types("  \t\n  "), [])
    }

    func testLineCommentSkipped() throws {
        let toks = try lex("// this is a comment\ncomponent")
        XCTAssertEqual(toks.count, 1)
        XCTAssertEqual(toks[0].type, "COMPONENT")
    }

    func testBlockCommentSkipped() throws {
        let toks = try lex("/* block */ slot")
        XCTAssertEqual(toks.count, 1)
        XCTAssertEqual(toks[0].type, "SLOT")
    }

    // -------------------------------------------------------------------------
    // 10. Source position tracking
    // -------------------------------------------------------------------------

    func testLineAndColumnTracking() throws {
        let src = "component\nCard"
        let toks = try lex(src)
        XCTAssertEqual(toks[0].line, 1)
        XCTAssertEqual(toks[0].column, 1)
        XCTAssertEqual(toks[1].line, 2)
        XCTAssertEqual(toks[1].column, 1)
    }

    // -------------------------------------------------------------------------
    // 11. Full mini-component
    // -------------------------------------------------------------------------

    func testMiniComponent() throws {
        let src = """
        component Label {
          slot text: text;
          Text { content: @text; }
        }
        """
        let toks = try lex(src)
        let typeSeq = toks.map(\.type)
        XCTAssertTrue(typeSeq.contains("COMPONENT"))
        XCTAssertTrue(typeSeq.contains("SLOT"))
        XCTAssertTrue(typeSeq.contains("AT"))
        XCTAssertTrue(typeSeq.contains("KEYWORD"))  // "text" type
    }

    // -------------------------------------------------------------------------
    // 12. @slot reference in property
    // -------------------------------------------------------------------------

    func testAtSlotRef() throws {
        let toks = try lex("@title")
        XCTAssertEqual(toks.map(\.type), ["AT", "NAME"])
        XCTAssertEqual(toks[1].value, "title")
    }

    // -------------------------------------------------------------------------
    // 13. list<text> type annotation
    // -------------------------------------------------------------------------

    func testListTypeTokens() throws {
        let toks = try lex("list<text>")
        XCTAssertEqual(toks.map(\.type), ["KEYWORD", "LANGLE", "KEYWORD", "RANGLE"])
    }

    // -------------------------------------------------------------------------
    // 14. Error: unexpected character
    // -------------------------------------------------------------------------

    func testUnexpectedCharacter() {
        XCTAssertThrowsError(try tokenize("component $bad")) { err in
            XCTAssertTrue(err is LexError)
        }
    }

    // -------------------------------------------------------------------------
    // 15. Error: unterminated string
    // -------------------------------------------------------------------------

    func testUnterminatedString() {
        XCTAssertThrowsError(try tokenize("\"open")) { err in
            XCTAssertTrue(err is LexError)
        }
    }
}

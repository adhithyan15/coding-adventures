import XCTest
@testable import XMLLexer
import Lexer

final class XMLLexerTests: XCTestCase {
    func testSimpleTags() throws {
        let tokens = try XMLLexer.tokenize("<p>Hello</p>")
        XCTAssertEqual(tokens.count, 8)
        // [OPEN_TAG_START, TAG_NAME, TAG_CLOSE, TEXT, CLOSE_TAG_START, TAG_NAME, TAG_CLOSE, EOF]
        XCTAssertEqual(tokens[0].type, "OPEN_TAG_START")
        XCTAssertEqual(tokens[1].type, "TAG_NAME")
        XCTAssertEqual(tokens[1].value, "p")
        XCTAssertEqual(tokens[2].type, "TAG_CLOSE")
        XCTAssertEqual(tokens[3].type, "TEXT")
        XCTAssertEqual(tokens[3].value, "Hello")
        XCTAssertEqual(tokens[4].type, "CLOSE_TAG_START")
        XCTAssertEqual(tokens[5].type, "TAG_NAME")
        XCTAssertEqual(tokens[5].value, "p")
        XCTAssertEqual(tokens[6].type, "TAG_CLOSE")
        XCTAssertEqual(tokens[7].type, "EOF")
    }

    func testAttributes() throws {
        let tokens = try XMLLexer.tokenize("<div class=\"main\"></div>")
        let types = tokens.map { $0.type }
        XCTAssertEqual(types, [
            "OPEN_TAG_START", "TAG_NAME", "TAG_NAME", "ATTR_EQUALS", "ATTR_VALUE", "TAG_CLOSE",
            "CLOSE_TAG_START", "TAG_NAME", "TAG_CLOSE", "EOF"
        ])
    }
    
    func testComment() throws {
        let tokens = try XMLLexer.tokenize("<!-- hello -->")
        let types = tokens.map { $0.type }
        XCTAssertEqual(types, [
            "COMMENT_START", "COMMENT_TEXT", "COMMENT_END", "EOF"
        ])
        XCTAssertEqual(tokens[1].value, " hello ") // Whitespace is not skipped
    }
    
    func testCDATA() throws {
        let tokens = try XMLLexer.tokenize("<![CDATA[ <not xml> ]]>")
        let types = tokens.map { $0.type }
        XCTAssertEqual(types, [
            "CDATA_START", "CDATA_TEXT", "CDATA_END", "EOF"
        ])
        XCTAssertEqual(tokens[1].value, " <not xml> ") // Whitespace is not skipped
    }
}

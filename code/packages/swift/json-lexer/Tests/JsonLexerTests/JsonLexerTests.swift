import XCTest
import Lexer
@testable import JsonLexer

final class JsonLexerTests: XCTestCase {
    
    func testTokenizeBasicObject() throws {
        let json = "{\"key\": 123}"
        let tokens = try tokenizeJson(json)
        for (i, token) in tokens.enumerated() {
            print("[\(i)] Type: \(token.type) Value: '\(token.value)'")
        }
        
        XCTAssertEqual(tokens.count, 6)
        XCTAssertEqual(tokens[0].type, "LBRACE")
        XCTAssertEqual(tokens[0].value, "{")
        
        XCTAssertEqual(tokens[1].type, "STRING")
        XCTAssertEqual(tokens[1].value, "key")
        
        XCTAssertEqual(tokens[2].type, "COLON")
        XCTAssertEqual(tokens[3].type, "NUMBER")
        XCTAssertEqual(tokens[3].value, "123")
        XCTAssertEqual(tokens[4].type, "RBRACE")
        XCTAssertEqual(tokens[5].type, "EOF")
    }

    func testTokenizeBooleansAndNull() throws {
        let json = "[true, false, null]"
        let tokens = try tokenizeJson(json)
        
        XCTAssertEqual(tokens.count, 8)
        XCTAssertEqual(tokens[0].type, "LBRACKET")
        XCTAssertEqual(tokens[1].type, "TRUE")
        XCTAssertEqual(tokens[2].type, "COMMA")
        XCTAssertEqual(tokens[3].type, "FALSE")
        XCTAssertEqual(tokens[4].type, "COMMA")
        XCTAssertEqual(tokens[5].type, "NULL")
        XCTAssertEqual(tokens[6].type, "RBRACKET")
        XCTAssertEqual(tokens[7].type, "EOF")
    }

    func testTokenizeComplexNumbers() throws {
        let json = "-123.456e-78"
        let tokens = try tokenizeJson(json)
        
        XCTAssertEqual(tokens.count, 2)
        XCTAssertEqual(tokens[0].type, "NUMBER")
        XCTAssertEqual(tokens[0].value, "-123.456e-78")
    }

    func testLexerErrorOnInvalidChar() throws {
        let json = "{\"key\": #}"
        XCTAssertThrowsError(try tokenizeJson(json)) { error in
            guard let lexError = error as? LexerError else {
                XCTFail("Expected LexerError")
                return
            }
            XCTAssertTrue(lexError.message.contains("Unexpected character"))
        }
    }
}

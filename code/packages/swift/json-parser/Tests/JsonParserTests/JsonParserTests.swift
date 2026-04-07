import XCTest
import Lexer
import Parser
import JsonLexer
@testable import JsonParser

final class JsonParserTests: XCTestCase {
    
    func testParseBasicObject() throws {
        let json = "{\"key\": 123}"
        let ast = try parseJson(json)
        
        XCTAssertEqual(ast.ruleName, "value")
        XCTAssertEqual(ast.children.count, 1)
        
        guard case let .node(objectNode) = ast.children[0] else { return XCTFail("Expected ASTNode") }
        XCTAssertEqual(objectNode.ruleName, "object")
        XCTAssertEqual(objectNode.children.count, 3) // LBRACE, pair, RBRACE
        
        guard case let .token(lbrace) = objectNode.children[0] else { return XCTFail("Expected Token") }
        XCTAssertEqual(lbrace.type, "LBRACE")
        XCTAssertEqual(lbrace.value, "{")
        
        guard case let .node(pairNode) = objectNode.children[1] else { return XCTFail("Expected ASTNode") }
        XCTAssertEqual(pairNode.ruleName, "pair")
        
        guard case let .token(rbrace) = objectNode.children[2] else { return XCTFail("Expected Token") }
        XCTAssertEqual(rbrace.type, "RBRACE")
        XCTAssertEqual(rbrace.value, "}")
    }

    func testParseArray() throws {
        let json = "[1, 2, 3]"
        let ast = try parseJson(json)
        
        XCTAssertEqual(ast.ruleName, "value")
        guard case let .node(arrayNode) = ast.children[0] else { return XCTFail("Expected ASTNode") }
        XCTAssertEqual(arrayNode.ruleName, "array")
        
        // LBRACKET, value, COMMA, value, COMMA, value, RBRACKET -> 7 children
        XCTAssertEqual(arrayNode.children.count, 7)
    }

    func testParserThrowsOnInvalidSyntax() {
        let json = "{\"key\": }"
        XCTAssertThrowsError(try parseJson(json)) { error in
            XCTAssertTrue(error is GrammarParseError || error is LexerError, "Should throw a parsing or lexing error")
        }
    }
}

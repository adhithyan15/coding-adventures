import XCTest
@testable import TOMLLexer
import Lexer

final class TOMLLexerTests: XCTestCase {
    func testSimpleAssignment() throws {
        let tokens = try TOMLLexer.tokenize("name = \"TOML\"\n")
        let types = tokens.map { $0.type }
        XCTAssertEqual(types, ["BARE_KEY", "EQUALS", "BASIC_STRING", "NEWLINE", "EOF"])
    }
    
    func testBasicTypes() throws {
        let tokens = try TOMLLexer.tokenize("float = 3.14\nbool = true")
        let types = tokens.map { $0.type }
        XCTAssertEqual(types, [
            "BARE_KEY", "EQUALS", "FLOAT", "NEWLINE", 
            "BARE_KEY", "EQUALS", "TRUE", "EOF"
        ])
    }
    
    func testDates() throws {
        let tokens = try TOMLLexer.tokenize("d = 1979-05-27T07:32:00Z")
        let types = tokens.map { $0.type }
        XCTAssertEqual(types, ["BARE_KEY", "EQUALS", "OFFSET_DATETIME", "EOF"])
    }
}

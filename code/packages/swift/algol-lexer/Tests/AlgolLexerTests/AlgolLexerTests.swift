import XCTest
@testable import AlgolLexer

final class AlgolLexerTests: XCTestCase {
    func testVersionIsPresent() {
        XCTAssertFalse(AlgolLexer.version.isEmpty)
    }

    func testTokenizeSimpleProgram() throws {
        let tokens = try AlgolLexer.tokenize("begin integer x; x := 42 end")
        XCTAssertEqual(tokens.first?.value, "begin")
        XCTAssertEqual(tokens.last?.type, "EOF")
    }

    func testRejectsUnknownVersion() {
        XCTAssertThrowsError(try AlgolLexer.loadGrammar(version: "algol58"))
    }
}

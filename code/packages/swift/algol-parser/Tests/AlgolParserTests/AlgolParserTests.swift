import XCTest
@testable import AlgolParser

final class AlgolParserTests: XCTestCase {
    func testVersionIsPresent() {
        XCTAssertFalse(AlgolParser.version.isEmpty)
    }

    func testParseSimpleProgram() throws {
        let ast = try AlgolParser.parse("begin integer x; x := 42 end")
        XCTAssertEqual(ast.ruleName, "program")
    }

    func testRejectsUnknownVersion() {
        XCTAssertThrowsError(try AlgolParser.loadGrammar(version: "algol58"))
    }
}

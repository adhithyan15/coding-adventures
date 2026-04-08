import XCTest
@testable import CSVParser

final class CSVParserTests: XCTestCase {
    
    func testSimpleTwoColumnTable() throws {
        let source = "name,age\nAlice,30\nBob,25"
        let result = try parseCSV(source)
        XCTAssertEqual(result.count, 2)
        XCTAssertEqual(result[0], ["name": "Alice", "age": "30"])
        XCTAssertEqual(result[1], ["name": "Bob", "age": "25"])
    }
    
    func testTrailingNewlineProducesNoExtraRow() throws {
        let source = "a,b\n1,2\n"
        let result = try parseCSV(source)
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0], ["a": "1", "b": "2"])
    }
    
    func testCRLFLineEndings() throws {
        let source = "name,age\r\nAlice,30\r\nBob,25"
        let result = try parseCSV(source)
        XCTAssertEqual(result.count, 2)
        XCTAssertEqual(result[0], ["name": "Alice", "age": "30"])
        XCTAssertEqual(result[1], ["name": "Bob", "age": "25"])
    }
    
    func testQuotedFieldWithComma() throws {
        let source = "product,description\nWidget,\"A small, round widget\""
        let result = try parseCSV(source)
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0], ["product": "Widget", "description": "A small, round widget"])
    }
    
    func testEscapedDoubleQuote() throws {
        let source = "id,value\n1,\"She said \"\"hello\"\"\""
        let result = try parseCSV(source)
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0], ["id": "1", "value": "She said \"hello\""])
    }
    
    func testEmptyString() throws {
        let result = try parseCSV("")
        XCTAssertTrue(result.isEmpty)
    }
    
    func testEmptyFieldsMiddle() throws {
        let source = "a,b,c\n1,,3"
        let result = try parseCSV(source)
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0], ["a": "1", "b": "", "c": "3"])
    }
    
    func testShortRowPaddedWithEmptyStrings() throws {
        let source = "a,b,c\n1,2"
        let result = try parseCSV(source)
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0], ["a": "1", "b": "2", "c": ""])
    }
    
    func testLongRowTruncatedToHeaderLength() throws {
        let source = "a,b\n1,2,3,4,5"
        let result = try parseCSV(source)
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0], ["a": "1", "b": "2"])
    }
    
    func testWhitespaceIsSignificant() throws {
        let source = "a,b\n  hello  ,  world  "
        let result = try parseCSV(source)
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0], ["a": "  hello  ", "b": "  world  "])
    }
    
    func testTabDelimitedTSV() throws {
        let source = "name\tage\nAlice\t30"
        let result = try parseCSV(source, delimiter: "\t")
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0], ["name": "Alice", "age": "30"])
    }
    
    func testUnclosedQuoteRaisesError() {
        let source = "a,b\n\"unclosed,value"
        do {
            _ = try parseCSV(source)
            XCTFail("Expected UnclosedQuoteError")
        } catch let error as UnclosedQuoteError {
            XCTAssertTrue(error.message.contains("Unclosed"))
        } catch {
            XCTFail("Unexpected error type: \(error)")
        }
    }
}

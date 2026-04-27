import XCTest
@testable import CSVParser

final class CSVParserTests: XCTestCase {

    // MARK: - 1. Basic parsing
    
    func testBasicParsingThreeColumnTable() throws {
        let csv = "name,age,city\nAlice,30,New York\nBob,25,London\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(rows[0], ["name": "Alice", "age": "30", "city": "New York"])
        XCTAssertEqual(rows[1], ["name": "Bob", "age": "25", "city": "London"])
    }
    
    func testReturnsAllValuesAsStrings() throws {
        let csv = "x,y\n1,2\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows[0]["x"], "1")
        XCTAssertEqual(rows[0]["y"], "2")
    }
    
    func testHandlesNoTrailingNewline() throws {
        let csv = "name,value\nhello,world"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["name"], "hello")
        XCTAssertEqual(rows[0]["value"], "world")
    }
    
    func testParsesSingleColumnFile() throws {
        let csv = "fruit\napple\nbanana\ncherry\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows.count, 3)
        XCTAssertEqual(rows[0]["fruit"], "apple")
        XCTAssertEqual(rows[1]["fruit"], "banana")
        XCTAssertEqual(rows[2]["fruit"], "cherry")
    }
    
    func testParsesManyRows() throws {
        var lines = ["id,value"]
        for i in 1...100 {
            lines.append("\(i),item\(i)")
        }
        let csv = lines.joined(separator: "\n") + "\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows.count, 100)
        XCTAssertEqual(rows[0]["id"], "1")
        XCTAssertEqual(rows[0]["value"], "item1")
        XCTAssertEqual(rows[99]["id"], "100")
        XCTAssertEqual(rows[99]["value"], "item100")
    }
    
    // MARK: - 2. Quoted fields
    
    func testQuotedFieldWithEmbeddedComma() throws {
        let csv = "product,price,description\nWidget,9.99,\"A small, round widget\"\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["description"], "A small, round widget")
    }
    
    func testQuotedFieldWithEmbeddedNewline() throws {
        let csv = "id,note\n1,\"Line one\nLine two\"\n2,Single line\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(rows[0]["note"], "Line one\nLine two")
        XCTAssertEqual(rows[1]["note"], "Single line")
    }
    
    func testEscapedDoubleQuote() throws {
        let csv = "id,value\n1,\"She said \"\"hello\"\"\"\n2,plain\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows[0]["value"], "She said \"hello\"")
        XCTAssertEqual(rows[1]["value"], "plain")
    }
    
    func testEmptyQuotedField() throws {
        let csv = "a,b,c\n1,\"\",3\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows[0]["b"], "")
    }
    
    func testAllFieldsQuoted() throws {
        let csv = "\"name\",\"age\"\n\"Alice\",\"30\"\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["name"], "Alice")
        XCTAssertEqual(rows[0]["age"], "30")
    }
    
    func testQuotedFieldAtRowStart() throws {
        let csv = "a,b\n\"quoted start\",normal\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows[0]["a"], "quoted start")
        XCTAssertEqual(rows[0]["b"], "normal")
    }
    
    func testQuotedFieldAtRowEnd() throws {
        let csv = "a,b\nnormal,\"quoted end\"\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows[0]["b"], "quoted end")
    }
    
    func testQuotedFieldOnlyDoubleQuote() throws {
        let csv = "a,b\n1,\"\"\"\"\"\"\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows[0]["b"], "\"\"")
    }
    
    func testQuotedFieldWithManyDelimiters() throws {
        let csv = "a,b\n1,\"x,y,z,w\"\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows[0]["b"], "x,y,z,w")
    }
    
    // MARK: - 3. Empty fields
    
    func testEmptyFieldInMiddle() throws {
        let csv = "a,b,c\n1,,3\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows[0]["a"], "1")
        XCTAssertEqual(rows[0]["b"], "")
        XCTAssertEqual(rows[0]["c"], "3")
    }
    
    func testEmptyLeadingAndTrailing() throws {
        let csv = "a,b,c\n,2,\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows[0]["a"], "")
        XCTAssertEqual(rows[0]["b"], "2")
        XCTAssertEqual(rows[0]["c"], "")
    }
    
    func testAllEmptyFields() throws {
        let csv = "a,b,c\n,,\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows[0]["a"], "")
        XCTAssertEqual(rows[0]["b"], "")
        XCTAssertEqual(rows[0]["c"], "")
    }
    
    func testSingleEmptyFieldAfterHeader() throws {
        let csv = "a\n\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["a"], "")
    }
    
    // MARK: - 4. Ragged rows
    
    func testShortRowPaddedWithEmptyStrings() throws {
        let csv = "name,age,city\nAlice,30\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["name"], "Alice")
        XCTAssertEqual(rows[0]["age"], "30")
        XCTAssertEqual(rows[0]["city"], "")
    }
    
    func testVeryShortRow() throws {
        let csv = "a,b,c,d\nonly\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows[0]["a"], "only")
        XCTAssertEqual(rows[0]["b"], "")
        XCTAssertEqual(rows[0]["c"], "")
        XCTAssertEqual(rows[0]["d"], "")
    }
    
    func testLongRowTruncated() throws {
        let csv = "a,b,c\n1,2,3,4,5\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["a"], "1")
        XCTAssertEqual(rows[0]["b"], "2")
        XCTAssertEqual(rows[0]["c"], "3")
        XCTAssertNil(rows[0]["4"])
        XCTAssertEqual(rows[0].keys.count, 3)
    }
    
    func testMixedRaggedRows() throws {
        let csv = "a,b,c\n1\n2,two\n3,three,THREE\n4,four,FOUR,extra\n"
        let rows = try CSVParser.parseCSV(csv)
        
        XCTAssertEqual(rows.count, 4)
        XCTAssertEqual(rows[0], ["a": "1", "b": "", "c": ""])
        XCTAssertEqual(rows[1], ["a": "2", "b": "two", "c": ""])
        XCTAssertEqual(rows[2], ["a": "3", "b": "three", "c": "THREE"])
        XCTAssertEqual(rows[3], ["a": "4", "b": "four", "c": "FOUR"])
    }
    
    // MARK: - 5. Edge cases
    
    func testEmptyStringReturnsEmptyArray() throws {
        XCTAssertEqual(try CSVParser.parseCSV("").count, 0)
    }
    
    func testHeaderOnlyWithTrailingNewline() throws {
        XCTAssertEqual(try CSVParser.parseCSV("name,age,city\n").count, 0)
    }
    
    func testHeaderOnlyWithoutTrailingNewline() throws {
        XCTAssertEqual(try CSVParser.parseCSV("name,age").count, 0)
    }
    
    func testSingleCellCSVWithNewline() throws {
        let csv = "x\nhello\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["x"], "hello")
    }
    
    func testSingleCellCSVWithoutNewline() throws {
        let csv = "x\nhello"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["x"], "hello")
    }
    
    // MARK: - 6. Line endings
    
    func testUnixLF() throws {
        let csv = "name,age\nAlice,30\nBob,25\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(rows[0]["name"], "Alice")
    }
    
    func testWindowsCRLF() throws {
        let csv = "name,age\r\nAlice,30\r\nBob,25\r\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(rows[0]["name"], "Alice")
        XCTAssertEqual(rows[1]["name"], "Bob")
    }
    
    func testOldMacCR() throws {
        let csv = "name,age\rAlice,30\rBob,25\r"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(rows[0]["name"], "Alice")
        XCTAssertEqual(rows[1]["name"], "Bob")
    }
    
    func testCRLFMightNotProduceEmptyRow() throws {
        let csv = "a,b\r\n1,2\r\n3,4\r\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows.count, 2)
    }
    
    func testEmbeddedLFInsideQuote() throws {
        let csv = "id,note\n1,\"first\nsecond\"\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows[0]["note"], "first\nsecond")
    }
    
    func testEmbeddedCRLFInsideQuote() throws {
        let csv = "id,note\n1,\"first\r\nsecond\"\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows[0]["note"], "first\r\nsecond")
    }
    
    // MARK: - 7. Custom delimiters
    
    func testTabDelimiter() throws {
        let tsv = "name\tage\nAlice\t30\nBob\t25\n"
        let rows = try CSVParser.parseCSVWithDelimiter(tsv, delimiter: "\t")
        
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(rows[0]["name"], "Alice")
        XCTAssertEqual(rows[0]["age"], "30")
        XCTAssertEqual(rows[1]["name"], "Bob")
    }
    
    func testSemicolonDelimiter() throws {
        let csv = "name;age;city\nAlice;30;Paris\n"
        let rows = try CSVParser.parseCSVWithDelimiter(csv, delimiter: ";")
        
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["name"], "Alice")
        XCTAssertEqual(rows[0]["city"], "Paris")
    }
    
    func testPipeDelimiter() throws {
        let csv = "a|b|c\n1|2|3\n"
        let rows = try CSVParser.parseCSVWithDelimiter(csv, delimiter: "|")
        
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["a"], "1")
        XCTAssertEqual(rows[0]["b"], "2")
        XCTAssertEqual(rows[0]["c"], "3")
    }
    
    func testCommaIsLiteralWhenTabDelimiter() throws {
        let tsv = "a\tb\n1,2\t3,4\n"
        let rows = try CSVParser.parseCSVWithDelimiter(tsv, delimiter: "\t")
        
        XCTAssertEqual(rows[0]["a"], "1,2")
        XCTAssertEqual(rows[0]["b"], "3,4")
    }
    
    // MARK: - 8. Error handling
    
    func testThrowsUnclosedQuoteError() {
        let csv = "name,value\n1,\"unclosed\n"
        XCTAssertThrowsError(try CSVParser.parseCSV(csv)) { error in
            XCTAssertEqual(error as? CSVParserError, .unclosedQuoteError)
            XCTAssertEqual((error as? CSVParserError)?.description, "Unclosed quoted field: EOF reached inside a quoted field")
        }
    }
    
    func testUnclosedQuoteAtVeryStart() {
        XCTAssertThrowsError(try CSVParser.parseCSV("\"never closed")) { error in
            XCTAssertEqual(error as? CSVParserError, .unclosedQuoteError)
        }
    }
    
    // MARK: - 9. Whitespace preservation
    
    func testSpacesAroundUnquotedFieldsPreserved() throws {
        let csv = "key,value\nspaced,  hello  \n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows[0]["value"], "  hello  ")
    }
    
    func testSpacesInsideQuotedFieldsPreserved() throws {
        let csv = "key,value\nspaced,\"  hello  \"\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows[0]["value"], "  hello  ")
    }
    
    func testTabInsideUnquotedFieldPreserved() throws {
        let csv = "key,value\nwith tab,\there\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows[0]["value"], "\there")
    }
    
    // MARK: - 11. Integration / realistic data tests
    
    func testRealisticProductsTable() throws {
        let csv = "product,price,description,in_stock\n" +
                  "Widget,9.99,\"A small, round widget\",true\n" +
                  "Gadget,19.99,Electronic device,false\n" +
                  "Doohickey,4.50,\"Says \"\"hello\"\"\",true\n"
        
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows.count, 3)
        XCTAssertEqual(rows[0], [
            "product": "Widget",
            "price": "9.99",
            "description": "A small, round widget",
            "in_stock": "true"
        ])
        XCTAssertEqual(rows[1], [
            "product": "Gadget",
            "price": "19.99",
            "description": "Electronic device",
            "in_stock": "false"
        ])
        XCTAssertEqual(rows[2]["description"], "Says \"hello\"")
    }
    
    func testEOFUnquotedQuote() throws {
        let csv = "a,b\n1,\"hello\""
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["b"], "hello")
    }
    
    func testLenientModeQuotedFieldUnexpectedChar() throws {
        let csv = "a,b\n1,\"hello\"world\n"
        let rows = try CSVParser.parseCSV(csv)
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows[0]["b"], "helloworld")
    }
}

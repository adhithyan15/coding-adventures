import XCTest
import Lexer
@testable import XMLLexer

final class XMLLexerTests: XCTestCase {

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------
    
    private func tokenPairs(_ source: String) throws -> [(String, String)] {
        let tokens = try XMLLexer.tokenize(source)
        return tokens.filter { $0.type != "EOF" }.map { ($0.type, $0.value) }
    }
    
    private func tokenTypes(_ source: String) throws -> [String] {
        let tokens = try XMLLexer.tokenize(source)
        return tokens.filter { $0.type != "EOF" }.map { $0.type }
    }
    
    // ===========================================================================
    // Basic Tags
    // ===========================================================================
    
    func testBasicTagsSimpleElement() throws {
        let pairs = try tokenPairs("<p>text</p>")
        XCTAssertEqual(pairs.count, 7)
        XCTAssertEqual(pairs[0].0, "OPEN_TAG_START")
        XCTAssertEqual(pairs[0].1, "<")
        XCTAssertEqual(pairs[1].0, "TAG_NAME")
        XCTAssertEqual(pairs[1].1, "p")
        XCTAssertEqual(pairs[2].0, "TAG_CLOSE")
        XCTAssertEqual(pairs[2].1, ">")
        XCTAssertEqual(pairs[3].0, "TEXT")
        XCTAssertEqual(pairs[3].1, "text")
        XCTAssertEqual(pairs[4].0, "CLOSE_TAG_START")
        XCTAssertEqual(pairs[4].1, "</")
        XCTAssertEqual(pairs[5].0, "TAG_NAME")
        XCTAssertEqual(pairs[5].1, "p")
        XCTAssertEqual(pairs[6].0, "TAG_CLOSE")
        XCTAssertEqual(pairs[6].1, ">")
    }
    
    func testTagsWithNamespacePrefixes() throws {
        let types = try tokenTypes("<ns:tag>content</ns:tag>")
        XCTAssertEqual(types, [
            "OPEN_TAG_START", "TAG_NAME", "TAG_CLOSE",
            "TEXT",
            "CLOSE_TAG_START", "TAG_NAME", "TAG_CLOSE"
        ])
        let pairs = try tokenPairs("<ns:tag>content</ns:tag>")
        XCTAssertEqual(pairs[1].0, "TAG_NAME")
        XCTAssertEqual(pairs[1].1, "ns:tag")
    }
    
    func testExplicitlyEmptyElement() throws {
        let pairs = try tokenPairs("<div></div>")
        XCTAssertEqual(pairs.count, 6)
        XCTAssertEqual(pairs[0].0, "OPEN_TAG_START")
        XCTAssertEqual(pairs[0].1, "<")
        XCTAssertEqual(pairs[1].0, "TAG_NAME")
        XCTAssertEqual(pairs[1].1, "div")
        XCTAssertEqual(pairs[2].0, "TAG_CLOSE")
        XCTAssertEqual(pairs[2].1, ">")
        XCTAssertEqual(pairs[3].0, "CLOSE_TAG_START")
        XCTAssertEqual(pairs[3].1, "</")
        XCTAssertEqual(pairs[4].0, "TAG_NAME")
        XCTAssertEqual(pairs[4].1, "div")
        XCTAssertEqual(pairs[5].0, "TAG_CLOSE")
        XCTAssertEqual(pairs[5].1, ">")
    }
    
    func testSelfClosingTag() throws {
        let pairs = try tokenPairs("<br/>")
        XCTAssertEqual(pairs.count, 3)
        XCTAssertEqual(pairs[0].0, "OPEN_TAG_START")
        XCTAssertEqual(pairs[0].1, "<")
        XCTAssertEqual(pairs[1].0, "TAG_NAME")
        XCTAssertEqual(pairs[1].1, "br")
        XCTAssertEqual(pairs[2].0, "SELF_CLOSE")
        XCTAssertEqual(pairs[2].1, "/>")
    }
    
    func testSelfClosingTagWithSpace() throws {
        let pairs = try tokenPairs("<br />")
        XCTAssertEqual(pairs.count, 3)
        XCTAssertEqual(pairs[0].0, "OPEN_TAG_START")
        XCTAssertEqual(pairs[0].1, "<")
        XCTAssertEqual(pairs[1].0, "TAG_NAME")
        XCTAssertEqual(pairs[1].1, "br")
        XCTAssertEqual(pairs[2].0, "SELF_CLOSE")
        XCTAssertEqual(pairs[2].1, "/>")
    }
    
    // ===========================================================================
    // Attributes
    // ===========================================================================
    
    func testDoubleQuotedAttribute() throws {
        let pairs = try tokenPairs("<div class=\"main\">")
        XCTAssertEqual(pairs.count, 6)
        XCTAssertEqual(pairs[0].0, "OPEN_TAG_START")
        XCTAssertEqual(pairs[1].0, "TAG_NAME")
        XCTAssertEqual(pairs[1].1, "div")
        XCTAssertEqual(pairs[2].0, "TAG_NAME")
        XCTAssertEqual(pairs[2].1, "class")
        XCTAssertEqual(pairs[3].0, "ATTR_EQUALS")
        XCTAssertEqual(pairs[3].1, "=")
        XCTAssertEqual(pairs[4].0, "ATTR_VALUE")
        XCTAssertEqual(pairs[4].1, "\"main\"")
        XCTAssertEqual(pairs[5].0, "TAG_CLOSE")
    }
    
    func testSingleQuotedAttribute() throws {
        let pairs = try tokenPairs("<div class='main'>")
        XCTAssertEqual(pairs.count, 6)
        XCTAssertEqual(pairs[0].0, "OPEN_TAG_START")
        XCTAssertEqual(pairs[1].1, "div")
        XCTAssertEqual(pairs[2].1, "class")
        XCTAssertEqual(pairs[3].1, "=")
        XCTAssertEqual(pairs[4].0, "ATTR_VALUE")
        XCTAssertEqual(pairs[4].1, "'main'")
        XCTAssertEqual(pairs[5].0, "TAG_CLOSE")
    }
    
    func testMultipleAttributesOnOneTag() throws {
        let pairs = try tokenPairs("<a href=\"url\" target=\"_blank\">")
        let tagNames = pairs.filter { $0.0 == "TAG_NAME" }.map { $0.1 }
        XCTAssertEqual(tagNames, ["a", "href", "target"])
        let attrValues = pairs.filter { $0.0 == "ATTR_VALUE" }.map { $0.1 }
        XCTAssertEqual(attrValues, ["\"url\"", "\"_blank\""])
    }
    
    func testAttributeOnSelfClosingTag() throws {
        let types = try tokenTypes("<img src=\"photo.jpg\"/>")
        XCTAssertTrue(types.contains("SELF_CLOSE"))
        XCTAssertTrue(types.contains("ATTR_VALUE"))
    }
    
    // ===========================================================================
    // Comments
    // ===========================================================================
    
    func testSimpleComment() throws {
        let pairs = try tokenPairs("<!-- hello -->")
        XCTAssertEqual(pairs.count, 3)
        XCTAssertEqual(pairs[0].0, "COMMENT_START")
        XCTAssertEqual(pairs[0].1, "<!--")
        XCTAssertEqual(pairs[1].0, "COMMENT_TEXT")
        XCTAssertEqual(pairs[1].1, " hello ")
        XCTAssertEqual(pairs[2].0, "COMMENT_END")
        XCTAssertEqual(pairs[2].1, "-->")
    }
    
    func testWhitespaceInsideCommentsPreserved() throws {
        let pairs = try tokenPairs("<!--  spaces  and\ttabs  -->")
        let text = pairs.filter { $0.0 == "COMMENT_TEXT" }.map { $0.1 }
        XCTAssertEqual(text, ["  spaces  and\ttabs  "])
    }
    
    func testSingleDashesInsideCommentsAllowed() throws {
        let pairs = try tokenPairs("<!-- a-b-c -->")
        let text = pairs.filter { $0.0 == "COMMENT_TEXT" }.map { $0.1 }
        XCTAssertEqual(text, [" a-b-c "])
    }
    
    func testCommentBetweenElements() throws {
        let types = try tokenTypes("<a/><!-- mid --><b/>")
        XCTAssertTrue(types.contains("COMMENT_START"))
        XCTAssertTrue(types.contains("COMMENT_END"))
    }
    
    // ===========================================================================
    // CDATA Sections
    // ===========================================================================
    
    func testSimpleCDATASection() throws {
        let pairs = try tokenPairs("<![CDATA[raw text]]>")
        XCTAssertEqual(pairs.count, 3)
        XCTAssertEqual(pairs[0].0, "CDATA_START")
        XCTAssertEqual(pairs[0].1, "<![CDATA[")
        XCTAssertEqual(pairs[1].0, "CDATA_TEXT")
        XCTAssertEqual(pairs[1].1, "raw text")
        XCTAssertEqual(pairs[2].0, "CDATA_END")
        XCTAssertEqual(pairs[2].1, "]]>")
    }
    
    func testAngleBracketsInsideCDATA() throws {
        let pairs = try tokenPairs("<![CDATA[<not a tag>]]>")
        let text = pairs.filter { $0.0 == "CDATA_TEXT" }.map { $0.1 }
        XCTAssertEqual(text, ["<not a tag>"])
    }
    
    func testWhitespaceInsideCDATAPreserved() throws {
        let pairs = try tokenPairs("<![CDATA[  hello\n  world  ]]>")
        let text = pairs.filter { $0.0 == "CDATA_TEXT" }.map { $0.1 }
        XCTAssertEqual(text, ["  hello\n  world  "])
    }
    
    func testSingleBracketsInsideCDATA() throws {
        let pairs = try tokenPairs("<![CDATA[a]b]]>")
        let text = pairs.filter { $0.0 == "CDATA_TEXT" }.map { $0.1 }
        XCTAssertEqual(text, ["a]b"])
    }
    
    // ===========================================================================
    // Processing Instructions
    // ===========================================================================
    
    func testXMLDeclaration() throws {
        let pairs = try tokenPairs("<?xml version=\"1.0\"?>")
        XCTAssertEqual(pairs.count, 4)
        XCTAssertEqual(pairs[0].0, "PI_START")
        XCTAssertEqual(pairs[0].1, "<?")
        XCTAssertEqual(pairs[1].0, "PI_TARGET")
        XCTAssertEqual(pairs[1].1, "xml")
        XCTAssertEqual(pairs[2].0, "PI_TEXT")
        XCTAssertEqual(pairs[2].1, " version=\"1.0\"")
        XCTAssertEqual(pairs[3].0, "PI_END")
        XCTAssertEqual(pairs[3].1, "?>")
    }
    
    func testStylesheetProcessingInstruction() throws {
        let types = try tokenTypes("<?xml-stylesheet type=\"text/xsl\"?>")
        XCTAssertEqual(types[0], "PI_START")
        XCTAssertEqual(types[1], "PI_TARGET")
        XCTAssertEqual(types.last!, "PI_END")
    }
    
    // ===========================================================================
    // Entity and Character References
    // ===========================================================================
    
    func testNamedEntityReference() throws {
        let pairs = try tokenPairs("a&amp;b")
        XCTAssertEqual(pairs.count, 3)
        XCTAssertEqual(pairs[0].0, "TEXT")
        XCTAssertEqual(pairs[0].1, "a")
        XCTAssertEqual(pairs[1].0, "ENTITY_REF")
        XCTAssertEqual(pairs[1].1, "&amp;")
        XCTAssertEqual(pairs[2].0, "TEXT")
        XCTAssertEqual(pairs[2].1, "b")
    }
    
    func testDecimalCharacterReference() throws {
        let pairs = try tokenPairs("&#65;")
        XCTAssertEqual(pairs.count, 1)
        XCTAssertEqual(pairs[0].0, "CHAR_REF")
        XCTAssertEqual(pairs[0].1, "&#65;")
    }
    
    func testHexCharacterReference() throws {
        let pairs = try tokenPairs("&#x41;")
        XCTAssertEqual(pairs.count, 1)
        XCTAssertEqual(pairs[0].0, "CHAR_REF")
        XCTAssertEqual(pairs[0].1, "&#x41;")
    }
    
    func testMultipleEntityReferences() throws {
        let types = try tokenTypes("&lt;hello&gt;")
        XCTAssertEqual(types, ["ENTITY_REF", "TEXT", "ENTITY_REF"])
    }
    
    // ===========================================================================
    // Nested and Mixed Content
    // ===========================================================================
    
    func testNestedElements() throws {
        let types = try tokenTypes("<a><b>text</b></a>")
        XCTAssertEqual(types.filter { $0 == "OPEN_TAG_START" }.count, 2)
        XCTAssertEqual(types.filter { $0 == "CLOSE_TAG_START" }.count, 2)
    }
    
    func testMixedContent() throws {
        let pairs = try tokenPairs("<p>Hello <b>world</b>!</p>")
        let texts = pairs.filter { $0.0 == "TEXT" }.map { $0.1 }
        XCTAssertEqual(texts, ["Hello ", "world", "!"])
    }
    
    func testCompleteXMLDocument() throws {
        let source = "<?xml version=\"1.0\"?><!-- A greeting --><root lang=\"en\"><greeting>Hello &amp; welcome</greeting></root>"
        let tokens = try XMLLexer.tokenize(source)
        let types = tokens.map { $0.type }
        
        XCTAssertTrue(types.contains("PI_START"))
        XCTAssertTrue(types.contains("PI_END"))
        XCTAssertTrue(types.contains("COMMENT_START"))
        XCTAssertTrue(types.contains("COMMENT_END"))
        XCTAssertEqual(types.filter { $0 == "OPEN_TAG_START" }.count, 2)
        XCTAssertEqual(types.filter { $0 == "CLOSE_TAG_START" }.count, 2)
        XCTAssertTrue(types.contains("ENTITY_REF"))
        XCTAssertEqual(types.last!, "EOF")
    }
    
    func testCDATAInsideElement() throws {
        let types = try tokenTypes("<script><![CDATA[x < y]]></script>")
        XCTAssertTrue(types.contains("CDATA_START"))
        XCTAssertTrue(types.contains("CDATA_TEXT"))
        XCTAssertTrue(types.contains("CDATA_END"))
    }
    
    // ===========================================================================
    // Edge Cases
    // ===========================================================================
    
    func testEOFForEmptyInput() throws {
        let tokens = try XMLLexer.tokenize("")
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0].type, "EOF")
    }
    
    func testPlainTextWithNoTags() throws {
        let pairs = try tokenPairs("just text")
        XCTAssertEqual(pairs.count, 1)
        XCTAssertEqual(pairs[0].0, "TEXT")
        XCTAssertEqual(pairs[0].1, "just text")
    }
    
    func testWhitespaceBetweenTagsSkipped() throws {
        let pairs = try tokenPairs("<a> <b> </b> </a>")
        let texts = pairs.filter { $0.0 == "TEXT" }.map { $0.1 }
        XCTAssertEqual(texts.count, 0)
    }
    
    func testAlwaysEndsWithEOF() throws {
        let tokens = try XMLLexer.tokenize("<root/>")
        XCTAssertEqual(tokens.last!.type, "EOF")
    }
    
    // ===========================================================================
    // Position Tracking
    // ===========================================================================
    
    func testPositionTracking() throws {
        let tokens = try XMLLexer.tokenize("<a>text</a>")
        XCTAssertEqual(tokens[0].line, 1)
        XCTAssertEqual(tokens[0].column, 1)
    }
    
    func testColumnTrackingAcrossLine() throws {
        let tokens = try XMLLexer.tokenize("<div/>")
        XCTAssertEqual(tokens[0].column, 1)  // <
        XCTAssertEqual(tokens[1].column, 2)  // div
    }
}

import XCTest
import Lexer
@testable import TOMLLexer

final class TOMLLexerTests: XCTestCase {

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------
    
    private func tokenTypes(_ source: String) throws -> [String] {
        let tokens = try TOMLLexer.tokenize(source)
        return tokens.map { $0.type }
    }
    
    private func tokenValues(_ source: String) throws -> [String] {
        let tokens = try TOMLLexer.tokenize(source)
        return tokens.map { $0.value }
    }
    
    private func meaningfulTypes(_ source: String) throws -> [String] {
        let tokens = try TOMLLexer.tokenize(source)
        return tokens.filter { $0.type != "NEWLINE" && $0.type != "EOF" }.map { $0.type }
    }
    
    // =========================================================================
    // String Tokens
    // =========================================================================
    
    func testBasicStringsDoubleQuoted() throws {
        let tokens = try TOMLLexer.tokenize("\"hello\"")
        XCTAssertEqual(tokens[0].type, "BASIC_STRING")
        XCTAssertEqual(tokens[0].value, "hello")
        
        let emptyTokens = try TOMLLexer.tokenize("\"\"")
        XCTAssertEqual(emptyTokens[0].type, "BASIC_STRING")
        XCTAssertEqual(emptyTokens[0].value, "")
        
        let escapedTokens = try TOMLLexer.tokenize("\"hello\\nworld\"")
        XCTAssertEqual(escapedTokens[0].type, "BASIC_STRING")
        XCTAssertEqual(escapedTokens[0].value, "hello\\nworld")
    }
    
    func testLiteralStringsSingleQuoted() throws {
        let tokens = try TOMLLexer.tokenize("'hello'")
        XCTAssertEqual(tokens[0].type, "LITERAL_STRING")
        XCTAssertEqual(tokens[0].value, "hello")
        
        let backslashTokens = try TOMLLexer.tokenize("'C:\\\\Users\\\\name'")
        XCTAssertEqual(backslashTokens[0].type, "LITERAL_STRING")
        XCTAssertEqual(backslashTokens[0].value, "C:\\\\Users\\\\name")
    }
    
    func testMultiLineBasicStrings() throws {
        let tokens = try TOMLLexer.tokenize("\"\"\"hello\nworld\"\"\"")
        XCTAssertEqual(tokens[0].type, "ML_BASIC_STRING")
        XCTAssertEqual(tokens[0].value, "hello\nworld")
        
        let oneLineTokens = try TOMLLexer.tokenize("\"\"\"hello\"\"\"")
        XCTAssertEqual(oneLineTokens[0].type, "ML_BASIC_STRING")
        XCTAssertEqual(oneLineTokens[0].value, "hello")
    }
    
    func testMultiLineLiteralStrings() throws {
        let tokens = try TOMLLexer.tokenize("'''hello\nworld'''")
        XCTAssertEqual(tokens[0].type, "ML_LITERAL_STRING")
        XCTAssertEqual(tokens[0].value, "hello\nworld")
    }
    
    // =========================================================================
    // Number Tokens
    // =========================================================================
    
    func testIntegerTokens() throws {
        let decimal = try TOMLLexer.tokenize("42")
        XCTAssertEqual(decimal[0].type, "INTEGER")
        XCTAssertEqual(decimal[0].value, "42")
        
        let zero = try TOMLLexer.tokenize("0")
        XCTAssertEqual(zero[0].type, "INTEGER")
        XCTAssertEqual(zero[0].value, "0")
        
        let pos = try TOMLLexer.tokenize("+42")
        XCTAssertEqual(pos[0].type, "INTEGER")
        XCTAssertEqual(pos[0].value, "+42")
        
        let neg = try TOMLLexer.tokenize("-42")
        XCTAssertEqual(neg[0].type, "INTEGER")
        XCTAssertEqual(neg[0].value, "-42")
        
        let sep = try TOMLLexer.tokenize("1_000_000")
        XCTAssertEqual(sep[0].type, "INTEGER")
        XCTAssertEqual(sep[0].value, "1_000_000")
        
        let hex = try TOMLLexer.tokenize("0xDEADBEEF")
        XCTAssertEqual(hex[0].type, "INTEGER")
        XCTAssertEqual(hex[0].value, "0xDEADBEEF")
        
        let oct = try TOMLLexer.tokenize("0o755")
        XCTAssertEqual(oct[0].type, "INTEGER")
        XCTAssertEqual(oct[0].value, "0o755")
        
        let bin = try TOMLLexer.tokenize("0b11010110")
        XCTAssertEqual(bin[0].type, "INTEGER")
        XCTAssertEqual(bin[0].value, "0b11010110")
    }
    
    func testFloatTokens() throws {
        let dec = try TOMLLexer.tokenize("3.14")
        XCTAssertEqual(dec[0].type, "FLOAT")
        XCTAssertEqual(dec[0].value, "3.14")
        
        let exp = try TOMLLexer.tokenize("1e10")
        XCTAssertEqual(exp[0].type, "FLOAT")
        XCTAssertEqual(exp[0].value, "1e10")
        
        let both = try TOMLLexer.tokenize("6.626e-34")
        XCTAssertEqual(both[0].type, "FLOAT")
        XCTAssertEqual(both[0].value, "6.626e-34")
        
        let pinf = try TOMLLexer.tokenize("inf")
        XCTAssertEqual(pinf[0].type, "FLOAT")
        XCTAssertEqual(pinf[0].value, "inf")
        
        let ninf = try TOMLLexer.tokenize("-inf")
        XCTAssertEqual(ninf[0].type, "FLOAT")
        XCTAssertEqual(ninf[0].value, "-inf")
        
        let nan = try TOMLLexer.tokenize("nan")
        XCTAssertEqual(nan[0].type, "FLOAT")
        XCTAssertEqual(nan[0].value, "nan")
        
        let sep = try TOMLLexer.tokenize("1_000.000_1")
        XCTAssertEqual(sep[0].type, "FLOAT")
        XCTAssertEqual(sep[0].value, "1_000.000_1")
    }
    
    // =========================================================================
    // Boolean Tokens
    // =========================================================================
    
    func testBooleanTokens() throws {
        let t = try TOMLLexer.tokenize("true")
        XCTAssertEqual(t[0].type, "TRUE")
        XCTAssertEqual(t[0].value, "true")
        
        let f = try TOMLLexer.tokenize("false")
        XCTAssertEqual(f[0].type, "FALSE")
        XCTAssertEqual(f[0].value, "false")
    }
    
    // =========================================================================
    // Date/Time Tokens
    // =========================================================================
    
    func testDateTimeTokens() throws {
        let offset = try TOMLLexer.tokenize("1979-05-27T07:32:00Z")
        XCTAssertEqual(offset[0].type, "OFFSET_DATETIME")
        XCTAssertEqual(offset[0].value, "1979-05-27T07:32:00Z")
        
        let offsetNum = try TOMLLexer.tokenize("1979-05-27T07:32:00+09:00")
        XCTAssertEqual(offsetNum[0].type, "OFFSET_DATETIME")
        XCTAssertEqual(offsetNum[0].value, "1979-05-27T07:32:00+09:00")
        
        let local = try TOMLLexer.tokenize("1979-05-27T07:32:00")
        XCTAssertEqual(local[0].type, "LOCAL_DATETIME")
        XCTAssertEqual(local[0].value, "1979-05-27T07:32:00")
        
        let date = try TOMLLexer.tokenize("1979-05-27")
        XCTAssertEqual(date[0].type, "LOCAL_DATE")
        XCTAssertEqual(date[0].value, "1979-05-27")
        
        let time = try TOMLLexer.tokenize("07:32:00")
        XCTAssertEqual(time[0].type, "LOCAL_TIME")
        XCTAssertEqual(time[0].value, "07:32:00")
        
        let timeFrac = try TOMLLexer.tokenize("07:32:00.999999")
        XCTAssertEqual(timeFrac[0].type, "LOCAL_TIME")
        XCTAssertEqual(timeFrac[0].value, "07:32:00.999999")
    }
    
    // =========================================================================
    // Bare Key Tokens
    // =========================================================================
    
    func testBareKeyTokens() throws {
        let t1 = try TOMLLexer.tokenize("server")
        XCTAssertEqual(t1[0].type, "BARE_KEY")
        XCTAssertEqual(t1[0].value, "server")
        
        let t2 = try TOMLLexer.tokenize("my-key")
        XCTAssertEqual(t2[0].type, "BARE_KEY")
        XCTAssertEqual(t2[0].value, "my-key")
        
        let t3 = try TOMLLexer.tokenize("my_key")
        XCTAssertEqual(t3[0].type, "BARE_KEY")
        XCTAssertEqual(t3[0].value, "my_key")
        
        let t4 = try TOMLLexer.tokenize("key123")
        XCTAssertEqual(t4[0].type, "BARE_KEY")
        XCTAssertEqual(t4[0].value, "key123")
    }
    
    // =========================================================================
    // Structural Tokens
    // =========================================================================
    
    func testStructuralTokens() throws {
        let eq = try meaningfulTypes("key = value")
        XCTAssertTrue(eq.contains("EQUALS"))
        
        let dot = try meaningfulTypes("a.b")
        XCTAssertTrue(dot.contains("DOT"))
        
        let comma = try meaningfulTypes("[1, 2]")
        XCTAssertTrue(comma.contains("COMMA"))
        
        let bracket = try meaningfulTypes("[table]")
        XCTAssertTrue(bracket.contains("LBRACKET"))
        XCTAssertTrue(bracket.contains("RBRACKET"))
        
        let brace = try meaningfulTypes("{ key = 1 }")
        XCTAssertTrue(brace.contains("LBRACE"))
        XCTAssertTrue(brace.contains("RBRACE"))
    }
    
    // =========================================================================
    // Newline Handling
    // =========================================================================
    
    func testNewlineHandling() throws {
        let types = try tokenTypes("a = 1\nb = 2")
        XCTAssertTrue(types.contains("NEWLINE"))
        
        let tokens = try TOMLLexer.tokenize("a = 1\n\nb = 2")
        let newlineCount = tokens.filter { $0.type == "NEWLINE" }.count
        XCTAssertEqual(newlineCount, 2)
    }
    
    // =========================================================================
    // Comment Handling
    // =========================================================================
    
    func testCommentHandling() throws {
        let types1 = try meaningfulTypes("# this is a comment\nkey = 1")
        XCTAssertFalse(types1.contains("COMMENT"))
        XCTAssertTrue(types1.contains("BARE_KEY"))
        
        let types2 = try meaningfulTypes("key = \"value\" # inline comment")
        XCTAssertTrue(types2.contains("BARE_KEY"))
        XCTAssertTrue(types2.contains("EQUALS"))
        XCTAssertTrue(types2.contains("BASIC_STRING"))
    }
    
    // =========================================================================
    // Complete TOML Documents
    // =========================================================================
    
    func testKeyValuePairs() throws {
        let t1 = try meaningfulTypes("title = \"TOML Example\"")
        XCTAssertEqual(t1, ["BARE_KEY", "EQUALS", "BASIC_STRING"])
        
        let t2 = try meaningfulTypes("a.b.c = 1")
        XCTAssertEqual(t2, ["BARE_KEY", "DOT", "BARE_KEY", "DOT", "BARE_KEY", "EQUALS", "INTEGER"])
        
        let t3 = try meaningfulTypes("\"key with spaces\" = \"value\"")
        XCTAssertEqual(t3, ["BASIC_STRING", "EQUALS", "BASIC_STRING"])
    }
    
    func testTableHeaders() throws {
        let t1 = try meaningfulTypes("[server]")
        XCTAssertEqual(t1, ["LBRACKET", "BARE_KEY", "RBRACKET"])
        
        let t2 = try meaningfulTypes("[a.b.c]")
        XCTAssertEqual(t2, ["LBRACKET", "BARE_KEY", "DOT", "BARE_KEY", "DOT", "BARE_KEY", "RBRACKET"])
        
        let t3 = try meaningfulTypes("[[products]]")
        XCTAssertEqual(t3, ["LBRACKET", "LBRACKET", "BARE_KEY", "RBRACKET", "RBRACKET"])
    }
    
    func testArrays() throws {
        let t1 = try meaningfulTypes("[1, 2, 3]")
        XCTAssertEqual(t1, ["LBRACKET", "INTEGER", "COMMA", "INTEGER", "COMMA", "INTEGER", "RBRACKET"])
        
        let t2 = try meaningfulTypes("[\"red\", \"green\", \"blue\"]")
        XCTAssertEqual(t2, ["LBRACKET", "BASIC_STRING", "COMMA", "BASIC_STRING", "COMMA", "BASIC_STRING", "RBRACKET"])
    }
    
    func testInlineTables() throws {
        let t1 = try meaningfulTypes("{ x = 1, y = 2 }")
        XCTAssertEqual(t1, ["LBRACE", "BARE_KEY", "EQUALS", "INTEGER", "COMMA", "BARE_KEY", "EQUALS", "INTEGER", "RBRACE"])
    }
    
    func testCompleteTOMLDocuments() throws {
        let source1 = "[server]\nhost = \"localhost\"\nport = 8080"
        let t1 = try meaningfulTypes(source1)
        XCTAssertEqual(t1, [
            "LBRACKET", "BARE_KEY", "RBRACKET",
            "BARE_KEY", "EQUALS", "BASIC_STRING",
            "BARE_KEY", "EQUALS", "INTEGER"
        ])
        
        let source2 = "name = \"TOML\"\nversion = 1\npi = 3.14\nenabled = true"
        let t2 = try meaningfulTypes(source2)
        XCTAssertEqual(t2, [
            "BARE_KEY", "EQUALS", "BASIC_STRING",
            "BARE_KEY", "EQUALS", "INTEGER",
            "BARE_KEY", "EQUALS", "FLOAT",
            "BARE_KEY", "EQUALS", "TRUE"
        ])
    }
    
    // =========================================================================
    // Position Tracking
    // =========================================================================
    
    func testPositionTracking() throws {
        let tokens = try TOMLLexer.tokenize("key = 1")
        XCTAssertEqual(tokens[0].line, 1)
        XCTAssertEqual(tokens[0].column, 1)
        
        let multiTokens = try TOMLLexer.tokenize("a = 1\nb = 2")
        guard let bToken = multiTokens.first(where: { $0.type == "BARE_KEY" && $0.value == "b" }) else {
            XCTFail("Could not find b token")
            return
        }
        XCTAssertEqual(bToken.line, 2)
    }
    
    // =========================================================================
    // Edge Cases
    // =========================================================================
    
    func testEdgeCases() throws {
        let t1 = try TOMLLexer.tokenize("")
        XCTAssertEqual(t1.count, 1)
        XCTAssertEqual(t1[0].type, "EOF")
        
        let t2 = try TOMLLexer.tokenize("   \t  ")
        XCTAssertEqual(t2.count, 1)
        XCTAssertEqual(t2[0].type, "EOF")
        
        let t3 = try meaningfulTypes("# just a comment")
        XCTAssertEqual(t3.count, 0)
    }
}

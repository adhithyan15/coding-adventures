// ============================================================================
// ParserTests.swift — Tests for the grammar-driven parser.
// ============================================================================

import XCTest
import Lexer
import GrammarTools
@testable import Parser

final class ParserTests: XCTestCase {

    // -----------------------------------------------------------------------
    // MARK: - Test Helpers
    // -----------------------------------------------------------------------

    /// Create a token with sensible defaults for testing.
    private func tok(_ type: String, _ value: String, line: Int = 1, column: Int = 1) -> Token {
        Token(type: type, value: value, line: line, column: column)
    }

    /// Create an EOF token.
    private func eof(line: Int = 1, column: Int = 1) -> Token {
        Token(type: "EOF", value: "", line: line, column: column)
    }

    // -----------------------------------------------------------------------
    // MARK: - Basic Parsing
    // -----------------------------------------------------------------------

    func testParseSingleTokenRule() throws {
        // Grammar: factor = NUMBER ;
        let grammar = try parseParserGrammar(source: "factor = NUMBER ;")
        let tokens = [tok("NUMBER", "42"), eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        let ast = try parser.parse()

        XCTAssertEqual(ast.ruleName, "factor")
        XCTAssertEqual(ast.children.count, 1)
        if case .token(let t) = ast.children[0] {
            XCTAssertEqual(t.type, "NUMBER")
            XCTAssertEqual(t.value, "42")
        } else {
            XCTFail("Expected token child")
        }
    }

    func testParseSequence() throws {
        // Grammar: assignment = NAME EQUALS NUMBER ;
        let grammar = try parseParserGrammar(source: "assignment = NAME EQUALS NUMBER ;")
        let tokens = [
            tok("NAME", "x", column: 1),
            tok("EQUALS", "=", column: 3),
            tok("NUMBER", "5", column: 5),
            eof(),
        ]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        let ast = try parser.parse()

        XCTAssertEqual(ast.ruleName, "assignment")
        XCTAssertEqual(ast.children.count, 3)
    }

    func testParseAlternation() throws {
        // Grammar: atom = NUMBER | NAME ;
        let grammar = try parseParserGrammar(source: "atom = NUMBER | NAME ;")

        // Try NUMBER
        let tokens1 = [tok("NUMBER", "42"), eof()]
        let parser1 = GrammarParser(tokens: tokens1, grammar: grammar)
        let ast1 = try parser1.parse()
        XCTAssertEqual(ast1.ruleName, "atom")
        if case .token(let t) = ast1.children[0] {
            XCTAssertEqual(t.type, "NUMBER")
        } else {
            XCTFail("Expected token")
        }

        // Try NAME
        let tokens2 = [tok("NAME", "x"), eof()]
        let parser2 = GrammarParser(tokens: tokens2, grammar: grammar)
        let ast2 = try parser2.parse()
        if case .token(let t) = ast2.children[0] {
            XCTAssertEqual(t.type, "NAME")
        } else {
            XCTFail("Expected token")
        }
    }

    func testParseRepetition() throws {
        // Grammar: program = { NUMBER } ;
        let grammar = try parseParserGrammar(source: "program = { NUMBER } ;")

        // Zero elements
        let tokens0 = [eof()]
        let parser0 = GrammarParser(tokens: tokens0, grammar: grammar)
        let ast0 = try parser0.parse()
        XCTAssertEqual(ast0.children.count, 0)

        // Two elements
        let tokens2 = [tok("NUMBER", "1"), tok("NUMBER", "2"), eof()]
        let parser2 = GrammarParser(tokens: tokens2, grammar: grammar)
        let ast2 = try parser2.parse()
        XCTAssertEqual(ast2.children.count, 2)
    }

    func testParseOptional() throws {
        // Grammar: maybe_num = [ NUMBER ] ;
        let grammar = try parseParserGrammar(source: "maybe_num = [ NUMBER ] ;")

        // Without optional
        let tokens0 = [eof()]
        let parser0 = GrammarParser(tokens: tokens0, grammar: grammar)
        let ast0 = try parser0.parse()
        XCTAssertEqual(ast0.children.count, 0)

        // With optional
        let tokens1 = [tok("NUMBER", "7"), eof()]
        let parser1 = GrammarParser(tokens: tokens1, grammar: grammar)
        let ast1 = try parser1.parse()
        XCTAssertEqual(ast1.children.count, 1)
    }

    func testParseLiteral() throws {
        // Grammar: op = "+" ;
        let grammar = try parseParserGrammar(source: #"op = "+" ;"#)
        let tokens = [tok("PLUS", "+"), eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        let ast = try parser.parse()

        XCTAssertEqual(ast.children.count, 1)
        if case .token(let t) = ast.children[0] {
            XCTAssertEqual(t.value, "+")
        } else {
            XCTFail("Expected token")
        }
    }

    func testParseRuleReference() throws {
        // Grammar: program = expr ; expr = NUMBER ;
        let grammar = try parseParserGrammar(source: """
        program = expr ;
        expr = NUMBER ;
        """)
        let tokens = [tok("NUMBER", "42"), eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        let ast = try parser.parse()

        XCTAssertEqual(ast.ruleName, "program")
        XCTAssertEqual(ast.children.count, 1)
        if case .node(let exprNode) = ast.children[0] {
            XCTAssertEqual(exprNode.ruleName, "expr")
        } else {
            XCTFail("Expected node child")
        }
    }

    // -----------------------------------------------------------------------
    // MARK: - New Element Types
    // -----------------------------------------------------------------------

    func testParsePositiveLookahead() throws {
        // Grammar: safe_num = & NUMBER NUMBER ;
        // Positive lookahead checks but doesn't consume
        let grammar = try parseParserGrammar(source: "safe_num = & NUMBER NUMBER ;")
        let tokens = [tok("NUMBER", "42"), eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        let ast = try parser.parse()

        // Lookahead produces no children, then NUMBER matches
        XCTAssertEqual(ast.ruleName, "safe_num")
        XCTAssertEqual(ast.children.count, 1) // Only the consumed NUMBER
    }

    func testParsePositiveLookaheadFails() throws {
        // Grammar: safe_num = & NUMBER NAME ;
        // Positive lookahead for NUMBER, but we have NAME first
        let grammar = try parseParserGrammar(source: "safe_num = & NUMBER NAME ;")
        let tokens = [tok("NAME", "x"), eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)

        XCTAssertThrowsError(try parser.parse())
    }

    func testParseNegativeLookahead() throws {
        // Grammar: not_number = ! NUMBER NAME ;
        // Negative lookahead succeeds when NUMBER doesn't match
        let grammar = try parseParserGrammar(source: "not_number = ! NUMBER NAME ;")
        let tokens = [tok("NAME", "x"), eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        let ast = try parser.parse()

        XCTAssertEqual(ast.ruleName, "not_number")
        XCTAssertEqual(ast.children.count, 1) // Only the consumed NAME
    }

    func testParseNegativeLookaheadFails() throws {
        // Grammar: not_number = ! NUMBER NUMBER ;
        // Negative lookahead fails when NUMBER matches
        let grammar = try parseParserGrammar(source: "not_number = ! NUMBER NUMBER ;")
        let tokens = [tok("NUMBER", "42"), eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)

        XCTAssertThrowsError(try parser.parse())
    }

    func testParseOneOrMore() throws {
        // Grammar: nums = NUMBER + ;
        let grammar = try parseParserGrammar(source: "nums = NUMBER + ;")

        // One element
        let tokens1 = [tok("NUMBER", "1"), eof()]
        let parser1 = GrammarParser(tokens: tokens1, grammar: grammar)
        let ast1 = try parser1.parse()
        XCTAssertEqual(ast1.children.count, 1)

        // Three elements
        let tokens3 = [
            tok("NUMBER", "1"),
            tok("NUMBER", "2"),
            tok("NUMBER", "3"),
            eof(),
        ]
        let parser3 = GrammarParser(tokens: tokens3, grammar: grammar)
        let ast3 = try parser3.parse()
        XCTAssertEqual(ast3.children.count, 3)
    }

    func testParseOneOrMoreFailsOnZero() throws {
        // Grammar: nums = NUMBER + ;
        let grammar = try parseParserGrammar(source: "nums = NUMBER + ;")
        let tokens = [eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)

        XCTAssertThrowsError(try parser.parse())
    }

    func testParseSeparatedRepetition() throws {
        // Grammar: args = NUMBER // COMMA ;
        let grammar = try parseParserGrammar(source: "args = NUMBER // COMMA ;")

        // One element (no separator needed)
        let tokens1 = [tok("NUMBER", "1"), eof()]
        let parser1 = GrammarParser(tokens: tokens1, grammar: grammar)
        let ast1 = try parser1.parse()
        XCTAssertEqual(ast1.children.count, 1)

        // Three elements separated by commas
        let tokens3 = [
            tok("NUMBER", "1"),
            tok("COMMA", ","),
            tok("NUMBER", "2"),
            tok("COMMA", ","),
            tok("NUMBER", "3"),
            eof(),
        ]
        let parser3 = GrammarParser(tokens: tokens3, grammar: grammar)
        let ast3 = try parser3.parse()
        XCTAssertEqual(ast3.children.count, 5) // 3 numbers + 2 commas
    }

    func testParseSeparatedRepetitionFailsOnZero() throws {
        // Grammar: args = NUMBER // COMMA ;
        let grammar = try parseParserGrammar(source: "args = NUMBER // COMMA ;")
        let tokens = [eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)

        XCTAssertThrowsError(try parser.parse())
    }

    // -----------------------------------------------------------------------
    // MARK: - AST Position Tracking
    // -----------------------------------------------------------------------

    func testASTPositionsFromChildren() throws {
        let grammar = try parseParserGrammar(source: "stmt = NAME EQUALS NUMBER ;")
        let tokens = [
            tok("NAME", "x", line: 1, column: 1),
            tok("EQUALS", "=", line: 1, column: 3),
            tok("NUMBER", "5", line: 1, column: 5),
            eof(line: 1, column: 6),
        ]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        let ast = try parser.parse()

        XCTAssertEqual(ast.startLine, 1)
        XCTAssertEqual(ast.startColumn, 1)
        XCTAssertEqual(ast.endLine, 1)
        XCTAssertEqual(ast.endColumn, 5)
    }

    func testASTPositionsMultiLine() throws {
        let grammar = try parseParserGrammar(source: """
        program = expr ;
        expr = NUMBER ;
        """)
        let tokens = [
            tok("NUMBER", "42", line: 3, column: 5),
            eof(line: 3, column: 7),
        ]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        let ast = try parser.parse()

        // The program node's positions come from its child (expr node)
        if case .node(let exprNode) = ast.children[0] {
            XCTAssertEqual(exprNode.startLine, 3)
            XCTAssertEqual(exprNode.startColumn, 5)
        }
    }

    // -----------------------------------------------------------------------
    // MARK: - Left Recursion
    // -----------------------------------------------------------------------

    func testLeftRecursiveGrammar() throws {
        // Grammar: expr = expr PLUS NUMBER | NUMBER ;
        // This is left-recursive -- Warth's algorithm should handle it.
        let grammar = try parseParserGrammar(source: "expr = expr PLUS NUMBER | NUMBER ;")
        let tokens = [
            tok("NUMBER", "1"),
            tok("PLUS", "+"),
            tok("NUMBER", "2"),
            tok("PLUS", "+"),
            tok("NUMBER", "3"),
            eof(),
        ]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        let ast = try parser.parse()

        // Should consume all tokens
        XCTAssertEqual(ast.ruleName, "expr")
        let allTokens = collectTokens(from: ast)
        XCTAssertEqual(allTokens.count, 5)
    }

    // -----------------------------------------------------------------------
    // MARK: - Error Reporting
    // -----------------------------------------------------------------------

    func testFurthestFailureReporting() throws {
        let grammar = try parseParserGrammar(source: "expr = NUMBER PLUS NUMBER ;")
        let tokens = [
            tok("NUMBER", "1", line: 1, column: 1),
            tok("PLUS", "+", line: 1, column: 3),
            tok("PLUS", "+", line: 1, column: 5), // Wrong! Should be NUMBER
            eof(),
        ]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)

        XCTAssertThrowsError(try parser.parse()) { error in
            let gpe = error as! GrammarParseError
            XCTAssertTrue(gpe.message.contains("NUMBER") || gpe.message.contains("Expected"))
        }
    }

    func testEmptyGrammarError() throws {
        let grammar = ParserGrammar(rules: [])
        let tokens = [eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)

        XCTAssertThrowsError(try parser.parse()) { error in
            let gpe = error as! GrammarParseError
            XCTAssertTrue(gpe.message.contains("no rules"))
        }
    }

    func testUnconsumedTokensError() throws {
        let grammar = try parseParserGrammar(source: "atom = NUMBER ;")
        let tokens = [
            tok("NUMBER", "1"),
            tok("PLUS", "+"),
            eof(),
        ]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)

        XCTAssertThrowsError(try parser.parse()) { error in
            let gpe = error as! GrammarParseError
            XCTAssertTrue(gpe.message.contains("Unexpected") || gpe.message.contains("Expected"))
        }
    }

    // -----------------------------------------------------------------------
    // MARK: - Hooks
    // -----------------------------------------------------------------------

    func testPreParseHook() throws {
        let grammar = try parseParserGrammar(source: "prog = NUMBER ;")
        let tokens = [
            tok("WHITESPACE", " "),
            tok("NUMBER", "42"),
            eof(),
        ]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)

        // Filter out whitespace tokens
        parser.addPreParse { toks in
            toks.removeAll { $0.type == "WHITESPACE" }
        }

        let ast = try parser.parse()
        XCTAssertEqual(ast.children.count, 1)
    }

    func testPostParseHook() throws {
        let grammar = try parseParserGrammar(source: "prog = NUMBER ;")
        let tokens = [tok("NUMBER", "42"), eof()]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)

        // Wrap the AST in a new node
        parser.addPostParse { ast in
            ASTNode(
                ruleName: "wrapper",
                children: [.node(ast)]
            )
        }

        let ast = try parser.parse()
        XCTAssertEqual(ast.ruleName, "wrapper")
    }

    // -----------------------------------------------------------------------
    // MARK: - Newline Significance
    // -----------------------------------------------------------------------

    func testNewlinesInsignificantByDefault() throws {
        let grammar = try parseParserGrammar(source: "pair = NUMBER NUMBER ;")
        let tokens = [
            tok("NUMBER", "1"),
            tok("NEWLINE", "\n"),
            tok("NUMBER", "2"),
            eof(),
        ]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        XCTAssertFalse(parser.isNewlinesSignificant)
        let ast = try parser.parse()
        XCTAssertEqual(ast.children.count, 2) // Newline skipped
    }

    func testNewlinesSignificantWhenReferenced() throws {
        let grammar = try parseParserGrammar(source: "line = NUMBER NEWLINE ;")
        let tokens = [
            tok("NUMBER", "1"),
            tok("NEWLINE", "\n"),
            eof(),
        ]
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        XCTAssertTrue(parser.isNewlinesSignificant)
        let ast = try parser.parse()
        XCTAssertEqual(ast.children.count, 2) // NUMBER + NEWLINE
    }
}

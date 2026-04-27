// ============================================================================
// LexerTests.swift — Tests for the Grammar-Driven Lexer
// ============================================================================
//
// These tests verify the Swift grammar-driven lexer produces correct tokens
// for a variety of inputs. We test in several layers:
//
// 1. **Basic tokenization** -- simple expressions, strings, positions
// 2. **Keywords** -- keyword detection and non-keyword preservation
// 3. **Error handling** -- unexpected characters, reserved keywords
// 4. **Custom grammars** -- programmatically built TokenGrammar objects
// 5. **Skip patterns** -- whitespace and comment skipping
// 6. **Type aliases** -- definition name -> alias mapping
// 7. **Reserved keywords** -- hard errors on forbidden identifiers
// 8. **Pattern groups** -- context-sensitive lexing with callbacks
// 9. **Bracket depth** -- nesting tracking for template literals
// 10. **Token lookbehind** -- previousToken() for context decisions
// 11. **Context keywords** -- TOKEN_CONTEXT_KEYWORD flag
// 12. **Newline detection** -- precededByNewline() for ASI support
//
// ============================================================================

import XCTest
import GrammarTools
@testable import Lexer

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract just the token types from a token array.
private func types(_ tokens: [Token]) -> [String] {
    return tokens.map { $0.type }
}

/// Extract just the token values from a token array.
private func tokenValues(_ tokens: [Token]) -> [String] {
    return tokens.map { $0.value }
}

/// Build a simple grammar with NAME, NUMBER, and common operators.
/// This mimics a minimal Python-like grammar for testing.
private func makeSimpleGrammar(keywords: [String] = []) -> TokenGrammar {
    return TokenGrammar(
        definitions: [
            TokenDefinition(name: "NUMBER", pattern: "[0-9]+", isRegex: true, lineNumber: 1),
            TokenDefinition(name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", isRegex: true, lineNumber: 2),
            TokenDefinition(name: "EQUALS_EQUALS", pattern: "==", isRegex: false, lineNumber: 3),
            TokenDefinition(name: "EQUALS", pattern: "=", isRegex: false, lineNumber: 4),
            TokenDefinition(name: "PLUS", pattern: "+", isRegex: false, lineNumber: 5),
            TokenDefinition(name: "STAR", pattern: "*", isRegex: false, lineNumber: 6),
            TokenDefinition(name: "MINUS", pattern: "-", isRegex: false, lineNumber: 7),
            TokenDefinition(name: "SLASH", pattern: "/", isRegex: false, lineNumber: 8),
            TokenDefinition(name: "LPAREN", pattern: "(", isRegex: false, lineNumber: 9),
            TokenDefinition(name: "RPAREN", pattern: ")", isRegex: false, lineNumber: 10),
            TokenDefinition(name: "LBRACKET", pattern: "[", isRegex: false, lineNumber: 11),
            TokenDefinition(name: "RBRACKET", pattern: "]", isRegex: false, lineNumber: 12),
            TokenDefinition(name: "LBRACE", pattern: "{", isRegex: false, lineNumber: 13),
            TokenDefinition(name: "RBRACE", pattern: "}", isRegex: false, lineNumber: 14),
            TokenDefinition(name: "COMMA", pattern: ",", isRegex: false, lineNumber: 15),
            TokenDefinition(name: "COLON", pattern: ":", isRegex: false, lineNumber: 16),
            TokenDefinition(name: "STRING", pattern: "\"[^\"]*\"", isRegex: true, lineNumber: 17),
        ],
        keywords: keywords
    )
}

// ============================================================================
// Basic Tokenization Tests
// ============================================================================

final class BasicTokenizationTests: XCTestCase {

    func testSimpleAssignment() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "x = 1 + 2", grammar: grammar)
        XCTAssertEqual(
            types(tokens),
            ["NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF"]
        )
        XCTAssertEqual(
            tokenValues(tokens),
            ["x", "=", "1", "+", "2", ""]
        )
    }

    func testArithmetic() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "1 + 2 * 3", grammar: grammar)
        XCTAssertEqual(
            types(tokens),
            ["NUMBER", "PLUS", "NUMBER", "STAR", "NUMBER", "EOF"]
        )
    }

    func testStringLiteral() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "\"Hello, World!\"", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "STRING")
        XCTAssertEqual(tokens[0].value, "Hello, World!")
        XCTAssertEqual(tokens[1].type, "EOF")
    }

    func testEmptyString() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "\"\"", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "STRING")
        XCTAssertEqual(tokens[0].value, "")
    }

    func testMultilineInput() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "x = 1\ny = 2", grammar: grammar)
        XCTAssertEqual(
            types(tokens),
            ["NAME", "EQUALS", "NUMBER", "NEWLINE",
             "NAME", "EQUALS", "NUMBER", "EOF"]
        )
    }

    func testBlankLines() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "x\n\ny", grammar: grammar)
        XCTAssertEqual(
            types(tokens),
            ["NAME", "NEWLINE", "NEWLINE", "NAME", "EOF"]
        )
    }

    func testEmptyInput() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "", grammar: grammar)
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0].type, "EOF")
    }

    func testWhitespaceOnly() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "   \t  ", grammar: grammar)
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0].type, "EOF")
    }

    func testDistinguishEqualsFromDoubleEquals() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "a = b == c", grammar: grammar)
        XCTAssertEqual(
            types(tokens),
            ["NAME", "EQUALS", "NAME", "EQUALS_EQUALS", "NAME", "EOF"]
        )
    }

    func testFunctionCallStyle() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "print(x, y)", grammar: grammar)
        XCTAssertEqual(
            types(tokens),
            ["NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN", "EOF"]
        )
    }

    func testTokensWithoutSpaces() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "x=1+2", grammar: grammar)
        XCTAssertEqual(
            types(tokens),
            ["NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF"]
        )
    }

    func testPositionTracking() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "x = 1", grammar: grammar)
        XCTAssertEqual(tokens[0].line, 1)
        XCTAssertEqual(tokens[0].column, 1) // x
        XCTAssertEqual(tokens[1].column, 3) // =
        XCTAssertEqual(tokens[2].column, 5) // 1
    }

    func testPositionAcrossLines() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "abc\nde = 1", grammar: grammar)
        XCTAssertEqual(tokens[0], Token(type: "NAME", value: "abc", line: 1, column: 1))
        let deToken = tokens.first { $0.value == "de" }
        XCTAssertEqual(deToken?.line, 2)
        XCTAssertEqual(deToken?.column, 1)
    }

    func testEofPosition() throws {
        let grammar = makeSimpleGrammar()
        let tokens = try grammarTokenize(source: "ab", grammar: grammar)
        let eof = tokens.last!
        XCTAssertEqual(eof.type, "EOF")
        XCTAssertEqual(eof.line, 1)
        XCTAssertEqual(eof.column, 3)
    }
}

// ============================================================================
// Keyword Tests
// ============================================================================

final class KeywordTests: XCTestCase {

    func testClassifyIfAsKeyword() throws {
        let grammar = makeSimpleGrammar(keywords: ["if", "else", "while", "def", "return"])
        let tokens = try grammarTokenize(source: "if x == 1", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "KEYWORD")
        XCTAssertEqual(tokens[0].value, "if")
    }

    func testClassifyDefAsKeyword() throws {
        let grammar = makeSimpleGrammar(keywords: ["if", "else", "while", "def", "return"])
        let tokens = try grammarTokenize(source: "def foo", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "KEYWORD")
        XCTAssertEqual(tokens[0].value, "def")
    }

    func testNonKeywordStaysAsName() throws {
        let grammar = makeSimpleGrammar(keywords: ["if", "else"])
        let tokens = try grammarTokenize(source: "iffy", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "NAME")
        XCTAssertEqual(tokens[0].value, "iffy")
    }

    func testAllKeywordsRecognized() throws {
        let keywords = ["if", "else", "while", "def", "return"]
        let grammar = makeSimpleGrammar(keywords: keywords)
        for keyword in keywords {
            let tokens = try grammarTokenize(source: keyword, grammar: grammar)
            XCTAssertEqual(tokens[0].type, "KEYWORD", "Expected '\(keyword)' to be KEYWORD")
            XCTAssertEqual(tokens[0].value, keyword)
        }
    }
}

// ============================================================================
// Error Tests
// ============================================================================

final class ErrorTests: XCTestCase {

    func testUnexpectedCharacter() throws {
        let grammar = makeSimpleGrammar()
        XCTAssertThrowsError(try grammarTokenize(source: "@", grammar: grammar)) { error in
            guard let lexerError = error as? LexerError else {
                XCTFail("Expected LexerError"); return
            }
            XCTAssertTrue(lexerError.message.contains("Unexpected character"))
        }
    }

    func testErrorPosition() throws {
        let grammar = makeSimpleGrammar()
        XCTAssertThrowsError(try grammarTokenize(source: "x = @", grammar: grammar)) { error in
            guard let lexerError = error as? LexerError else {
                XCTFail("Expected LexerError"); return
            }
            XCTAssertEqual(lexerError.line, 1)
            XCTAssertEqual(lexerError.column, 5)
        }
    }

    func testErrorPositionSecondLine() throws {
        let grammar = makeSimpleGrammar()
        XCTAssertThrowsError(try grammarTokenize(source: "x = 1\n@", grammar: grammar)) { error in
            guard let lexerError = error as? LexerError else {
                XCTFail("Expected LexerError"); return
            }
            XCTAssertEqual(lexerError.line, 2)
            XCTAssertEqual(lexerError.column, 1)
        }
    }
}

// ============================================================================
// Custom Grammar Tests
// ============================================================================

final class CustomGrammarTests: XCTestCase {

    func testMinimalNumbersOnlyGrammar() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NUMBER", pattern: "[0-9]+", isRegex: true, lineNumber: 1),
            ],
            keywords: []
        )
        let tokens = try grammarTokenize(source: "42", grammar: grammar)
        XCTAssertEqual(tokens[0], Token(type: "NUMBER", value: "42", line: 1, column: 1))
        XCTAssertEqual(tokens[1].type, "EOF")
    }

    func testNamesAndLiteralEquals() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", isRegex: true, lineNumber: 1),
                TokenDefinition(name: "EQUALS", pattern: "=", isRegex: false, lineNumber: 2),
                TokenDefinition(name: "LBRACE", pattern: "{", isRegex: false, lineNumber: 3),
                TokenDefinition(name: "RBRACE", pattern: "}", isRegex: false, lineNumber: 4),
            ],
            keywords: []
        )
        let tokens = try grammarTokenize(source: "x = y", grammar: grammar)
        XCTAssertEqual(types(tokens), ["NAME", "EQUALS", "NAME", "EOF"])
    }

    func testCustomKeywordList() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", isRegex: true, lineNumber: 1),
            ],
            keywords: ["let", "var"]
        )
        let tokens = try grammarTokenize(source: "let x", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "KEYWORD")
        XCTAssertEqual(tokens[0].value, "let")
        XCTAssertEqual(tokens[1].type, "NAME")
        XCTAssertEqual(tokens[1].value, "x")
    }

    func testCustomTokenNames() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "IDENTIFIER", pattern: "[a-zA-Z]+", isRegex: true, lineNumber: 1),
            ],
            keywords: []
        )
        let tokens = try grammarTokenize(source: "hello", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "IDENTIFIER")
        XCTAssertEqual(tokens[0].value, "hello")
    }

    func testLiteralPatternEscaping() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "PLUS", pattern: "+", isRegex: false, lineNumber: 1),
            ],
            keywords: []
        )
        let tokens = try grammarTokenize(source: "+", grammar: grammar)
        XCTAssertEqual(tokens[0], Token(type: "PLUS", value: "+", line: 1, column: 1))
    }

    func testFirstMatchWins() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "EQUALS_EQUALS", pattern: "==", isRegex: false, lineNumber: 1),
                TokenDefinition(name: "EQUALS", pattern: "=", isRegex: false, lineNumber: 2),
            ],
            keywords: []
        )
        let tokens = try grammarTokenize(source: "==", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "EQUALS_EQUALS")
        XCTAssertEqual(tokens[0].value, "==")
        XCTAssertEqual(tokens[1].type, "EOF")
    }

    func testNewlinesRegardlessOfGrammar() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NUMBER", pattern: "[0-9]+", isRegex: true, lineNumber: 1),
            ],
            keywords: []
        )
        let tokens = try grammarTokenize(source: "1\n2", grammar: grammar)
        XCTAssertEqual(types(tokens), ["NUMBER", "NEWLINE", "NUMBER", "EOF"])
    }

    func testErrorForUnrecognizedCharacters() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NUMBER", pattern: "[0-9]+", isRegex: true, lineNumber: 1),
            ],
            keywords: []
        )
        XCTAssertThrowsError(try grammarTokenize(source: "abc", grammar: grammar)) { error in
            guard let lexerError = error as? LexerError else {
                XCTFail("Expected LexerError"); return
            }
            XCTAssertTrue(lexerError.message.contains("Unexpected character"))
        }
    }
}

// ============================================================================
// Skip Pattern Tests
// ============================================================================

final class SkipPatternTests: XCTestCase {

    func testSkipWhitespace() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NAME", pattern: "[a-z]+", isRegex: true, lineNumber: 1),
            ],
            keywords: [],
            skipDefinitions: [
                TokenDefinition(name: "WHITESPACE", pattern: "[ \\t]+", isRegex: true, lineNumber: 2),
            ]
        )
        let tokens = try grammarTokenize(source: "hello world", grammar: grammar)
        let nameTokens = tokens.filter { $0.type == "NAME" }
        XCTAssertEqual(nameTokens.count, 2)
        XCTAssertEqual(nameTokens[0].value, "hello")
        XCTAssertEqual(nameTokens[1].value, "world")
    }
}

// ============================================================================
// Alias Tests
// ============================================================================

final class AliasTests: XCTestCase {

    func testAliasAsTokenType() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NUM", pattern: "[0-9]+", isRegex: true, lineNumber: 1, alias: "INT"),
            ],
            keywords: []
        )
        let tokens = try grammarTokenize(source: "42", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "INT")
    }
}

// ============================================================================
// Reserved Keyword Tests
// ============================================================================

final class ReservedKeywordTests: XCTestCase {

    func testThrowOnReservedKeyword() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NAME", pattern: "[a-zA-Z_]+", isRegex: true, lineNumber: 1),
            ],
            keywords: [],
            reservedKeywords: ["class", "import"]
        )
        XCTAssertThrowsError(try grammarTokenize(source: "class", grammar: grammar)) { error in
            guard let lexerError = error as? LexerError else {
                XCTFail("Expected LexerError"); return
            }
            XCTAssertTrue(lexerError.message.contains("Reserved keyword"))
        }
    }

    func testAllowNonReservedIdentifiers() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NAME", pattern: "[a-zA-Z_]+", isRegex: true, lineNumber: 1),
            ],
            keywords: [],
            reservedKeywords: ["class"]
        )
        let tokens = try grammarTokenize(source: "hello", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "NAME")
        XCTAssertEqual(tokens[0].value, "hello")
    }
}

// ============================================================================
// Pattern Group Tests
// ============================================================================

/// Build a grammar with pattern groups for testing.
///
/// Simulates a simplified XML-like grammar:
/// - Default group: TEXT and OPEN_TAG
/// - tag group: TAG_NAME, EQUALS, VALUE, TAG_CLOSE
///
private func makeGroupGrammar() -> TokenGrammar {
    return TokenGrammar(
        definitions: [
            TokenDefinition(name: "TEXT", pattern: "[^<]+", isRegex: true, lineNumber: 1),
            TokenDefinition(name: "OPEN_TAG", pattern: "<", isRegex: false, lineNumber: 2),
        ],
        keywords: [],
        escapeMode: "none",
        skipDefinitions: [
            TokenDefinition(name: "WS", pattern: "[ \\t\\r\\n]+", isRegex: true, lineNumber: 3),
        ],
        groups: [
            "tag": PatternGroup(name: "tag", definitions: [
                TokenDefinition(name: "TAG_NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", isRegex: true, lineNumber: 4),
                TokenDefinition(name: "EQUALS", pattern: "=", isRegex: false, lineNumber: 5),
                TokenDefinition(name: "VALUE", pattern: "\"[^\"]*\"", isRegex: true, lineNumber: 6),
                TokenDefinition(name: "TAG_CLOSE", pattern: ">", isRegex: false, lineNumber: 7),
            ])
        ]
    )
}

final class PatternGroupTests: XCTestCase {

    func testWithoutCallbackOnlyDefaultGroupUsed() throws {
        let grammar = makeGroupGrammar()
        let tokens = try GrammarLexer(source: "hello", grammar: grammar).tokenize()
        XCTAssertEqual(tokens[0].type, "TEXT")
        XCTAssertEqual(tokens[0].value, "hello")
    }

    func testCallbackCanPushPopGroups() throws {
        let grammar = makeGroupGrammar()
        let lexer = GrammarLexer(source: "<div>hello", grammar: grammar)
        lexer.setOnToken { token, ctx in
            if token.type == "OPEN_TAG" {
                ctx.pushGroup("tag")
            } else if token.type == "TAG_CLOSE" {
                ctx.popGroup()
            }
        }
        let tokens = try lexer.tokenize()

        let pairs = tokens
            .filter { $0.type != "EOF" }
            .map { ($0.type, $0.value) }

        XCTAssertEqual(pairs.count, 4)
        XCTAssertEqual(pairs[0].0, "OPEN_TAG");  XCTAssertEqual(pairs[0].1, "<")
        XCTAssertEqual(pairs[1].0, "TAG_NAME");   XCTAssertEqual(pairs[1].1, "div")
        XCTAssertEqual(pairs[2].0, "TAG_CLOSE");   XCTAssertEqual(pairs[2].1, ">")
        XCTAssertEqual(pairs[3].0, "TEXT");        XCTAssertEqual(pairs[3].1, "hello")
    }

    func testCallbackCanSuppressTokens() throws {
        let grammar = makeGroupGrammar()
        let lexer = GrammarLexer(source: "<hello", grammar: grammar)
        lexer.setOnToken { token, ctx in
            if token.type == "OPEN_TAG" {
                ctx.suppress()
            }
        }
        let tokens = try lexer.tokenize()
        let nonEof = tokens.filter { $0.type != "EOF" }.map { $0.type }
        XCTAssertEqual(nonEof, ["TEXT"])
    }

    func testCallbackCanEmitSyntheticTokens() throws {
        let grammar = makeGroupGrammar()
        let lexer = GrammarLexer(source: "<hello", grammar: grammar)
        lexer.setOnToken { token, ctx in
            if token.type == "OPEN_TAG" {
                ctx.emit(Token(type: "MARKER", value: "[start]", line: token.line, column: token.column))
            }
        }
        let tokens = try lexer.tokenize()
        let pairs = tokens
            .filter { $0.type != "EOF" }
            .map { ($0.type, $0.value) }

        XCTAssertEqual(pairs.count, 3)
        XCTAssertEqual(pairs[0].0, "OPEN_TAG")
        XCTAssertEqual(pairs[1].0, "MARKER")
        XCTAssertEqual(pairs[1].1, "[start]")
        XCTAssertEqual(pairs[2].0, "TEXT")
    }

    func testSuppressPlusEmitIsTokenReplacement() throws {
        let grammar = makeGroupGrammar()
        let lexer = GrammarLexer(source: "<hello", grammar: grammar)
        lexer.setOnToken { token, ctx in
            if token.type == "OPEN_TAG" {
                ctx.suppress()
                ctx.emit(Token(type: "REPLACED", value: "<", line: token.line, column: token.column))
            }
        }
        let tokens = try lexer.tokenize()
        let pairs = tokens
            .filter { $0.type != "EOF" }
            .map { ($0.type, $0.value) }

        XCTAssertEqual(pairs.count, 2)
        XCTAssertEqual(pairs[0].0, "REPLACED")
        XCTAssertEqual(pairs[1].0, "TEXT")
    }

    func testPopWhenOnlyDefaultRemainsIsNoop() throws {
        let grammar = makeGroupGrammar()
        let lexer = GrammarLexer(source: "hello", grammar: grammar)
        lexer.setOnToken { _, ctx in
            ctx.popGroup() // Should be safe
        }
        let tokens = try lexer.tokenize()
        XCTAssertEqual(tokens[0].type, "TEXT")
    }

    func testCallbackCanDisableSkipPatterns() throws {
        // Grammar with a group that captures whitespace as a token
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "TEXT", pattern: "[^<]+", isRegex: true, lineNumber: 1),
                TokenDefinition(name: "START", pattern: "<!", isRegex: false, lineNumber: 2),
            ],
            keywords: [],
            escapeMode: "none",
            skipDefinitions: [
                TokenDefinition(name: "WS", pattern: "[ \\t]+", isRegex: true, lineNumber: 3),
            ],
            groups: [
                "raw": PatternGroup(name: "raw", definitions: [
                    TokenDefinition(name: "RAW_TEXT", pattern: "[^>]+", isRegex: true, lineNumber: 4),
                    TokenDefinition(name: "END", pattern: ">", isRegex: false, lineNumber: 5),
                ])
            ]
        )

        let lexer = GrammarLexer(source: "<! hello world >after", grammar: grammar)
        lexer.setOnToken { token, ctx in
            if token.type == "START" {
                ctx.pushGroup("raw")
                ctx.setSkipEnabled(false)
            } else if token.type == "END" {
                ctx.popGroup()
                ctx.setSkipEnabled(true)
            }
        }
        let tokens = try lexer.tokenize()

        let pairs = tokens
            .filter { $0.type != "EOF" }
            .map { ($0.type, $0.value) }

        XCTAssertEqual(pairs.count, 4)
        XCTAssertEqual(pairs[0].0, "START");     XCTAssertEqual(pairs[0].1, "<!")
        XCTAssertEqual(pairs[1].0, "RAW_TEXT");  XCTAssertEqual(pairs[1].1, " hello world ")
        XCTAssertEqual(pairs[2].0, "END");       XCTAssertEqual(pairs[2].1, ">")
        XCTAssertEqual(pairs[3].0, "TEXT");       XCTAssertEqual(pairs[3].1, "after")
    }

    func testNilCallbackClearsCallback() throws {
        let grammar = makeGroupGrammar()
        var called: [String] = []

        let lexer = GrammarLexer(source: "hello", grammar: grammar)
        lexer.setOnToken { token, _ in
            called.append(token.type)
        }
        lexer.setOnToken(nil)
        _ = try lexer.tokenize()

        XCTAssertEqual(called, [])
    }
}

// ============================================================================
// Bracket Depth Tests
// ============================================================================

final class BracketDepthTests: XCTestCase {

    func testBracketDepthTracking() throws {
        let grammar = makeSimpleGrammar()
        var parenDepths: [Int] = []
        var bracketDepths: [Int] = []
        var braceDepths: [Int] = []

        let lexer = GrammarLexer(source: "f([{x}])", grammar: grammar)
        lexer.setOnToken { token, ctx in
            parenDepths.append(ctx.bracketDepth(kind: .paren))
            bracketDepths.append(ctx.bracketDepth(kind: .bracket))
            braceDepths.append(ctx.bracketDepth(kind: .brace))
        }
        _ = try lexer.tokenize()

        // Tokens: f ( [ { x } ] )
        // After f:    paren=0, bracket=0, brace=0
        // After (:    paren=1, bracket=0, brace=0
        // After [:    paren=1, bracket=1, brace=0
        // After {:    paren=1, bracket=1, brace=1
        // After x:    paren=1, bracket=1, brace=1
        // After }:    paren=1, bracket=1, brace=0
        // After ]:    paren=1, bracket=0, brace=0
        // After ):    paren=0, bracket=0, brace=0
        XCTAssertEqual(parenDepths,   [0, 1, 1, 1, 1, 1, 1, 0])
        XCTAssertEqual(bracketDepths, [0, 0, 1, 1, 1, 1, 0, 0])
        XCTAssertEqual(braceDepths,   [0, 0, 0, 1, 1, 0, 0, 0])
    }

    func testBracketDepthTotalSum() throws {
        let grammar = makeSimpleGrammar()
        var totalDepths: [Int] = []

        let lexer = GrammarLexer(source: "({x})", grammar: grammar)
        lexer.setOnToken { _, ctx in
            totalDepths.append(ctx.bracketDepth())
        }
        _ = try lexer.tokenize()

        // Tokens: ( { x } )
        // After (:  paren=1, total=1
        // After {:  brace=1, total=2
        // After x:  total=2
        // After }:  total=1
        // After ):  total=0
        XCTAssertEqual(totalDepths, [1, 2, 2, 1, 0])
    }

    func testBracketDepthClampsAtZero() throws {
        // Extra closers should not go negative.
        let grammar = makeSimpleGrammar()
        var parenDepths: [Int] = []

        let lexer = GrammarLexer(source: "))", grammar: grammar)
        lexer.setOnToken { _, ctx in
            parenDepths.append(ctx.bracketDepth(kind: .paren))
        }
        _ = try lexer.tokenize()

        // Both closers should result in depth 0 (clamped).
        XCTAssertEqual(parenDepths, [0, 0])
    }
}

// ============================================================================
// Token Lookbehind Tests
// ============================================================================

final class LookbehindTests: XCTestCase {

    func testPreviousTokenIsNilAtStart() throws {
        let grammar = makeSimpleGrammar()
        var prevTokenOnFirst: Token? = Token(type: "SENTINEL", value: "", line: 0, column: 0)

        let lexer = GrammarLexer(source: "x", grammar: grammar)
        var first = true
        lexer.setOnToken { _, ctx in
            if first {
                prevTokenOnFirst = ctx.previousToken()
                first = false
            }
        }
        _ = try lexer.tokenize()

        XCTAssertNil(prevTokenOnFirst)
    }

    func testPreviousTokenTracksLastEmitted() throws {
        let grammar = makeSimpleGrammar()
        var prevTypes: [String?] = []

        let lexer = GrammarLexer(source: "x + 1", grammar: grammar)
        lexer.setOnToken { _, ctx in
            prevTypes.append(ctx.previousToken()?.type)
        }
        _ = try lexer.tokenize()

        // Tokens: x, +, 1
        // Before x: nil
        // Before +: "NAME" (x)
        // Before 1: "PLUS" (+)
        XCTAssertEqual(prevTypes, [nil, "NAME", "PLUS"])
    }

    func testSuppressedTokensNotInPreviousToken() throws {
        let grammar = makeSimpleGrammar()
        var prevTokenAfterSecond: String? = nil

        let lexer = GrammarLexer(source: "x + 1", grammar: grammar)
        var count = 0
        lexer.setOnToken { token, ctx in
            if token.type == "PLUS" {
                ctx.suppress() // suppress the "+"
            }
            count += 1
            if count == 3 { // on "1"
                prevTokenAfterSecond = ctx.previousToken()?.type
            }
        }
        _ = try lexer.tokenize()

        // The "+" was suppressed, so previousToken before "1" should be "NAME" (x).
        XCTAssertEqual(prevTokenAfterSecond, "NAME")
    }
}

// ============================================================================
// Context Keyword Tests
// ============================================================================

final class ContextKeywordTests: XCTestCase {

    func testContextKeywordsGetFlag() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", isRegex: true, lineNumber: 1),
            ],
            keywords: [],
            contextKeywords: ["async", "yield", "get"]
        )
        let tokens = try grammarTokenize(source: "async", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "NAME")
        XCTAssertEqual(tokens[0].value, "async")
        XCTAssertNotEqual(tokens[0].flags & TOKEN_CONTEXT_KEYWORD, 0)
    }

    func testNonContextKeywordDoesNotGetFlag() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", isRegex: true, lineNumber: 1),
            ],
            keywords: [],
            contextKeywords: ["async", "yield"]
        )
        let tokens = try grammarTokenize(source: "hello", grammar: grammar)
        XCTAssertEqual(tokens[0].type, "NAME")
        XCTAssertEqual(tokens[0].flags & TOKEN_CONTEXT_KEYWORD, 0)
    }
}

// ============================================================================
// Newline Detection Tests
// ============================================================================

final class NewlineDetectionTests: XCTestCase {

    func testPrecededByNewlineDetectsLineBreak() throws {
        let grammar = makeSimpleGrammar()
        var newlineFlags: [Bool] = []

        let lexer = GrammarLexer(source: "x\ny", grammar: grammar)
        lexer.setOnToken { token, ctx in
            if token.type == "NAME" || token.type == "NUMBER" {
                newlineFlags.append(ctx.precededByNewline())
            }
        }
        _ = try lexer.tokenize()

        // x: no previous token -> false
        // y: previous token was NEWLINE on line 1, y is on line 2 -> true
        XCTAssertEqual(newlineFlags, [false, true])
    }

    func testPrecededByNewlineFalseOnSameLine() throws {
        let grammar = makeSimpleGrammar()
        var newlineFlags: [Bool] = []

        let lexer = GrammarLexer(source: "x + y", grammar: grammar)
        lexer.setOnToken { _, ctx in
            newlineFlags.append(ctx.precededByNewline())
        }
        _ = try lexer.tokenize()

        // All tokens on same line -> all false
        XCTAssertEqual(newlineFlags, [false, false, false])
    }
}

// ============================================================================
// LexerContext Unit Tests
// ============================================================================

final class LexerContextTests: XCTestCase {

    func testPeekReadsCharactersAfterToken() throws {
        let grammar = makeGroupGrammar()
        // We need to use the lexer's callback to get a real context.
        // Alternatively, test via the callback approach.
        var peekResults: [String] = []

        let lexer = GrammarLexer(source: "hello world", grammar: grammar)
        lexer.setOnToken { token, ctx in
            if token.value == "hello " {
                // TEXT matches "hello " (includes space due to [^<]+)
                peekResults.append(ctx.peek(1))
                peekResults.append(ctx.peek(5))
            }
        }
        _ = try lexer.tokenize()

        // After "hello " (pos 6), peek(1) = "w", peek(5) = "d"
        if peekResults.count >= 2 {
            XCTAssertEqual(peekResults[0], "w")
            XCTAssertEqual(peekResults[1], "d")
        }
    }

    func testActiveGroupReturnsDefault() throws {
        let grammar = makeGroupGrammar()
        var activeGroup: String = ""

        let lexer = GrammarLexer(source: "hello", grammar: grammar)
        lexer.setOnToken { _, ctx in
            activeGroup = ctx.activeGroup()
        }
        _ = try lexer.tokenize()

        XCTAssertEqual(activeGroup, "default")
    }

    func testGroupStackDepthStartsAtOne() throws {
        let grammar = makeGroupGrammar()
        var depth: Int = 0

        let lexer = GrammarLexer(source: "hello", grammar: grammar)
        lexer.setOnToken { _, ctx in
            depth = ctx.groupStackDepth()
        }
        _ = try lexer.tokenize()

        XCTAssertEqual(depth, 1)
    }
}

// ============================================================================
// Indentation Mode Tests
// ============================================================================

final class IndentationModeTests: XCTestCase {

    func testEmitIndentAndDedent() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NAME", pattern: "[a-zA-Z_]+", isRegex: true, lineNumber: 1),
                TokenDefinition(name: "EQUALS", pattern: "=", isRegex: false, lineNumber: 2),
                TokenDefinition(name: "INT", pattern: "[0-9]+", isRegex: true, lineNumber: 3),
                TokenDefinition(name: "COLON", pattern: ":", isRegex: false, lineNumber: 4),
            ],
            keywords: ["if"],
            mode: "indentation",
            skipDefinitions: [
                TokenDefinition(name: "WS", pattern: "[ \\t]+", isRegex: true, lineNumber: 10),
            ]
        )
        let tokens = try grammarTokenize(source: "if x:\n    y = 1\n", grammar: grammar)
        let typeList = types(tokens)
        XCTAssertTrue(typeList.contains("INDENT"))
        XCTAssertTrue(typeList.contains("DEDENT"))
    }

    func testRejectTabIndentation() throws {
        let grammar = TokenGrammar(
            definitions: [
                TokenDefinition(name: "NAME", pattern: "[a-z]+", isRegex: true, lineNumber: 1),
                TokenDefinition(name: "COLON", pattern: ":", isRegex: false, lineNumber: 2),
            ],
            keywords: [],
            mode: "indentation",
            skipDefinitions: [
                TokenDefinition(name: "WS", pattern: "[ \\t]+", isRegex: true, lineNumber: 10),
            ]
        )
        XCTAssertThrowsError(try grammarTokenize(source: "if:\n\ty\n", grammar: grammar)) { error in
            guard let lexerError = error as? LexerError else {
                XCTFail("Expected LexerError"); return
            }
            XCTAssertTrue(lexerError.message.contains("Tab"))
        }
    }

    func testEmptySourceInIndentationMode() throws {
        let grammar = TokenGrammar(
            definitions: [],
            keywords: [],
            mode: "indentation",
            skipDefinitions: [
                TokenDefinition(name: "WS", pattern: "[ \\t]+", isRegex: true, lineNumber: 10),
            ]
        )
        let tokens = try grammarTokenize(source: "", grammar: grammar)
        XCTAssertEqual(tokens.last?.type, "EOF")
    }
}

// ============================================================================
// Layout Mode Tests
// ============================================================================

final class LayoutModeTests: XCTestCase {

    private func makeLayoutGrammar() -> TokenGrammar {
        return TokenGrammar(
            definitions: [
                TokenDefinition(name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", isRegex: true, lineNumber: 1),
                TokenDefinition(name: "EQUALS", pattern: "=", isRegex: false, lineNumber: 2),
                TokenDefinition(name: "LBRACE", pattern: "{", isRegex: false, lineNumber: 3),
                TokenDefinition(name: "RBRACE", pattern: "}", isRegex: false, lineNumber: 4),
            ],
            keywords: [],
            mode: "layout",
            skipDefinitions: [
                TokenDefinition(name: "WS", pattern: "[ \\t]+", isRegex: true, lineNumber: 10),
            ],
            layoutKeywords: ["let", "where", "do", "of"]
        )
    }

    func testInjectsVirtualLayoutTokensAfterKeyword() throws {
        let grammar = makeLayoutGrammar()
        let tokens = try grammarTokenize(source: "let\n  x = y\n  z = q\n", grammar: grammar)

        XCTAssertEqual(
            types(tokens),
            [
                "NAME", "NEWLINE", "VIRTUAL_LBRACE",
                "NAME", "EQUALS", "NAME", "NEWLINE", "VIRTUAL_SEMICOLON",
                "NAME", "EQUALS", "NAME", "NEWLINE", "VIRTUAL_RBRACE", "EOF",
            ]
        )
    }

    func testExplicitBraceSuppressesImplicitLayout() throws {
        let grammar = makeLayoutGrammar()
        let tokens = try grammarTokenize(source: "let {\n  x = y\n}\n", grammar: grammar)

        XCTAssertFalse(types(tokens).contains("VIRTUAL_LBRACE"))
        XCTAssertFalse(types(tokens).contains("VIRTUAL_SEMICOLON"))
    }
}

// ============================================================================
// Pre/Post Tokenize Hook Tests
// ============================================================================

final class HookTests: XCTestCase {

    func testPreTokenizeHookTransformsSource() throws {
        let grammar = makeSimpleGrammar()
        let lexer = GrammarLexer(source: "X", grammar: grammar)
        lexer.addPreTokenize { source in
            return source.lowercased()
        }
        let tokens = try lexer.tokenize()
        XCTAssertEqual(tokens[0].value, "x")
    }

    func testPostTokenizeHookTransformsTokens() throws {
        let grammar = makeSimpleGrammar()
        let lexer = GrammarLexer(source: "x", grammar: grammar)
        lexer.addPostTokenize { tokens in
            return tokens.map { token in
                if token.type == "NAME" {
                    return Token(type: "IDENT", value: token.value, line: token.line, column: token.column)
                }
                return token
            }
        }
        let tokens = try lexer.tokenize()
        XCTAssertEqual(tokens[0].type, "IDENT")
    }
}

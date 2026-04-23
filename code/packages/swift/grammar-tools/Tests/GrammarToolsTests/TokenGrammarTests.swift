// ============================================================================
// TokenGrammarTests.swift — Tests for token grammar parsing and validation.
// ============================================================================

import XCTest
@testable import GrammarTools

final class TokenGrammarTests: XCTestCase {

    // -----------------------------------------------------------------------
    // MARK: - Basic Parsing
    // -----------------------------------------------------------------------

    func testParseSimpleDefinitions() throws {
        let source = """
        NUMBER = /[0-9]+/
        PLUS = "+"
        MINUS = "-"
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.definitions.count, 3)
        XCTAssertEqual(grammar.definitions[0].name, "NUMBER")
        XCTAssertEqual(grammar.definitions[0].pattern, "[0-9]+")
        XCTAssertTrue(grammar.definitions[0].isRegex)
        XCTAssertEqual(grammar.definitions[0].lineNumber, 1)

        XCTAssertEqual(grammar.definitions[1].name, "PLUS")
        XCTAssertEqual(grammar.definitions[1].pattern, "+")
        XCTAssertFalse(grammar.definitions[1].isRegex)

        XCTAssertEqual(grammar.definitions[2].name, "MINUS")
        XCTAssertEqual(grammar.definitions[2].pattern, "-")
    }

    func testParseAliasedDefinitions() throws {
        let source = """
        IDENT = /[a-z]+/ -> NAME
        ARROW = "->" -> FAT_ARROW
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.definitions[0].name, "IDENT")
        XCTAssertEqual(grammar.definitions[0].alias, "NAME")
        XCTAssertEqual(grammar.definitions[1].name, "ARROW")
        XCTAssertEqual(grammar.definitions[1].alias, "FAT_ARROW")
    }

    func testParseKeywordsSection() throws {
        let source = """
        NAME = /[a-z]+/
        keywords:
            if
            else
            while
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.keywords, ["if", "else", "while"])
    }

    func testParseReservedSection() throws {
        let source = """
        NAME = /[a-z]+/
        reserved:
            class
            import
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.reservedKeywords, ["class", "import"])
    }

    func testParseContextKeywordsSection() throws {
        let source = """
        NAME = /[a-z]+/
        context_keywords:
            async
            yield
            get
            set
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.contextKeywords, ["async", "yield", "get", "set"])
    }

    func testParseLayoutKeywordsSection() throws {
        let source = """
        mode: layout
        NAME = /[a-z]+/
        layout_keywords:
            let
            where
            do
            of
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.mode, "layout")
        XCTAssertEqual(grammar.layoutKeywords, ["let", "where", "do", "of"])
    }

    func testParseSkipSection() throws {
        let source = """
        NUMBER = /[0-9]+/
        skip:
            WHITESPACE = /[ \\t]+/
            COMMENT = /\\/\\/.*/
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertNotNil(grammar.skipDefinitions)
        XCTAssertEqual(grammar.skipDefinitions?.count, 2)
        XCTAssertEqual(grammar.skipDefinitions?[0].name, "WHITESPACE")
        XCTAssertEqual(grammar.skipDefinitions?[1].name, "COMMENT")
    }

    func testParseModeDirective() throws {
        let source = """
        mode: indentation
        NAME = /[a-z]+/
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.mode, "indentation")
    }

    func testParsePatternGroups() throws {
        let source = """
        OPEN_TAG = "<"
        group tag:
            ATTR_NAME = /[a-z]+/
            ATTR_EQ = "="
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertNotNil(grammar.groups)
        XCTAssertNotNil(grammar.groups?["tag"])
        XCTAssertEqual(grammar.groups?["tag"]?.definitions.count, 2)
        XCTAssertEqual(grammar.groups?["tag"]?.definitions[0].name, "ATTR_NAME")
    }

    // -----------------------------------------------------------------------
    // MARK: - Magic Comments
    // -----------------------------------------------------------------------

    func testParseMagicCommentVersion() throws {
        let source = """
        # @version 3
        NUMBER = /[0-9]+/
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.version, 3)
    }

    func testParseMagicCommentCaseInsensitive() throws {
        let source = """
        # @case_insensitive true
        NAME = /[a-z]+/
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertTrue(grammar.caseInsensitive)
    }

    func testDefaultVersionIsZero() throws {
        let source = "NUMBER = /[0-9]+/"
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.version, 0)
        XCTAssertFalse(grammar.caseInsensitive)
    }

    // -----------------------------------------------------------------------
    // MARK: - Comments and Blank Lines
    // -----------------------------------------------------------------------

    func testSkipCommentsAndBlanks() throws {
        let source = """
        # This is a comment
        NUMBER = /[0-9]+/

        # Another comment
        PLUS = "+"
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.definitions.count, 2)
    }

    // -----------------------------------------------------------------------
    // MARK: - Error Cases
    // -----------------------------------------------------------------------

    func testMissingPattern() {
        let source = "NUMBER ="
        XCTAssertThrowsError(try parseTokenGrammar(source: source)) { error in
            let tge = error as! TokenGrammarError
            XCTAssertEqual(tge.lineNumber, 1)
        }
    }

    func testUnclosedRegex() {
        let source = "NUMBER = /[0-9]+"
        XCTAssertThrowsError(try parseTokenGrammar(source: source))
    }

    func testEmptyRegex() {
        let source = "NUMBER = //"
        XCTAssertThrowsError(try parseTokenGrammar(source: source))
    }

    func testInvalidTokenName() {
        let source = "123BAD = /[0-9]+/"
        XCTAssertThrowsError(try parseTokenGrammar(source: source))
    }

    func testMissingAliasAfterArrow() {
        let source = "IDENT = /[a-z]+/ ->"
        XCTAssertThrowsError(try parseTokenGrammar(source: source))
    }

    func testDuplicateGroupName() {
        let source = """
        group tag:
            A = "a"
        group tag:
            B = "b"
        """
        XCTAssertThrowsError(try parseTokenGrammar(source: source))
    }

    func testReservedGroupName() {
        let source = """
        group default:
            A = "a"
        """
        XCTAssertThrowsError(try parseTokenGrammar(source: source))
    }

    // -----------------------------------------------------------------------
    // MARK: - Token Name Helpers
    // -----------------------------------------------------------------------

    func testTokenNamesIncludesAliases() throws {
        let source = """
        IDENT = /[a-z]+/ -> NAME
        NUMBER = /[0-9]+/
        """
        let grammar = try parseTokenGrammar(source: source)
        let names = tokenNames(grammar)

        XCTAssertTrue(names.contains("IDENT"))
        XCTAssertTrue(names.contains("NAME"))
        XCTAssertTrue(names.contains("NUMBER"))
    }

    func testEffectiveTokenNamesUsesAliases() throws {
        let source = """
        IDENT = /[a-z]+/ -> NAME
        NUMBER = /[0-9]+/
        """
        let grammar = try parseTokenGrammar(source: source)
        let names = effectiveTokenNames(grammar)

        XCTAssertTrue(names.contains("NAME"))
        XCTAssertTrue(names.contains("NUMBER"))
        // IDENT is replaced by its alias NAME
        XCTAssertFalse(names.contains("IDENT"))
    }

    // -----------------------------------------------------------------------
    // MARK: - Validation
    // -----------------------------------------------------------------------

    func testValidationDuplicateNames() throws {
        let source = """
        NUMBER = /[0-9]+/
        NUMBER = /\\d+/
        """
        let grammar = try parseTokenGrammar(source: source)
        let issues = validateTokenGrammar(grammar)

        XCTAssertTrue(issues.contains { $0.contains("Duplicate") })
    }

    func testValidationNonUpperCase() throws {
        let source = "number = /[0-9]+/"
        let grammar = try parseTokenGrammar(source: source)
        let issues = validateTokenGrammar(grammar)

        XCTAssertTrue(issues.contains { $0.contains("UPPER_CASE") })
    }

    func testValidationUnknownMode() throws {
        let grammar = TokenGrammar(
            definitions: [],
            keywords: [],
            mode: "unknown_mode"
        )
        let issues = validateTokenGrammar(grammar)

        XCTAssertTrue(issues.contains { $0.contains("Unknown mode") })
    }

    func testValidationLayoutModeRequiresLayoutKeywords() throws {
        let grammar = TokenGrammar(
            definitions: [],
            keywords: [],
            mode: "layout"
        )
        let issues = validateTokenGrammar(grammar)

        XCTAssertTrue(issues.contains { $0.contains("layout_keywords") })
    }

    func testValidationEmptyGroup() throws {
        let grammar = TokenGrammar(
            definitions: [],
            keywords: [],
            groups: ["empty": PatternGroup(name: "empty", definitions: [])]
        )
        let issues = validateTokenGrammar(grammar)

        XCTAssertTrue(issues.contains { $0.contains("Empty pattern group") })
    }

    func testValidGrammarNoIssues() throws {
        let source = """
        NUMBER = /[0-9]+/
        PLUS = "+"
        NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
        keywords:
            if
            else
        """
        let grammar = try parseTokenGrammar(source: source)
        let issues = validateTokenGrammar(grammar)

        XCTAssertTrue(issues.isEmpty, "Expected no issues, got: \(issues)")
    }

    // -----------------------------------------------------------------------
    // MARK: - Section Transitions
    // -----------------------------------------------------------------------

    func testNonIndentedLineExitsSection() throws {
        let source = """
        keywords:
            if
            else
        NUMBER = /[0-9]+/
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.keywords, ["if", "else"])
        XCTAssertEqual(grammar.definitions.count, 1)
        XCTAssertEqual(grammar.definitions[0].name, "NUMBER")
    }

    func testMultipleSections() throws {
        let source = """
        NAME = /[a-z]+/
        NUMBER = /[0-9]+/
        keywords:
            if
            else
        reserved:
            class
        context_keywords:
            async
            yield
        skip:
            WS = /[ \\t]+/
        """
        let grammar = try parseTokenGrammar(source: source)

        XCTAssertEqual(grammar.definitions.count, 2)
        XCTAssertEqual(grammar.keywords, ["if", "else"])
        XCTAssertEqual(grammar.reservedKeywords, ["class"])
        XCTAssertEqual(grammar.contextKeywords, ["async", "yield"])
        XCTAssertEqual(grammar.skipDefinitions?.count, 1)
    }
}

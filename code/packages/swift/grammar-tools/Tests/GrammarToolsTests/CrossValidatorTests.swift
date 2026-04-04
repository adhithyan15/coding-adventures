// ============================================================================
// CrossValidatorTests.swift — Tests for cross-validation of grammar files.
// ============================================================================

import XCTest
@testable import GrammarTools

final class CrossValidatorTests: XCTestCase {

    // -----------------------------------------------------------------------
    // MARK: - Consistent Grammars
    // -----------------------------------------------------------------------

    func testConsistentGrammarsNoIssues() throws {
        let tokenSource = """
        NUMBER = /[0-9]+/
        PLUS = "+"
        SEMI = ";"
        """
        let grammarSource = """
        program = { statement } ;
        statement = expression SEMI ;
        expression = NUMBER { PLUS NUMBER } ;
        """

        let tokenGrammar = try parseTokenGrammar(source: tokenSource)
        let parserGrammar = try parseParserGrammar(source: grammarSource)
        let issues = crossValidate(tokenGrammar: tokenGrammar, parserGrammar: parserGrammar)

        XCTAssertTrue(issues.isEmpty, "Expected no issues, got: \(issues)")
    }

    // -----------------------------------------------------------------------
    // MARK: - Missing Token References
    // -----------------------------------------------------------------------

    func testMissingTokenReference() throws {
        let tokenSource = """
        NUMBER = /[0-9]+/
        """
        let grammarSource = """
        expr = NUMBER PLUS NUMBER ;
        """

        let tokenGrammar = try parseTokenGrammar(source: tokenSource)
        let parserGrammar = try parseParserGrammar(source: grammarSource)
        let issues = crossValidate(tokenGrammar: tokenGrammar, parserGrammar: parserGrammar)

        XCTAssertTrue(issues.contains { $0.contains("Error") && $0.contains("PLUS") })
    }

    // -----------------------------------------------------------------------
    // MARK: - Unused Tokens
    // -----------------------------------------------------------------------

    func testUnusedTokenWarning() throws {
        let tokenSource = """
        NUMBER = /[0-9]+/
        TILDE = "~"
        PLUS = "+"
        """
        let grammarSource = """
        expr = NUMBER PLUS NUMBER ;
        """

        let tokenGrammar = try parseTokenGrammar(source: tokenSource)
        let parserGrammar = try parseParserGrammar(source: grammarSource)
        let issues = crossValidate(tokenGrammar: tokenGrammar, parserGrammar: parserGrammar)

        XCTAssertTrue(issues.contains { $0.contains("Warning") && $0.contains("TILDE") })
        // NUMBER and PLUS are used, so no warnings for them
        XCTAssertFalse(issues.contains { $0.contains("Warning") && $0.contains("NUMBER") })
        XCTAssertFalse(issues.contains { $0.contains("Warning") && $0.contains("PLUS") })
    }

    // -----------------------------------------------------------------------
    // MARK: - Implicit Tokens
    // -----------------------------------------------------------------------

    func testEOFIsAlwaysImplicit() throws {
        let tokenSource = """
        NUMBER = /[0-9]+/
        """
        let grammarSource = """
        program = NUMBER EOF ;
        """

        let tokenGrammar = try parseTokenGrammar(source: tokenSource)
        let parserGrammar = try parseParserGrammar(source: grammarSource)
        let issues = crossValidate(tokenGrammar: tokenGrammar, parserGrammar: parserGrammar)

        // EOF should not be flagged as missing
        XCTAssertFalse(issues.contains { $0.contains("Error") && $0.contains("EOF") })
    }

    func testNEWLINEIsImplicit() throws {
        let tokenSource = """
        NUMBER = /[0-9]+/
        """
        let grammarSource = """
        line = NUMBER NEWLINE ;
        """

        let tokenGrammar = try parseTokenGrammar(source: tokenSource)
        let parserGrammar = try parseParserGrammar(source: grammarSource)
        let issues = crossValidate(tokenGrammar: tokenGrammar, parserGrammar: parserGrammar)

        XCTAssertFalse(issues.contains { $0.contains("Error") && $0.contains("NEWLINE") })
    }

    func testIndentationModeImplicitTokens() throws {
        let tokenSource = """
        mode: indentation
        NUMBER = /[0-9]+/
        """
        let grammarSource = """
        block = INDENT NUMBER DEDENT ;
        """

        let tokenGrammar = try parseTokenGrammar(source: tokenSource)
        let parserGrammar = try parseParserGrammar(source: grammarSource)
        let issues = crossValidate(tokenGrammar: tokenGrammar, parserGrammar: parserGrammar)

        XCTAssertFalse(issues.contains { $0.contains("Error") && $0.contains("INDENT") })
        XCTAssertFalse(issues.contains { $0.contains("Error") && $0.contains("DEDENT") })
    }

    // -----------------------------------------------------------------------
    // MARK: - Aliases
    // -----------------------------------------------------------------------

    func testAliasedTokensCountAsUsed() throws {
        let tokenSource = """
        IDENT = /[a-z]+/ -> NAME
        """
        let grammarSource = """
        program = NAME ;
        """

        let tokenGrammar = try parseTokenGrammar(source: tokenSource)
        let parserGrammar = try parseParserGrammar(source: grammarSource)
        let issues = crossValidate(tokenGrammar: tokenGrammar, parserGrammar: parserGrammar)

        // IDENT has alias NAME which is used, so no warning
        XCTAssertFalse(issues.contains { $0.contains("Warning") && $0.contains("IDENT") })
    }
}

// ============================================================================
// ParserGrammarTests.swift — Tests for parser grammar parsing and validation.
// ============================================================================

import XCTest
@testable import GrammarTools

final class ParserGrammarTests: XCTestCase {

    // -----------------------------------------------------------------------
    // MARK: - Basic Parsing
    // -----------------------------------------------------------------------

    func testParseSingleRule() throws {
        let source = "program = statement ;"
        let grammar = try parseParserGrammar(source: source)

        XCTAssertEqual(grammar.rules.count, 1)
        XCTAssertEqual(grammar.rules[0].name, "program")
        XCTAssertEqual(grammar.rules[0].body, .ruleReference("statement"))
    }

    func testParseTokenReference() throws {
        let source = "factor = NUMBER ;"
        let grammar = try parseParserGrammar(source: source)

        XCTAssertEqual(grammar.rules[0].body, .tokenReference("NUMBER"))
    }

    func testParseLiteral() throws {
        let source = #"op = "+" ;"#
        let grammar = try parseParserGrammar(source: source)

        XCTAssertEqual(grammar.rules[0].body, .literal("+"))
    }

    func testParseSequence() throws {
        let source = "assignment = NAME EQUALS expression ;"
        let grammar = try parseParserGrammar(source: source)

        if case .sequence(let elements) = grammar.rules[0].body {
            XCTAssertEqual(elements.count, 3)
            XCTAssertEqual(elements[0], .tokenReference("NAME"))
            XCTAssertEqual(elements[1], .tokenReference("EQUALS"))
            XCTAssertEqual(elements[2], .ruleReference("expression"))
        } else {
            XCTFail("Expected sequence")
        }
    }

    func testParseAlternation() throws {
        let source = "atom = NUMBER | NAME | STRING ;"
        let grammar = try parseParserGrammar(source: source)

        if case .alternation(let choices) = grammar.rules[0].body {
            XCTAssertEqual(choices.count, 3)
            XCTAssertEqual(choices[0], .tokenReference("NUMBER"))
            XCTAssertEqual(choices[1], .tokenReference("NAME"))
            XCTAssertEqual(choices[2], .tokenReference("STRING"))
        } else {
            XCTFail("Expected alternation")
        }
    }

    func testParseRepetition() throws {
        let source = "program = { statement } ;"
        let grammar = try parseParserGrammar(source: source)

        if case .repetition(let element) = grammar.rules[0].body {
            XCTAssertEqual(element, .ruleReference("statement"))
        } else {
            XCTFail("Expected repetition")
        }
    }

    func testParseOptional() throws {
        let source = "if_stmt = IF expression block [ ELSE block ] ;"
        let grammar = try parseParserGrammar(source: source)

        if case .sequence(let elements) = grammar.rules[0].body {
            XCTAssertEqual(elements.count, 4)
            if case .optional(let inner) = elements[3] {
                if case .sequence(let optElements) = inner {
                    XCTAssertEqual(optElements.count, 2)
                } else {
                    XCTFail("Expected sequence inside optional")
                }
            } else {
                XCTFail("Expected optional")
            }
        } else {
            XCTFail("Expected sequence")
        }
    }

    func testParseGroup() throws {
        let source = "expr = term { ( PLUS | MINUS ) term } ;"
        let grammar = try parseParserGrammar(source: source)

        // The body is a sequence: term, repetition(sequence(group(alt), term))
        if case .sequence(let elements) = grammar.rules[0].body {
            XCTAssertEqual(elements.count, 2)
            if case .repetition(let repElement) = elements[1] {
                if case .sequence(let repSeq) = repElement {
                    XCTAssertEqual(repSeq.count, 2)
                    if case .group(let groupElement) = repSeq[0] {
                        if case .alternation(let choices) = groupElement {
                            XCTAssertEqual(choices.count, 2)
                        } else {
                            XCTFail("Expected alternation inside group")
                        }
                    } else {
                        XCTFail("Expected group")
                    }
                } else {
                    XCTFail("Expected sequence inside repetition")
                }
            } else {
                XCTFail("Expected repetition")
            }
        } else {
            XCTFail("Expected sequence")
        }
    }

    // -----------------------------------------------------------------------
    // MARK: - New Element Types
    // -----------------------------------------------------------------------

    func testParsePositiveLookahead() throws {
        let source = "line_end = & NEWLINE ;"
        let grammar = try parseParserGrammar(source: source)

        if case .positiveLookahead(let element) = grammar.rules[0].body {
            XCTAssertEqual(element, .tokenReference("NEWLINE"))
        } else {
            XCTFail("Expected positive lookahead, got \(grammar.rules[0].body)")
        }
    }

    func testParseNegativeLookahead() throws {
        let source = #"non_else = ! "else" NAME ;"#
        let grammar = try parseParserGrammar(source: source)

        if case .sequence(let elements) = grammar.rules[0].body {
            XCTAssertEqual(elements.count, 2)
            if case .negativeLookahead(let lookElement) = elements[0] {
                XCTAssertEqual(lookElement, .literal("else"))
            } else {
                XCTFail("Expected negative lookahead")
            }
            XCTAssertEqual(elements[1], .tokenReference("NAME"))
        } else {
            XCTFail("Expected sequence")
        }
    }

    func testParseOneOrMore() throws {
        let source = "statements = statement + ;"
        let grammar = try parseParserGrammar(source: source)

        if case .oneOrMore(let element) = grammar.rules[0].body {
            XCTAssertEqual(element, .ruleReference("statement"))
        } else {
            XCTFail("Expected oneOrMore, got \(grammar.rules[0].body)")
        }
    }

    func testParseSeparatedRepetition() throws {
        let source = "args = expression // COMMA ;"
        let grammar = try parseParserGrammar(source: source)

        if case .separatedRepetition(let element, let separator) = grammar.rules[0].body {
            XCTAssertEqual(element, .ruleReference("expression"))
            XCTAssertEqual(separator, .tokenReference("COMMA"))
        } else {
            XCTFail("Expected separated repetition, got \(grammar.rules[0].body)")
        }
    }

    func testParseOneOrMoreWithSeparator() throws {
        // element + // separator is also valid
        let source = "params = param + // COMMA ;"
        let grammar = try parseParserGrammar(source: source)

        if case .separatedRepetition(let element, let separator) = grammar.rules[0].body {
            XCTAssertEqual(element, .ruleReference("param"))
            XCTAssertEqual(separator, .tokenReference("COMMA"))
        } else {
            XCTFail("Expected separated repetition, got \(grammar.rules[0].body)")
        }
    }

    // -----------------------------------------------------------------------
    // MARK: - Multiple Rules
    // -----------------------------------------------------------------------

    func testParseMultipleRules() throws {
        let source = """
        program = { statement } ;
        statement = expression SEMI ;
        expression = NUMBER ;
        """
        let grammar = try parseParserGrammar(source: source)

        XCTAssertEqual(grammar.rules.count, 3)
        XCTAssertEqual(grammar.rules[0].name, "program")
        XCTAssertEqual(grammar.rules[1].name, "statement")
        XCTAssertEqual(grammar.rules[2].name, "expression")
    }

    // -----------------------------------------------------------------------
    // MARK: - Magic Comments
    // -----------------------------------------------------------------------

    func testMagicCommentVersion() throws {
        let source = """
        # @version 2
        program = statement ;
        """
        let grammar = try parseParserGrammar(source: source)

        XCTAssertEqual(grammar.version, 2)
    }

    func testDefaultVersionZero() throws {
        let source = "program = statement ;"
        let grammar = try parseParserGrammar(source: source)

        XCTAssertEqual(grammar.version, 0)
    }

    // -----------------------------------------------------------------------
    // MARK: - Comments and Blanks
    // -----------------------------------------------------------------------

    func testSkipCommentsAndBlanks() throws {
        let source = """
        # Top-level comment
        program = statement ;

        # Another comment
        statement = NUMBER ;
        """
        let grammar = try parseParserGrammar(source: source)

        XCTAssertEqual(grammar.rules.count, 2)
    }

    // -----------------------------------------------------------------------
    // MARK: - Error Cases
    // -----------------------------------------------------------------------

    func testUnterminatedString() {
        let source = #"op = "unterminated ;"#
        XCTAssertThrowsError(try parseParserGrammar(source: source))
    }

    func testMissingSemicolon() {
        let source = "program = statement"
        XCTAssertThrowsError(try parseParserGrammar(source: source))
    }

    func testUnexpectedCharacter() {
        let source = "program = @ ;"
        XCTAssertThrowsError(try parseParserGrammar(source: source))
    }

    func testEmptySequence() {
        // An empty alternation branch should fail
        let source = "program = | NUMBER ;"
        XCTAssertThrowsError(try parseParserGrammar(source: source))
    }

    // -----------------------------------------------------------------------
    // MARK: - Reference Extraction
    // -----------------------------------------------------------------------

    func testTokenReferences() throws {
        let source = """
        expr = NUMBER PLUS term ;
        term = NUMBER ;
        """
        let grammar = try parseParserGrammar(source: source)
        let refs = tokenReferences(grammar)

        XCTAssertTrue(refs.contains("NUMBER"))
        XCTAssertTrue(refs.contains("PLUS"))
        XCTAssertEqual(refs.count, 2)
    }

    func testRuleReferences() throws {
        let source = """
        program = { statement } ;
        statement = expression SEMI ;
        expression = NUMBER ;
        """
        let grammar = try parseParserGrammar(source: source)
        let refs = ruleReferences(grammar)

        XCTAssertTrue(refs.contains("statement"))
        XCTAssertTrue(refs.contains("expression"))
        XCTAssertFalse(refs.contains("program"))
    }

    func testTokenReferencesInNewElements() throws {
        let source = """
        expr = arg // COMMA ;
        safe = & NEWLINE NUMBER ;
        block = ! EOF statement + ;
        """
        let grammar = try parseParserGrammar(source: source)
        let refs = tokenReferences(grammar)

        XCTAssertTrue(refs.contains("COMMA"))
        XCTAssertTrue(refs.contains("NEWLINE"))
        XCTAssertTrue(refs.contains("EOF"))
        XCTAssertTrue(refs.contains("NUMBER"))
    }

    // -----------------------------------------------------------------------
    // MARK: - Validation
    // -----------------------------------------------------------------------

    func testValidationDuplicateRules() throws {
        let grammar = ParserGrammar(rules: [
            GrammarRule(name: "program", body: .ruleReference("statement"), lineNumber: 1),
            GrammarRule(name: "program", body: .tokenReference("NUMBER"), lineNumber: 2),
        ])
        let issues = validateParserGrammar(grammar)

        XCTAssertTrue(issues.contains { $0.contains("Duplicate rule name") })
    }

    func testValidationUndefinedRuleRef() throws {
        let grammar = ParserGrammar(rules: [
            GrammarRule(name: "program", body: .ruleReference("undefined_rule"), lineNumber: 1),
        ])
        let issues = validateParserGrammar(grammar)

        XCTAssertTrue(issues.contains { $0.contains("Undefined rule reference") })
    }

    func testValidationUndefinedTokenRef() throws {
        let grammar = ParserGrammar(rules: [
            GrammarRule(name: "program", body: .tokenReference("MISSING_TOKEN"), lineNumber: 1),
        ])
        let tokenSet: Set<String> = ["NUMBER", "PLUS"]
        let issues = validateParserGrammar(grammar, tokenNamesSet: tokenSet)

        XCTAssertTrue(issues.contains { $0.contains("Undefined token reference") })
    }

    func testValidationSyntheticTokensAllowed() throws {
        let grammar = ParserGrammar(rules: [
            GrammarRule(name: "program", body: .sequence([
                .tokenReference("NEWLINE"),
                .tokenReference("EOF"),
                .tokenReference("INDENT"),
                .tokenReference("DEDENT"),
            ]), lineNumber: 1),
        ])
        let tokenSet: Set<String> = []
        let issues = validateParserGrammar(grammar, tokenNamesSet: tokenSet)

        // Synthetic tokens should not produce undefined warnings
        XCTAssertFalse(issues.contains { $0.contains("Undefined token reference") })
    }

    func testValidationUnreachableRule() throws {
        let grammar = ParserGrammar(rules: [
            GrammarRule(name: "program", body: .tokenReference("NUMBER"), lineNumber: 1),
            GrammarRule(name: "orphan", body: .tokenReference("STRING"), lineNumber: 2),
        ])
        let issues = validateParserGrammar(grammar)

        XCTAssertTrue(issues.contains { $0.contains("unreachable") })
    }

    func testValidationNonLowercaseRule() throws {
        let grammar = ParserGrammar(rules: [
            GrammarRule(name: "Program", body: .tokenReference("NUMBER"), lineNumber: 1),
        ])
        let issues = validateParserGrammar(grammar)

        XCTAssertTrue(issues.contains { $0.contains("lowercase") })
    }

    func testValidNoIssues() throws {
        let source = """
        program = { statement } ;
        statement = NUMBER ;
        """
        let grammar = try parseParserGrammar(source: source)
        let issues = validateParserGrammar(grammar)

        XCTAssertTrue(issues.isEmpty, "Expected no issues, got: \(issues)")
    }
}

// ============================================================================
// DartmouthBasicLexerTests.swift -- Tests for the Dartmouth BASIC (1964) Lexer
// ============================================================================
//
// These tests exercise every observable behaviour of DartmouthBasicLexer:
//
//   1. Module surface: version, loadGrammar()
//   2. Trivial inputs: empty, whitespace-only
//   3. Line number detection: relabelLineNumbers pass
//   4. REM suppression: suppressRemContent pass
//   5. Keywords: all 20 BASIC keywords
//   6. Built-in functions: all 11 mathematical functions
//   7. User-defined functions: FNA, FNB, FNZ
//   8. Numeric literals: integers, decimals, scientific notation, leading-dot
//   9. Variable names: single-letter and letter+digit
//  10. Operators: arithmetic, comparison (single and double-char)
//  11. Punctuation: LPAREN, RPAREN, COMMA, SEMICOLON
//  12. Multi-line programs
//  13. Position tracking: line and column numbers
//  14. Token ordering invariants
//  15. Edge cases: empty REM, multiple lines, trailing whitespace
//
// ============================================================================

import XCTest
@testable import DartmouthBasicLexer
import Lexer

final class DartmouthBasicLexerTests: XCTestCase {

    // =========================================================================
    // MARK: - Helpers
    // =========================================================================

    /// Tokenize and return only non-EOF tokens.
    func tokens(_ source: String) throws -> [Token] {
        let all = try DartmouthBasicLexer.tokenize(source)
        return all.filter { $0.type != "EOF" }
    }

    /// Tokenize and return only the token types (no EOF).
    func types(_ source: String) throws -> [String] {
        return try tokens(source).map(\.type)
    }

    /// Tokenize and return (type, value) pairs (no EOF).
    func pairs(_ source: String) throws -> [(String, String)] {
        return try tokens(source).map { ($0.type, $0.value) }
    }

    // =========================================================================
    // MARK: - Module Surface
    // =========================================================================

    /// The version string should be a non-empty semver string like "0.1.0".
    func testVersionIsNonEmpty() {
        XCTAssertFalse(DartmouthBasicLexer.version.isEmpty,
            "version must not be empty")
    }

    /// loadGrammar() should succeed and return a grammar with at least one definition.
    /// If this fails, the .tokens file is either missing or at the wrong path.
    func testLoadGrammarReturnsDefinitions() throws {
        let grammar = try DartmouthBasicLexer.loadGrammar()
        XCTAssertFalse(grammar.definitions.isEmpty,
            "grammar must have at least one token definition")
    }

    // =========================================================================
    // MARK: - Trivial Inputs
    // =========================================================================

    /// An empty string produces exactly one token: EOF.
    /// This is the minimal valid BASIC program (a file with no lines).
    func testEmptyStringProducesEOF() throws {
        let all = try DartmouthBasicLexer.tokenize("")
        XCTAssertEqual(all.count, 1)
        XCTAssertEqual(all[0].type, "EOF")
    }

    /// Horizontal whitespace alone (spaces and tabs) produces only EOF.
    /// In BASIC, whitespace between tokens is insignificant.
    func testWhitespaceOnlyProducesEOF() throws {
        let all = try DartmouthBasicLexer.tokenize("   \t   ")
        XCTAssertEqual(all.count, 1)
        XCTAssertEqual(all[0].type, "EOF")
    }

    /// EOF is always the final token regardless of input.
    func testEOFIsAlwaysLast() throws {
        let all = try DartmouthBasicLexer.tokenize("10 END\n")
        XCTAssertEqual(all.last?.type, "EOF")
    }

    // =========================================================================
    // MARK: - Line Number Detection (relabelLineNumbers)
    // =========================================================================

    /// The first NUMBER on a line is relabelled LINE_NUM.
    /// "10 LET X = 5" should produce LINE_NUM("10"), not NUMBER("10").
    func testLineNumberIsRelabelled() throws {
        let t = try tokens("10 LET X = 5\n")
        XCTAssertEqual(t[0].type, "LINE_NUM")
        XCTAssertEqual(t[0].value, "10")
    }

    /// The LINE_NUM token carries the correct integer string.
    func testLineNumberValue() throws {
        let t = try tokens("999 END\n")
        XCTAssertEqual(t[0].value, "999")
    }

    /// Arithmetic numbers after the line label stay as NUMBER.
    /// In "10 LET X = 5": the 10 is LINE_NUM, the 5 is NUMBER.
    func testArithmeticNumberStaysNumber() throws {
        let t = try tokens("10 LET X = 5\n")
        // Find the NUMBER token (not LINE_NUM)
        let numbers = t.filter { $0.type == "NUMBER" }
        XCTAssertEqual(numbers.count, 1)
        XCTAssertEqual(numbers[0].value, "5")
    }

    /// After a NEWLINE, the next NUMBER on the following line is LINE_NUM.
    func testSecondLineNumberRelabelled() throws {
        let t = try tokens("10 END\n20 STOP\n")
        // First line: LINE_NUM("10"), KEYWORD("END"), NEWLINE
        XCTAssertEqual(t[0].type, "LINE_NUM")
        XCTAssertEqual(t[0].value, "10")
        // Second line: LINE_NUM("20"), KEYWORD("STOP"), NEWLINE
        let secondLineNum = t.first(where: { $0.type == "LINE_NUM" && $0.value == "20" })
        XCTAssertNotNil(secondLineNum, "Second line number 20 must be LINE_NUM")
    }

    /// Line numbers of various sizes are all correctly relabelled.
    func testLineNumberVariousSizes() throws {
        let t = try tokens("1 END\n100 END\n9999 END\n")
        let lineNums = t.filter { $0.type == "LINE_NUM" }.map(\.value)
        XCTAssertEqual(lineNums, ["1", "100", "9999"])
    }

    // =========================================================================
    // MARK: - REM Suppression (suppressRemContent)
    // =========================================================================

    /// After KEYWORD("REM"), tokens up to (but not including) NEWLINE are removed.
    func testRemContentSuppressed() throws {
        // "10 REM HELLO WORLD\n" — HELLO and WORLD should be gone
        let t = try tokens("10 REM HELLO WORLD\n")
        let types = t.map(\.type)
        XCTAssertFalse(types.contains("NAME"),
            "NAME tokens in REM comment must be suppressed")
    }

    /// The KEYWORD("REM") itself is kept — it is needed for the parser.
    func testRemKeywordKept() throws {
        let t = try tokens("10 REM HELLO\n")
        let remTokens = t.filter { $0.type == "KEYWORD" && $0.value == "REM" }
        XCTAssertEqual(remTokens.count, 1, "KEYWORD(REM) must be preserved")
    }

    /// The NEWLINE after REM content is kept — it terminates the line.
    func testRemNewlineKept() throws {
        let t = try tokens("10 REM HELLO\n")
        let newlines = t.filter { $0.type == "NEWLINE" }
        XCTAssertEqual(newlines.count, 1, "NEWLINE after REM must be preserved")
    }

    /// An empty REM (nothing after the keyword before NEWLINE) is valid.
    func testEmptyRem() throws {
        let t = try tokens("10 REM\n")
        let types = t.map(\.type)
        // Should be: LINE_NUM, KEYWORD(REM), NEWLINE
        XCTAssertEqual(types, ["LINE_NUM", "KEYWORD", "NEWLINE"])
    }

    /// REM suppression only affects its own line — subsequent lines are normal.
    func testRemOnlyAffectsItsLine() throws {
        let source = "10 REM THIS IS A COMMENT\n20 LET X = 1\n"
        let t = try tokens(source)
        // Line 20 should be fully present
        let lineTwo = t.filter { $0.type == "LINE_NUM" && $0.value == "20" }
        XCTAssertEqual(lineTwo.count, 1)
        // Line 20 should have a NAME token for X
        let names = t.filter { $0.type == "NAME" }
        XCTAssertTrue(names.contains(where: { $0.value == "x" }),
            "Variable x on line 20 must not be suppressed")
    }

    // =========================================================================
    // MARK: - Keywords
    // =========================================================================

    // Each of the 20 keywords defined in dartmouth_basic.tokens is tested.
    // The keywords are emitted as KEYWORD tokens with uppercase values
    // (guaranteed by GrammarLexer's case-insensitive normalisation).

    func testKeywordLET() throws {
        let t = try tokens("10 LET X = 1\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "LET" }
        XCTAssertNotNil(kw, "LET keyword")
    }

    func testKeywordPRINT() throws {
        let t = try tokens("10 PRINT X\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "PRINT" }
        XCTAssertNotNil(kw, "PRINT keyword")
    }

    func testKeywordINPUT() throws {
        let t = try tokens("10 INPUT X\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "INPUT" }
        XCTAssertNotNil(kw, "INPUT keyword")
    }

    func testKeywordIF() throws {
        let t = try tokens("10 IF X > 0 THEN 100\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "IF" }
        XCTAssertNotNil(kw, "IF keyword")
    }

    func testKeywordTHEN() throws {
        let t = try tokens("10 IF X > 0 THEN 100\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "THEN" }
        XCTAssertNotNil(kw, "THEN keyword")
    }

    func testKeywordGOTO() throws {
        let t = try tokens("10 GOTO 50\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "GOTO" }
        XCTAssertNotNil(kw, "GOTO keyword")
    }

    func testKeywordGOSUB() throws {
        let t = try tokens("10 GOSUB 200\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "GOSUB" }
        XCTAssertNotNil(kw, "GOSUB keyword")
    }

    func testKeywordRETURN() throws {
        let t = try tokens("200 RETURN\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "RETURN" }
        XCTAssertNotNil(kw, "RETURN keyword")
    }

    func testKeywordFOR() throws {
        let t = try tokens("10 FOR I = 1 TO 10\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "FOR" }
        XCTAssertNotNil(kw, "FOR keyword")
    }

    func testKeywordTO() throws {
        let t = try tokens("10 FOR I = 1 TO 10\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "TO" }
        XCTAssertNotNil(kw, "TO keyword")
    }

    func testKeywordSTEP() throws {
        let t = try tokens("10 FOR I = 10 TO 1 STEP -1\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "STEP" }
        XCTAssertNotNil(kw, "STEP keyword")
    }

    func testKeywordNEXT() throws {
        let t = try tokens("20 NEXT I\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "NEXT" }
        XCTAssertNotNil(kw, "NEXT keyword")
    }

    func testKeywordEND() throws {
        let t = try tokens("10 END\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "END" }
        XCTAssertNotNil(kw, "END keyword")
    }

    func testKeywordSTOP() throws {
        let t = try tokens("10 STOP\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "STOP" }
        XCTAssertNotNil(kw, "STOP keyword")
    }

    func testKeywordREM() throws {
        let t = try tokens("10 REM\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "REM" }
        XCTAssertNotNil(kw, "REM keyword")
    }

    func testKeywordREAD() throws {
        let t = try tokens("10 READ X\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "READ" }
        XCTAssertNotNil(kw, "READ keyword")
    }

    func testKeywordDATA() throws {
        let t = try tokens("10 DATA 1, 2, 3\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "DATA" }
        XCTAssertNotNil(kw, "DATA keyword")
    }

    func testKeywordRESTORE() throws {
        let t = try tokens("10 RESTORE\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "RESTORE" }
        XCTAssertNotNil(kw, "RESTORE keyword")
    }

    func testKeywordDIM() throws {
        let t = try tokens("10 DIM A(10)\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "DIM" }
        XCTAssertNotNil(kw, "DIM keyword")
    }

    func testKeywordDEF() throws {
        let t = try tokens("10 DEF FNA(X) = X * X\n")
        let kw = t.first { $0.type == "KEYWORD" && $0.value == "DEF" }
        XCTAssertNotNil(kw, "DEF keyword")
    }

    /// Case-insensitive: lowercase "let" should produce the same KEYWORD("LET").
    func testKeywordsAreCaseInsensitive() throws {
        let t1 = try tokens("10 LET X = 1\n")
        let t2 = try tokens("10 let x = 1\n")
        let kw1 = t1.first { $0.type == "KEYWORD" }
        let kw2 = t2.first { $0.type == "KEYWORD" }
        XCTAssertEqual(kw1?.value, kw2?.value,
            "Keywords are case-insensitive")
    }

    // =========================================================================
    // MARK: - Built-in Functions
    // =========================================================================

    // All 11 built-in mathematical functions defined in dartmouth_basic.tokens.
    // They are emitted as BUILTIN_FN tokens.

    func testBuiltinSIN() throws {
        let t = try tokens("10 LET Y = SIN(X)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "sin" })
    }

    func testBuiltinCOS() throws {
        let t = try tokens("10 LET Y = COS(X)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "cos" })
    }

    func testBuiltinTAN() throws {
        let t = try tokens("10 LET Y = TAN(X)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "tan" })
    }

    func testBuiltinATN() throws {
        let t = try tokens("10 LET Y = ATN(X)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "atn" })
    }

    func testBuiltinEXP() throws {
        let t = try tokens("10 LET Y = EXP(X)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "exp" })
    }

    func testBuiltinLOG() throws {
        let t = try tokens("10 LET Y = LOG(X)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "log" })
    }

    func testBuiltinABS() throws {
        let t = try tokens("10 LET Y = ABS(X)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "abs" })
    }

    func testBuiltinSQR() throws {
        let t = try tokens("10 LET Y = SQR(X)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "sqr" })
    }

    func testBuiltinINT() throws {
        let t = try tokens("10 LET Y = INT(X)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "int" })
    }

    func testBuiltinRND() throws {
        let t = try tokens("10 LET Y = RND(1)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "rnd" })
    }

    func testBuiltinSGN() throws {
        let t = try tokens("10 LET Y = SGN(X)\n")
        XCTAssertTrue(t.contains { $0.type == "BUILTIN_FN" && $0.value == "sgn" })
    }

    // =========================================================================
    // MARK: - User-Defined Functions
    // =========================================================================

    // USER_FN tokens match the pattern fn[a-z] (fn + one letter A-Z).
    // They are emitted as USER_FN tokens with lowercase values (e.g., "fna").

    func testUserFnFNA() throws {
        let t = try tokens("10 LET Y = FNA(X)\n")
        XCTAssertTrue(t.contains { $0.type == "USER_FN" && $0.value == "fna" })
    }

    func testUserFnFNB() throws {
        let t = try tokens("10 LET Y = FNB(X)\n")
        XCTAssertTrue(t.contains { $0.type == "USER_FN" && $0.value == "fnb" })
    }

    func testUserFnFNZ() throws {
        let t = try tokens("10 LET Y = FNZ(X)\n")
        XCTAssertTrue(t.contains { $0.type == "USER_FN" && $0.value == "fnz" })
    }

    func testDefWithUserFn() throws {
        let t = try tokens("10 DEF FNA(X) = X * X\n")
        XCTAssertTrue(t.contains { $0.type == "USER_FN" && $0.value == "fna" },
            "DEF statement must produce USER_FN token")
    }

    // =========================================================================
    // MARK: - Numeric Literals
    // =========================================================================

    // BASIC stores all numbers as floating-point, so the grammar has a single
    // NUMBER pattern that handles all numeric forms.

    func testIntegerLiteral() throws {
        let t = try tokens("10 LET X = 42\n")
        XCTAssertTrue(t.contains { $0.type == "NUMBER" && $0.value == "42" })
    }

    func testDecimalLiteral() throws {
        let t = try tokens("10 LET X = 3.14\n")
        XCTAssertTrue(t.contains { $0.type == "NUMBER" && $0.value == "3.14" })
    }

    func testScientificNotation() throws {
        // 1.5E3 = 1500.0
        let t = try tokens("10 LET X = 1.5E3\n")
        XCTAssertTrue(t.contains { $0.type == "NUMBER" && $0.value == "1.5e3" },
            "Scientific notation 1.5E3 (normalised to lowercase)")
    }

    func testScientificNotationNegExponent() throws {
        // 1.5E-3 = 0.0015
        let t = try tokens("10 LET X = 1.5E-3\n")
        XCTAssertTrue(t.contains { $0.type == "NUMBER" })
    }

    func testLeadingDotDecimal() throws {
        // .5 is valid — equivalent to 0.5
        let t = try tokens("10 LET X = .5\n")
        XCTAssertTrue(t.contains { $0.type == "NUMBER" && $0.value == ".5" })
    }

    func testNegativeNumberViaUnaryMinus() throws {
        // -3 in BASIC is parsed as unary minus applied to 3
        let t = try tokens("10 LET X = -3\n")
        XCTAssertTrue(t.contains { $0.type == "MINUS" })
        XCTAssertTrue(t.contains { $0.type == "NUMBER" && $0.value == "3" })
    }

    // =========================================================================
    // MARK: - Variable Names
    // =========================================================================

    // Variable names in Dartmouth BASIC 1964:
    //   - Single uppercase letter: A through Z (26 scalars)
    //   - Letter + digit: A0 through Z9 (260 more)
    // After case-insensitive normalisation, they appear in lowercase.

    func testSingleLetterVariable() throws {
        let t = try tokens("10 LET X = 1\n")
        XCTAssertTrue(t.contains { $0.type == "NAME" && $0.value == "x" })
    }

    func testLetterDigitVariable() throws {
        // A1 is a valid BASIC variable name
        let t = try tokens("10 LET A1 = 5\n")
        XCTAssertTrue(t.contains { $0.type == "NAME" && $0.value == "a1" })
    }

    func testVariableZ9() throws {
        let t = try tokens("10 LET Z9 = 0\n")
        XCTAssertTrue(t.contains { $0.type == "NAME" && $0.value == "z9" })
    }

    // =========================================================================
    // MARK: - String Literals
    // =========================================================================

    func testStringLiteral() throws {
        // Strings appear in PRINT and DATA statements
        let t = try tokens("10 PRINT \"HELLO\"\n")
        XCTAssertTrue(t.contains { $0.type == "STRING" },
            "Quoted string should produce STRING token")
    }

    // =========================================================================
    // MARK: - Operators
    // =========================================================================

    func testPlusOperator() throws {
        let t = try tokens("10 LET X = 1 + 2\n")
        XCTAssertTrue(t.contains { $0.type == "PLUS" && $0.value == "+" })
    }

    func testMinusOperator() throws {
        let t = try tokens("10 LET X = 5 - 3\n")
        XCTAssertTrue(t.contains { $0.type == "MINUS" && $0.value == "-" })
    }

    func testStarOperator() throws {
        let t = try tokens("10 LET X = 2 * 3\n")
        XCTAssertTrue(t.contains { $0.type == "STAR" && $0.value == "*" })
    }

    func testSlashOperator() throws {
        let t = try tokens("10 LET X = 6 / 2\n")
        XCTAssertTrue(t.contains { $0.type == "SLASH" && $0.value == "/" })
    }

    func testCaretOperator() throws {
        // ^ is exponentiation in BASIC: 2^3 = 8
        let t = try tokens("10 LET X = 2 ^ 3\n")
        XCTAssertTrue(t.contains { $0.type == "CARET" && $0.value == "^" })
    }

    func testEqOperator() throws {
        let t = try tokens("10 LET X = 5\n")
        XCTAssertTrue(t.contains { $0.type == "EQ" && $0.value == "=" })
    }

    func testLtOperator() throws {
        let t = try tokens("10 IF X < 5 THEN 100\n")
        XCTAssertTrue(t.contains { $0.type == "LT" && $0.value == "<" })
    }

    func testGtOperator() throws {
        let t = try tokens("10 IF X > 5 THEN 100\n")
        XCTAssertTrue(t.contains { $0.type == "GT" && $0.value == ">" })
    }

    func testLeOperator() throws {
        // <= must be a single LE token (not LT + EQ)
        let t = try tokens("10 IF X <= 5 THEN 100\n")
        XCTAssertTrue(t.contains { $0.type == "LE" && $0.value == "<=" })
        XCTAssertFalse(t.contains { $0.type == "LT" },
            "<= must not split into LT + EQ")
    }

    func testGeOperator() throws {
        // >= must be a single GE token
        let t = try tokens("10 IF X >= 5 THEN 100\n")
        XCTAssertTrue(t.contains { $0.type == "GE" && $0.value == ">=" })
        XCTAssertFalse(t.contains { $0.type == "GT" },
            ">= must not split into GT + EQ")
    }

    func testNeOperator() throws {
        // <> is BASIC's not-equal operator
        let t = try tokens("10 IF X <> 5 THEN 100\n")
        XCTAssertTrue(t.contains { $0.type == "NE" && $0.value == "<>" })
    }

    // =========================================================================
    // MARK: - Punctuation
    // =========================================================================

    func testLParen() throws {
        let t = try tokens("10 LET Y = SIN(X)\n")
        XCTAssertTrue(t.contains { $0.type == "LPAREN" })
    }

    func testRParen() throws {
        let t = try tokens("10 LET Y = SIN(X)\n")
        XCTAssertTrue(t.contains { $0.type == "RPAREN" })
    }

    func testComma() throws {
        let t = try tokens("10 PRINT X, Y\n")
        XCTAssertTrue(t.contains { $0.type == "COMMA" })
    }

    func testSemicolon() throws {
        // In PRINT, semicolon means "no space between items"
        let t = try tokens("10 PRINT X; Y\n")
        XCTAssertTrue(t.contains { $0.type == "SEMICOLON" })
    }

    // =========================================================================
    // MARK: - Multi-line Programs
    // =========================================================================

    /// A full multi-line BASIC program is tokenized correctly.
    func testMultiLineProgramStructure() throws {
        let source = """
            10 LET X = 5
            20 IF X > 0 THEN 40
            30 GOTO 50
            40 PRINT X
            50 END
            """
        // Add trailing newlines to each line (the above uses implicit newlines)
        let normalized = source
            .split(separator: "\n", omittingEmptySubsequences: false)
            .map { $0 + "\n" }
            .joined()

        let t = try tokens(normalized)
        let lineNums = t.filter { $0.type == "LINE_NUM" }.map(\.value)
        XCTAssertEqual(lineNums, ["10", "20", "30", "40", "50"],
            "All five line numbers must be detected")
    }

    /// NEWLINE tokens appear at the end of each logical line.
    func testNewlinesPreserved() throws {
        let source = "10 END\n20 END\n30 END\n"
        let t = try tokens(source)
        let newlines = t.filter { $0.type == "NEWLINE" }
        XCTAssertEqual(newlines.count, 3, "Three NEWLINE tokens for three lines")
    }

    // =========================================================================
    // MARK: - Position Tracking
    // =========================================================================

    /// Tokens on the first line have line == 1.
    func testFirstLineTokensHaveLineOne() throws {
        let t = try tokens("10 LET X = 5\n")
        for tok in t.filter({ $0.type != "NEWLINE" && $0.type != "EOF" }) {
            XCTAssertEqual(tok.line, 1, "All tokens on line 1")
        }
    }

    /// Column tracking: LINE_NUM starts at column 1.
    func testLineNumStartsAtColumnOne() throws {
        let t = try tokens("10 LET X = 5\n")
        XCTAssertEqual(t[0].column, 1, "LINE_NUM starts at column 1")
    }

    /// Tokens on line 2 have line == 2.
    func testSecondLineTokensHaveLineTwo() throws {
        let source = "10 END\n20 STOP\n"
        let t = try tokens(source)
        let line2Tokens = t.filter { $0.type == "LINE_NUM" && $0.value == "20" }
        XCTAssertEqual(line2Tokens[0].line, 2, "Second LINE_NUM is on line 2")
    }

    // =========================================================================
    // MARK: - Statement Structure Verification
    // =========================================================================

    /// "10 LET X = 5\n" produces the canonical token sequence.
    func testLetStatementTokenSequence() throws {
        let t = try tokens("10 LET X = 5\n")
        let typesList = t.map(\.type)
        // LINE_NUM, KEYWORD, NAME, EQ, NUMBER, NEWLINE
        XCTAssertEqual(typesList, ["LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE"])
    }

    /// "10 PRINT\n" is valid — PRINT with no arguments is a blank line output.
    func testPrintNoArgs() throws {
        let t = try tokens("10 PRINT\n")
        let typesList = t.map(\.type)
        XCTAssertEqual(typesList, ["LINE_NUM", "KEYWORD", "NEWLINE"])
    }

    /// "10 GOTO 50\n" produces LINE_NUM KEYWORD LINE_NUM NEWLINE.
    /// Note: the destination "50" is relabelled to... actually in GOTO, the
    /// 50 is NOT at the start of a line, so it stays as NUMBER? Let's check
    /// the grammar — the parser uses LINE_NUM for the target, but the lexer
    /// only relabels the FIRST number on a new source line.
    /// In "10 GOTO 50\n", the 50 is mid-line, so it stays NUMBER.
    func testGotoTokenSequence() throws {
        let t = try tokens("10 GOTO 50\n")
        let typesList = t.map(\.type)
        // LINE_NUM("10"), KEYWORD("GOTO"), NUMBER("50"), NEWLINE
        XCTAssertEqual(typesList, ["LINE_NUM", "KEYWORD", "NUMBER", "NEWLINE"])
    }

    /// "10 IF X > 0 THEN 100\n" — THEN and 100 are correctly tokenized.
    func testIfStatementTokenTypes() throws {
        let t = try tokens("10 IF X > 0 THEN 100\n")
        let typesList = t.map(\.type)
        // LINE_NUM, KEYWORD(IF), NAME, GT, NUMBER, KEYWORD(THEN), NUMBER, NEWLINE
        XCTAssertEqual(typesList, ["LINE_NUM", "KEYWORD", "NAME", "GT", "NUMBER",
                                   "KEYWORD", "NUMBER", "NEWLINE"])
    }

    // =========================================================================
    // MARK: - Edge Cases
    // =========================================================================

    /// A bare line number with no statement is valid BASIC (no-op in stored program).
    func testBareLineNumber() throws {
        let t = try tokens("10\n")
        let typesList = t.map(\.type)
        XCTAssertEqual(typesList, ["LINE_NUM", "NEWLINE"])
    }

    /// FOR loop with STEP: the STEP keyword and its value appear correctly.
    func testForLoopWithStep() throws {
        let t = try tokens("10 FOR I = 1 TO 10 STEP 2\n")
        XCTAssertTrue(t.contains { $0.type == "KEYWORD" && $0.value == "STEP" })
        XCTAssertTrue(t.contains { $0.type == "NUMBER" && $0.value == "2" })
    }

    /// DIM statement: array name, parens, and size number.
    func testDimStatement() throws {
        let t = try tokens("10 DIM A(10)\n")
        XCTAssertTrue(t.contains { $0.type == "KEYWORD" && $0.value == "DIM" })
        XCTAssertTrue(t.contains { $0.type == "NAME" && $0.value == "a" })
        XCTAssertTrue(t.contains { $0.type == "LPAREN" })
        XCTAssertTrue(t.contains { $0.type == "NUMBER" && $0.value == "10" })
        XCTAssertTrue(t.contains { $0.type == "RPAREN" })
    }

    /// DATA with multiple values.
    func testDataMultipleValues() throws {
        let t = try tokens("10 DATA 1, 2, 3\n")
        let numbers = t.filter { $0.type == "NUMBER" }
        XCTAssertEqual(numbers.count, 3)
        let commas = t.filter { $0.type == "COMMA" }
        XCTAssertEqual(commas.count, 2)
    }
}

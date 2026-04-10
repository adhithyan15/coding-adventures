// ============================================================================
// DartmouthBasicParserTests.swift -- Tests for the Dartmouth BASIC (1964) Parser
// ============================================================================
//
// These tests exercise all 17 statement types in Dartmouth BASIC 1964, plus
// expression precedence, right-associativity of ^, and edge cases.
//
// Every test parses a short BASIC program and checks:
//   1. The root node has ruleName == "program"
//   2. A specific statement rule (e.g., "let_stmt") appears in the tree
//   3. For deeper assertions: specific child rules or leaf tokens
//
// ============================================================================
// TEST ORGANISATION
// ============================================================================
//
//  1. Module surface: version, loadGrammar()
//  2. LET statement
//  3. PRINT statement (various forms)
//  4. INPUT statement
//  5. IF-THEN statement (all relational operators)
//  6. GOTO statement
//  7. GOSUB / RETURN
//  8. FOR / NEXT (with and without STEP)
//  9. END / STOP
// 10. REM statement
// 11. READ / DATA / RESTORE
// 12. DIM statement
// 13. DEF statement
// 14. Expression precedence
// 15. Exponentiation right-associativity
// 16. Empty line (bare line number)
// 17. Multi-statement programs
//
// ============================================================================

import XCTest
@testable import DartmouthBasicParser
import Lexer
import Parser

// ============================================================================
// Helper utilities
// ============================================================================

/// Recursively collect all rule names in an AST tree.
///
/// This walks the entire AST and returns the set of all `ruleName` values.
/// Used in tests to assert "this statement type appears somewhere in the tree."
///
func allRuleNames(_ node: ASTNode) -> Set<String> {
    var names: Set<String> = [node.ruleName]
    for child in node.children {
        if case .node(let subNode) = child {
            names.formUnion(allRuleNames(subNode))
        }
    }
    return names
}

/// Check if a rule name appears anywhere in the AST.
func hasRule(_ node: ASTNode, _ rule: String) -> Bool {
    allRuleNames(node).contains(rule)
}

/// Collect all leaf tokens from an AST tree (depth-first).
///
/// Useful for checking that specific tokens are present in the tree.
///
func allTokens(_ node: ASTNode) -> [Token] {
    var tokens: [Token] = []
    for child in node.children {
        switch child {
        case .token(let t):
            tokens.append(t)
        case .node(let subNode):
            tokens.append(contentsOf: allTokens(subNode))
        }
    }
    return tokens
}

// ============================================================================
// Test class
// ============================================================================

final class DartmouthBasicParserTests: XCTestCase {

    // =========================================================================
    // MARK: - Module Surface
    // =========================================================================

    /// Version string must be non-empty.
    func testVersion() {
        XCTAssertFalse(DartmouthBasicParser.version.isEmpty,
            "version must not be empty")
    }

    /// loadGrammar() must return a grammar with rules.
    func testLoadGrammarReturnsRules() throws {
        let grammar = try DartmouthBasicParser.loadGrammar()
        XCTAssertFalse(grammar.rules.isEmpty,
            "grammar must have at least one rule")
    }

    /// Grammar must contain all 17 statement-type rules.
    func testGrammarHasAllStatementRules() throws {
        let grammar = try DartmouthBasicParser.loadGrammar()
        let names = Set(grammar.rules.map(\.name))
        let expectedRules = [
            "program", "line", "statement",
            "let_stmt", "print_stmt", "input_stmt", "if_stmt",
            "goto_stmt", "gosub_stmt", "return_stmt", "for_stmt", "next_stmt",
            "end_stmt", "stop_stmt", "rem_stmt", "read_stmt", "data_stmt",
            "restore_stmt", "dim_stmt", "def_stmt"
        ]
        for rule in expectedRules {
            XCTAssertTrue(names.contains(rule), "Grammar must have rule '\(rule)'")
        }
    }

    // =========================================================================
    // MARK: - Root Node
    // =========================================================================

    /// Every program produces a root node with ruleName == "program".
    func testRootIsProgram() throws {
        let ast = try DartmouthBasicParser.parse("10 END\n")
        XCTAssertEqual(ast.ruleName, "program")
    }

    // =========================================================================
    // MARK: - Statement 1: LET
    // =========================================================================
    //
    // Grammar: let_stmt = "LET" variable EQ expr ;
    //
    // LET assigns an expression to a variable. The = is always assignment
    // here, never comparison (unlike in IF where = tests equality).

    func testLetSimple() throws {
        // 10 LET X = 5
        // The simplest LET: scalar variable, integer literal
        let ast = try DartmouthBasicParser.parse("10 LET X = 5\n")
        XCTAssertEqual(ast.ruleName, "program")
        XCTAssertTrue(hasRule(ast, "let_stmt"),
            "program must contain let_stmt")
    }

    func testLetWithExpression() throws {
        // 10 LET X = 2 + 3
        // RHS is an arithmetic expression, not just a literal
        let ast = try DartmouthBasicParser.parse("10 LET X = 2 + 3\n")
        XCTAssertTrue(hasRule(ast, "let_stmt"))
        XCTAssertTrue(hasRule(ast, "expr"))
    }

    func testLetArrayVariable() throws {
        // 10 LET A(3) = 7
        // Assigns to an array element; the variable rule matches NAME LPAREN expr RPAREN
        let ast = try DartmouthBasicParser.parse("10 LET A(3) = 7\n")
        XCTAssertTrue(hasRule(ast, "let_stmt"))
        XCTAssertTrue(hasRule(ast, "variable"))
    }

    // =========================================================================
    // MARK: - Statement 2: PRINT
    // =========================================================================
    //
    // Grammar: print_stmt = "PRINT" [ print_list ] ;
    //          print_list = print_item { print_sep print_item } [ print_sep ] ;
    //          print_item = STRING | expr ;
    //          print_sep  = COMMA | SEMICOLON ;

    func testPrintNoArgs() throws {
        // 10 PRINT    — produces a blank line
        // The print_list is optional; this tests the empty case.
        let ast = try DartmouthBasicParser.parse("10 PRINT\n")
        XCTAssertEqual(ast.ruleName, "program")
        XCTAssertTrue(hasRule(ast, "print_stmt"))
    }

    func testPrintVariable() throws {
        // 10 PRINT X  — print the value of variable X
        let ast = try DartmouthBasicParser.parse("10 PRINT X\n")
        XCTAssertTrue(hasRule(ast, "print_stmt"))
    }

    func testPrintComma() throws {
        // 10 PRINT X, Y  — print X, advance to next print zone, print Y
        // COMMA means advance to the next column multiple of 14.
        let ast = try DartmouthBasicParser.parse("10 PRINT X, Y\n")
        XCTAssertTrue(hasRule(ast, "print_stmt"))
        XCTAssertTrue(hasRule(ast, "print_list"))
    }

    func testPrintSemicolon() throws {
        // 10 PRINT X; Y  — print X immediately followed by Y, no space
        // SEMICOLON means concatenate output with no separator.
        let ast = try DartmouthBasicParser.parse("10 PRINT X; Y\n")
        XCTAssertTrue(hasRule(ast, "print_stmt"))
    }

    func testPrintString() throws {
        // 10 PRINT "HELLO"  — print a string literal
        let ast = try DartmouthBasicParser.parse("10 PRINT \"HELLO\"\n")
        XCTAssertTrue(hasRule(ast, "print_stmt"))
    }

    // =========================================================================
    // MARK: - Statement 3: INPUT
    // =========================================================================
    //
    // Grammar: input_stmt = "INPUT" variable { COMMA variable } ;

    func testInputSingleVar() throws {
        // 10 INPUT X  — read one value from the user into X
        let ast = try DartmouthBasicParser.parse("10 INPUT X\n")
        XCTAssertTrue(hasRule(ast, "input_stmt"))
    }

    func testInputMultipleVars() throws {
        // 10 INPUT A, B  — read two values; each variable gets one input value
        let ast = try DartmouthBasicParser.parse("10 INPUT A, B\n")
        XCTAssertTrue(hasRule(ast, "input_stmt"))
    }

    // =========================================================================
    // MARK: - Statement 4: IF-THEN
    // =========================================================================
    //
    // Grammar: if_stmt = "IF" expr relop expr "THEN" LINE_NUM ;
    //          relop   = EQ | LT | GT | LE | GE | NE ;
    //
    // All six relational operators are tested individually.

    func testIfGT() throws {
        // 10 IF X > 0 THEN 100  — branch if X is positive
        let ast = try DartmouthBasicParser.parse("10 IF X > 0 THEN 100\n")
        XCTAssertTrue(hasRule(ast, "if_stmt"))
        XCTAssertTrue(hasRule(ast, "relop"))
    }

    func testIfEQ() throws {
        // 10 IF X = 5 THEN 50  — = in IF context means equality (not assignment)
        let ast = try DartmouthBasicParser.parse("10 IF X = 5 THEN 50\n")
        XCTAssertTrue(hasRule(ast, "if_stmt"))
    }

    func testIfLT() throws {
        // 10 IF X < 0 THEN 200
        let ast = try DartmouthBasicParser.parse("10 IF X < 0 THEN 200\n")
        XCTAssertTrue(hasRule(ast, "if_stmt"))
    }

    func testIfLE() throws {
        // 10 IF X <= 10 THEN 300  — less-than-or-equal
        let ast = try DartmouthBasicParser.parse("10 IF X <= 10 THEN 300\n")
        XCTAssertTrue(hasRule(ast, "if_stmt"))
    }

    func testIfGE() throws {
        // 10 IF X >= 5 THEN 400  — greater-than-or-equal
        let ast = try DartmouthBasicParser.parse("10 IF X >= 5 THEN 400\n")
        XCTAssertTrue(hasRule(ast, "if_stmt"))
    }

    func testIfNE() throws {
        // 10 IF X <> 0 THEN 500  — not-equal: <> is BASIC's notation
        // (C uses !=, Pascal uses <>, BASIC uses <>)
        let ast = try DartmouthBasicParser.parse("10 IF X <> 0 THEN 500\n")
        XCTAssertTrue(hasRule(ast, "if_stmt"))
    }

    // =========================================================================
    // MARK: - Statement 5: GOTO
    // =========================================================================
    //
    // Grammar: goto_stmt = "GOTO" LINE_NUM ;
    //
    // GOTO is the unconditional jump. Edsger Dijkstra's famous 1968 paper
    // "Go To Statement Considered Harmful" was partly inspired by BASIC's
    // heavy use of GOTO.

    func testGoto() throws {
        // 10 GOTO 50  — unconditional jump to line 50
        let ast = try DartmouthBasicParser.parse("10 GOTO 50\n")
        XCTAssertEqual(ast.ruleName, "program")
        XCTAssertTrue(hasRule(ast, "goto_stmt"),
            "program must contain goto_stmt")
    }

    func testGotoLargeLineNumber() throws {
        // 10 GOTO 9999
        let ast = try DartmouthBasicParser.parse("10 GOTO 9999\n")
        XCTAssertTrue(hasRule(ast, "goto_stmt"))
    }

    // =========================================================================
    // MARK: - Statement 6 & 7: GOSUB / RETURN
    // =========================================================================
    //
    // Grammar: gosub_stmt  = "GOSUB" LINE_NUM ;
    //          return_stmt = "RETURN" ;
    //
    // GOSUB/RETURN implements subroutines. GOSUB pushes the return address
    // (next line) onto an implicit stack and jumps. RETURN pops and resumes.

    func testGosub() throws {
        // 10 GOSUB 200  — call subroutine at line 200
        let ast = try DartmouthBasicParser.parse("10 GOSUB 200\n")
        XCTAssertTrue(hasRule(ast, "gosub_stmt"))
    }

    func testReturn() throws {
        // 200 RETURN  — return from subroutine
        let ast = try DartmouthBasicParser.parse("200 RETURN\n")
        XCTAssertTrue(hasRule(ast, "return_stmt"))
    }

    func testGosubAndReturn() throws {
        // Complete GOSUB/RETURN round-trip
        let source = "10 GOSUB 200\n20 END\n200 PRINT \"SUB\"\n210 RETURN\n"
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertTrue(hasRule(ast, "gosub_stmt"))
        XCTAssertTrue(hasRule(ast, "return_stmt"))
    }

    // =========================================================================
    // MARK: - Statement 8 & 9: FOR / NEXT
    // =========================================================================
    //
    // Grammar: for_stmt  = "FOR" NAME EQ expr "TO" expr [ "STEP" expr ] ;
    //          next_stmt = "NEXT" NAME ;

    func testForNextSimple() throws {
        // 10 FOR I = 1 TO 10   — loop I from 1 to 10, step +1
        // 20 NEXT I
        let source = "10 FOR I = 1 TO 10\n20 NEXT I\n"
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertTrue(hasRule(ast, "for_stmt"))
        XCTAssertTrue(hasRule(ast, "next_stmt"))
    }

    func testForNextWithStep() throws {
        // 10 FOR I = 10 TO 1 STEP -1  — count down with step -1
        // STEP defaults to +1 if omitted; here we override it.
        let source = "10 FOR I = 10 TO 1 STEP -1\n20 NEXT I\n"
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertTrue(hasRule(ast, "for_stmt"))
    }

    func testForNextWithStepPositive() throws {
        // 10 FOR I = 1 TO 10 STEP 2  — count 1, 3, 5, 7, 9
        let source = "10 FOR I = 1 TO 10 STEP 2\n20 NEXT I\n"
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertTrue(hasRule(ast, "for_stmt"))
    }

    // =========================================================================
    // MARK: - Statement 10 & 11: END / STOP
    // =========================================================================
    //
    // Grammar: end_stmt  = "END" ;
    //          stop_stmt = "STOP" ;

    func testEnd() throws {
        // 10 END  — normal program termination
        let ast = try DartmouthBasicParser.parse("10 END\n")
        XCTAssertTrue(hasRule(ast, "end_stmt"))
    }

    func testStop() throws {
        // 10 STOP  — halt with "STOP IN LINE 10" message
        let ast = try DartmouthBasicParser.parse("10 STOP\n")
        XCTAssertTrue(hasRule(ast, "stop_stmt"))
    }

    // =========================================================================
    // MARK: - Statement 12: REM
    // =========================================================================
    //
    // Grammar: rem_stmt = "REM" ;
    //
    // The lexer's suppressRemContent pass removes all tokens between
    // KEYWORD("REM") and NEWLINE. So by the time the parser sees the stream,
    // a REM line is: LINE_NUM KEYWORD("REM") NEWLINE — and rem_stmt = "REM" matches.

    func testRem() throws {
        // 10 REM THIS IS A COMMENT  — the lexer strips "THIS IS A COMMENT"
        let ast = try DartmouthBasicParser.parse("10 REM THIS IS A COMMENT\n")
        XCTAssertTrue(hasRule(ast, "rem_stmt"),
            "REM line must produce rem_stmt node")
    }

    func testEmptyRem() throws {
        // 10 REM  — empty comment
        let ast = try DartmouthBasicParser.parse("10 REM\n")
        XCTAssertTrue(hasRule(ast, "rem_stmt"))
    }

    // =========================================================================
    // MARK: - Statement 13, 14, 15: READ / DATA / RESTORE
    // =========================================================================
    //
    // Grammar: read_stmt    = "READ" variable { COMMA variable } ;
    //          data_stmt    = "DATA" NUMBER { COMMA NUMBER } ;
    //          restore_stmt = "RESTORE" ;
    //
    // DATA defines a pool of values; READ pops them sequentially.
    // RESTORE resets the pool pointer to the beginning.

    func testRead() throws {
        // 10 READ X  — read one value from DATA pool into X
        let ast = try DartmouthBasicParser.parse("10 READ X\n")
        XCTAssertTrue(hasRule(ast, "read_stmt"))
    }

    func testReadMultiple() throws {
        // 10 READ A, B, C  — read three values
        let ast = try DartmouthBasicParser.parse("10 READ A, B, C\n")
        XCTAssertTrue(hasRule(ast, "read_stmt"))
    }

    func testData() throws {
        // 10 DATA 3  — define a single value in the DATA pool
        let ast = try DartmouthBasicParser.parse("10 DATA 3\n")
        XCTAssertTrue(hasRule(ast, "data_stmt"))
    }

    func testDataMultiple() throws {
        // 10 DATA 1, 2, 3  — define three values
        let ast = try DartmouthBasicParser.parse("10 DATA 1, 2, 3\n")
        XCTAssertTrue(hasRule(ast, "data_stmt"))
    }

    func testReadDataPair() throws {
        // Full READ/DATA pair
        let source = "10 READ X\n20 DATA 3\n"
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertTrue(hasRule(ast, "read_stmt"))
        XCTAssertTrue(hasRule(ast, "data_stmt"))
    }

    func testRestore() throws {
        // 10 RESTORE  — reset DATA pool pointer
        let ast = try DartmouthBasicParser.parse("10 RESTORE\n")
        XCTAssertTrue(hasRule(ast, "restore_stmt"))
    }

    // =========================================================================
    // MARK: - Statement 16: DIM
    // =========================================================================
    //
    // Grammar: dim_stmt = "DIM" dim_decl { COMMA dim_decl } ;
    //          dim_decl = NAME LPAREN NUMBER RPAREN ;

    func testDimSingle() throws {
        // 10 DIM A(10)  — declare array A with 10 elements (indices 0–10)
        let ast = try DartmouthBasicParser.parse("10 DIM A(10)\n")
        XCTAssertTrue(hasRule(ast, "dim_stmt"))
        XCTAssertTrue(hasRule(ast, "dim_decl"))
    }

    func testDimMultiple() throws {
        // 10 DIM A(10), B(20)  — declare two arrays
        let ast = try DartmouthBasicParser.parse("10 DIM A(10), B(20)\n")
        XCTAssertTrue(hasRule(ast, "dim_stmt"))
    }

    // =========================================================================
    // MARK: - Statement 17: DEF
    // =========================================================================
    //
    // Grammar: def_stmt = "DEF" USER_FN LPAREN NAME RPAREN EQ expr ;
    //
    // DEF defines a user-defined single-argument function. The function name
    // ranges from FNA through FNZ (26 functions).

    func testDef() throws {
        // 10 DEF FNA(X) = X * X  — define FNA as squaring function
        let ast = try DartmouthBasicParser.parse("10 DEF FNA(X) = X * X\n")
        XCTAssertTrue(hasRule(ast, "def_stmt"))
    }

    func testDefWithBuiltin() throws {
        // 10 DEF FNB(T) = SIN(T) / COS(T)  — user-defined tangent
        let ast = try DartmouthBasicParser.parse("10 DEF FNB(T) = SIN(T) / COS(T)\n")
        XCTAssertTrue(hasRule(ast, "def_stmt"))
    }

    // =========================================================================
    // MARK: - Expression Precedence
    // =========================================================================
    //
    // Standard arithmetic precedence encoded as grammar nesting:
    //   expr (+ -) > term (* /) > power (^) > unary (-) > primary (atoms)

    func testExprPrecedenceAddMul() throws {
        // 10 LET X = 2 + 3 * 4
        // Standard math: 3 * 4 = 12, then 2 + 12 = 14.
        // Grammar encodes this: `+` is in `expr`, `*` is in `term`.
        let ast = try DartmouthBasicParser.parse("10 LET X = 2 + 3 * 4\n")
        XCTAssertEqual(ast.ruleName, "program")
        XCTAssertTrue(hasRule(ast, "let_stmt"))
        XCTAssertTrue(hasRule(ast, "expr"))
        XCTAssertTrue(hasRule(ast, "term"))
    }

    func testExprPrecedenceUnaryMinus() throws {
        // 10 LET X = -5
        // Unary minus: `unary = MINUS primary | primary`
        let ast = try DartmouthBasicParser.parse("10 LET X = -5\n")
        XCTAssertTrue(hasRule(ast, "unary"))
    }

    func testExprParenthesised() throws {
        // 10 LET X = (2 + 3) * 4
        // Parentheses override precedence: (2+3)=5, 5*4=20
        let ast = try DartmouthBasicParser.parse("10 LET X = (2 + 3) * 4\n")
        XCTAssertTrue(hasRule(ast, "primary"))
    }

    func testBuiltinFunctionCall() throws {
        // 10 LET Y = SIN(X)
        // Built-in function call: `primary = BUILTIN_FN LPAREN expr RPAREN`
        let ast = try DartmouthBasicParser.parse("10 LET Y = SIN(X)\n")
        XCTAssertTrue(hasRule(ast, "primary"))
    }

    func testUserFunctionCall() throws {
        // 10 DEF FNA(X) = X * X
        // 20 LET Y = FNA(3)
        // User function call: `primary = USER_FN LPAREN expr RPAREN`
        let source = "10 DEF FNA(X) = X * X\n20 LET Y = FNA(3)\n"
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertTrue(hasRule(ast, "def_stmt"))
        XCTAssertTrue(hasRule(ast, "let_stmt"))
    }

    // =========================================================================
    // MARK: - Exponentiation Right-Associativity
    // =========================================================================
    //
    // Grammar: power = unary [ CARET power ] ;
    //
    // The recursive right-hand `power` reference makes ^ right-associative:
    //   2 ^ 3 ^ 2 = 2 ^ (3 ^ 2) = 2 ^ 9 = 512
    //   (not (2^3)^2 = 8^2 = 64)
    //
    // This matches the Dartmouth BASIC 1964 specification and standard
    // mathematical convention for exponentiation.

    func testExponentiation() throws {
        // 10 LET X = 2 ^ 3 ^ 2
        // Should parse as 2 ^ (3 ^ 2) due to right-associativity.
        let ast = try DartmouthBasicParser.parse("10 LET X = 2 ^ 3 ^ 2\n")
        XCTAssertEqual(ast.ruleName, "program")
        XCTAssertTrue(hasRule(ast, "power"))
    }

    // =========================================================================
    // MARK: - Empty Line (Bare Line Number)
    // =========================================================================
    //
    // Grammar: line = LINE_NUM [ statement ] NEWLINE ;
    //
    // The `[ statement ]` part is optional. A bare line number with no
    // statement is valid BASIC — in interactive mode it deletes that line.
    // In stored program mode it's a no-op.

    func testBareLineNumber() throws {
        // 10   — a line with no statement
        let ast = try DartmouthBasicParser.parse("10\n")
        XCTAssertEqual(ast.ruleName, "program")
        XCTAssertTrue(hasRule(ast, "line"),
            "Bare line number must produce a 'line' node")
    }

    // =========================================================================
    // MARK: - Multi-Statement Programs
    // =========================================================================

    func testTwoStatements() throws {
        let source = "10 LET X = 1\n20 END\n"
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertEqual(ast.ruleName, "program")
        XCTAssertTrue(hasRule(ast, "let_stmt"))
        XCTAssertTrue(hasRule(ast, "end_stmt"))
    }

    func testReadDataProgram() throws {
        // Classic READ/DATA pattern: populate variables from a data pool
        let source = "10 READ X\n20 DATA 3\n30 PRINT X\n40 END\n"
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertTrue(hasRule(ast, "read_stmt"))
        XCTAssertTrue(hasRule(ast, "data_stmt"))
        XCTAssertTrue(hasRule(ast, "print_stmt"))
        XCTAssertTrue(hasRule(ast, "end_stmt"))
    }

    func testCountingLoop() throws {
        // The quintessential BASIC program: count to 10
        let source = "10 FOR I = 1 TO 10\n20 PRINT I\n30 NEXT I\n40 END\n"
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertTrue(hasRule(ast, "for_stmt"))
        XCTAssertTrue(hasRule(ast, "print_stmt"))
        XCTAssertTrue(hasRule(ast, "next_stmt"))
        XCTAssertTrue(hasRule(ast, "end_stmt"))
    }

    func testConditionalProgram() throws {
        // A simple conditional with GOTO
        let source = [
            "10 INPUT X\n",
            "20 IF X > 0 THEN 50\n",
            "30 PRINT \"NEGATIVE OR ZERO\"\n",
            "40 GOTO 60\n",
            "50 PRINT \"POSITIVE\"\n",
            "60 END\n"
        ].joined()
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertTrue(hasRule(ast, "input_stmt"))
        XCTAssertTrue(hasRule(ast, "if_stmt"))
        XCTAssertTrue(hasRule(ast, "print_stmt"))
        XCTAssertTrue(hasRule(ast, "goto_stmt"))
        XCTAssertTrue(hasRule(ast, "end_stmt"))
    }

    func testSubroutineProgram() throws {
        // A program that uses a GOSUB subroutine
        let source = [
            "10 GOSUB 100\n",
            "20 END\n",
            "100 PRINT \"SUBROUTINE\"\n",
            "110 RETURN\n"
        ].joined()
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertTrue(hasRule(ast, "gosub_stmt"))
        XCTAssertTrue(hasRule(ast, "return_stmt"))
    }

    func testFibonacciProgram() throws {
        // Fibonacci sequence — a complex multi-statement program that exercises
        // LET, FOR, NEXT, PRINT, END.
        let source = [
            "10 LET A = 1\n",
            "20 LET B = 1\n",
            "30 FOR N = 1 TO 8\n",
            "40 PRINT A\n",
            "50 LET C = A + B\n",
            "60 LET A = B\n",
            "70 LET B = C\n",
            "80 NEXT N\n",
            "90 END\n"
        ].joined()
        let ast = try DartmouthBasicParser.parse(source)
        XCTAssertEqual(ast.ruleName, "program")
        XCTAssertTrue(hasRule(ast, "let_stmt"))
        XCTAssertTrue(hasRule(ast, "for_stmt"))
        XCTAssertTrue(hasRule(ast, "next_stmt"))
        XCTAssertTrue(hasRule(ast, "print_stmt"))
        XCTAssertTrue(hasRule(ast, "end_stmt"))
    }

    // =========================================================================
    // MARK: - Line Structure
    // =========================================================================

    /// Each line node in the AST contains a LINE_NUM token.
    func testLineNodeContainsLineNum() throws {
        let ast = try DartmouthBasicParser.parse("10 LET X = 1\n")
        // The AST root is program; find the first "line" node
        let lineNodes = ast.children.compactMap { child -> ASTNode? in
            if case .node(let n) = child, n.ruleName == "line" { return n }
            return nil
        }
        XCTAssertFalse(lineNodes.isEmpty, "program must have at least one line node")
        let firstLine = lineNodes[0]
        // First child of the line node must be a LINE_NUM token
        if case .token(let tok) = firstLine.children.first {
            XCTAssertEqual(tok.type, "LINE_NUM")
            XCTAssertEqual(tok.value, "10")
        } else {
            XCTFail("First child of line must be a LINE_NUM token")
        }
    }

    // =========================================================================
    // MARK: - Error Cases
    // =========================================================================

    /// A completely empty input produces a valid empty "program".
    func testEmptyInput() throws {
        // A file with no lines is a valid BASIC program (zero lines).
        let ast = try DartmouthBasicParser.parse("")
        XCTAssertEqual(ast.ruleName, "program")
    }
}

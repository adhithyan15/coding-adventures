package dartmouthbasicparser

import (
	"testing"
)

// =============================================================================
// Tests for dartmouth-basic-parser
//
// These tests exercise the full parsing pipeline:
//   BASIC source text → TokenizeDartmouthBasic → GrammarParser.Parse() → AST
//
// Every successful parse must produce an *ASTNode whose RuleName is "program",
// because "program" is the top-level rule in dartmouth_basic.grammar.
//
// Test organisation:
//   - CreateDartmouthBasicParser  — verifies the factory function
//   - LET statement               — assignment
//   - PRINT statement             — bare, expression, string, comma, semicolon
//   - INPUT statement             — single and multiple variables
//   - IF statement                — all six relational operators
//   - GOTO / GOSUB / RETURN
//   - FOR / NEXT                  — with and without STEP
//   - END / STOP / REM
//   - READ / DATA / RESTORE
//   - DIM / DEF
//   - Expression precedence       — +/-, *//, ^, unary -, parentheses, functions
//   - Multi-line programs         — Hello World, FOR loop
//   - Error cases                 — missing =, missing THEN, incomplete FOR
//   - Edge cases                  — bare line number, empty program
// =============================================================================

// =============================================================================
// TestCreateDartmouthBasicParser
// =============================================================================
//
// Verifies that the factory function returns a non-nil GrammarParser and that
// it can parse a simple one-line program. This validates the two-step API:
// CreateDartmouthBasicParser → Parse(), as opposed to the one-shot
// ParseDartmouthBasic convenience function.
func TestCreateDartmouthBasicParser(t *testing.T) {
	source := "10 LET X = 5\n"
	basicParser, err := CreateDartmouthBasicParser(source)
	if err != nil {
		t.Fatalf("CreateDartmouthBasicParser failed: %v", err)
	}
	if basicParser == nil {
		t.Fatal("CreateDartmouthBasicParser returned nil")
	}

	ast, err := basicParser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected root rule 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// LET statement tests
// =============================================================================
//
// The LET statement assigns a value to a variable:
//   LET variable = expr
//
// In Dartmouth BASIC, "=" in LET is always assignment. Comparison uses the
// relop rule only inside IF statements.

func TestLetSimpleAssignment(t *testing.T) {
	// 10 LET X = 5  — assign literal 5 to scalar variable X
	ast, err := ParseDartmouthBasic("10 LET X = 5\n")
	if err != nil {
		t.Fatalf("Failed to parse LET: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestLetWithExpression(t *testing.T) {
	// 10 LET X = Y + 1  — right-hand side can be any expr
	ast, err := ParseDartmouthBasic("10 LET X = Y + 1\n")
	if err != nil {
		t.Fatalf("Failed to parse LET with expression: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestLetArrayElement(t *testing.T) {
	// 10 LET A(3) = 42  — assign to an array element
	// The grammar's variable rule covers: NAME LPAREN expr RPAREN | NAME
	ast, err := ParseDartmouthBasic("10 LET A(3) = 42\n")
	if err != nil {
		t.Fatalf("Failed to parse LET array: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// PRINT statement tests
// =============================================================================
//
// PRINT is the most flexible output statement:
//   PRINT                 — print a blank line
//   PRINT expr            — print an expression
//   PRINT "STRING"        — print a string literal
//   PRINT X, Y            — zone-aligned output (comma = advance to next zone)
//   PRINT X; Y            — compact output (semicolon = no space between items)
//   PRINT X,              — trailing comma suppresses final newline

func TestPrintBare(t *testing.T) {
	// 10 PRINT  — emits a blank line at runtime
	ast, err := ParseDartmouthBasic("10 PRINT\n")
	if err != nil {
		t.Fatalf("Failed to parse bare PRINT: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestPrintExpression(t *testing.T) {
	// 10 PRINT X + 1
	ast, err := ParseDartmouthBasic("10 PRINT X + 1\n")
	if err != nil {
		t.Fatalf("Failed to parse PRINT expr: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestPrintString(t *testing.T) {
	// 10 PRINT "HELLO"  — STRING token produced by the lexer
	ast, err := ParseDartmouthBasic("10 PRINT \"HELLO\"\n")
	if err != nil {
		t.Fatalf("Failed to parse PRINT string: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestPrintComma(t *testing.T) {
	// 10 PRINT X, Y  — comma advances output to the next 15-char zone
	ast, err := ParseDartmouthBasic("10 PRINT X, Y\n")
	if err != nil {
		t.Fatalf("Failed to parse PRINT comma: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestPrintSemicolon(t *testing.T) {
	// 10 PRINT X; Y  — semicolon prints items without any space between them
	ast, err := ParseDartmouthBasic("10 PRINT X; Y\n")
	if err != nil {
		t.Fatalf("Failed to parse PRINT semicolon: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestPrintTrailingComma(t *testing.T) {
	// 10 PRINT X,  — trailing comma suppresses the final newline
	ast, err := ParseDartmouthBasic("10 PRINT X,\n")
	if err != nil {
		t.Fatalf("Failed to parse PRINT trailing comma: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// INPUT statement tests
// =============================================================================
//
// INPUT reads one value per variable from stdin at runtime.
// The grammar: input_stmt = "INPUT" variable { COMMA variable }

func TestInputSingleVariable(t *testing.T) {
	// 10 INPUT X
	ast, err := ParseDartmouthBasic("10 INPUT X\n")
	if err != nil {
		t.Fatalf("Failed to parse INPUT: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestInputMultipleVariables(t *testing.T) {
	// 10 INPUT A, B, C  — reads three values in one INPUT statement
	ast, err := ParseDartmouthBasic("10 INPUT A, B, C\n")
	if err != nil {
		t.Fatalf("Failed to parse INPUT multiple: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// IF statement tests — all six relational operators
// =============================================================================
//
// The 1964 IF statement form:  IF expr relop expr THEN LINE_NUM
// No ELSE clause; branch target must be a literal line number.
// relop = EQ | LT | GT | LE | GE | NE

func TestIfWithEquals(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 IF X = Y THEN 50\n")
	if err != nil {
		t.Fatalf("Failed to parse IF =: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestIfWithLessThan(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 IF X < 0 THEN 100\n")
	if err != nil {
		t.Fatalf("Failed to parse IF <: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestIfWithGreaterThan(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 IF X > 0 THEN 100\n")
	if err != nil {
		t.Fatalf("Failed to parse IF >: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestIfWithLessEqual(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 IF X <= 10 THEN 200\n")
	if err != nil {
		t.Fatalf("Failed to parse IF <=: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestIfWithGreaterEqual(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 IF X >= 10 THEN 200\n")
	if err != nil {
		t.Fatalf("Failed to parse IF >=: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestIfWithNotEqual(t *testing.T) {
	// <> is the BASIC not-equal operator
	ast, err := ParseDartmouthBasic("10 IF X <> 0 THEN 300\n")
	if err != nil {
		t.Fatalf("Failed to parse IF <>: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// GOTO statement test
// =============================================================================
//
// GOTO jumps unconditionally to the named line.
// goto_stmt = "GOTO" LINE_NUM

func TestGoto(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 GOTO 50\n")
	if err != nil {
		t.Fatalf("Failed to parse GOTO: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// GOSUB and RETURN statement tests
// =============================================================================
//
// GOSUB pushes return address, jumps to target. RETURN pops and resumes.

func TestGosub(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 GOSUB 200\n")
	if err != nil {
		t.Fatalf("Failed to parse GOSUB: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestReturn(t *testing.T) {
	ast, err := ParseDartmouthBasic("200 RETURN\n")
	if err != nil {
		t.Fatalf("Failed to parse RETURN: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// FOR / NEXT loop tests
// =============================================================================
//
// for_stmt  = "FOR" NAME EQ expr "TO" expr [ "STEP" expr ]
// next_stmt = "NEXT" NAME
//
// The grammar parses FOR and NEXT as independent statements; it is the
// compiler/VM that enforces matching variable names and nesting structure.

func TestForNextWithoutStep(t *testing.T) {
	// FOR with no STEP clause — default step is 1 at runtime
	source := "10 FOR I = 1 TO 10\n20 NEXT I\n"
	ast, err := ParseDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to parse FOR/NEXT: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestForNextWithPositiveStep(t *testing.T) {
	// 10 FOR I = 0 TO 100 STEP 5  — counts 0, 5, 10, ..., 100
	source := "10 FOR I = 0 TO 100 STEP 5\n20 NEXT I\n"
	ast, err := ParseDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to parse FOR STEP: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestForNextWithNegativeStep(t *testing.T) {
	// 10 FOR I = 10 TO 1 STEP -1  — countdown: 10, 9, 8, ..., 1
	// The STEP expression here is the unary minus applied to 1.
	source := "10 FOR I = 10 TO 1 STEP -1\n20 NEXT I\n"
	ast, err := ParseDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to parse FOR STEP -1: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// END and STOP statement tests
// =============================================================================
//
// END is the normal terminator. STOP halts with a diagnostic message.
// Both have empty bodies in the grammar: end_stmt = "END" ; stop_stmt = "STOP" ;

func TestEnd(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 END\n")
	if err != nil {
		t.Fatalf("Failed to parse END: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestStop(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 STOP\n")
	if err != nil {
		t.Fatalf("Failed to parse STOP: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// REM statement test
// =============================================================================
//
// The lexer's suppressRemContent hook drops all tokens between REM and NEWLINE.
// By the time the parser sees the stream, a REM line is just:
//   LINE_NUM KEYWORD("REM") NEWLINE
// The rem_stmt grammar rule matches only "REM" (the body is empty).

func TestRem(t *testing.T) {
	// 10 REM A COMMENT  — the "A COMMENT" tokens are suppressed by the lexer
	ast, err := ParseDartmouthBasic("10 REM A COMMENT\n")
	if err != nil {
		t.Fatalf("Failed to parse REM: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// READ / DATA / RESTORE statement tests
// =============================================================================
//
// The data pool mechanism:
//   DATA 1, 2, 3   — declares pool values (collected by the compiler)
//   READ X, Y      — pops next two values and assigns to X and Y
//   RESTORE        — resets the pool pointer to the beginning
//
// DATA values are numeric literals only in the 1964 spec.

func TestReadSingleVariable(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 READ X\n")
	if err != nil {
		t.Fatalf("Failed to parse READ: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestReadMultipleVariables(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 READ A, B, C\n")
	if err != nil {
		t.Fatalf("Failed to parse READ multiple: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestDataSingleValue(t *testing.T) {
	ast, err := ParseDartmouthBasic("20 DATA 42\n")
	if err != nil {
		t.Fatalf("Failed to parse DATA single: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestDataMultipleValues(t *testing.T) {
	ast, err := ParseDartmouthBasic("20 DATA 1, 2, 3, 4, 5\n")
	if err != nil {
		t.Fatalf("Failed to parse DATA multiple: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestRestore(t *testing.T) {
	ast, err := ParseDartmouthBasic("30 RESTORE\n")
	if err != nil {
		t.Fatalf("Failed to parse RESTORE: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// DIM statement tests
// =============================================================================
//
// DIM declares array sizes. Default size (without DIM) is indices 0..10.
// dim_stmt = "DIM" dim_decl { COMMA dim_decl }
// dim_decl = NAME LPAREN NUMBER RPAREN

func TestDimSingleArray(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 DIM A(10)\n")
	if err != nil {
		t.Fatalf("Failed to parse DIM: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestDimMultipleArrays(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 DIM A(10), B(20)\n")
	if err != nil {
		t.Fatalf("Failed to parse DIM multiple: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// DEF statement tests
// =============================================================================
//
// DEF defines a single-argument user function. Names FNA through FNZ.
// def_stmt = "DEF" USER_FN LPAREN NAME RPAREN EQ expr

func TestDefSimple(t *testing.T) {
	// 10 DEF FNA(X) = X * X  — square function
	ast, err := ParseDartmouthBasic("10 DEF FNA(X) = X * X\n")
	if err != nil {
		t.Fatalf("Failed to parse DEF: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestDefWithBuiltin(t *testing.T) {
	// 10 DEF FNB(T) = SIN(T) / COS(T)  — tangent via sin/cos
	ast, err := ParseDartmouthBasic("10 DEF FNB(T) = SIN(T) / COS(T)\n")
	if err != nil {
		t.Fatalf("Failed to parse DEF with builtin: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// Expression precedence and associativity tests
// =============================================================================
//
// The grammar encodes precedence via rule nesting:
//
//   expr  → term { (+ | -) term }          lowest: addition/subtraction
//   term  → power { (* | /) power }         medium: multiplication/division
//   power → unary [ ^ power ]               high, RIGHT-associative
//   unary → - primary | primary             unary minus
//   primary → NUMBER | FN(expr) | var | (expr)
//
// Right-associativity of ^ means 2^3^2 = 2^(3^2) = 512, not (2^3)^2 = 64.

func TestExprAddition(t *testing.T) {
	ast, err := ParseDartmouthBasic("10 LET X = 2 + 3\n")
	if err != nil {
		t.Fatalf("Failed to parse addition: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestExprMultiplicationBindsTighter(t *testing.T) {
	// 2 + 3 * 4 parses as 2 + (3*4) = 14 because * is in 'term', not 'expr'
	ast, err := ParseDartmouthBasic("10 LET X = 2 + 3 * 4\n")
	if err != nil {
		t.Fatalf("Failed to parse mixed precedence: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestExprRightAssociativeExponentiation(t *testing.T) {
	// 2 ^ 3 ^ 2 parses as 2 ^ (3^2) = 512 (not 64)
	// The grammar rule: power = unary [ CARET power ]  — right recursion
	ast, err := ParseDartmouthBasic("10 LET X = 2 ^ 3 ^ 2\n")
	if err != nil {
		t.Fatalf("Failed to parse exponentiation: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestExprUnaryMinus(t *testing.T) {
	// -Y  — negate variable Y. Unary plus is NOT in the 1964 spec.
	ast, err := ParseDartmouthBasic("10 LET X = -Y\n")
	if err != nil {
		t.Fatalf("Failed to parse unary minus: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestExprParentheses(t *testing.T) {
	// (2 + 3) * 4 = 20 — parentheses override default precedence
	ast, err := ParseDartmouthBasic("10 LET X = (2 + 3) * 4\n")
	if err != nil {
		t.Fatalf("Failed to parse parentheses: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestExprBuiltinFunction(t *testing.T) {
	// SIN(Y) — built-in function call
	// The grammar rule: primary = BUILTIN_FN LPAREN expr RPAREN | ...
	ast, err := ParseDartmouthBasic("10 LET X = SIN(Y)\n")
	if err != nil {
		t.Fatalf("Failed to parse builtin function: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestExprUserFunction(t *testing.T) {
	// FNA(X) — calls user-defined function FNA
	// USER_FN is a distinct token type; the grammar handles it separately from BUILTIN_FN
	ast, err := ParseDartmouthBasic("10 LET Y = FNA(X)\n")
	if err != nil {
		t.Fatalf("Failed to parse user function: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestExprArrayAccess(t *testing.T) {
	// A(3) — access element 3 of array A
	// The variable rule: NAME LPAREN expr RPAREN | NAME
	// The array form must be tried first (it is listed first in the grammar).
	ast, err := ParseDartmouthBasic("10 LET X = A(3)\n")
	if err != nil {
		t.Fatalf("Failed to parse array access: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// Multi-line program tests
// =============================================================================
//
// The grammar's top-level rule:  program = { line }
// Zero or more lines. These tests confirm that multi-line programs parse as a
// whole into a single "program" AST node with multiple "line" children.

func TestHelloWorld(t *testing.T) {
	source := "10 PRINT \"HELLO, WORLD\"\n20 END\n"
	ast, err := ParseDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to parse Hello World: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
	// A two-line program should produce at least two children in the AST.
	if len(ast.Children) == 0 {
		t.Error("Expected non-empty children for multi-line program")
	}
}

func TestForLoopProgram(t *testing.T) {
	// Classic counted loop — prints 1 through 5
	source := "10 FOR I = 1 TO 5\n20 PRINT I\n30 NEXT I\n40 END\n"
	ast, err := ParseDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to parse FOR loop program: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestGosubReturnProgram(t *testing.T) {
	source := "10 GOSUB 100\n20 END\n100 PRINT \"IN SUB\"\n110 RETURN\n"
	ast, err := ParseDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to parse GOSUB program: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestReadDataProgram(t *testing.T) {
	source := "10 DATA 1, 2, 3\n20 READ A\n30 READ B\n40 READ C\n50 PRINT A + B + C\n60 END\n"
	ast, err := ParseDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to parse READ/DATA program: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestConditionalBranchProgram(t *testing.T) {
	source := "10 INPUT X\n20 IF X > 0 THEN 50\n30 PRINT \"NEGATIVE OR ZERO\"\n40 GOTO 60\n50 PRINT \"POSITIVE\"\n60 END\n"
	ast, err := ParseDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to parse conditional program: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestCompleteProgram(t *testing.T) {
	// A realistic program: compute sum of 1 to N using FOR loop.
	source := "10 REM COMPUTE SUM 1 TO N\n" +
		"20 INPUT N\n" +
		"30 LET S = 0\n" +
		"40 FOR I = 1 TO N\n" +
		"50 LET S = S + I\n" +
		"60 NEXT I\n" +
		"70 PRINT \"SUM =\", S\n" +
		"80 END\n"
	ast, err := ParseDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to parse complete program: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// Error case tests
// =============================================================================
//
// The parser should return a non-nil error for syntactically invalid BASIC.
// We test the three most common error patterns:
//   1. LET missing =        — let_stmt needs EQ between variable and expr
//   2. IF missing THEN      — if_stmt needs THEN before the line number
//   3. FOR missing TO       — for_stmt needs TO after the start expression

func TestErrorLetMissingEquals(t *testing.T) {
	// "10 LET X 5\n" — missing the = sign
	_, err := ParseDartmouthBasic("10 LET X 5\n")
	if err == nil {
		t.Error("Expected error for LET missing =, got nil")
	}
}

func TestErrorIfMissingThen(t *testing.T) {
	// "10 IF X > 0 100\n" — missing THEN keyword
	_, err := ParseDartmouthBasic("10 IF X > 0 100\n")
	if err == nil {
		t.Error("Expected error for IF missing THEN, got nil")
	}
}

func TestErrorForMissingTo(t *testing.T) {
	// "10 FOR I = 1\n" — missing TO and limit expression
	_, err := ParseDartmouthBasic("10 FOR I = 1\n")
	if err == nil {
		t.Error("Expected error for FOR missing TO, got nil")
	}
}

// =============================================================================
// Edge case tests
// =============================================================================

func TestEmptyProgram(t *testing.T) {
	// An empty string is a valid BASIC program (zero lines).
	// The grammar: program = { line }  — zero repetitions matches.
	ast, err := ParseDartmouthBasic("")
	if err != nil {
		t.Fatalf("Failed to parse empty program: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestBareLineNumber(t *testing.T) {
	// "10\n" — a line number with no statement.
	// The grammar: line = LINE_NUM [ statement ] NEWLINE
	// The [ statement ] is optional, so this is valid.
	// In the original DTSS, typing a bare line number deleted that line from
	// the stored program. In a stored program file it produces a no-op node.
	ast, err := ParseDartmouthBasic("10\n")
	if err != nil {
		t.Fatalf("Failed to parse bare line number: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

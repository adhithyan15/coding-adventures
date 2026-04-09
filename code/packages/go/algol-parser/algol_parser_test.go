package algolparser

import (
	"testing"
)

// =============================================================================
// TestParseAlgol_MinimalProgram
// =============================================================================
//
// Verifies that the simplest useful ALGOL 60 program can be parsed.
//
// The program declares an integer variable and assigns it a value:
//   begin integer x; x := 42 end
//
// ALGOL 60 grammar requires:
//   1. A "begin" keyword to open the block
//   2. All declarations (integer x) before all statements (x := 42)
//   3. A semicolon separating the declaration from the statement
//   4. An "end" keyword to close the block
//
// Note: there is no semicolon after the last statement. Semicolons in ALGOL 60
// are separators (between statements), not terminators (after each statement).
// This differs from C and Java, which use semicolons as terminators.
func TestParseAlgol_MinimalProgram(t *testing.T) {
	source := "begin integer x; x := 42 end"
	ast, err := ParseAlgol(source)
	if err != nil {
		t.Fatalf("Failed to parse minimal program: %v", err)
	}

	// The root node should be "program" — the entry rule in algol.grammar
	if ast.RuleName != "program" {
		t.Errorf("Expected root rule 'program', got %q", ast.RuleName)
	}

	// A non-trivial program should produce a non-empty AST
	if len(ast.Children) == 0 {
		t.Error("Expected non-empty AST for minimal program")
	}
}

// =============================================================================
// TestParseAlgol_Assignment
// =============================================================================
//
// Verifies that simple assignment statements parse correctly and that the
// root AST node is always "program".
//
// ALGOL 60 assignments use := (not =). The left-hand side is a variable
// (IDENT, optionally subscripted for arrays). The right-hand side is an
// expression (arithmetic or boolean).
//
// Multiple left-hand sides are allowed: "x := y := 0" assigns 0 to y first,
// then to x (right-to-left evaluation).
func TestParseAlgol_Assignment(t *testing.T) {
	testCases := []string{
		"begin integer x; x := 0 end",
		"begin real r; r := 3.14 end",
		"begin integer x; x := 1 + 2 end",
		"begin integer x; integer y; x := y := 0 end",
	}

	for _, source := range testCases {
		ast, err := ParseAlgol(source)
		if err != nil {
			t.Fatalf("Failed to parse assignment %q: %v", source, err)
		}

		if ast.RuleName != "program" {
			t.Errorf("Assignment %q: expected root 'program', got %q",
				source, ast.RuleName)
		}
	}
}

// =============================================================================
// TestParseAlgol_IfStatement
// =============================================================================
//
// Verifies that conditional (if/then/else) statements parse correctly.
//
// ALGOL 60 conditional syntax:
//   if bool_expr then unlabeled_stmt [ else statement ]
//
// The grammar resolves the "dangling else" problem structurally: the
// then-branch uses "unlabeled_stmt" (which cannot contain conditionals),
// while the else-branch uses "statement" (which can). This means you
// cannot write:
//   if a then if b then x := 1 else x := 2
// without wrapping the inner if in begin...end. The grammar rejects it.
//
// In C and Java, the dangling else is "resolved by convention" (else binds
// to the nearest if), but this is not enforced by the grammar — it's just
// a rule compilers follow. ALGOL's approach is cleaner.
func TestParseAlgol_IfStatement(t *testing.T) {
	testCases := []string{
		// Simple if/then
		"begin integer x; if x = 0 then x := 1 end",
		// if/then/else
		"begin integer x; if x = 0 then x := 1 else x := 2 end",
		// Condition with comparison operators
		"begin integer x; if x > 0 then x := x end",
		"begin integer x; if x != 0 then x := 0 end",
	}

	for _, source := range testCases {
		ast, err := ParseAlgol(source)
		if err != nil {
			t.Fatalf("Failed to parse if statement %q: %v", source, err)
		}

		if ast.RuleName != "program" {
			t.Errorf("If statement %q: expected root 'program', got %q",
				source, ast.RuleName)
		}
	}
}

// =============================================================================
// TestParseAlgol_ForLoop
// =============================================================================
//
// Verifies that ALGOL 60 for loops parse correctly.
//
// ALGOL 60's for loop is more powerful than C's: it supports multiple
// "for elements" separated by commas, each of which can be:
//
//   1. step/until:  for i := 1 step 1 until 10 do ...
//      Classic counted range: start, increment, end.
//
//   2. while:       for i := x while x > 0 do ...
//      Conditional: re-evaluate x, loop while condition holds.
//
//   3. simple:      for i := 5 do ...
//      Single value: execute body once with i = 5.
//
// Multiple forms can be combined with commas:
//   for i := 1 step 1 until 5, 10, 20 do ...
//   (loop with i = 1,2,3,4,5, then 10, then 20)
//
// This flexibility means ALGOL's for loop can express iteration patterns
// that require complex logic in C (nested loops, irregular sequences).
func TestParseAlgol_ForLoop(t *testing.T) {
	testCases := []string{
		// step/until form
		"begin integer i; for i := 1 step 1 until 3 do i := i + 1 end",
		// Simple form (single value)
		"begin integer i; for i := 5 do i := i + 1 end",
	}

	for _, source := range testCases {
		ast, err := ParseAlgol(source)
		if err != nil {
			t.Fatalf("Failed to parse for loop %q: %v", source, err)
		}

		if ast.RuleName != "program" {
			t.Errorf("For loop %q: expected root 'program', got %q",
				source, ast.RuleName)
		}
	}
}

// =============================================================================
// TestParseAlgol_Arithmetic
// =============================================================================
//
// Verifies that arithmetic expressions parse correctly.
//
// ALGOL 60 arithmetic expression grammar (lowest to highest precedence):
//
//   simple_arith  → [ +|- ] term { (+|-) term }        (addition/subtraction)
//   term          → factor { (*|/|div|mod) factor }     (multiplication/division)
//   factor        → primary { (^|**) primary }          (exponentiation, LEFT-associative)
//   primary       → INTEGER_LIT | REAL_LIT | variable | ( arith_expr )
//
// Note: div and mod are keywords, not symbols. "15 div 4" = 3 (integer division).
// Note: exponentiation is LEFT-associative: 2^3^4 = (2^3)^4 = 4096, not 2^81.
//
// ALGOL 60's arithmetic is richer than C's: it has both / (real division) and
// div (integer division) as first-class operators, and exponentiation is built in.
func TestParseAlgol_Arithmetic(t *testing.T) {
	testCases := []string{
		"begin integer x; x := 1 + 2 * 3 end",
		"begin integer x; x := (1 + 2) * 3 end",
		"begin real x; x := 2.0 ** 10 end",
		"begin integer x; x := 15 div 4 end",
		"begin integer x; x := 17 mod 5 end",
		"begin integer x; x := -42 end",
	}

	for _, source := range testCases {
		ast, err := ParseAlgol(source)
		if err != nil {
			t.Fatalf("Failed to parse arithmetic %q: %v", source, err)
		}

		if ast.RuleName != "program" {
			t.Errorf("Arithmetic %q: expected root 'program', got %q",
				source, ast.RuleName)
		}
	}
}

// =============================================================================
// TestParseAlgol_NestedBlocks
// =============================================================================
//
// Verifies that nested begin...end blocks parse correctly.
//
// ALGOL 60 allows any statement to be replaced by a block. A block has its
// own declarations and creates a new lexical scope. Variables declared in an
// inner block are only visible inside that block — they shadow outer variables
// of the same name and disappear when the block ends.
//
// This is the foundation of lexical scoping, which every modern language uses.
// Before ALGOL 60, FORTRAN had only global and subroutine-local variables —
// no intermediate nesting, no shadowing.
//
//   begin
//     integer x;
//     x := 1;
//     begin
//       integer x;    -- this x is different from the outer x
//       x := 2
//     end
//   end
func TestParseAlgol_NestedBlocks(t *testing.T) {
	source := "begin integer x; x := 1; begin integer y; y := 2 end end"
	ast, err := ParseAlgol(source)
	if err != nil {
		t.Fatalf("Failed to parse nested blocks: %v", err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Nested blocks: expected root 'program', got %q", ast.RuleName)
	}

	if len(ast.Children) == 0 {
		t.Error("Expected non-empty AST for nested blocks")
	}
}

// =============================================================================
// TestParseAlgol_BooleanExpr
// =============================================================================
//
// Verifies that boolean expressions with compound conditions parse correctly.
//
// ALGOL 60 boolean operator precedence (lowest to highest):
//   eqv   — logical equivalence  (a eqv b = a iff b)
//   impl  — logical implication  (a impl b = not a or b)
//   or    — logical disjunction
//   and   — logical conjunction
//   not   — logical negation (unary, highest among booleans)
//
// These are all keywords, not symbols. This differs from C (&&, ||, !) and
// is closer to mathematical and natural language notation.
//
// "not a and b" means "(not a) and b" because not has higher precedence than and.
// "a or b and c" means "a or (b and c)" because and has higher precedence than or.
func TestParseAlgol_BooleanExpr(t *testing.T) {
	testCases := []string{
		// Single condition
		"begin integer x; if x > 0 then x := 1 end",
		// AND condition
		"begin integer x; if x > 0 and x < 10 then x := 0 end",
		// OR condition
		"begin integer x; if x < 0 or x > 100 then x := 0 end",
		// NOT condition
		"begin integer x; if not x = 0 then x := 0 end",
	}

	for _, source := range testCases {
		ast, err := ParseAlgol(source)
		if err != nil {
			t.Fatalf("Failed to parse boolean expr %q: %v", source, err)
		}

		if ast.RuleName != "program" {
			t.Errorf("Boolean expr %q: expected root 'program', got %q",
				source, ast.RuleName)
		}
	}
}

// =============================================================================
// TestCreateAlgolParser
// =============================================================================
//
// Verifies that CreateAlgolParser returns a valid GrammarParser instance
// that can be used for parsing. This tests the two-step API (create parser,
// then call Parse) as opposed to the one-shot ParseAlgol convenience function.
func TestCreateAlgolParser(t *testing.T) {
	source := "begin integer x; x := 42 end"
	algolParser, err := CreateAlgolParser(source)
	if err != nil {
		t.Fatalf("Failed to create ALGOL parser: %v", err)
	}

	if algolParser == nil {
		t.Fatal("CreateAlgolParser returned nil parser")
	}

	ast, err := algolParser.Parse()
	if err != nil {
		t.Fatalf("Failed to parse with created parser: %v", err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Expected root rule 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// TestParseAlgol_MultipleDeclarations
// =============================================================================
//
// Verifies that blocks with multiple variable declarations parse correctly.
//
// ALGOL 60 declarations come at the start of a block, before any statements.
// Multiple declarations are separated by semicolons. Each declaration can
// declare multiple variables of the same type.
//
//   begin
//     integer x, y, z;
//     real sum;
//     x := 1; y := 2; z := 3;
//     sum := x + y + z
//   end
func TestParseAlgol_MultipleDeclarations(t *testing.T) {
	source := "begin integer x; real r; x := 1; r := 2.5 end"
	ast, err := ParseAlgol(source)
	if err != nil {
		t.Fatalf("Failed to parse multiple declarations: %v", err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Multiple declarations: expected root 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// TestParseAlgol_RealArithmetic
// =============================================================================
//
// Verifies that programs using real (floating-point) values parse correctly.
// ALGOL 60 has a distinct "real" type (IEEE 754 double in modern terms).
// Real literals include decimal points and optional exponents.
func TestParseAlgol_RealArithmetic(t *testing.T) {
	source := "begin real pi; pi := 3.14159 end"
	ast, err := ParseAlgol(source)
	if err != nil {
		t.Fatalf("Failed to parse real arithmetic: %v", err)
	}

	if ast.RuleName != "program" {
		t.Errorf("Real arithmetic: expected root 'program', got %q", ast.RuleName)
	}
}

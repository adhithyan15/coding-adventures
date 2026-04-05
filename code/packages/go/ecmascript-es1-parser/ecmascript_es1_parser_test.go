package ecmascriptes1parser

// ============================================================================
// ECMAScript 1 Parser Tests
// ============================================================================
//
// These tests verify that the ES1 parser correctly produces ASTs for
// the subset of JavaScript defined in ECMA-262, 1st Edition (1997).
//
// The parser uses PEG semantics with packrat memoization. Each test
// verifies that a particular syntactic construct parses successfully
// and produces a tree rooted at "program".
//
// Key ES1 syntactic features:
//   - var declarations (no let/const)
//   - function declarations and expressions
//   - if/else, while, do-while, for, for-in, switch
//   - All expression types with correct precedence
//   - Object and array literals
//
// Key ES1 syntactic ABSENCES:
//   - No try/catch/finally/throw (ES3)
//   - No === or !== in expressions (ES3)
//   - No instanceof operator (ES3)
//   - No debugger statement (ES5)

import (
	"testing"
)

// ---------------------------------------------------------------------------
// Test: Basic variable declaration
// ---------------------------------------------------------------------------
//
// The simplest possible program: `var x = 1;`
// Verifies the parser can handle the fundamental building blocks.

func TestParseEs1_BasicVarDeclaration(t *testing.T) {
	source := "var x = 1;"
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse ES1 source: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Multiple variable declarations
// ---------------------------------------------------------------------------

func TestParseEs1_MultipleVarDeclarations(t *testing.T) {
	source := "var x = 1; var y = 2; var z = 3;"
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Function declaration
// ---------------------------------------------------------------------------

func TestParseEs1_FunctionDeclaration(t *testing.T) {
	source := "function add(a, b) { return a + b; }"
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: If/else statement
// ---------------------------------------------------------------------------

func TestParseEs1_IfElse(t *testing.T) {
	source := "if (x == 1) { foo(2); } else { foo(3); }"
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: While loop
// ---------------------------------------------------------------------------

func TestParseEs1_WhileLoop(t *testing.T) {
	source := "while (i != 0) { i--; }"
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: For loop
// ---------------------------------------------------------------------------

func TestParseEs1_ForLoop(t *testing.T) {
	source := "for (var i = 0; i < 10; i++) { foo(i); }"
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Switch statement
// ---------------------------------------------------------------------------

func TestParseEs1_Switch(t *testing.T) {
	source := `switch (x) { case 1: break; case 2: break; default: break; }`
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Object literal
// ---------------------------------------------------------------------------

func TestParseEs1_ObjectLiteral(t *testing.T) {
	source := `var obj = {a: 1, b: "two"};`
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Array literal
// ---------------------------------------------------------------------------

func TestParseEs1_ArrayLiteral(t *testing.T) {
	source := `var arr = [1, 2, 3];`
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Function expression
// ---------------------------------------------------------------------------

func TestParseEs1_FunctionExpression(t *testing.T) {
	source := `var fn = function(x) { return x + 1; };`
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Nested expressions with operator precedence
// ---------------------------------------------------------------------------

func TestParseEs1_OperatorPrecedence(t *testing.T) {
	source := `var x = 1 + 2 * 3;`
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: CreateEs1Parser returns a valid parser
// ---------------------------------------------------------------------------

func TestCreateEs1Parser(t *testing.T) {
	p, err := CreateEs1Parser("var x = 1;")
	if err != nil {
		t.Fatalf("Failed to create ES1 parser: %v", err)
	}
	if p == nil {
		t.Fatal("Expected non-nil parser")
	}
}

// ---------------------------------------------------------------------------
// Test: Empty program
// ---------------------------------------------------------------------------

func TestParseEs1_EmptyProgram(t *testing.T) {
	program, err := ParseEs1("")
	if err != nil {
		t.Fatalf("Failed to parse empty program: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Do-while loop
// ---------------------------------------------------------------------------

func TestParseEs1_DoWhile(t *testing.T) {
	source := "do { x++; } while (x < 10);"
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Ternary expression
// ---------------------------------------------------------------------------

func TestParseEs1_TernaryExpression(t *testing.T) {
	source := "var x = a ? b : c;"
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Property access
// ---------------------------------------------------------------------------

func TestParseEs1_PropertyAccess(t *testing.T) {
	source := `var x = obj.prop;`
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Function call
// ---------------------------------------------------------------------------

func TestParseEs1_FunctionCall(t *testing.T) {
	source := `foo(1, 2, 3);`
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Comma expression
// ---------------------------------------------------------------------------

func TestParseEs1_CommaExpression(t *testing.T) {
	source := `var x = (1, 2, 3);`
	program, err := ParseEs1(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

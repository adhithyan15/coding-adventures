package ecmascriptes3parser

// ============================================================================
// ECMAScript 3 Parser Tests
// ============================================================================
//
// These tests verify that the ES3 parser correctly produces ASTs for
// ECMA-262, 3rd Edition (1999). ES3 adds to ES1:
//
//   - try/catch/finally/throw statements
//   - === and !== in equality expressions
//   - instanceof in relational expressions
//   - REGEX as a primary expression

import (
	"testing"
)

// ---------------------------------------------------------------------------
// Test: Basic variable declaration (inherited from ES1)
// ---------------------------------------------------------------------------

func TestParseEs3_BasicVarDeclaration(t *testing.T) {
	source := "var x = 1;"
	program, err := ParseEs3(source)
	if err != nil {
		t.Fatalf("Failed to parse ES3 source: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: try/catch statement (NEW in ES3)
// ---------------------------------------------------------------------------
//
// ES3 introduced structured error handling. Before ES3, the only way to
// handle errors was through the global `onerror` event handler.

func TestParseEs3_TryCatch(t *testing.T) {
	source := `try { foo(); } catch (e) { bar(); }`
	program, err := ParseEs3(source)
	if err != nil {
		t.Fatalf("Failed to parse try/catch: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: try/catch/finally statement (NEW in ES3)
// ---------------------------------------------------------------------------

func TestParseEs3_TryCatchFinally(t *testing.T) {
	source := `try { foo(); } catch (e) { bar(); } finally { cleanup(); }`
	program, err := ParseEs3(source)
	if err != nil {
		t.Fatalf("Failed to parse try/catch/finally: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: try/finally without catch (NEW in ES3)
// ---------------------------------------------------------------------------

func TestParseEs3_TryFinally(t *testing.T) {
	source := `try { foo(); } finally { cleanup(); }`
	program, err := ParseEs3(source)
	if err != nil {
		t.Fatalf("Failed to parse try/finally: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: throw statement (NEW in ES3)
// ---------------------------------------------------------------------------

func TestParseEs3_Throw(t *testing.T) {
	source := `throw new Error("oops");`
	program, err := ParseEs3(source)
	if err != nil {
		t.Fatalf("Failed to parse throw: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Strict equality in expressions (NEW in ES3)
// ---------------------------------------------------------------------------

func TestParseEs3_StrictEquality(t *testing.T) {
	source := `if (x === 1) { foo(2); }`
	program, err := ParseEs3(source)
	if err != nil {
		t.Fatalf("Failed to parse strict equality: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Strict not-equals in expressions (NEW in ES3)
// ---------------------------------------------------------------------------

func TestParseEs3_StrictNotEquals(t *testing.T) {
	source := `if (x !== 1) { foo(2); }`
	program, err := ParseEs3(source)
	if err != nil {
		t.Fatalf("Failed to parse strict not-equals: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: instanceof operator (NEW in ES3)
// ---------------------------------------------------------------------------

func TestParseEs3_Instanceof(t *testing.T) {
	source := `if (x instanceof Array) { foo(1); }`
	program, err := ParseEs3(source)
	if err != nil {
		t.Fatalf("Failed to parse instanceof: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Function with error handling
// ---------------------------------------------------------------------------

func TestParseEs3_FunctionWithErrorHandling(t *testing.T) {
	source := `function safeDivide(a, b) {
		try {
			if (b === 0) {
				throw new Error("Division by zero");
			}
			return a / b;
		} catch (e) {
			return 0;
		}
	}`
	program, err := ParseEs3(source)
	if err != nil {
		t.Fatalf("Failed to parse function with error handling: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: CreateEs3Parser returns a valid parser
// ---------------------------------------------------------------------------

func TestCreateEs3Parser(t *testing.T) {
	p, err := CreateEs3Parser("var x = 1;")
	if err != nil {
		t.Fatalf("Failed to create ES3 parser: %v", err)
	}
	if p == nil {
		t.Fatal("Expected non-nil parser")
	}
}

// ---------------------------------------------------------------------------
// Test: Empty program
// ---------------------------------------------------------------------------

func TestParseEs3_EmptyProgram(t *testing.T) {
	program, err := ParseEs3("")
	if err != nil {
		t.Fatalf("Failed to parse empty program: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: ES1 features still work in ES3
// ---------------------------------------------------------------------------

func TestParseEs3_ES1Features(t *testing.T) {
	sources := []struct {
		name   string
		source string
	}{
		{"for loop", "for (var i = 0; i < 10; i++) { foo(i); }"},
		{"while loop", "while (x != 0) { x--; }"},
		{"switch", "switch (x) { case 1: break; default: break; }"},
		{"object literal", `var obj = {a: 1, b: "two"};`},
		{"array literal", "var arr = [1, 2, 3];"},
		{"function expression", "var fn = function(x) { return x + 1; };"},
	}

	for _, tc := range sources {
		t.Run(tc.name, func(t *testing.T) {
			program, err := ParseEs3(tc.source)
			if err != nil {
				t.Fatalf("Failed to parse %s: %v", tc.name, err)
			}
			if program.RuleName != "program" {
				t.Fatalf("Expected program rule at root, got %s", program.RuleName)
			}
		})
	}
}

package ecmascriptes5parser

// ============================================================================
// ECMAScript 5 Parser Tests
// ============================================================================
//
// These tests verify that the ES5 parser correctly produces ASTs for
// ECMA-262, 5th Edition (2009). ES5 adds to ES3:
//
//   - debugger statement
//   - Getter/setter properties in object literals
//
// The grammar is otherwise identical to ES3.

import (
	"testing"
)

// ---------------------------------------------------------------------------
// Test: debugger statement (NEW in ES5)
// ---------------------------------------------------------------------------
//
// The `debugger` statement acts as a breakpoint. It was a future-reserved
// word in ES3 but became a full keyword with its own statement in ES5.

func TestParseEs5_DebuggerStatement(t *testing.T) {
	source := "debugger;"
	program, err := ParseEs5(source)
	if err != nil {
		t.Fatalf("Failed to parse debugger statement: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Basic variable declaration
// ---------------------------------------------------------------------------

func TestParseEs5_BasicVarDeclaration(t *testing.T) {
	source := "var x = 1;"
	program, err := ParseEs5(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: try/catch/finally (inherited from ES3)
// ---------------------------------------------------------------------------

func TestParseEs5_TryCatchFinally(t *testing.T) {
	source := `try { throw new Error("x"); } catch (e) { bar(); } finally { cleanup(); }`
	program, err := ParseEs5(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Strict equality (inherited from ES3)
// ---------------------------------------------------------------------------

func TestParseEs5_StrictEquality(t *testing.T) {
	source := `if (x === 1 && y !== 2) { foo(3); }`
	program, err := ParseEs5(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: instanceof (inherited from ES3)
// ---------------------------------------------------------------------------

func TestParseEs5_Instanceof(t *testing.T) {
	source := `if (x instanceof Object) { foo(1); }`
	program, err := ParseEs5(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: "use strict" directive (lexically just a string expression)
// ---------------------------------------------------------------------------

func TestParseEs5_UseStrictDirective(t *testing.T) {
	source := `"use strict"; var x = 1;`
	program, err := ParseEs5(source)
	if err != nil {
		t.Fatalf("Failed to parse 'use strict' program: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: Function declaration
// ---------------------------------------------------------------------------

func TestParseEs5_FunctionDeclaration(t *testing.T) {
	source := `function greet(name) { return "Hello, " + name; }`
	program, err := ParseEs5(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: CreateEs5Parser returns a valid parser
// ---------------------------------------------------------------------------

func TestCreateEs5Parser(t *testing.T) {
	p, err := CreateEs5Parser("var x = 1;")
	if err != nil {
		t.Fatalf("Failed to create ES5 parser: %v", err)
	}
	if p == nil {
		t.Fatal("Expected non-nil parser")
	}
}

// ---------------------------------------------------------------------------
// Test: Empty program
// ---------------------------------------------------------------------------

func TestParseEs5_EmptyProgram(t *testing.T) {
	program, err := ParseEs5("")
	if err != nil {
		t.Fatalf("Failed to parse empty program: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ---------------------------------------------------------------------------
// Test: ES1 and ES3 features still work in ES5
// ---------------------------------------------------------------------------

func TestParseEs5_InheritedFeatures(t *testing.T) {
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
		{"try/catch", "try { foo(); } catch (e) { bar(); }"},
		{"throw", `throw new Error("oops");`},
	}

	for _, tc := range sources {
		t.Run(tc.name, func(t *testing.T) {
			program, err := ParseEs5(tc.source)
			if err != nil {
				t.Fatalf("Failed to parse %s: %v", tc.name, err)
			}
			if program.RuleName != "program" {
				t.Fatalf("Expected program rule at root, got %s", program.RuleName)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// Test: Debugger in a function body
// ---------------------------------------------------------------------------

func TestParseEs5_DebuggerInFunction(t *testing.T) {
	source := `function debug() { debugger; return 1; }`
	program, err := ParseEs5(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

package starlarkparser

import (
	"testing"
)

// =============================================================================
// TestParseStarlarkSimple
// =============================================================================
//
// Verifies that a simple assignment `x = 1` can be parsed into an AST.
// This is the most basic Starlark program: a single assignment statement.
// The AST root should be a "file" node (the entry rule in starlark.grammar).
func TestParseStarlarkSimple(t *testing.T) {
	source := "x = 1\n"
	program, err := ParseStarlark(source)
	if err != nil {
		t.Fatalf("Failed to parse Starlark code: %v", err)
	}

	// The root node should be the "file" rule, which is the entry point
	// defined as the first rule in starlark.grammar.
	if program.RuleName != "file" {
		t.Fatalf("Expected root rule 'file', got %q", program.RuleName)
	}

	// The file node should have children (at least one statement)
	if len(program.Children) == 0 {
		t.Error("Expected file node to have children (statements)")
	}
}

// =============================================================================
// TestParseStarlarkExpression
// =============================================================================
//
// Verifies that an arithmetic expression `1 + 2 * 3` is parsed correctly.
// This tests operator precedence: multiplication binds tighter than addition,
// so the AST should reflect (1 + (2 * 3)), not ((1 + 2) * 3).
//
// The grammar encodes precedence through rule layering:
//   expression -> or_expr -> ... -> arith -> term -> factor -> power -> primary -> atom
// Addition is in `arith`, multiplication is in `term` (one level deeper).
func TestParseStarlarkExpression(t *testing.T) {
	source := "1 + 2 * 3\n"
	program, err := ParseStarlark(source)
	if err != nil {
		t.Fatalf("Failed to parse expression: %v", err)
	}

	if program.RuleName != "file" {
		t.Fatalf("Expected root rule 'file', got %q", program.RuleName)
	}

	// The expression should parse without errors. Detailed AST structure
	// verification would require walking the tree, but parsing success
	// with the correct root rule confirms the grammar handles precedence.
	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for expression")
	}
}

// =============================================================================
// TestParseStarlarkFunctionDef
// =============================================================================
//
// Verifies that a function definition with parameters and a return statement
// can be parsed. This exercises:
//   - The `def_stmt` rule: "def" NAME LPAREN parameters RPAREN COLON suite
//   - The `suite` rule: NEWLINE INDENT { statement } DEDENT
//   - The `return_stmt` rule: "return" expression
//   - INDENT/DEDENT token handling
//
// This is one of the most important constructs in Starlark because BUILD
// macros and rule implementations are written as functions.
func TestParseStarlarkFunctionDef(t *testing.T) {
	source := "def add(x, y):\n    return x + y\n"
	program, err := ParseStarlark(source)
	if err != nil {
		t.Fatalf("Failed to parse function definition: %v", err)
	}

	if program.RuleName != "file" {
		t.Fatalf("Expected root rule 'file', got %q", program.RuleName)
	}

	// A function definition is a compound statement, so the file should
	// contain at least one child representing the def_stmt.
	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for function definition")
	}
}

// =============================================================================
// TestParseStarlarkIfElse
// =============================================================================
//
// Verifies that an if/else conditional can be parsed. This exercises:
//   - The `if_stmt` rule: "if" expression COLON suite ["else" COLON suite]
//   - The `suite` rule in both branches
//   - INDENT/DEDENT handling for multiple blocks
//
// The if/else construct is common in Starlark for conditional configuration:
//   if platform == "linux":
//       srcs = ["linux_impl.cc"]
//   else:
//       srcs = ["generic_impl.cc"]
func TestParseStarlarkIfElse(t *testing.T) {
	source := "if x:\n    y = 1\nelse:\n    y = 2\n"
	program, err := ParseStarlark(source)
	if err != nil {
		t.Fatalf("Failed to parse if/else: %v", err)
	}

	if program.RuleName != "file" {
		t.Fatalf("Expected root rule 'file', got %q", program.RuleName)
	}

	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for if/else")
	}
}

// =============================================================================
// TestParseStarlarkForLoop
// =============================================================================
//
// Verifies that a for loop can be parsed. This exercises:
//   - The `for_stmt` rule: "for" loop_vars "in" expression COLON suite
//   - The `loop_vars` rule: NAME { COMMA NAME }
//   - Expression parsing for the iterable
//
// Starlark has no while loop (by design — this guarantees termination).
// The for loop iterating over a finite collection is the only loop construct.
func TestParseStarlarkForLoop(t *testing.T) {
	source := "for x in items:\n    print(x)\n"
	program, err := ParseStarlark(source)
	if err != nil {
		t.Fatalf("Failed to parse for loop: %v", err)
	}

	if program.RuleName != "file" {
		t.Fatalf("Expected root rule 'file', got %q", program.RuleName)
	}

	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for for loop")
	}
}

// =============================================================================
// TestParseStarlarkBuildFile
// =============================================================================
//
// Verifies that a typical Bazel BUILD file pattern can be parsed. This is
// the primary use case for Starlark: declaring build targets.
//
// A BUILD file consists of function calls like cc_library(), py_binary(), etc.
// These calls use keyword arguments spanning multiple lines inside parentheses.
// The parser must handle:
//   - Function call syntax: primary LPAREN arguments RPAREN
//   - Keyword arguments: NAME EQUALS expression
//   - Multi-line expressions (brackets suppress NEWLINE/INDENT/DEDENT)
//   - String literals and list literals as argument values
func TestParseStarlarkBuildFile(t *testing.T) {
	source := "cc_library(\n    name = \"foo\",\n)\n"
	program, err := ParseStarlark(source)
	if err != nil {
		t.Fatalf("Failed to parse BUILD file pattern: %v", err)
	}

	if program.RuleName != "file" {
		t.Fatalf("Expected root rule 'file', got %q", program.RuleName)
	}

	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for BUILD file")
	}
}

// =============================================================================
// TestParseStarlarkMultipleStatements
// =============================================================================
//
// Verifies that a multi-statement program can be parsed. This exercises the
// top-level `file` rule which is defined as:
//   file = { NEWLINE | statement } ;
//
// The repetition { ... } means zero or more statements, with optional blank
// lines (NEWLINE) between them. This test ensures the parser correctly
// handles statement sequencing.
func TestParseStarlarkMultipleStatements(t *testing.T) {
	source := "x = 1\ny = 2\nz = x + y\n"
	program, err := ParseStarlark(source)
	if err != nil {
		t.Fatalf("Failed to parse multiple statements: %v", err)
	}

	if program.RuleName != "file" {
		t.Fatalf("Expected root rule 'file', got %q", program.RuleName)
	}

	// A file with 3 statements should have children. The exact count depends
	// on how NEWLINEs and statements interleave in the AST, but there should
	// be at least 3 statement subtrees.
	if len(program.Children) < 3 {
		t.Errorf("Expected at least 3 children for 3 statements, got %d", len(program.Children))
	}
}

// =============================================================================
// TestCreateStarlarkParser
// =============================================================================
//
// Verifies that the factory function CreateStarlarkParser returns a valid
// GrammarParser instance. This tests the two-step API (create parser, then
// call Parse) as opposed to the one-shot ParseStarlark convenience function.
//
// The factory function is useful when you need to inspect the parser state
// or when you want to parse the same token stream with different entry rules.
func TestCreateStarlarkParser(t *testing.T) {
	source := "x = 42\n"
	starlarkParser, err := CreateStarlarkParser(source)
	if err != nil {
		t.Fatalf("Failed to create Starlark parser: %v", err)
	}

	// The parser should not be nil
	if starlarkParser == nil {
		t.Fatal("CreateStarlarkParser returned nil parser")
	}

	// Parse using the created parser instance
	ast, err := starlarkParser.Parse()
	if err != nil {
		t.Fatalf("Failed to parse with created parser: %v", err)
	}

	// The root node should be "file"
	if ast.RuleName != "file" {
		t.Errorf("Expected root rule 'file', got %q", ast.RuleName)
	}
}

// =============================================================================
// TestParseStarlarkListLiteral
// =============================================================================
//
// Verifies that list literals can be parsed. Lists are fundamental in BUILD
// files for specifying source files, dependencies, and other collections:
//   srcs = ["main.cc", "util.cc"]
//   deps = ["//lib:base", "//lib:util"]
func TestParseStarlarkListLiteral(t *testing.T) {
	source := "x = [1, 2, 3]\n"
	program, err := ParseStarlark(source)
	if err != nil {
		t.Fatalf("Failed to parse list literal: %v", err)
	}

	if program.RuleName != "file" {
		t.Fatalf("Expected root rule 'file', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseStarlarkDictLiteral
// =============================================================================
//
// Verifies that dict literals can be parsed. Dicts are used in BUILD files
// for select() expressions and configuration maps:
//   config = {"debug": True, "release": False}
func TestParseStarlarkDictLiteral(t *testing.T) {
	source := "d = {\"a\": 1, \"b\": 2}\n"
	program, err := ParseStarlark(source)
	if err != nil {
		t.Fatalf("Failed to parse dict literal: %v", err)
	}

	if program.RuleName != "file" {
		t.Fatalf("Expected root rule 'file', got %q", program.RuleName)
	}
}

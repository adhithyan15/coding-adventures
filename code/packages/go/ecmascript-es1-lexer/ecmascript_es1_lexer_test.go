package ecmascriptes1lexer

// ============================================================================
// ECMAScript 1 Lexer Tests
// ============================================================================
//
// These tests verify that the ES1 lexer correctly tokenizes the subset of
// JavaScript defined in ECMA-262, 1st Edition (1997). The key characteristics
// of ES1's lexical grammar are:
//
//   - 23 keywords (var, function, if, while, etc.)
//   - Basic operators: arithmetic, bitwise, logical, comparison, assignment
//   - == and != but NO === or !== (strict equality came in ES3)
//   - No try/catch/finally/throw keywords
//   - No regex literals (implementation-defined in ES1)
//   - String literals (single and double quoted)
//   - Numeric literals (decimal, float, hex)
//   - The $ character is valid in identifiers

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ---------------------------------------------------------------------------
// Test: Basic variable declaration
// ---------------------------------------------------------------------------
//
// The simplest ES1 program: `var x = 1;`
// This tests that the lexer can handle the fundamental building blocks:
// a keyword, an identifier, an operator, a number, and a semicolon.

func TestTokenizeEs1_BasicVarDeclaration(t *testing.T) {
	source := `var x = 1;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize ES1 source: %v", err)
	}

	// Expected tokens: KEYWORD(var) NAME(x) EQUALS(=) NUMBER(1) SEMICOLON(;) EOF
	if len(tokens) != 6 {
		t.Fatalf("Expected 6 tokens, got %d: %v", len(tokens), tokens)
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "var" {
		t.Errorf("Expected KEYWORD 'var', got %v %q", tokens[0].Type, tokens[0].Value)
	}
	if tokens[1].Type != lexer.TokenName || tokens[1].Value != "x" {
		t.Errorf("Expected NAME 'x', got %v %q", tokens[1].Type, tokens[1].Value)
	}
	if tokens[4].Type != lexer.TokenSemicolon {
		t.Errorf("Expected SEMICOLON, got %v", tokens[4].Type)
	}
}

// ---------------------------------------------------------------------------
// Test: Arithmetic expression
// ---------------------------------------------------------------------------
//
// ES1 supports all basic arithmetic operators: + - * / %
// This test verifies they are correctly tokenized in a compound expression.

func TestTokenizeEs1_ArithmeticExpression(t *testing.T) {
	source := `var result = 1 + 2 * 3;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// KEYWORD(var) NAME(result) EQUALS(=) NUMBER(1) PLUS(+) NUMBER(2) STAR(*) NUMBER(3) SEMICOLON(;) EOF
	if len(tokens) != 10 {
		t.Fatalf("Expected 10 tokens, got %d", len(tokens))
	}
}

// ---------------------------------------------------------------------------
// Test: Function declaration
// ---------------------------------------------------------------------------
//
// Functions are first-class in ES1. This test verifies that the lexer
// handles the `function` keyword, parentheses, braces, and parameters.

func TestTokenizeEs1_FunctionDeclaration(t *testing.T) {
	source := `function add(a, b) { return a + b; }`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// function add ( a , b ) { return a + b ; }  EOF
	// KEYWORD  NAME LP NAME COMMA NAME RP LB KEYWORD NAME PLUS NAME SEMI RB EOF
	if len(tokens) != 15 {
		t.Fatalf("Expected 15 tokens, got %d", len(tokens))
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "function" {
		t.Errorf("Expected KEYWORD 'function', got %v %q", tokens[0].Type, tokens[0].Value)
	}
	if tokens[8].Type != lexer.TokenKeyword || tokens[8].Value != "return" {
		t.Errorf("Expected KEYWORD 'return', got %v %q", tokens[8].Type, tokens[8].Value)
	}
}

// ---------------------------------------------------------------------------
// Test: If/else statement
// ---------------------------------------------------------------------------
//
// Conditional branching is fundamental to ES1. Both `if` and `else` are
// keywords.

func TestTokenizeEs1_IfElse(t *testing.T) {
	source := `if (x == 1) { y = 2; } else { y = 3; }`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Verify the if and else keywords are present
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "if" {
		t.Errorf("Expected KEYWORD 'if', got %v %q", tokens[0].Type, tokens[0].Value)
	}

	// Find the == operator
	foundDoubleEquals := false
	for _, tok := range tokens {
		if tok.Value == "==" {
			foundDoubleEquals = true
			break
		}
	}
	if !foundDoubleEquals {
		t.Error("Expected to find == operator in if condition")
	}
}

// ---------------------------------------------------------------------------
// Test: ES1 does NOT have === or !==
// ---------------------------------------------------------------------------
//
// This is a critical version-specific test. ES1 only has == and !=.
// The strict equality operators === and !== were added in ES3.
// When the ES1 lexer encounters ===, it should tokenize it as
// == followed by = (two separate tokens), NOT as a single === token.

func TestTokenizeEs1_NoStrictEquality(t *testing.T) {
	source := `x === y;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// In ES1, === should be tokenized as == followed by =
	// because === is not a defined token in ES1.
	// Look for absence of a single === token:
	for _, tok := range tokens {
		if tok.Value == "===" {
			t.Error("ES1 lexer should NOT produce a === token; === was added in ES3")
		}
	}
}

// ---------------------------------------------------------------------------
// Test: ES1 does NOT have !== either
// ---------------------------------------------------------------------------

func TestTokenizeEs1_NoStrictNotEquals(t *testing.T) {
	source := `x !== y;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	for _, tok := range tokens {
		if tok.Value == "!==" {
			t.Error("ES1 lexer should NOT produce a !== token; !== was added in ES3")
		}
	}
}

// ---------------------------------------------------------------------------
// Test: While loop
// ---------------------------------------------------------------------------

func TestTokenizeEs1_WhileLoop(t *testing.T) {
	source := `while (i < 10) { i = i + 1; }`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "while" {
		t.Errorf("Expected KEYWORD 'while', got %v %q", tokens[0].Type, tokens[0].Value)
	}
}

// ---------------------------------------------------------------------------
// Test: String literals (both quote styles)
// ---------------------------------------------------------------------------
//
// ES1 supports both single-quoted and double-quoted strings.

func TestTokenizeEs1_StringLiterals(t *testing.T) {
	source := `var s1 = "hello"; var s2 = 'world';`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Find string tokens
	strings := []string{}
	for _, tok := range tokens {
		if tok.Type == lexer.TokenString {
			strings = append(strings, tok.Value)
		}
	}
	if len(strings) != 2 {
		t.Fatalf("Expected 2 string tokens, got %d: %v", len(strings), strings)
	}
}

// ---------------------------------------------------------------------------
// Test: Numeric literals
// ---------------------------------------------------------------------------
//
// ES1 supports decimal integers, floating point, and hex literals.

func TestTokenizeEs1_NumericLiterals(t *testing.T) {
	source := `var a = 42; var b = 3.14; var c = 0xFF;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	numbers := []string{}
	for _, tok := range tokens {
		if tok.Type == lexer.TokenNumber {
			numbers = append(numbers, tok.Value)
		}
	}
	if len(numbers) != 3 {
		t.Fatalf("Expected 3 number tokens, got %d: %v", len(numbers), numbers)
	}
}

// ---------------------------------------------------------------------------
// Test: Dollar sign in identifiers
// ---------------------------------------------------------------------------
//
// The $ character is valid in ES1 identifiers. This was unusual for 1997
// and was inspired by Java's inner class naming convention.

func TestTokenizeEs1_DollarInIdentifiers(t *testing.T) {
	source := `var $x = 1; var _y$ = 2;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	names := []string{}
	for _, tok := range tokens {
		if tok.Type == lexer.TokenName {
			names = append(names, tok.Value)
		}
	}
	if len(names) < 2 {
		t.Fatalf("Expected at least 2 NAME tokens, got %d: %v", len(names), names)
	}
	if names[0] != "$x" {
		t.Errorf("Expected NAME '$x', got %q", names[0])
	}
	if names[1] != "_y$" {
		t.Errorf("Expected NAME '_y$', got %q", names[1])
	}
}

// ---------------------------------------------------------------------------
// Test: ES1 keywords are recognized
// ---------------------------------------------------------------------------
//
// Verify that core ES1 keywords are classified as KEYWORD tokens.

func TestTokenizeEs1_Keywords(t *testing.T) {
	// Test a selection of ES1 keywords
	keywords := []string{"var", "function", "if", "else", "while", "for", "return",
		"switch", "case", "break", "continue", "do", "new", "delete", "typeof",
		"void", "with", "this", "in", "default", "true", "false", "null"}

	for _, kw := range keywords {
		source := kw + ";"
		tokens, err := TokenizeEs1(source)
		if err != nil {
			t.Fatalf("Failed to tokenize keyword %q: %v", kw, err)
		}
		if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != kw {
			t.Errorf("Expected KEYWORD %q, got %v %q", kw, tokens[0].Type, tokens[0].Value)
		}
	}
}

// ---------------------------------------------------------------------------
// Test: ES1 does NOT have try/catch/finally/throw as keywords
// ---------------------------------------------------------------------------
//
// Error handling was added in ES3. In ES1, these are just identifiers.

func TestTokenizeEs1_NoTryCatch(t *testing.T) {
	// In ES1, 'try' and 'catch' should be identifiers, not keywords
	es3Keywords := []string{"try", "catch", "finally", "throw", "instanceof"}
	for _, word := range es3Keywords {
		source := word + ";"
		tokens, err := TokenizeEs1(source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", word, err)
		}
		// These should NOT be keywords in ES1
		if tokens[0].Type == lexer.TokenKeyword && tokens[0].Value == word {
			t.Errorf("%q should NOT be a keyword in ES1 (it was added in ES3)", word)
		}
	}
}

// ---------------------------------------------------------------------------
// Test: Assignment operators
// ---------------------------------------------------------------------------

func TestTokenizeEs1_AssignmentOperators(t *testing.T) {
	source := `x += 1; y -= 2; z *= 3; w /= 4; v %= 5;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	ops := []string{}
	for _, tok := range tokens {
		if tok.Value == "+=" || tok.Value == "-=" || tok.Value == "*=" ||
			tok.Value == "/=" || tok.Value == "%=" {
			ops = append(ops, tok.Value)
		}
	}
	if len(ops) != 5 {
		t.Fatalf("Expected 5 compound assignment operators, got %d: %v", len(ops), ops)
	}
}

// ---------------------------------------------------------------------------
// Test: Bitwise operators
// ---------------------------------------------------------------------------

func TestTokenizeEs1_BitwiseOperators(t *testing.T) {
	source := `var a = x & y; var b = x | y; var c = x ^ y; var d = ~x; var e = x << 2; var f = x >> 1; var g = x >>> 1;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Just verify it tokenized without error and has a reasonable number of tokens
	if len(tokens) < 20 {
		t.Errorf("Expected many tokens for bitwise expression, got %d", len(tokens))
	}
}

// ---------------------------------------------------------------------------
// Test: Comments are skipped
// ---------------------------------------------------------------------------

func TestTokenizeEs1_Comments(t *testing.T) {
	source := `// this is a comment
var x = 1; /* block comment */ var y = 2;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Comments should be skipped; only real tokens remain
	for _, tok := range tokens {
		if tok.Value == "// this is a comment" || tok.Value == "/* block comment */" {
			t.Error("Comment tokens should be skipped")
		}
	}
}

// ---------------------------------------------------------------------------
// Test: Increment and decrement operators
// ---------------------------------------------------------------------------

func TestTokenizeEs1_IncrementDecrement(t *testing.T) {
	source := `x++; y--;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	found := map[string]bool{}
	for _, tok := range tokens {
		if tok.Value == "++" || tok.Value == "--" {
			found[tok.Value] = true
		}
	}
	if !found["++"] {
		t.Error("Expected ++ operator")
	}
	if !found["--"] {
		t.Error("Expected -- operator")
	}
}

// ---------------------------------------------------------------------------
// Test: Logical operators
// ---------------------------------------------------------------------------

func TestTokenizeEs1_LogicalOperators(t *testing.T) {
	source := `if (a && b || !c) {}`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	found := map[string]bool{}
	for _, tok := range tokens {
		if tok.Value == "&&" || tok.Value == "||" || tok.Value == "!" {
			found[tok.Value] = true
		}
	}
	if !found["&&"] {
		t.Error("Expected && operator")
	}
	if !found["||"] {
		t.Error("Expected || operator")
	}
	if !found["!"] {
		t.Error("Expected ! operator")
	}
}

// ---------------------------------------------------------------------------
// Test: Ternary operator
// ---------------------------------------------------------------------------

func TestTokenizeEs1_TernaryOperator(t *testing.T) {
	source := `var x = a ? b : c;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	found := map[string]bool{}
	for _, tok := range tokens {
		if tok.Value == "?" || tok.Value == ":" {
			found[tok.Value] = true
		}
	}
	if !found["?"] || !found[":"] {
		t.Error("Expected ? and : for ternary operator")
	}
}

// ---------------------------------------------------------------------------
// Test: CreateEs1Lexer returns a valid lexer
// ---------------------------------------------------------------------------

func TestCreateEs1Lexer(t *testing.T) {
	l, err := CreateEs1Lexer("var x = 1;")
	if err != nil {
		t.Fatalf("Failed to create ES1 lexer: %v", err)
	}
	if l == nil {
		t.Fatal("Expected non-nil lexer")
	}
}

// ---------------------------------------------------------------------------
// Test: Empty source
// ---------------------------------------------------------------------------

func TestTokenizeEs1_EmptySource(t *testing.T) {
	tokens, err := TokenizeEs1("")
	if err != nil {
		t.Fatalf("Failed to tokenize empty source: %v", err)
	}
	// Should have at least an EOF token
	if len(tokens) < 1 {
		t.Error("Expected at least one token (EOF) for empty source")
	}
}

// ---------------------------------------------------------------------------
// Test: Scientific notation numbers
// ---------------------------------------------------------------------------

func TestTokenizeEs1_ScientificNotation(t *testing.T) {
	source := `var x = 1e10; var y = 2.5E-3;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	numbers := []string{}
	for _, tok := range tokens {
		if tok.Type == lexer.TokenNumber {
			numbers = append(numbers, tok.Value)
		}
	}
	if len(numbers) != 2 {
		t.Fatalf("Expected 2 number tokens, got %d: %v", len(numbers), numbers)
	}
}

// ---------------------------------------------------------------------------
// Test: Object and array literals
// ---------------------------------------------------------------------------

func TestTokenizeEs1_ObjectArrayLiterals(t *testing.T) {
	source := `var obj = {a: 1, b: "two"}; var arr = [1, 2, 3];`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if len(tokens) < 10 {
		t.Errorf("Expected many tokens for object/array literals, got %d", len(tokens))
	}
}

// ---------------------------------------------------------------------------
// Test: Reserved words cause panics when used as identifiers
// ---------------------------------------------------------------------------
//
// In ES1, future-reserved words (class, const, enum, export, extends,
// import, super) cannot be used as identifiers. The lexer enforces this
// by panicking when it encounters one of these words in an identifier
// position. This is a lex-time error, not a parse-time error.

func TestTokenizeEs1_ReservedWordsPanic(t *testing.T) {
	reserved := []string{"class", "const", "enum", "export", "extends", "import", "super"}
	for _, rw := range reserved {
		rw := rw // capture loop variable
		t.Run(rw, func(t *testing.T) {
			defer func() {
				r := recover()
				if r == nil {
					t.Errorf("Expected panic for reserved word %q, but none occurred", rw)
				}
			}()
			// Using a reserved word should panic
			TokenizeEs1(rw + ";")
		})
	}
}

// ---------------------------------------------------------------------------
// Test: Comparison operators
// ---------------------------------------------------------------------------

func TestTokenizeEs1_ComparisonOperators(t *testing.T) {
	source := `x < y; x > y; x <= y; x >= y; x == y; x != y;`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	ops := map[string]bool{}
	for _, tok := range tokens {
		switch tok.Value {
		case "<", ">", "<=", ">=", "==", "!=":
			ops[tok.Value] = true
		}
	}
	expected := []string{"<", ">", "<=", ">=", "==", "!="}
	for _, op := range expected {
		if !ops[op] {
			t.Errorf("Expected operator %q to be found", op)
		}
	}
}

// ---------------------------------------------------------------------------
// Test: String with escape sequences
// ---------------------------------------------------------------------------

func TestTokenizeEs1_StringEscapes(t *testing.T) {
	source := `var s = "hello\nworld\t!";`
	tokens, err := TokenizeEs1(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	found := false
	for _, tok := range tokens {
		if tok.Type == lexer.TokenString {
			found = true
		}
	}
	if !found {
		t.Error("Expected to find a STRING token")
	}
}

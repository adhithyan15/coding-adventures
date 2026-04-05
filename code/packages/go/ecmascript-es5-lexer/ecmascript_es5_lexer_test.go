package ecmascriptes5lexer

// ============================================================================
// ECMAScript 5 Lexer Tests
// ============================================================================
//
// These tests verify that the ES5 lexer correctly tokenizes ECMA-262,
// 5th Edition (2009). ES5's lexical changes over ES3 are modest:
//
//   - `debugger` promoted from future-reserved to keyword
//   - Reduced future-reserved word list (many ES3 reserved words freed)
//   - All ES3 operators and syntax retained
//
// The big ES5 changes (strict mode, JSON, property descriptors) are
// semantic, not lexical — the token grammar is nearly identical to ES3.

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ---------------------------------------------------------------------------
// Test: debugger is a KEYWORD in ES5 (promoted from reserved in ES3)
// ---------------------------------------------------------------------------
//
// This is the key version-specific change. In ES3, `debugger` was a
// future-reserved word. ES5 promotes it to a full keyword with its own
// statement production: `debugger;` acts as a breakpoint.

func TestTokenizeEs5_DebuggerKeyword(t *testing.T) {
	source := `debugger;`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "debugger" {
		t.Errorf("Expected KEYWORD 'debugger' in ES5, got %v %q", tokens[0].Type, tokens[0].Value)
	}
}

// ---------------------------------------------------------------------------
// Test: All ES5 keywords
// ---------------------------------------------------------------------------
//
// ES5 has all ES3 keywords plus debugger.

func TestTokenizeEs5_AllKeywords(t *testing.T) {
	keywords := []string{
		"break", "case", "catch", "continue", "debugger", "default", "delete",
		"do", "else", "finally", "for", "function", "if", "in", "instanceof",
		"new", "return", "switch", "this", "throw", "try", "typeof",
		"var", "void", "while", "with", "true", "false", "null",
	}
	for _, kw := range keywords {
		source := kw + ";"
		tokens, err := TokenizeEs5(source)
		if err != nil {
			t.Fatalf("Failed to tokenize keyword %q: %v", kw, err)
		}
		if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != kw {
			t.Errorf("Expected KEYWORD %q, got %v %q", kw, tokens[0].Type, tokens[0].Value)
		}
	}
}

// ---------------------------------------------------------------------------
// Test: Strict equality operators (inherited from ES3)
// ---------------------------------------------------------------------------

func TestTokenizeEs5_StrictEquality(t *testing.T) {
	source := `x === y; a !== b;`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	found := map[string]bool{}
	for _, tok := range tokens {
		if tok.Value == "===" || tok.Value == "!==" {
			found[tok.Value] = true
		}
	}
	if !found["==="] {
		t.Error("Expected === operator in ES5")
	}
	if !found["!=="] {
		t.Error("Expected !== operator in ES5")
	}
}

// ---------------------------------------------------------------------------
// Test: try/catch/finally (inherited from ES3)
// ---------------------------------------------------------------------------

func TestTokenizeEs5_TryCatchFinally(t *testing.T) {
	source := `try { throw new Error("x"); } catch (e) {} finally {}`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	kws := map[string]bool{}
	for _, tok := range tokens {
		if tok.Type == lexer.TokenKeyword {
			kws[tok.Value] = true
		}
	}
	for _, expected := range []string{"try", "throw", "new", "catch", "finally"} {
		if !kws[expected] {
			t.Errorf("Expected keyword %q in ES5 source", expected)
		}
	}
}

// ---------------------------------------------------------------------------
// Test: ES5 reduced future-reserved word list
// ---------------------------------------------------------------------------
//
// ES5 significantly reduces the future-reserved list compared to ES3.
// Words like abstract, boolean, byte, char, etc. are no longer reserved
// in ES5 non-strict mode. Only class, const, enum, export, extends,
// import, and super remain reserved.

func TestTokenizeEs5_ReservedWordsPanic(t *testing.T) {
	reserved := []string{"class", "const", "enum", "export", "extends", "import", "super"}
	for _, rw := range reserved {
		rw := rw
		t.Run(rw, func(t *testing.T) {
			defer func() {
				r := recover()
				if r == nil {
					t.Errorf("Expected panic for reserved word %q in ES5, but none occurred", rw)
				}
			}()
			TokenizeEs5(rw + ";")
		})
	}
}

// ---------------------------------------------------------------------------
// Test: ES3-only reserved words are identifiers in ES5
// ---------------------------------------------------------------------------
//
// Many words that were reserved in ES3 are freed in ES5 non-strict mode.
// For example, `abstract`, `boolean`, `byte`, etc. should now be valid
// identifiers (NAME tokens).

func TestTokenizeEs5_FreedReservedWords(t *testing.T) {
	// Words that were reserved in ES3 but are freed in ES5 non-strict mode.
	// These should tokenize as NAME tokens without panicking.
	freedWords := []string{
		"abstract", "boolean", "byte", "char", "double", "final", "float",
		"goto", "int", "long", "native", "short", "synchronized",
		"throws", "transient", "volatile",
	}
	for _, word := range freedWords {
		word := word
		t.Run(word, func(t *testing.T) {
			source := word + ";"
			tokens, err := TokenizeEs5(source)
			if err != nil {
				t.Fatalf("Failed to tokenize freed reserved word %q: %v", word, err)
			}
			// These should be NAME in ES5 (no longer reserved)
			if tokens[0].Type != lexer.TokenName {
				t.Errorf("Expected NAME for freed word %q in ES5, got %v %q", word, tokens[0].Type, tokens[0].Value)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// Test: Basic variable declaration
// ---------------------------------------------------------------------------

func TestTokenizeEs5_BasicVarDeclaration(t *testing.T) {
	source := `var x = 1;`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if len(tokens) != 6 {
		t.Fatalf("Expected 6 tokens, got %d", len(tokens))
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "var" {
		t.Errorf("Expected KEYWORD 'var', got %v %q", tokens[0].Type, tokens[0].Value)
	}
}

// ---------------------------------------------------------------------------
// Test: Function declaration
// ---------------------------------------------------------------------------

func TestTokenizeEs5_FunctionDeclaration(t *testing.T) {
	source := `function greet(name) { return "Hello, " + name; }`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "function" {
		t.Errorf("Expected KEYWORD 'function', got %v %q", tokens[0].Type, tokens[0].Value)
	}
}

// ---------------------------------------------------------------------------
// Test: "use strict" directive (lexed as a string literal)
// ---------------------------------------------------------------------------
//
// ES5 introduced strict mode via the "use strict" directive prologue.
// Lexically, this is just a string expression statement — the lexer
// treats it as a regular string. The parser/runtime recognizes the
// semantic significance.

func TestTokenizeEs5_UseStrictDirective(t *testing.T) {
	source := `"use strict"; var x = 1;`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if tokens[0].Type != lexer.TokenString {
		t.Errorf("Expected STRING for 'use strict' directive, got %v", tokens[0].Type)
	}
}

// ---------------------------------------------------------------------------
// Test: Object literal with getter/setter syntax tokens
// ---------------------------------------------------------------------------
//
// ES5 added getter/setter syntax. Lexically, `get` and `set` are just
// NAME tokens — they're not keywords. The parser uses them contextually.

func TestTokenizeEs5_GetterSetterTokens(t *testing.T) {
	source := `var obj = { get name() { return this._name; }, set name(v) { this._name = v; } };`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// `get` and `set` should be NAME tokens (not keywords)
	names := []string{}
	for _, tok := range tokens {
		if tok.Type == lexer.TokenName {
			names = append(names, tok.Value)
		}
	}
	foundGet := false
	foundSet := false
	for _, n := range names {
		if n == "get" {
			foundGet = true
		}
		if n == "set" {
			foundSet = true
		}
	}
	if !foundGet {
		t.Error("Expected NAME 'get' in getter/setter object literal")
	}
	if !foundSet {
		t.Error("Expected NAME 'set' in getter/setter object literal")
	}
}

// ---------------------------------------------------------------------------
// Test: Numeric and string literals
// ---------------------------------------------------------------------------

func TestTokenizeEs5_Literals(t *testing.T) {
	source := `var a = 42; var b = 3.14; var c = 0xFF; var d = "hello"; var e = 'world'; var f = 1e10;`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	numbers := 0
	strings := 0
	for _, tok := range tokens {
		if tok.Type == lexer.TokenNumber {
			numbers++
		}
		if tok.Type == lexer.TokenString {
			strings++
		}
	}
	if numbers != 4 {
		t.Errorf("Expected 4 number tokens, got %d", numbers)
	}
	if strings != 2 {
		t.Errorf("Expected 2 string tokens, got %d", strings)
	}
}

// ---------------------------------------------------------------------------
// Test: CreateEs5Lexer returns a valid lexer
// ---------------------------------------------------------------------------

func TestCreateEs5Lexer(t *testing.T) {
	l, err := CreateEs5Lexer("debugger;")
	if err != nil {
		t.Fatalf("Failed to create ES5 lexer: %v", err)
	}
	if l == nil {
		t.Fatal("Expected non-nil lexer")
	}
}

// ---------------------------------------------------------------------------
// Test: Empty source
// ---------------------------------------------------------------------------

func TestTokenizeEs5_EmptySource(t *testing.T) {
	tokens, err := TokenizeEs5("")
	if err != nil {
		t.Fatalf("Failed to tokenize empty source: %v", err)
	}
	if len(tokens) < 1 {
		t.Error("Expected at least one token (EOF) for empty source")
	}
}

// ---------------------------------------------------------------------------
// Test: Comments
// ---------------------------------------------------------------------------

func TestTokenizeEs5_Comments(t *testing.T) {
	source := `// ES5 comment
var x = 1; /* block */ var y = 2;`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	for _, tok := range tokens {
		if tok.Value == "// ES5 comment" || tok.Value == "/* block */" {
			t.Error("Comments should be skipped")
		}
	}
}

// ---------------------------------------------------------------------------
// Test: Dollar and underscore in identifiers
// ---------------------------------------------------------------------------

func TestTokenizeEs5_IdentifierCharacters(t *testing.T) {
	source := `var $jquery = 1; var _private = 2; var camelCase = 3;`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	names := []string{}
	for _, tok := range tokens {
		if tok.Type == lexer.TokenName {
			names = append(names, tok.Value)
		}
	}
	if len(names) < 3 {
		t.Fatalf("Expected at least 3 NAME tokens, got %d: %v", len(names), names)
	}
}

// ---------------------------------------------------------------------------
// Test: All operator types
// ---------------------------------------------------------------------------

func TestTokenizeEs5_AllOperators(t *testing.T) {
	source := `a + b - c * d / e % f; a & b | c ^ d; ~a; a << b >> c >>> d; a && b || c; !a; a == b != c === d !== e; a < b > c <= d >= e;`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if len(tokens) < 30 {
		t.Errorf("Expected many tokens for operator expression, got %d", len(tokens))
	}
}

// ---------------------------------------------------------------------------
// Test: instanceof (inherited from ES3)
// ---------------------------------------------------------------------------

func TestTokenizeEs5_Instanceof(t *testing.T) {
	source := `x instanceof Object;`
	tokens, err := TokenizeEs5(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	found := false
	for _, tok := range tokens {
		if tok.Type == lexer.TokenKeyword && tok.Value == "instanceof" {
			found = true
			break
		}
	}
	if !found {
		t.Error("Expected 'instanceof' keyword in ES5")
	}
}

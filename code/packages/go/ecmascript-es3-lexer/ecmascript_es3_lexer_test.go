package ecmascriptes3lexer

// ============================================================================
// ECMAScript 3 Lexer Tests
// ============================================================================
//
// These tests verify that the ES3 lexer correctly tokenizes the expanded
// token set defined in ECMA-262, 3rd Edition (1999). ES3 added:
//
//   - === and !== (strict equality operators)
//   - try/catch/finally/throw keywords
//   - instanceof keyword
//   - Regular expression literals (/pattern/flags)
//   - Expanded future-reserved words

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ---------------------------------------------------------------------------
// Test: Basic variable declaration (same as ES1)
// ---------------------------------------------------------------------------

func TestTokenizeEs3_BasicVarDeclaration(t *testing.T) {
	source := `var x = 1;`
	tokens, err := TokenizeEs3(source)
	if err != nil {
		t.Fatalf("Failed to tokenize ES3 source: %v", err)
	}

	if len(tokens) != 6 {
		t.Fatalf("Expected 6 tokens, got %d: %v", len(tokens), tokens)
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "var" {
		t.Errorf("Expected KEYWORD 'var', got %v %q", tokens[0].Type, tokens[0].Value)
	}
}

// ---------------------------------------------------------------------------
// Test: Strict equality operator === (NEW in ES3)
// ---------------------------------------------------------------------------
//
// This is the marquee feature of ES3's lexical grammar. The strict equality
// operator === compares without type coercion:
//   "" == 0    -> true  (abstract equality coerces types)
//   "" === 0   -> false (strict equality, different types)

func TestTokenizeEs3_StrictEquals(t *testing.T) {
	source := `x === y;`
	tokens, err := TokenizeEs3(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Expected: NAME(x) STRICT_EQUALS(===) NAME(y) SEMICOLON(;) EOF
	found := false
	for _, tok := range tokens {
		if tok.Value == "===" {
			found = true
			break
		}
	}
	if !found {
		t.Error("ES3 lexer should produce a === token")
	}
}

// ---------------------------------------------------------------------------
// Test: Strict not-equals operator !== (NEW in ES3)
// ---------------------------------------------------------------------------

func TestTokenizeEs3_StrictNotEquals(t *testing.T) {
	source := `x !== y;`
	tokens, err := TokenizeEs3(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	found := false
	for _, tok := range tokens {
		if tok.Value == "!==" {
			found = true
			break
		}
	}
	if !found {
		t.Error("ES3 lexer should produce a !== token")
	}
}

// ---------------------------------------------------------------------------
// Test: Both strict and abstract equality in same source
// ---------------------------------------------------------------------------
//
// Verifies the lexer correctly distinguishes === from == and !== from !=
// when they appear in the same source.

func TestTokenizeEs3_MixedEquality(t *testing.T) {
	source := `if (x === y && a == b && c !== d && e != f) {}`
	tokens, err := TokenizeEs3(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	ops := map[string]int{}
	for _, tok := range tokens {
		switch tok.Value {
		case "===", "!==", "==", "!=":
			ops[tok.Value]++
		}
	}
	if ops["==="] != 1 {
		t.Errorf("Expected 1 ===, got %d", ops["==="])
	}
	if ops["!=="] != 1 {
		t.Errorf("Expected 1 !==, got %d", ops["!=="])
	}
	if ops["=="] != 1 {
		t.Errorf("Expected 1 ==, got %d", ops["=="])
	}
	if ops["!="] != 1 {
		t.Errorf("Expected 1 !=, got %d", ops["!="])
	}
}

// ---------------------------------------------------------------------------
// Test: try/catch/finally keywords (NEW in ES3)
// ---------------------------------------------------------------------------
//
// ES3 introduced structured error handling. Before ES3, the only way to
// handle errors was through the global `onerror` event handler.

func TestTokenizeEs3_TryCatchFinally(t *testing.T) {
	source := `try { x = 1; } catch (e) { x = 0; } finally { cleanup(); }`
	tokens, err := TokenizeEs3(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expectedKeywords := map[string]bool{"try": false, "catch": false, "finally": false}
	for _, tok := range tokens {
		if tok.Type == lexer.TokenKeyword {
			if _, ok := expectedKeywords[tok.Value]; ok {
				expectedKeywords[tok.Value] = true
			}
		}
	}
	for kw, found := range expectedKeywords {
		if !found {
			t.Errorf("Expected keyword %q to be found in ES3 source", kw)
		}
	}
}

// ---------------------------------------------------------------------------
// Test: throw keyword (NEW in ES3)
// ---------------------------------------------------------------------------

func TestTokenizeEs3_Throw(t *testing.T) {
	source := `throw new Error("oops");`
	tokens, err := TokenizeEs3(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "throw" {
		t.Errorf("Expected KEYWORD 'throw', got %v %q", tokens[0].Type, tokens[0].Value)
	}
}

// ---------------------------------------------------------------------------
// Test: instanceof keyword (NEW in ES3)
// ---------------------------------------------------------------------------
//
// The instanceof operator checks whether an object's prototype chain
// includes a constructor's prototype: `myArray instanceof Array`

func TestTokenizeEs3_Instanceof(t *testing.T) {
	source := `if (x instanceof Array) {}`
	tokens, err := TokenizeEs3(source)
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
		t.Error("Expected 'instanceof' keyword in ES3")
	}
}

// ---------------------------------------------------------------------------
// Test: All ES3 keywords
// ---------------------------------------------------------------------------

func TestTokenizeEs3_AllKeywords(t *testing.T) {
	keywords := []string{
		"break", "case", "catch", "continue", "default", "delete", "do",
		"else", "finally", "for", "function", "if", "in", "instanceof",
		"new", "return", "switch", "this", "throw", "try", "typeof",
		"var", "void", "while", "with", "true", "false", "null",
	}
	for _, kw := range keywords {
		source := kw + ";"
		tokens, err := TokenizeEs3(source)
		if err != nil {
			t.Fatalf("Failed to tokenize keyword %q: %v", kw, err)
		}
		if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != kw {
			t.Errorf("Expected KEYWORD %q, got %v %q", kw, tokens[0].Type, tokens[0].Value)
		}
	}
}

// ---------------------------------------------------------------------------
// Test: ES3 expanded reserved words
// ---------------------------------------------------------------------------
//
// ES3 significantly expanded the future-reserved word list compared to ES1.
// Many of these eventually became real keywords in ES2015.

func TestTokenizeEs3_ReservedWordsPanic(t *testing.T) {
	reserved := []string{
		"abstract", "boolean", "byte", "char", "class", "const", "debugger",
		"double", "enum", "export", "extends", "final", "float", "goto",
		"implements", "import", "int", "interface", "long", "native",
		"package", "private", "protected", "public", "short", "static",
		"super", "synchronized", "throws", "transient", "volatile",
	}
	for _, rw := range reserved {
		rw := rw
		t.Run(rw, func(t *testing.T) {
			defer func() {
				r := recover()
				if r == nil {
					t.Errorf("Expected panic for reserved word %q, but none occurred", rw)
				}
			}()
			TokenizeEs3(rw + ";")
		})
	}
}

// ---------------------------------------------------------------------------
// Test: Function with try/catch
// ---------------------------------------------------------------------------

func TestTokenizeEs3_FunctionWithTryCatch(t *testing.T) {
	source := `function safeDivide(a, b) {
		try {
			return a / b;
		} catch (e) {
			return 0;
		}
	}`
	tokens, err := TokenizeEs3(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if len(tokens) < 20 {
		t.Errorf("Expected many tokens, got %d", len(tokens))
	}
}

// ---------------------------------------------------------------------------
// Test: CreateEs3Lexer returns a valid lexer
// ---------------------------------------------------------------------------

func TestCreateEs3Lexer(t *testing.T) {
	l, err := CreateEs3Lexer("var x = 1;")
	if err != nil {
		t.Fatalf("Failed to create ES3 lexer: %v", err)
	}
	if l == nil {
		t.Fatal("Expected non-nil lexer")
	}
}

// ---------------------------------------------------------------------------
// Test: Empty source
// ---------------------------------------------------------------------------

func TestTokenizeEs3_EmptySource(t *testing.T) {
	tokens, err := TokenizeEs3("")
	if err != nil {
		t.Fatalf("Failed to tokenize empty source: %v", err)
	}
	if len(tokens) < 1 {
		t.Error("Expected at least one token (EOF) for empty source")
	}
}

// ---------------------------------------------------------------------------
// Test: Numeric and string literals (inherited from ES1)
// ---------------------------------------------------------------------------

func TestTokenizeEs3_Literals(t *testing.T) {
	source := `var a = 42; var b = 3.14; var c = 0xFF; var d = "hello"; var e = 'world';`
	tokens, err := TokenizeEs3(source)
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
	if numbers != 3 {
		t.Errorf("Expected 3 number tokens, got %d", numbers)
	}
	if strings != 2 {
		t.Errorf("Expected 2 string tokens, got %d", strings)
	}
}

// ---------------------------------------------------------------------------
// Test: Comments are skipped
// ---------------------------------------------------------------------------

func TestTokenizeEs3_Comments(t *testing.T) {
	source := `// line comment
var x = 1; /* block */ var y = 2;`
	tokens, err := TokenizeEs3(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	for _, tok := range tokens {
		if tok.Value == "// line comment" || tok.Value == "/* block */" {
			t.Error("Comments should be skipped")
		}
	}
}

// ---------------------------------------------------------------------------
// Test: Assignment and compound operators
// ---------------------------------------------------------------------------

func TestTokenizeEs3_CompoundAssignment(t *testing.T) {
	source := `x += 1; y -= 2; z *= 3; w /= 4; v %= 5; a &= b; c |= d; e ^= f; g <<= 1; h >>= 1; i >>>= 1;`
	tokens, err := TokenizeEs3(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	ops := []string{}
	for _, tok := range tokens {
		if len(tok.Value) >= 2 && tok.Value[len(tok.Value)-1] == '=' &&
			tok.Value != "==" && tok.Value != "!=" && tok.Value != "===" && tok.Value != "!==" &&
			tok.Value != "<=" && tok.Value != ">=" {
			ops = append(ops, tok.Value)
		}
	}
	if len(ops) != 11 {
		t.Errorf("Expected 11 compound assignment operators, got %d: %v", len(ops), ops)
	}
}

// ---------------------------------------------------------------------------
// Test: debugger is reserved in ES3 (not a keyword)
// ---------------------------------------------------------------------------

func TestTokenizeEs3_DebuggerIsReserved(t *testing.T) {
	// In ES3, `debugger` is a future-reserved word. Using it should panic
	// because the lexer treats reserved words as lex-time errors.
	defer func() {
		r := recover()
		if r == nil {
			t.Error("Expected panic for reserved word 'debugger' in ES3, but none occurred")
		}
	}()
	TokenizeEs3(`debugger;`)
}

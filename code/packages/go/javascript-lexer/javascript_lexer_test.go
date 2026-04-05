package javascriptlexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ─────────────────────────────────────────────────────────────────────────────
// Generic-grammar tests (version = "")
//
// These tests use the default grammar, which is the superset of all supported
// ECMAScript versions. They confirm that the version-agnostic code path still
// works exactly as it did in v0.1.0.
// ─────────────────────────────────────────────────────────────────────────────

func TestTokenizeJavascript(t *testing.T) {
	source := `let x = 1 + 2;`
	tokens, err := TokenizeJavascript(source, "")
	if err != nil {
		t.Fatalf("Failed to tokenize JavaScript source: %v", err)
	}

	// Expected: KEYWORD(let) NAME(x) EQUALS(=) NUMBER(1) PLUS(+) NUMBER(2) SEMICOLON(;) EOF
	if len(tokens) != 8 {
		t.Fatalf("Expected 8 tokens, got %v", len(tokens))
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "let" {
		t.Errorf("Expected first token to be KEYWORD 'let', got %v %v", tokens[0].Type, tokens[0].Value)
	}

	if tokens[1].Type != lexer.TokenName || tokens[1].Value != "x" {
		t.Errorf("Expected second token to be NAME 'x', got %v %v", tokens[1].Type, tokens[1].Value)
	}

	if tokens[6].Type != lexer.TokenSemicolon || tokens[6].Value != ";" {
		t.Errorf("Expected semicolon token, got %v %v", tokens[6].Type, tokens[6].Value)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Versioned-grammar tests
//
// Each test uses a specific ECMAScript version grammar. We only assert that
// the request does not error and produces a recognisable first token, because
// the exact keyword lists differ per version. The goal is to confirm that the
// file-routing logic reaches the correct grammar file.
// ─────────────────────────────────────────────────────────────────────────────

// TestTokenizeJavascriptVersion_es1 verifies ECMAScript 1 grammar loading.
// ECMAScript 1 (June 1997): the first ISO standard; `var` was the only
// declaration keyword, there was no `let` or `const`.
func TestTokenizeJavascriptVersion_es1(t *testing.T) {
	tokens, err := TokenizeJavascript("var x = 1;", "es1")
	if err != nil {
		t.Fatalf("Failed to tokenize with es1: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "var" {
		t.Errorf("Expected KEYWORD 'var', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

// TestTokenizeJavascriptVersion_es3 verifies ECMAScript 3 grammar loading.
// ECMAScript 3 (December 1999): added regular expressions, try/catch,
// do-while, and switch. This was the first version widely implemented.
func TestTokenizeJavascriptVersion_es3(t *testing.T) {
	tokens, err := TokenizeJavascript("var x = 1;", "es3")
	if err != nil {
		t.Fatalf("Failed to tokenize with es3: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "var" {
		t.Errorf("Expected KEYWORD 'var', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

// TestTokenizeJavascriptVersion_es5 verifies ECMAScript 5 grammar loading.
// ECMAScript 5 (December 2009): added strict mode, JSON.parse/stringify,
// Array.forEach, and Object.defineProperty.
func TestTokenizeJavascriptVersion_es5(t *testing.T) {
	tokens, err := TokenizeJavascript("var x = 1;", "es5")
	if err != nil {
		t.Fatalf("Failed to tokenize with es5: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavascriptVersion_es2015 verifies ECMAScript 2015 grammar loading.
// ES2015 (June 2015): the landmark "ES6" release adding classes, modules,
// arrow functions, let/const, template literals, destructuring, and more.
func TestTokenizeJavascriptVersion_es2015(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2015")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2015: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "const" {
		t.Errorf("Expected KEYWORD 'const', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

// TestTokenizeJavascriptVersion_es2016 verifies ECMAScript 2016 grammar loading.
// ES2016 (June 2016): added the exponentiation operator (**) and
// Array.prototype.includes.
func TestTokenizeJavascriptVersion_es2016(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2016")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2016: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavascriptVersion_es2017 verifies ECMAScript 2017 grammar loading.
// ES2017 (June 2017): introduced async/await, Object.entries/values,
// String.padStart/padEnd, and SharedArrayBuffer.
func TestTokenizeJavascriptVersion_es2017(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2017")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2017: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavascriptVersion_es2018 verifies ECMAScript 2018 grammar loading.
// ES2018 (June 2018): added rest/spread for objects, async iteration,
// Promise.finally, and named capture groups in RegExp.
func TestTokenizeJavascriptVersion_es2018(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2018")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2018: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavascriptVersion_es2019 verifies ECMAScript 2019 grammar loading.
// ES2019 (June 2019): added Array.flat/flatMap, Object.fromEntries,
// optional catch binding, and String.trimStart/trimEnd.
func TestTokenizeJavascriptVersion_es2019(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2019")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2019: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavascriptVersion_es2020 verifies ECMAScript 2020 grammar loading.
// ES2020 (June 2020): added BigInt, optional chaining (?.),
// nullish coalescing (??), Promise.allSettled, and globalThis.
func TestTokenizeJavascriptVersion_es2020(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2020")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2020: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavascriptVersion_es2021 verifies ECMAScript 2021 grammar loading.
// ES2021 (June 2021): added logical assignment (&&=, ||=, ??=),
// numeric separators (1_000_000), and Promise.any.
func TestTokenizeJavascriptVersion_es2021(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2021")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2021: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavascriptVersion_es2022 verifies ECMAScript 2022 grammar loading.
// ES2022 (June 2022): added class fields (public/private), static blocks,
// Array.at(), Object.hasOwn, and top-level await in modules.
func TestTokenizeJavascriptVersion_es2022(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2022")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2022: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavascriptVersion_es2023 verifies ECMAScript 2023 grammar loading.
// ES2023 (June 2023): added Array.findLast/findLastIndex, Symbols as
// WeakMap/WeakSet keys, and Array.toSorted/toReversed/with.
func TestTokenizeJavascriptVersion_es2023(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2023")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2023: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavascriptVersion_es2024 verifies ECMAScript 2024 grammar loading.
// ES2024 (June 2024): added Promise.withResolvers, Object.groupBy,
// ArrayBuffer.resize, RegExp /v flag, and Atomics.waitAsync.
func TestTokenizeJavascriptVersion_es2024(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2024")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2024: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavascriptVersion_es2025 verifies ECMAScript 2025 grammar loading.
// ES2025 (June 2025): the latest stable ECMAScript standard.
func TestTokenizeJavascriptVersion_es2025(t *testing.T) {
	tokens, err := TokenizeJavascript("const x = 1;", "es2025")
	if err != nil {
		t.Fatalf("Failed to tokenize with es2025: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Error-path tests
// ─────────────────────────────────────────────────────────────────────────────

// TestTokenizeJavascriptUnknownVersion confirms that an unrecognised version
// string returns an error rather than silently falling back to another grammar.
// This prevents silent misbehaviour when a caller makes a typo.
func TestTokenizeJavascriptUnknownVersion(t *testing.T) {
	_, err := TokenizeJavascript("let x = 1;", "es99")
	if err == nil {
		t.Fatal("Expected error for unknown version, got nil")
	}
}

package typescriptlexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ─────────────────────────────────────────────────────────────────────────────
// Generic-grammar tests (version = "")
//
// These tests use the default grammar, which is the superset of all supported
// TypeScript versions. They confirm that the version-agnostic code path still
// works exactly as it did in v0.1.0.
// ─────────────────────────────────────────────────────────────────────────────

func TestTokenizeTypescript(t *testing.T) {
	source := `let x = 1 + 2;`
	tokens, err := TokenizeTypescript(source, "")
	if err != nil {
		t.Fatalf("Failed to tokenize TypeScript source: %v", err)
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

	if tokens[6].Value != ";" {
		t.Errorf("Expected semicolon value ';', got %v", tokens[6].Value)
	}
}

func TestTokenizeTypescriptKeywordInterface(t *testing.T) {
	tokens, err := TokenizeTypescript("interface", "")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "interface" {
		t.Errorf("Expected KEYWORD 'interface', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

func TestTokenizeTypescriptKeywordType(t *testing.T) {
	tokens, err := TokenizeTypescript("type", "")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "type" {
		t.Errorf("Expected KEYWORD 'type', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

func TestTokenizeTypescriptKeywordNumber(t *testing.T) {
	tokens, err := TokenizeTypescript("number", "")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "number" {
		t.Errorf("Expected KEYWORD 'number', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Versioned-grammar tests
//
// Each test uses a specific TypeScript version grammar. We only assert that
// the request does not error and produces a recognisable first token, because
// the exact keyword lists differ per version. The goal is to confirm that the
// file-routing logic reaches the correct grammar file.
// ─────────────────────────────────────────────────────────────────────────────

// TestTokenizeTypescriptVersion_ts10 verifies TypeScript 1.0 grammar loading.
// TypeScript 1.0 (April 2014): the first public release; `var` was the primary
// declaration keyword (let/const were not widely used yet).
func TestTokenizeTypescriptVersion_ts10(t *testing.T) {
	tokens, err := TokenizeTypescript("var x = 1;", "ts1.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with ts1.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "var" {
		t.Errorf("Expected KEYWORD 'var', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

// TestTokenizeTypescriptVersion_ts20 verifies TypeScript 2.0 grammar loading.
// TypeScript 2.0 (September 2016) introduced strict null checks and non-null
// assertion operators.
func TestTokenizeTypescriptVersion_ts20(t *testing.T) {
	tokens, err := TokenizeTypescript("var x = 1;", "ts2.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with ts2.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeTypescriptVersion_ts30 verifies TypeScript 3.0 grammar loading.
// TypeScript 3.0 (July 2018) added project references and the `unknown` type.
func TestTokenizeTypescriptVersion_ts30(t *testing.T) {
	tokens, err := TokenizeTypescript("var x = 1;", "ts3.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with ts3.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeTypescriptVersion_ts40 verifies TypeScript 4.0 grammar loading.
// TypeScript 4.0 (August 2020) added variadic tuple types and labelled tuples.
func TestTokenizeTypescriptVersion_ts40(t *testing.T) {
	tokens, err := TokenizeTypescript("const x = 1;", "ts4.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with ts4.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "const" {
		t.Errorf("Expected KEYWORD 'const', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

// TestTokenizeTypescriptVersion_ts50 verifies TypeScript 5.0 grammar loading.
// TypeScript 5.0 (March 2023) introduced the updated decorators specification.
func TestTokenizeTypescriptVersion_ts50(t *testing.T) {
	tokens, err := TokenizeTypescript("const x = 1;", "ts5.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with ts5.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeTypescriptVersion_ts58 verifies TypeScript 5.8 grammar loading.
// TypeScript 5.8 (February 2025) is the latest stable release.
func TestTokenizeTypescriptVersion_ts58(t *testing.T) {
	tokens, err := TokenizeTypescript("const x = 1;", "ts5.8")
	if err != nil {
		t.Fatalf("Failed to tokenize with ts5.8: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Error-path tests
// ─────────────────────────────────────────────────────────────────────────────

// TestTokenizeTypescriptUnknownVersion confirms that an unrecognised version
// string returns an error rather than silently falling back to another grammar.
// This prevents silent misbehaviour when a caller makes a typo.
func TestTokenizeTypescriptUnknownVersion(t *testing.T) {
	_, err := TokenizeTypescript("let x = 1;", "ts99.0")
	if err == nil {
		t.Fatal("Expected error for unknown version, got nil")
	}
}

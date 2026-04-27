package javalexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ─────────────────────────────────────────────────────────────────────────────
// Default-version tests (version = "")
//
// These tests use the default version (Java 21), confirming that callers
// who do not specify a version get a working lexer out of the box.
// ─────────────────────────────────────────────────────────────────────────────

func TestTokenizeJava(t *testing.T) {
	source := `int x = 1 + 2;`
	tokens, err := TokenizeJava(source, "")
	if err != nil {
		t.Fatalf("Failed to tokenize Java source: %v", err)
	}

	// Expected: KEYWORD(int) NAME(x) EQUALS(=) NUMBER(1) PLUS(+) NUMBER(2) SEMICOLON(;) EOF
	if len(tokens) != 8 {
		t.Fatalf("Expected 8 tokens, got %v", len(tokens))
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "int" {
		t.Errorf("Expected first token to be KEYWORD 'int', got %v %v", tokens[0].Type, tokens[0].Value)
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
// Each test uses a specific Java version grammar. We only assert that
// the request does not error and produces a recognisable first token, because
// the exact keyword lists differ per version. The goal is to confirm that the
// file-routing logic reaches the correct grammar file.
// ─────────────────────────────────────────────────────────────────────────────

// TestTokenizeJavaVersion_1_0 verifies Java 1.0 grammar loading.
// Java 1.0 (January 1996): the original release — the language that started
// it all, with classes, interfaces, exceptions, and garbage collection.
func TestTokenizeJavaVersion_1_0(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "1.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 1.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "int" {
		t.Errorf("Expected KEYWORD 'int', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

// TestTokenizeJavaVersion_1_1 verifies Java 1.1 grammar loading.
// Java 1.1 (February 1997): added inner classes, JavaBeans, JDBC, RMI,
// and reflection.
func TestTokenizeJavaVersion_1_1(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "1.1")
	if err != nil {
		t.Fatalf("Failed to tokenize with 1.1: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "int" {
		t.Errorf("Expected KEYWORD 'int', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

// TestTokenizeJavaVersion_1_4 verifies Java 1.4 grammar loading.
// Java 1.4 (February 2002): added assertions, NIO, regular expressions,
// and chained exceptions.
func TestTokenizeJavaVersion_1_4(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "1.4")
	if err != nil {
		t.Fatalf("Failed to tokenize with 1.4: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavaVersion_5 verifies Java 5 grammar loading.
// Java 5 (September 2004): the landmark release that added generics,
// annotations, enums, autoboxing, varargs, and the enhanced for loop.
func TestTokenizeJavaVersion_5(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "5")
	if err != nil {
		t.Fatalf("Failed to tokenize with 5: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavaVersion_7 verifies Java 7 grammar loading.
// Java 7 (July 2011): added the diamond operator (<>), try-with-resources,
// multi-catch, strings in switch, and binary literals.
func TestTokenizeJavaVersion_7(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "7")
	if err != nil {
		t.Fatalf("Failed to tokenize with 7: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavaVersion_8 verifies Java 8 grammar loading.
// Java 8 (March 2014): the transformative release that added lambda
// expressions, the Stream API, default interface methods, and the
// java.time package.
func TestTokenizeJavaVersion_8(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "8")
	if err != nil {
		t.Fatalf("Failed to tokenize with 8: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavaVersion_10 verifies Java 10 grammar loading.
// Java 10 (March 2018): introduced local-variable type inference with the
// `var` keyword, and the first release under the new six-month cadence.
func TestTokenizeJavaVersion_10(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "10")
	if err != nil {
		t.Fatalf("Failed to tokenize with 10: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavaVersion_14 verifies Java 14 grammar loading.
// Java 14 (March 2020): added switch expressions (standard), records
// (preview), and helpful NullPointerExceptions.
func TestTokenizeJavaVersion_14(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "14")
	if err != nil {
		t.Fatalf("Failed to tokenize with 14: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavaVersion_17 verifies Java 17 grammar loading.
// Java 17 (September 2021): the latest-but-one LTS, adding sealed classes,
// pattern matching for instanceof, and enhanced pseudo-random number
// generators.
func TestTokenizeJavaVersion_17(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "17")
	if err != nil {
		t.Fatalf("Failed to tokenize with 17: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeJavaVersion_21 verifies Java 21 grammar loading.
// Java 21 (September 2023): the latest LTS, adding virtual threads,
// record patterns, pattern matching for switch, and sequenced collections.
func TestTokenizeJavaVersion_21(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "21")
	if err != nil {
		t.Fatalf("Failed to tokenize with 21: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Error-path tests
// ─────────────────────────────────────────────────────────────────────────────

// TestTokenizeJavaUnknownVersion confirms that an unrecognised version
// string returns an error rather than silently falling back to another grammar.
// This prevents silent misbehaviour when a caller makes a typo.
func TestTokenizeJavaUnknownVersion(t *testing.T) {
	_, err := TokenizeJava("int x = 1;", "99")
	if err == nil {
		t.Fatal("Expected error for unknown version, got nil")
	}
}

// TestDefaultVersionIsUsedWhenEmpty confirms that passing an empty version
// string uses DefaultVersion (Java 21) rather than erroring.
func TestDefaultVersionIsUsedWhenEmpty(t *testing.T) {
	tokens, err := TokenizeJava("int x = 1;", "")
	if err != nil {
		t.Fatalf("Expected no error with empty version, got: %v", err)
	}
	if len(tokens) == 0 {
		t.Fatal("Expected tokens, got empty slice")
	}
}

// TestCreateJavaLexer confirms that the factory function returns a non-nil
// lexer for a valid version.
func TestCreateJavaLexer(t *testing.T) {
	jl, err := CreateJavaLexer("int x = 1;", "21")
	if err != nil {
		t.Fatalf("Expected no error, got: %v", err)
	}
	if jl == nil {
		t.Fatal("Expected non-nil lexer")
	}
}

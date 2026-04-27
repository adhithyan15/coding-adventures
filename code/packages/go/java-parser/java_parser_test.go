package javaparser

import (
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// Default-version tests (version = "")
//
// These tests use the default version (Java 21), confirming that callers
// who do not specify a version get a working parser out of the box.
// ─────────────────────────────────────────────────────────────────────────────

func TestParseJava(t *testing.T) {
	source := "int x = 1 + 2;"
	program, err := ParseJava(source, "")
	if err != nil {
		t.Fatalf("Failed to parse Java code: %v", err)
	}

	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Versioned-grammar tests
//
// Each test uses a specific Java version grammar. We only assert that
// the parse does not error and produces a "program" root node, because the
// exact grammar rules differ per version. The goal is to confirm that the
// file-routing logic reaches the correct grammar file and that the lexer and
// parser grammars stay in sync for each version.
// ─────────────────────────────────────────────────────────────────────────────

// TestParseJavaVersion_1_0 verifies Java 1.0 grammar loading.
// Java 1.0 (January 1996): the original release.
func TestParseJavaVersion_1_0(t *testing.T) {
	program, err := ParseJava("int x = 1;", "1.0")
	if err != nil {
		t.Fatalf("Failed to parse with 1.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavaVersion_1_1 verifies Java 1.1 grammar loading.
// Java 1.1 (February 1997): inner classes, JavaBeans, JDBC.
func TestParseJavaVersion_1_1(t *testing.T) {
	program, err := ParseJava("int x = 1;", "1.1")
	if err != nil {
		t.Fatalf("Failed to parse with 1.1: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavaVersion_1_4 verifies Java 1.4 grammar loading.
// Java 1.4 (February 2002): assertions, NIO, regular expressions.
func TestParseJavaVersion_1_4(t *testing.T) {
	program, err := ParseJava("int x = 1;", "1.4")
	if err != nil {
		t.Fatalf("Failed to parse with 1.4: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavaVersion_5 verifies Java 5 grammar loading.
// Java 5 (September 2004): generics, annotations, enums, autoboxing.
func TestParseJavaVersion_5(t *testing.T) {
	program, err := ParseJava("int x = 1;", "5")
	if err != nil {
		t.Fatalf("Failed to parse with 5: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavaVersion_7 verifies Java 7 grammar loading.
// Java 7 (July 2011): diamond operator, try-with-resources, multi-catch.
func TestParseJavaVersion_7(t *testing.T) {
	program, err := ParseJava("int x = 1;", "7")
	if err != nil {
		t.Fatalf("Failed to parse with 7: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavaVersion_8 verifies Java 8 grammar loading.
// Java 8 (March 2014): lambdas, streams, default interface methods.
func TestParseJavaVersion_8(t *testing.T) {
	program, err := ParseJava("int x = 1;", "8")
	if err != nil {
		t.Fatalf("Failed to parse with 8: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavaVersion_10 verifies Java 10 grammar loading.
// Java 10 (March 2018): local-variable type inference (var).
func TestParseJavaVersion_10(t *testing.T) {
	program, err := ParseJava("int x = 1;", "10")
	if err != nil {
		t.Fatalf("Failed to parse with 10: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavaVersion_14 verifies Java 14 grammar loading.
// Java 14 (March 2020): switch expressions, records (preview).
func TestParseJavaVersion_14(t *testing.T) {
	program, err := ParseJava("int x = 1;", "14")
	if err != nil {
		t.Fatalf("Failed to parse with 14: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavaVersion_17 verifies Java 17 grammar loading.
// Java 17 (September 2021): sealed classes, pattern matching for instanceof.
func TestParseJavaVersion_17(t *testing.T) {
	program, err := ParseJava("int x = 1;", "17")
	if err != nil {
		t.Fatalf("Failed to parse with 17: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavaVersion_21 verifies Java 21 grammar loading.
// Java 21 (September 2023): virtual threads, record patterns.
func TestParseJavaVersion_21(t *testing.T) {
	program, err := ParseJava("int x = 1;", "21")
	if err != nil {
		t.Fatalf("Failed to parse with 21: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Error-path tests
// ─────────────────────────────────────────────────────────────────────────────

// TestParseJavaUnknownVersion confirms that an unrecognised version
// string returns an error rather than silently falling back to another grammar.
func TestParseJavaUnknownVersion(t *testing.T) {
	_, err := ParseJava("int x = 1;", "99")
	if err == nil {
		t.Fatal("Expected error for unknown version, got nil")
	}
}

// TestParseJavaDefaultVersion confirms that an empty version string
// uses the default (Java 21) and produces a valid parse tree.
func TestParseJavaDefaultVersion(t *testing.T) {
	program, err := ParseJava("int x = 1;", "")
	if err != nil {
		t.Fatalf("Expected no error with empty version, got: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

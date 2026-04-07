package typescriptparser

import (
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// Generic-grammar tests (version = "")
//
// These tests use the default grammar and confirm that the version-agnostic
// code path continues to work exactly as it did in v0.1.x.
// ─────────────────────────────────────────────────────────────────────────────

func TestParseTypescript(t *testing.T) {
	source := "let x = 1 + 2;"
	program, err := ParseTypescript(source, "")
	if err != nil {
		t.Fatalf("Failed to parse TypeScript code: %v", err)
	}

	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Versioned-grammar tests
//
// Each test confirms that the file-routing logic reaches a real grammar file
// and that the parser produces a valid program node. We do not inspect deeper
// into the AST because the exact shape differs per version.
// ─────────────────────────────────────────────────────────────────────────────

// TestParseTypescriptVersion_ts10 verifies TypeScript 1.0 grammar loading.
// TypeScript 1.0 (April 2014): `var` was the primary declaration keyword.
func TestParseTypescriptVersion_ts10(t *testing.T) {
	program, err := ParseTypescript("var x = 1;", "ts1.0")
	if err != nil {
		t.Fatalf("Failed to parse with ts1.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule, got %s", program.RuleName)
	}
}

// TestParseTypescriptVersion_ts20 verifies TypeScript 2.0 grammar loading.
// TypeScript 2.0 (September 2016) introduced strict null checks.
func TestParseTypescriptVersion_ts20(t *testing.T) {
	program, err := ParseTypescript("var x = 1;", "ts2.0")
	if err != nil {
		t.Fatalf("Failed to parse with ts2.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule, got %s", program.RuleName)
	}
}

// TestParseTypescriptVersion_ts30 verifies TypeScript 3.0 grammar loading.
// TypeScript 3.0 (July 2018) added project references and the `unknown` type.
func TestParseTypescriptVersion_ts30(t *testing.T) {
	program, err := ParseTypescript("var x = 1;", "ts3.0")
	if err != nil {
		t.Fatalf("Failed to parse with ts3.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule, got %s", program.RuleName)
	}
}

// TestParseTypescriptVersion_ts40 verifies TypeScript 4.0 grammar loading.
// TypeScript 4.0 (August 2020) added variadic tuple types.
func TestParseTypescriptVersion_ts40(t *testing.T) {
	program, err := ParseTypescript("const x = 1;", "ts4.0")
	if err != nil {
		t.Fatalf("Failed to parse with ts4.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule, got %s", program.RuleName)
	}
}

// TestParseTypescriptVersion_ts50 verifies TypeScript 5.0 grammar loading.
// TypeScript 5.0 (March 2023) introduced the updated decorators specification.
func TestParseTypescriptVersion_ts50(t *testing.T) {
	program, err := ParseTypescript("const x = 1;", "ts5.0")
	if err != nil {
		t.Fatalf("Failed to parse with ts5.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule, got %s", program.RuleName)
	}
}

// TestParseTypescriptVersion_ts58 verifies TypeScript 5.8 grammar loading.
// TypeScript 5.8 (February 2025) is the latest stable release.
func TestParseTypescriptVersion_ts58(t *testing.T) {
	program, err := ParseTypescript("const x = 1;", "ts5.8")
	if err != nil {
		t.Fatalf("Failed to parse with ts5.8: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule, got %s", program.RuleName)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Error-path tests
// ─────────────────────────────────────────────────────────────────────────────

// TestParseTypescriptUnknownVersion confirms that an unrecognised version
// string returns an error rather than silently routing to the wrong grammar.
func TestParseTypescriptUnknownVersion(t *testing.T) {
	_, err := ParseTypescript("let x = 1;", "ts99.0")
	if err == nil {
		t.Fatal("Expected error for unknown version, got nil")
	}
}

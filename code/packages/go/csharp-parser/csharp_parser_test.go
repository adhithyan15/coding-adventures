package csharpparser

import (
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// Default-version tests (version = "")
//
// These tests use the default version (C# 12.0), confirming that callers
// who do not specify a version get a working parser out of the box.
// ─────────────────────────────────────────────────────────────────────────────

func TestParseCSharp(t *testing.T) {
	// `int x = 1 + 2;` is valid in every version of C# and produces a simple
	// local variable declaration with an arithmetic initializer.
	source := "int x = 1 + 2;"
	program, err := ParseCSharp(source, "")
	if err != nil {
		t.Fatalf("Failed to parse C# code: %v", err)
	}

	// The grammar-driven parser always returns a root node whose RuleName is
	// "program", matching the top-level production rule in the grammar.
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpClassDeclaration verifies that a simple class declaration
// produces a valid parse tree.
//
// The class is the fundamental unit of object-oriented programming in C#.
// Every method, property, field, and event lives inside a class (or another
// type like struct or interface). A minimal class looks like:
//
//	public class Greeter {
//	    public string Greet(string name) => $"Hello, {name}!";
//	}
//
// Classes support single inheritance and multiple interface implementation.
func TestParseCSharpClassDeclaration(t *testing.T) {
	source := "public class Foo {}"
	program, err := ParseCSharp(source, "")
	if err != nil {
		t.Fatalf("Failed to parse class declaration: %v", err)
	}

	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Versioned-grammar tests
//
// Each test uses a specific C# version grammar. We only assert that
// the parse does not error and produces a "program" root node, because the
// exact grammar rules differ per version. The goal is to confirm that the
// file-routing logic reaches the correct grammar file and that the lexer and
// parser grammars stay in sync for each version.
// ─────────────────────────────────────────────────────────────────────────────

// TestParseCSharpVersion_1_0 verifies C# 1.0 grammar loading.
// C# 1.0 (January 2002): the original release with .NET Framework 1.0.
func TestParseCSharpVersion_1_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "1.0")
	if err != nil {
		t.Fatalf("Failed to parse with 1.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_2_0 verifies C# 2.0 grammar loading.
// C# 2.0 (November 2005): generics, iterators, nullable value types.
func TestParseCSharpVersion_2_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "2.0")
	if err != nil {
		t.Fatalf("Failed to parse with 2.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_3_0 verifies C# 3.0 grammar loading.
// C# 3.0 (November 2007): LINQ, lambda expressions, auto-properties, var.
func TestParseCSharpVersion_3_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "3.0")
	if err != nil {
		t.Fatalf("Failed to parse with 3.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_4_0 verifies C# 4.0 grammar loading.
// C# 4.0 (April 2010): dynamic binding, named/optional parameters, covariance.
func TestParseCSharpVersion_4_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "4.0")
	if err != nil {
		t.Fatalf("Failed to parse with 4.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_5_0 verifies C# 5.0 grammar loading.
// C# 5.0 (August 2012): async/await, caller info attributes.
func TestParseCSharpVersion_5_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "5.0")
	if err != nil {
		t.Fatalf("Failed to parse with 5.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_6_0 verifies C# 6.0 grammar loading.
// C# 6.0 (July 2015): string interpolation, null-conditional operator,
// expression-bodied members, nameof expressions.
func TestParseCSharpVersion_6_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "6.0")
	if err != nil {
		t.Fatalf("Failed to parse with 6.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_7_0 verifies C# 7.0 grammar loading.
// C# 7.0 (March 2017): tuples, pattern matching, local functions,
// out variable declarations, deconstruction.
func TestParseCSharpVersion_7_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "7.0")
	if err != nil {
		t.Fatalf("Failed to parse with 7.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_8_0 verifies C# 8.0 grammar loading.
// C# 8.0 (September 2019): nullable reference types, async streams,
// switch expressions, ranges and indices.
func TestParseCSharpVersion_8_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "8.0")
	if err != nil {
		t.Fatalf("Failed to parse with 8.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_9_0 verifies C# 9.0 grammar loading.
// C# 9.0 (November 2020, .NET 5): records, init-only setters,
// top-level statements.
func TestParseCSharpVersion_9_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "9.0")
	if err != nil {
		t.Fatalf("Failed to parse with 9.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_10_0 verifies C# 10.0 grammar loading.
// C# 10.0 (November 2021, .NET 6 LTS): global usings, file-scoped
// namespaces, record structs.
func TestParseCSharpVersion_10_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "10.0")
	if err != nil {
		t.Fatalf("Failed to parse with 10.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_11_0 verifies C# 11.0 grammar loading.
// C# 11.0 (November 2022, .NET 7): required members, raw string literals,
// generic math, list patterns.
func TestParseCSharpVersion_11_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "11.0")
	if err != nil {
		t.Fatalf("Failed to parse with 11.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseCSharpVersion_12_0 verifies C# 12.0 grammar loading.
// C# 12.0 (November 2023, .NET 8 LTS): primary constructors, collection
// expressions, inline arrays.
func TestParseCSharpVersion_12_0(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "12.0")
	if err != nil {
		t.Fatalf("Failed to parse with 12.0: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Error-path tests
// ─────────────────────────────────────────────────────────────────────────────

// TestParseCSharpUnknownVersion confirms that an unrecognised version
// string returns an error rather than silently falling back to another grammar.
func TestParseCSharpUnknownVersion(t *testing.T) {
	_, err := ParseCSharp("int x = 1;", "99")
	if err == nil {
		t.Fatal("Expected error for unknown version, got nil")
	}
}

// TestParseCSharpDefaultVersion confirms that an empty version string
// uses the default (C# 12.0) and produces a valid parse tree.
func TestParseCSharpDefaultVersion(t *testing.T) {
	program, err := ParseCSharp("int x = 1;", "")
	if err != nil {
		t.Fatalf("Expected no error with empty version, got: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestNewCSharpParser confirms that the factory function returns a non-nil
// parser for a valid version.
func TestNewCSharpParser(t *testing.T) {
	cp, err := NewCSharpParser("int x = 1;", "12.0")
	if err != nil {
		t.Fatalf("Expected no error, got: %v", err)
	}
	if cp == nil {
		t.Fatal("Expected non-nil parser")
	}
}

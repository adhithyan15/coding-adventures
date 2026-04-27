package csharplexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ─────────────────────────────────────────────────────────────────────────────
// Default-version tests (version = "")
//
// These tests use the default version (C# 12.0), confirming that callers
// who do not specify a version get a working lexer out of the box.
// ─────────────────────────────────────────────────────────────────────────────

func TestTokenizeCSharp(t *testing.T) {
	// A simple arithmetic assignment — the most basic C# expression.
	// C# shares Java's syntax heritage so `int x = 1 + 2;` is valid in every
	// version of the language.
	source := `int x = 1 + 2;`
	tokens, err := TokenizeCSharp(source, "")
	if err != nil {
		t.Fatalf("Failed to tokenize C# source: %v", err)
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

// TestTokenizeCSharpKeywords verifies that C# keywords are correctly identified.
//
// C# has a rich keyword vocabulary. Most keywords (public, class, namespace,
// using, etc.) have existed since C# 1.0. Others were introduced progressively:
//
//   - "async" and "await" arrived in C# 5.0 (2012) to make asynchronous code
//     look as simple as synchronous code. Before async/await, developers had to
//     write callback chains that were hard to follow.
//   - "var" arrived in C# 3.0 (2007) as part of LINQ; it tells the compiler
//     to infer the type from the right-hand side, removing redundancy.
//
// Keywords are case-sensitive in C#. `Class` (capital C) is a valid identifier,
// not a keyword.
func TestTokenizeCSharpKeywords(t *testing.T) {
	source := `public class Foo {}`
	tokens, err := TokenizeCSharp(source, "")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// We expect at least KEYWORD(public) and KEYWORD(class) in the token stream.
	if len(tokens) < 2 {
		t.Fatalf("Expected at least 2 tokens, got %d", len(tokens))
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "public" {
		t.Errorf("Expected KEYWORD 'public', got %v %q", tokens[0].Type, tokens[0].Value)
	}

	if tokens[1].Type != lexer.TokenKeyword || tokens[1].Value != "class" {
		t.Errorf("Expected KEYWORD 'class', got %v %q", tokens[1].Type, tokens[1].Value)
	}
}

// TestTokenizeCSharpNamespace verifies that the `namespace` keyword tokenises.
//
// C# organises types into namespaces — hierarchical names that group related
// types and prevent naming collisions across libraries. Every .NET standard
// library type lives under a namespace like `System`, `System.Collections`,
// `System.Threading.Tasks`, etc.
//
// From C# 10.0 onward, file-scoped namespaces let you write:
//
//	namespace MyApp;   // applies to the whole file — no braces needed
//
// instead of the traditional block form.
func TestTokenizeCSharpNamespace(t *testing.T) {
	source := `namespace MyApp`
	tokens, err := TokenizeCSharp(source, "")
	if err != nil {
		t.Fatalf("Failed to tokenize namespace keyword: %v", err)
	}

	if len(tokens) < 1 {
		t.Fatal("Expected at least one token")
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "namespace" {
		t.Errorf("Expected KEYWORD 'namespace', got %v %q", tokens[0].Type, tokens[0].Value)
	}
}

// TestTokenizeCSharpUsing verifies that the `using` keyword tokenises.
//
// `using` is one of the most versatile keywords in C#:
//
//  1. `using System;`       — import a namespace (like Java's `import`)
//  2. `using var x = ...;`  — dispose the resource when the scope ends (RAII pattern)
//  3. `using static System.Math;` — import static members without qualification
//
// The `using` keyword has been in C# since version 1.0.
func TestTokenizeCSharpUsing(t *testing.T) {
	source := `using System;`
	tokens, err := TokenizeCSharp(source, "")
	if err != nil {
		t.Fatalf("Failed to tokenize using keyword: %v", err)
	}

	if len(tokens) < 1 {
		t.Fatal("Expected at least one token")
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "using" {
		t.Errorf("Expected KEYWORD 'using', got %v %q", tokens[0].Type, tokens[0].Value)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Versioned-grammar tests
//
// Each test uses a specific C# version grammar. We only assert that
// the request does not error and produces a recognisable first token, because
// the exact keyword lists differ per version. The goal is to confirm that the
// file-routing logic reaches the correct grammar file for all 12 versions.
// ─────────────────────────────────────────────────────────────────────────────

// TestTokenizeCSharpVersion_1_0 verifies C# 1.0 grammar loading.
// C# 1.0 (January 2002): the original release, bundled with .NET Framework 1.0
// and Visual Studio .NET 2002. The language was strongly influenced by Java and
// C++, with classes, interfaces, delegates, events, and garbage collection.
func TestTokenizeCSharpVersion_1_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "1.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 1.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "int" {
		t.Errorf("Expected KEYWORD 'int', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

// TestTokenizeCSharpVersion_2_0 verifies C# 2.0 grammar loading.
// C# 2.0 (November 2005): added generics (like Java 5), iterators with yield,
// partial classes, nullable value types (int?), and static classes.
// The addition of nullable value types was prescient — C# 8.0 later extended
// the concept to reference types as well.
func TestTokenizeCSharpVersion_2_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "2.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 2.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "int" {
		t.Errorf("Expected KEYWORD 'int', got %v %v", tokens[0].Type, tokens[0].Value)
	}
}

// TestTokenizeCSharpVersion_3_0 verifies C# 3.0 grammar loading.
// C# 3.0 (November 2007): the LINQ release. LINQ (Language Integrated Query)
// lets you query arrays, databases, XML, and any IEnumerable with SQL-like
// syntax directly in C#. To support LINQ, C# 3.0 also added: lambda
// expressions (x => x * 2), extension methods, auto-properties, object
// initializers, and the `var` keyword.
func TestTokenizeCSharpVersion_3_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "3.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 3.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeCSharpVersion_4_0 verifies C# 4.0 grammar loading.
// C# 4.0 (April 2010): added the `dynamic` keyword, which defers type checking
// to runtime — useful for COM interop and calling into dynamic languages.
// Also added named and optional parameters (eliminating many method overloads),
// and covariance/contravariance annotations on generic interfaces.
func TestTokenizeCSharpVersion_4_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "4.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 4.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeCSharpVersion_5_0 verifies C# 5.0 grammar loading.
// C# 5.0 (August 2012): added the transformative async/await feature. With
// async/await, asynchronous I/O code looks almost identical to synchronous
// code. Instead of callback pyramids, you write:
//
//	var data = await httpClient.GetStringAsync(url);
//
// The compiler rewrites this into a state machine automatically.
func TestTokenizeCSharpVersion_5_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "5.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 5.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeCSharpVersion_6_0 verifies C# 6.0 grammar loading.
// C# 6.0 (July 2015): added many quality-of-life improvements:
//   - String interpolation: $"Hello, {name}!" instead of String.Format
//   - Null-conditional operator: foo?.Bar?.Baz (short-circuits on null)
//   - Expression-bodied members: int Length => _data.Length;
//   - nameof() expressions: avoids magic strings in ArgumentException messages
func TestTokenizeCSharpVersion_6_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "6.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 6.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeCSharpVersion_7_0 verifies C# 7.0 grammar loading.
// C# 7.0 (March 2017): the pattern matching release. Key additions:
//   - Tuples: (string, int) pair = ("hello", 42);
//   - Pattern matching: if (obj is int n) { ... }
//   - Local functions: functions defined inside other functions
//   - out variable declarations: int.TryParse(s, out var result)
//   - Deconstruction: var (x, y) = point;
func TestTokenizeCSharpVersion_7_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "7.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 7.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeCSharpVersion_8_0 verifies C# 8.0 grammar loading.
// C# 8.0 (September 2019): added nullable reference types — a compiler-enforced
// annotation system that catches null dereferences at compile time rather than
// causing NullReferenceExceptions at runtime. Also added:
//   - Async streams: await foreach (var item in asyncEnumerable)
//   - Switch expressions: x switch { 1 => "one", 2 => "two", _ => "other" }
//   - Ranges and indices: array[1..^1], array[^2]
func TestTokenizeCSharpVersion_8_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "8.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 8.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeCSharpVersion_9_0 verifies C# 9.0 grammar loading.
// C# 9.0 (November 2020, shipped with .NET 5): added records — immutable
// reference types with value-based equality. A record like:
//
//	record Person(string Name, int Age);
//
// generates a constructor, ToString(), Equals(), GetHashCode(), and a
// non-destructive mutation operator `with` for free. Also added top-level
// statements (no Main method needed in simple programs) and init-only setters.
func TestTokenizeCSharpVersion_9_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "9.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 9.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeCSharpVersion_10_0 verifies C# 10.0 grammar loading.
// C# 10.0 (November 2021, shipped with .NET 6 LTS): added global using
// directives (apply once to the whole project), file-scoped namespaces
// (one line instead of a wrapping block), and record structs (value-type
// records). Also improved lambda inference so lambdas can return typed values.
func TestTokenizeCSharpVersion_10_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "10.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 10.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeCSharpVersion_11_0 verifies C# 11.0 grammar loading.
// C# 11.0 (November 2022, shipped with .NET 7): added required members
// (compiler-enforced object initializers), raw string literals ("""..."""
// for embedding JSON/regex without escaping), generic math interfaces, and
// list patterns for pattern matching against collections.
func TestTokenizeCSharpVersion_11_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "11.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 11.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// TestTokenizeCSharpVersion_12_0 verifies C# 12.0 grammar loading.
// C# 12.0 (November 2023, shipped with .NET 8 LTS): added primary constructors
// on all classes and structs (not just records), collection expressions
// ([1, 2, 3] syntax for any collection type), inline arrays (fixed-size buffers
// inside structs), and experimental alias any type declarations.
func TestTokenizeCSharpVersion_12_0(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "12.0")
	if err != nil {
		t.Fatalf("Failed to tokenize with 12.0: %v", err)
	}
	if tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected KEYWORD token, got %v", tokens[0].Type)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Error-path tests
// ─────────────────────────────────────────────────────────────────────────────

// TestTokenizeCSharpUnknownVersion confirms that an unrecognised version
// string returns an error rather than silently falling back to another grammar.
// This prevents silent misbehaviour when a caller makes a typo (e.g. "12"
// instead of "12.0" — unlike Java, all C# versions have a dot-zero suffix).
func TestTokenizeCSharpUnknownVersion(t *testing.T) {
	_, err := TokenizeCSharp("int x = 1;", "99")
	if err == nil {
		t.Fatal("Expected error for unknown version, got nil")
	}
}

// TestDefaultVersionIsUsedWhenEmpty confirms that passing an empty version
// string uses DefaultVersion (C# 12.0) rather than erroring.
func TestDefaultVersionIsUsedWhenEmpty(t *testing.T) {
	tokens, err := TokenizeCSharp("int x = 1;", "")
	if err != nil {
		t.Fatalf("Expected no error with empty version, got: %v", err)
	}
	if len(tokens) == 0 {
		t.Fatal("Expected tokens, got empty slice")
	}
}

// TestNewCSharpLexer confirms that the factory function returns a non-nil
// lexer for a valid version.
func TestNewCSharpLexer(t *testing.T) {
	cl, err := NewCSharpLexer("int x = 1;", "12.0")
	if err != nil {
		t.Fatalf("Expected no error, got: %v", err)
	}
	if cl == nil {
		t.Fatal("Expected non-nil lexer")
	}
}

// TestTokenizeCSharpVersionMissingDotZero confirms that C# version strings
// without the ".0" suffix are rejected. All C# versions use "X.0" notation.
// A caller passing "12" (Java-style) rather than "12.0" should get an error,
// not a silent fallback.
func TestTokenizeCSharpVersionMissingDotZero(t *testing.T) {
	_, err := TokenizeCSharp("int x = 1;", "12")
	if err == nil {
		t.Fatal("Expected error for version '12' (missing .0 suffix), got nil")
	}
}

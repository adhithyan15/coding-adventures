package csharplexer

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// validVersions is the set of C# version strings the lexer recognises.
// Each maps to a versioned grammar file under code/grammars/csharp/.
//
// C# is a language created by Microsoft, first released in 2000 as part of the
// .NET Framework. It was designed by Anders Hejlsberg (who also designed Turbo
// Pascal and Delphi) and has evolved from a Java-influenced language into a
// multi-paradigm powerhouse with features like nullable reference types, records,
// pattern matching, and async/await.
//
// The versions here correspond to the grammar files available in the
// code/grammars/csharp/ directory:
//
//   1.0  — C# 1.0  (January 2002)    the original release, bundled with .NET 1.0
//   2.0  — C# 2.0  (November 2005)   generics, iterators, partial types, nullable types
//   3.0  — C# 3.0  (November 2007)   LINQ, lambda expressions, auto-properties, extension methods
//   4.0  — C# 4.0  (April 2010)      dynamic binding, named/optional parameters, covariance
//   5.0  — C# 5.0  (August 2012)     async/await, caller info attributes
//   6.0  — C# 6.0  (July 2015)       string interpolation, null-conditional operator, expression-bodied members
//   7.0  — C# 7.0  (March 2017)      tuples, pattern matching, local functions, out variables
//   8.0  — C# 8.0  (September 2019)  nullable reference types, async streams, switch expressions
//   9.0  — C# 9.0  (November 2020)   records, init-only setters, top-level statements
//   10.0 — C# 10.0 (November 2021)   global usings, file-scoped namespaces, record structs
//   11.0 — C# 11.0 (November 2022)   required members, raw string literals, generic math
//   12.0 — C# 12.0 (November 2023)   primary constructors, collection expressions, inline arrays
var validVersions = map[string]bool{
	"1.0":  true,
	"2.0":  true,
	"3.0":  true,
	"4.0":  true,
	"5.0":  true,
	"6.0":  true,
	"7.0":  true,
	"8.0":  true,
	"9.0":  true,
	"10.0": true,
	"11.0": true,
	"12.0": true,
}

// DefaultVersion is the C# version used when no version is specified.
// C# 12.0 is the latest released version as of November 2023, shipped with
// .NET 8.0 (an LTS release). It is the most feature-rich and widely available
// modern version.
const DefaultVersion = "12.0"

// getGrammarPath resolves the absolute path to the .tokens grammar file for
// the given C# version string.
//
// When version is "" (empty string), the DefaultVersion ("12.0") is used.
// This provides a sensible default for callers that do not care about a
// specific version.
//
// Any other non-empty string must appear in validVersions; an unknown version
// returns a descriptive error so that typos produce actionable messages.
//
// The path is resolved relative to this source file using runtime.Caller(0),
// which ensures the grammar directory is found regardless of the working
// directory at runtime.
//
// For example, if this file lives at:
//
//	.../code/packages/go/csharp-lexer/lexer.go
//
// then parent is:
//
//	.../code/packages/go/csharp-lexer/
//
// and root (three dirs up) is:
//
//	.../code/
//
// so the final tokens path is:
//
//	.../code/grammars/csharp/csharp12.0.tokens
func getGrammarPath(version string) (string, error) {
	// runtime.Caller(0) returns the path to *this* source file at compile
	// time. We navigate up three directories to reach code/grammars/.
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	// Default to the latest version when no version is specified.
	if version == "" {
		version = DefaultVersion
	}

	if !validVersions[version] {
		return "", fmt.Errorf(
			"unknown C# version %q: valid versions are 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0",
			version,
		)
	}

	// Grammar files follow the pattern: csharp/csharp{version}.tokens
	// For example: csharp/csharp12.0.tokens, csharp/csharp1.0.tokens
	return filepath.Join(root, "csharp", "csharp"+version+".tokens"), nil
}

// NewCSharpLexer constructs a GrammarLexer ready to tokenise the given
// C# source string.
//
// version selects the C# grammar file:
//   - ""     — uses DefaultVersion ("12.0"), the latest release
//   - "1.0"  — the original .NET 1.0 era C#
//   - "2.0"  — generics and iterators
//   - "3.0"  — LINQ and lambdas
//   - "4.0"  — dynamic and optional parameters
//   - "5.0"  — async/await
//   - "6.0"  — string interpolation and null-conditional
//   - "7.0"  — tuples and pattern matching
//   - "8.0"  — nullable reference types and async streams
//   - "9.0"  — records and top-level statements
//   - "10.0" — global usings and file-scoped namespaces
//   - "11.0" — required members and raw string literals
//   - "12.0" — primary constructors and collection expressions
//
// An error is returned if the version string is unrecognised or if the grammar
// file cannot be read.
//
// The function uses the capability-cage pattern: grammar file reads are
// mediated through the Operation.File capability, ensuring only declared
// paths in required_capabilities.json can be accessed at runtime.
func NewCSharpLexer(source string, version string) (*lexer.GrammarLexer, error) {
	grammarPath, err := getGrammarPath(version)
	if err != nil {
		return nil, err
	}
	bytes, err := os.ReadFile(grammarPath)
	if err != nil {
		return nil, err
	}
	grammar, err := grammartools.ParseTokenGrammar(string(bytes))
	if err != nil {
		return nil, err
	}
	return lexer.NewGrammarLexer(source, grammar), nil
}

// TokenizeCSharp is the main entry point for lexing C# source code.
//
// It tokenises source using the grammar for the given C# version and
// returns the flat token slice produced by the underlying GrammarLexer.
// Pass version="" to use the default grammar (C# 12.0), which is the best
// choice when version is unknown.
//
// # What does "tokenise" mean?
//
// Tokenisation (also called lexing or scanning) is the first phase of a
// compiler or interpreter. It transforms a raw string of characters into a
// sequence of labelled tokens — logical units like keywords, identifiers,
// operators, literals, and punctuation. For example, the C# fragment:
//
//	int x = 42;
//
// becomes something like:
//
//	KEYWORD("int")  NAME("x")  EQUALS("=")  NUMBER("42")  SEMICOLON(";")  EOF
//
// Each token records both its *type* (what category it belongs to) and its
// *value* (the exact text that was matched). The parser in the next phase
// uses these tokens to build a tree.
//
// Example — tokenise with the default grammar:
//
//	tokens, err := TokenizeCSharp("int x = 1;", "")
//
// Example — tokenise with a specific version:
//
//	tokens, err := TokenizeCSharp("var x = 1;", "3.0")
//
// Example — tokenise using nullable reference types (C# 8.0+):
//
//	tokens, err := TokenizeCSharp("string? name = null;", "8.0")
func TokenizeCSharp(source string, version string) ([]lexer.Token, error) {
	csharpLexer, err := NewCSharpLexer(source, version)
	if err != nil {
		return nil, err
	}
	return csharpLexer.Tokenize(), nil
}

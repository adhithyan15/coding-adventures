// Package csharplexer tokenizes C# source code using versioned grammars.
//
// C# is a language created by Microsoft, first released in 2000 as part of the
// .NET Framework. It was designed by Anders Hejlsberg (who also designed Turbo
// Pascal and Delphi) and has evolved from a Java-influenced language into a
// multi-paradigm powerhouse with features like nullable reference types,
// records, pattern matching, and async/await.
//
// The lexer supports every released C# version, each with its own token
// grammar that captures the exact token set for that version:
//
//	1.0  — C# 1.0  (January 2002)    the original release, bundled with .NET 1.0
//	2.0  — C# 2.0  (November 2005)   generics, iterators, partial types, nullable types
//	3.0  — C# 3.0  (November 2007)   LINQ, lambda expressions, auto-properties, extension methods
//	4.0  — C# 4.0  (April 2010)      dynamic binding, named/optional parameters, covariance
//	5.0  — C# 5.0  (August 2012)     async/await, caller info attributes
//	6.0  — C# 6.0  (July 2015)       string interpolation, null-conditional operator, expression-bodied members
//	7.0  — C# 7.0  (March 2017)      tuples, pattern matching, local functions, out variables
//	8.0  — C# 8.0  (September 2019)  nullable reference types, async streams, switch expressions
//	9.0  — C# 9.0  (November 2020)   records, init-only setters, top-level statements
//	10.0 — C# 10.0 (November 2021)   global usings, file-scoped namespaces, record structs
//	11.0 — C# 11.0 (November 2022)   required members, raw string literals, generic math
//	12.0 — C# 12.0 (November 2023)   primary constructors, collection expressions, inline arrays
//
// The grammars are embedded at compile time as native Go data structures in
// grammar_data.go (VersionedTokenGrammars, keyed by version string). Nothing is
// read from disk at run time, so the lexer needs no filesystem capability and
// works unchanged when the package is built standalone.
//
// Usage:
//
//	tokens, err := csharplexer.TokenizeCSharp(source, "8.0")
//	tokens, err := csharplexer.TokenizeCSharp(source, "")  // defaults to 12.0
package csharplexer

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// DefaultVersion is the C# version used when no version is specified.
// C# 12.0 is the latest released version as of November 2023, shipped with
// .NET 8.0 (an LTS release). It is the most feature-rich and widely available
// modern version.
const DefaultVersion = "12.0"

// NewCSharpLexer constructs a GrammarLexer ready to tokenise the given
// C# source string.
//
// version selects the C# grammar:
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
// The grammar is selected from the compiled-in VersionedTokenGrammars map; no
// grammar file is read at run time. When version is "" the default grammar
// (C# 12.0) is used directly. An error is returned only when a non-empty
// version string has no embedded grammar.
func NewCSharpLexer(source string, version string) (*lexer.GrammarLexer, error) {
	if version == "" {
		return lexer.NewGrammarLexer(source, TokenGrammarData), nil
	}
	grammar, ok := VersionedTokenGrammars[version]
	if !ok {
		return nil, fmt.Errorf(
			"unknown C# version %q: valid versions are 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0",
			version,
		)
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

// Package csharpparser parses C# source code into an Abstract Syntax Tree (AST)
// using versioned grammars.
//
// The parser supports every released C# version (1.0 through 12.0). Each version
// has its own parser grammar that describes the syntactic structure of that
// release. See the csharp-lexer package for a description of each version.
//
// The grammars are embedded at compile time as native Go data structures in
// grammar_data.go (VersionedParserGrammars, keyed by version string). Nothing is
// read from disk at run time, so the parser needs no filesystem capability and
// works unchanged when the package is built standalone.
//
// Usage:
//
//	ast, err := csharpparser.ParseCSharp(source, "9.0")
//	ast, err := csharpparser.ParseCSharp(source, "")  // defaults to 12.0
package csharpparser

import (
	"fmt"

	csharplexer "github.com/adhithyan15/coding-adventures/code/packages/go/csharp-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// DefaultVersion is the C# version used when no version is specified.
// Kept in sync with the csharp-lexer package.
const DefaultVersion = "12.0"

// NewCSharpParser constructs a GrammarParser ready to parse the given
// C# source string.
//
// version selects the C# grammar pair:
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
// Both the lexer and parser grammars are selected by the same version string,
// guaranteeing that the token set and parse rules stay consistent. The parser
// grammar is selected from the compiled-in VersionedParserGrammars map; no
// grammar file is read at run time. When version is "" the default grammar
// (C# 12.0) is used directly. An error is returned if the version string is
// unrecognised, or if lexing fails.
//
// # Two-phase compilation: lexing then parsing
//
// Parsing is the second phase of compilation (lexing is the first). The lexer
// converts raw text into a flat list of tokens. The parser then reads those
// tokens and builds an Abstract Syntax Tree (AST) — a tree where each node
// represents a syntactic construct:
//
//	VariableDeclaration
//	├── Type: "int"
//	├── Name: "x"
//	└── Initializer
//	    └── BinaryExpression
//	        ├── Left: Literal(1)
//	        ├── Operator: "+"
//	        └── Right: Literal(2)
//
// The AST is the input to subsequent phases: semantic analysis, type checking,
// optimisation, and code generation.
func NewCSharpParser(source string, version string) (*parser.GrammarParser, error) {
	// Tokenise first; any version-error is surfaced here before we select
	// the parser grammar.
	tokens, err := csharplexer.TokenizeCSharp(source, version)
	if err != nil {
		return nil, err
	}
	if version == "" {
		return parser.NewGrammarParser(tokens, ParserGrammarData), nil
	}
	grammar, ok := VersionedParserGrammars[version]
	if !ok {
		return nil, fmt.Errorf(
			"unknown C# version %q: valid versions are 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0",
			version,
		)
	}
	return parser.NewGrammarParser(tokens, grammar), nil
}

// ParseCSharp is the main entry point for parsing C# source code.
//
// It parses source using the grammar for the given C# version and returns
// the root AST node produced by the underlying GrammarParser. Pass version=""
// to use the default grammar (C# 12.0), which is the best choice when
// version is unknown.
//
// # What does the AST root represent?
//
// The root node returned for a C# compilation unit has RuleName "program".
// In C# terminology a "compilation unit" is one source file. It can contain:
//
//   - using directives (namespace imports)
//   - global attributes
//   - namespace declarations
//   - top-level type declarations (class, struct, interface, enum, delegate)
//   - top-level statements (C# 9.0+): code outside any class, used in minimal
//     programs like ASP.NET Minimal APIs
//
// Example — parse with the default grammar:
//
//	node, err := ParseCSharp("int x = 1 + 2;", "")
//
// Example — parse with a specific version:
//
//	node, err := ParseCSharp("var x = 1;", "3.0")
//
// Example — parse a record (C# 9.0+):
//
//	node, err := ParseCSharp("record Point(int X, int Y);", "9.0")
func ParseCSharp(source string, version string) (*parser.ASTNode, error) {
	csharpParser, err := NewCSharpParser(source, version)
	if err != nil {
		return nil, err
	}
	return csharpParser.Parse()
}

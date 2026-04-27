package csharpparser

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	csharplexer "github.com/adhithyan15/coding-adventures/code/packages/go/csharp-lexer"
)

// validVersions is the set of C# version strings the parser recognises.
// These must stay in sync with the same map in the csharp-lexer package so
// that the lexer and parser always agree on which grammars are available.
//
// See the csharp-lexer package for a detailed description of each release.
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
// Kept in sync with the csharp-lexer package.
const DefaultVersion = "12.0"

// getGrammarPath resolves the absolute path to the .grammar file for the given
// C# version string.
//
// When version is "" (empty string), the DefaultVersion ("12.0") is used.
// This provides a sensible default for callers that do not care about a
// specific version.
//
// Any other non-empty string must appear in validVersions; an unknown version
// returns a descriptive error so that typos produce actionable messages.
//
// # How grammar files are found
//
// The .grammar file describes the *syntactic structure* of the language —
// production rules like "a class declaration consists of access modifiers,
// the keyword 'class', a name, an optional base list, and a body". These rules
// are separate from the .tokens file (which the lexer uses), because the two
// tools need different representations:
//
//   - The lexer uses a flat list of regex-like patterns to classify individual
//     characters into tokens.
//   - The parser uses a context-free grammar (CFG) to describe how tokens
//     combine into larger structures (expressions, statements, declarations).
//
// Both files live under code/grammars/csharp/ and share the same version
// naming convention (csharp{version}.tokens / csharp{version}.grammar).
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

	// Grammar files follow the pattern: csharp/csharp{version}.grammar
	// For example: csharp/csharp12.0.grammar, csharp/csharp1.0.grammar
	return filepath.Join(root, "csharp", "csharp"+version+".grammar"), nil
}

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
// Both the lexer and parser grammar files are selected by the same version
// string, guaranteeing that the token set and parse rules stay consistent.
//
// An error is returned if the version string is unrecognised, or if any
// grammar file cannot be read.
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
	// Tokenise first; any version-error is surfaced here before we attempt
	// to open the parser grammar file.
	tokens, err := csharplexer.TokenizeCSharp(source, version)
	if err != nil {
		return nil, err
	}
	grammarPath, err := getGrammarPath(version)
	if err != nil {
		return nil, err
	}
	bytes, err := os.ReadFile(grammarPath)
	if err != nil {
		return nil, err
	}
	grammar, err := grammartools.ParseParserGrammar(string(bytes))
	if err != nil {
		return nil, err
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

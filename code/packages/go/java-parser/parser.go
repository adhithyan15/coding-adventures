// Package javaparser parses Java source code using versioned grammars.
// It supports Java 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, and 21, each with its
// own parser grammar. See the java-lexer package for a description of each
// release.
//
// The grammars are embedded at compile time as native Go data structures in
// grammar_data.go (VersionedParserGrammars, keyed by version string). Nothing
// is read from disk at run time, so the parser needs no filesystem capability
// and works unchanged when the package is built standalone.
package javaparser

import (
	"fmt"

	javalexer "github.com/adhithyan15/coding-adventures/code/packages/go/java-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// DefaultVersion is the Java version used when no version is specified.
// Kept in sync with the java-lexer package.
const DefaultVersion = "21"

// CreateJavaParser constructs a GrammarParser ready to parse the given
// Java source string.
//
// version selects the Java grammar pair:
//   - ""     — uses DefaultVersion ("21"), the latest LTS release
//   - "1.0", "1.1", "1.4" — classic Java releases
//   - "5", "7", "8" — pre-modular Java releases
//   - "10", "14", "17", "21" — modern Java releases
//
// Both the lexer and parser grammars are selected by the same version string,
// guaranteeing that the token set and parse rules stay consistent. Both are
// read from the compiled-in grammar maps; no grammar file is read at run time.
//
// An error is returned only when the version string is unrecognised.
func CreateJavaParser(source string, version string) (*parser.GrammarParser, error) {
	// Tokenise first; any version-error is surfaced here before we look up
	// the parser grammar.
	tokens, err := javalexer.TokenizeJava(source, version)
	if err != nil {
		return nil, err
	}
	// Default to the latest LTS version when no version is specified.
	if version == "" {
		version = DefaultVersion
	}
	grammar, ok := VersionedParserGrammars[version]
	if !ok {
		return nil, fmt.Errorf(
			"unknown Java version %q: valid versions are 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, 21",
			version,
		)
	}
	return parser.NewGrammarParser(tokens, grammar), nil
}

// ParseJava is the main entry point for parsing Java source code.
//
// It parses source using the grammar for the given Java version and returns
// the root AST node produced by the underlying GrammarParser. Pass version=""
// to use the default grammar (Java 21), which is the best choice when
// version is unknown.
//
// Example — parse with the default grammar:
//
//	node, err := ParseJava("int x = 1 + 2;", "")
//
// Example — parse with a specific version:
//
//	node, err := ParseJava("var x = 1;", "10")
//
// Example — parse classic Java:
//
//	node, err := ParseJava("int x = 1;", "1.0")
func ParseJava(source string, version string) (*parser.ASTNode, error) {
	javaParser, err := CreateJavaParser(source, version)
	if err != nil {
		return nil, err
	}
	return javaParser.Parse()
}

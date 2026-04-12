package javaparser

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	javalexer "github.com/adhithyan15/coding-adventures/code/packages/go/java-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// validVersions is the set of Java version strings the parser recognises.
// These must stay in sync with the same map in the java-lexer package so
// that the lexer and parser always agree on which grammars are available.
//
// See the java-lexer package for a detailed description of each release.
var validVersions = map[string]bool{
	"1.0": true,
	"1.1": true,
	"1.4": true,
	"5":   true,
	"7":   true,
	"8":   true,
	"10":  true,
	"14":  true,
	"17":  true,
	"21":  true,
}

// DefaultVersion is the Java version used when no version is specified.
// Kept in sync with the java-lexer package.
const DefaultVersion = "21"

// getGrammarPath resolves the absolute path to the .grammar file for the given
// Java version string.
//
// When version is "" (empty string), the DefaultVersion ("21") is used.
// This provides a sensible default for callers that do not care about a
// specific version.
//
// Any other non-empty string must appear in validVersions; an unknown version
// returns a descriptive error so that typos produce actionable messages.
func getGrammarPath(version string) (string, error) {
	// runtime.Caller(0) returns the path to *this* source file at compile
	// time. We navigate up three directories to reach code/grammars/.
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	// Default to the latest LTS version when no version is specified.
	if version == "" {
		version = DefaultVersion
	}

	if !validVersions[version] {
		return "", fmt.Errorf(
			"unknown Java version %q: valid versions are 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, 21",
			version,
		)
	}

	// Grammar files follow the pattern: java/java{version}.grammar
	// For example: java/java21.grammar, java/java1.0.grammar
	return filepath.Join(root, "java", "java"+version+".grammar"), nil
}

// CreateJavaParser constructs a GrammarParser ready to parse the given
// Java source string.
//
// version selects the Java grammar pair:
//   - ""     — uses DefaultVersion ("21"), the latest LTS release
//   - "1.0", "1.1", "1.4" — classic Java releases
//   - "5", "7", "8" — pre-modular Java releases
//   - "10", "14", "17", "21" — modern Java releases
//
// Both the lexer and parser grammar files are selected by the same version
// string, guaranteeing that the token set and parse rules stay consistent.
//
// An error is returned if the version string is unrecognised, or if any
// grammar file cannot be read.
func CreateJavaParser(source string, version string) (*parser.GrammarParser, error) {
	// Tokenise first; any version-error is surfaced here before we attempt
	// to open the parser grammar file.
	tokens, err := javalexer.TokenizeJava(source, version)
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

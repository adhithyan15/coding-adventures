package javascriptparser

import (
	"fmt"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	javascriptlexer "github.com/adhithyan15/coding-adventures/code/packages/go/javascript-lexer"
)

// validVersions is the set of ECMAScript / JavaScript version strings the
// parser recognises. These must stay in sync with the same map in the
// javascript-lexer package so that the lexer and parser always agree on which
// grammars are available.
//
// See the javascript-lexer package for a detailed description of each edition.
var validVersions = map[string]bool{
	"es1":    true,
	"es3":    true,
	"es5":    true,
	"es2015": true,
	"es2016": true,
	"es2017": true,
	"es2018": true,
	"es2019": true,
	"es2020": true,
	"es2021": true,
	"es2022": true,
	"es2023": true,
	"es2024": true,
	"es2025": true,
}

// getGrammarPath resolves the absolute path to the .grammar file for the given
// JavaScript / ECMAScript version string.
//
// When version is "" (empty string) the generic grammar at
// code/grammars/javascript.grammar is used — preserving backward-compatible
// behaviour for callers that do not care about a specific version.
//
// Any other non-empty string must appear in validVersions; an unknown version
// returns a descriptive error so that typos produce actionable messages.
func getGrammarPath(version string) (string, error) {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	if version == "" {
		return filepath.Join(root, "javascript.grammar"), nil
	}
	if !validVersions[version] {
		return "", fmt.Errorf(
			"unknown JavaScript version %q: valid versions are es1, es3, es5, es2015–es2025",
			version,
		)
	}
	return filepath.Join(root, "ecmascript", version+".grammar"), nil
}

// CreateJavascriptParser constructs a GrammarParser ready to parse the given
// JavaScript source string.
//
// version selects the ECMAScript grammar pair:
//   - ""      — generic grammar (javascript.grammar / javascript.tokens);
//               same as pre-0.2.0 behaviour
//   - "es1", "es3", "es5" — classic ECMAScript editions
//   - "es2015" through "es2025" — modern ECMAScript yearly editions
//
// Both the lexer and parser grammar files are selected by the same version
// string, guaranteeing that the token set and parse rules stay consistent.
//
// An error is returned if the version string is unrecognised, or if any
// grammar file cannot be read.
func CreateJavascriptParser(source string, version string) (*parser.GrammarParser, error) {
	// Tokenise first; any version-error is surfaced here before we attempt
	// to open the parser grammar file.
	tokens, err := javascriptlexer.TokenizeJavascript(source, version)
	if err != nil {
		return nil, err
	}
	grammarPath, err := getGrammarPath(version)
	if err != nil {
		return nil, err
	}
	return StartNew[*parser.GrammarParser]("javascriptparser.CreateJavascriptParser", nil,
		func(op *Operation[*parser.GrammarParser], rf *ResultFactory[*parser.GrammarParser]) *OperationResult[*parser.GrammarParser] {
			bytes, err := op.File.ReadFile(grammarPath)
			if err != nil {
				return rf.Fail(nil, err)
			}
			grammar, err := grammartools.ParseParserGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, parser.NewGrammarParser(tokens, grammar))
		}).GetResult()
}

// ParseJavascript is the main entry point for parsing JavaScript source code.
//
// It parses source using the grammar for the given ECMAScript version and
// returns the root AST node produced by the underlying GrammarParser.
// Pass version="" to use the generic grammar, which covers the superset of all
// supported versions and is the best choice when version is unknown.
//
// Example — parse with the generic grammar:
//
//	node, err := ParseJavascript("let x = 1 + 2;", "")
//
// Example — parse with a specific version:
//
//	node, err := ParseJavascript("const x = 1;", "es2022")
func ParseJavascript(source string, version string) (*parser.ASTNode, error) {
	javascriptParser, err := CreateJavascriptParser(source, version)
	if err != nil {
		return nil, err
	}
	return javascriptParser.Parse()
}

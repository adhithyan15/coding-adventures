package typescriptparser

import (
	"fmt"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	typescriptlexer "github.com/adhithyan15/coding-adventures/code/packages/go/typescript-lexer"
)

// validVersions is the set of TypeScript version strings the parser recognises.
// These must stay in sync with the same map in the typescript-lexer package so
// that the lexer and parser always agree on which grammars are available.
//
//   ts1.0  — TypeScript 1.0  (April 2014)    first public release
//   ts2.0  — TypeScript 2.0  (September 2016) strict null checks era
//   ts3.0  — TypeScript 3.0  (July 2018)      project references era
//   ts4.0  — TypeScript 4.0  (August 2020)    variadic tuple types era
//   ts5.0  — TypeScript 5.0  (March 2023)     decorators era
//   ts5.8  — TypeScript 5.8  (February 2025)  latest stable
var validVersions = map[string]bool{
	"ts1.0": true,
	"ts2.0": true,
	"ts3.0": true,
	"ts4.0": true,
	"ts5.0": true,
	"ts5.8": true,
}

// getGrammarPath resolves the absolute path to the .grammar file for the given
// TypeScript version string.
//
// When version is "" (empty string) the generic grammar at
// code/grammars/typescript.grammar is used — preserving backward-compatible
// behaviour for callers that do not care about a specific version.
//
// Any other non-empty string must appear in validVersions; an unknown version
// returns a descriptive error so that typos produce actionable messages.
func getGrammarPath(version string) (string, error) {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	if version == "" {
		return filepath.Join(root, "typescript.grammar"), nil
	}
	if !validVersions[version] {
		return "", fmt.Errorf("unknown TypeScript version %q: valid versions are ts1.0, ts2.0, ts3.0, ts4.0, ts5.0, ts5.8", version)
	}
	return filepath.Join(root, "typescript", version+".grammar"), nil
}

// CreateTypescriptParser constructs a GrammarParser ready to parse the given
// TypeScript source string.
//
// version selects the TypeScript grammar pair:
//   - ""      — generic grammar (typescript.grammar / typescript.tokens);
//               same as pre-0.2.0 behaviour
//   - "ts1.0" through "ts5.8" — versioned grammar pair
//
// Both the lexer and parser grammar files are selected by the same version
// string, guaranteeing that the token set and parse rules stay consistent.
//
// An error is returned if the version string is unrecognised, or if any
// grammar file cannot be read.
func CreateTypescriptParser(source string, version string) (*parser.GrammarParser, error) {
	// Tokenise first; any version-error is surfaced here before we attempt
	// to open the parser grammar file.
	tokens, err := typescriptlexer.TokenizeTypescript(source, version)
	if err != nil {
		return nil, err
	}
	grammarPath, err := getGrammarPath(version)
	if err != nil {
		return nil, err
	}
	return StartNew[*parser.GrammarParser]("typescriptparser.CreateTypescriptParser", nil,
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

// ParseTypescript is the main entry point for parsing TypeScript source code.
//
// It tokenises and then parses source using the grammar for the given
// TypeScript version, returning the root ASTNode of the parse tree.
// Pass version="" to use the generic grammar.
//
// Example — parse with the generic grammar:
//
//	program, err := ParseTypescript("let x = 1 + 2;", "")
//
// Example — parse with a specific version:
//
//	program, err := ParseTypescript("const x: string = 'hi';", "ts5.8")
func ParseTypescript(source string, version string) (*parser.ASTNode, error) {
	typescriptParser, err := CreateTypescriptParser(source, version)
	if err != nil {
		return nil, err
	}
	return typescriptParser.Parse()
}

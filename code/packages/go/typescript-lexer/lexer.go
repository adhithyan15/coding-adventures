package typescriptlexer

import (
	"fmt"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// validVersions is the set of TypeScript version strings the lexer recognises.
// Each maps to a versioned grammar file under code/grammars/typescript/.
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

// getGrammarPath resolves the absolute path to the .tokens grammar file for the
// given TypeScript version string.
//
// When version is "" (empty string) the generic grammar at
// code/grammars/typescript.tokens is used — this preserves backward-compatible
// behaviour for callers that do not care about a specific version.
//
// Any other non-empty string must appear in validVersions; an unknown version
// returns a descriptive error so that typos produce actionable messages.
func getGrammarPath(version string) (string, error) {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	if version == "" {
		return filepath.Join(root, "typescript.tokens"), nil
	}
	if !validVersions[version] {
		return "", fmt.Errorf("unknown TypeScript version %q: valid versions are ts1.0, ts2.0, ts3.0, ts4.0, ts5.0, ts5.8", version)
	}
	return filepath.Join(root, "typescript", version+".tokens"), nil
}

// CreateTypescriptLexer constructs a GrammarLexer ready to tokenise the given
// TypeScript source string.
//
// version selects the TypeScript grammar file:
//   - ""      — generic grammar (typescript.tokens); same as pre-0.2.0 behaviour
//   - "ts1.0" through "ts5.8" — versioned grammar
//
// An error is returned if the version string is unrecognised or if the grammar
// file cannot be read.
func CreateTypescriptLexer(source string, version string) (*lexer.GrammarLexer, error) {
	grammarPath, err := getGrammarPath(version)
	if err != nil {
		return nil, err
	}
	return StartNew[*lexer.GrammarLexer]("typescriptlexer.CreateTypescriptLexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			bytes, err := op.File.ReadFile(grammarPath)
			if err != nil {
				return rf.Fail(nil, err)
			}
			grammar, err := grammartools.ParseTokenGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
}

// TokenizeTypescript is the main entry point for lexing TypeScript source code.
//
// It tokenises source using the grammar for the given TypeScript version and
// returns the flat token slice produced by the underlying GrammarLexer.
// Pass version="" to use the generic grammar, which covers the superset of all
// supported versions and is the best choice when version is unknown.
//
// Example — tokenise with the generic grammar:
//
//	tokens, err := TokenizeTypescript("let x = 1;", "")
//
// Example — tokenise with a specific version:
//
//	tokens, err := TokenizeTypescript("const x: string = 'hi';", "ts5.8")
func TokenizeTypescript(source string, version string) ([]lexer.Token, error) {
	typescriptLexer, err := CreateTypescriptLexer(source, version)
	if err != nil {
		return nil, err
	}
	return typescriptLexer.Tokenize(), nil
}

// Package jsonlexer tokenizes JSON text using a grammar-driven lexer.
//
// JSON (RFC 8259) is a lightweight data interchange format. Unlike programming
// languages, JSON has no keywords, no comments, no indentation significance,
// and no identifiers. Every token is either a literal delimiter or a value.
//
// This package is a thin wrapper around the generic grammar-driven lexer. It:
//   1. Loads the JSON token grammar from the json.tokens file
//   2. Passes it to the GrammarLexer, which compiles the regex patterns
//   3. The GrammarLexer handles skip patterns (whitespace) automatically
//      based on the grammar file
//
// The json.tokens grammar file defines:
//   - STRING: double-quoted strings with escape sequences (\" \\ \/ \b \f \n \r \t \uXXXX)
//   - NUMBER: integers, decimals, and scientific notation (including negative)
//   - TRUE, FALSE, NULL: the three JSON literal values
//   - Structural tokens: { } [ ] : ,
//   - skip: whitespace (spaces, tabs, carriage returns, newlines)
//
// JSON has no indentation mode, no keywords to reclassify, and no reserved
// words. This makes it the simplest practical grammar for the infrastructure.
//
// Usage:
//
//   // One-shot tokenization: JSON text in, token slice out
//   tokens, err := jsonlexer.TokenizeJSON(`{"name": "Alice", "age": 30}`)
//
//   // Or create a reusable lexer for more control
//   lex, err := jsonlexer.CreateJSONLexer(`[1, 2, 3]`)
//   tokens := lex.Tokenize()
package jsonlexer

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// getGrammarPath computes the absolute path to the json.tokens grammar file.
//
// We use runtime.Caller(0) to find the directory of this Go source file at
// runtime, then navigate up three levels (json-lexer -> go -> packages ->
// code) to reach the grammars directory. This approach works regardless of the
// working directory, which is important because tests and the build tool may
// run from different locations.
//
// Directory structure:
//
//	code/
//	  grammars/
//	    json.tokens        <-- this is what we want
//	  packages/
//	    go/
//	      json-lexer/
//	        lexer.go        <-- we are here (3 levels below code/)
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "json.tokens")
}

// CreateJSONLexer loads the JSON token grammar and returns a configured
// GrammarLexer ready to tokenize the given JSON text.
//
// The returned lexer operates in default mode (no indentation tracking).
// JSON's whitespace is handled by skip patterns: spaces, tabs, carriage
// returns, and newlines are all consumed silently between tokens.
//
// Returns an error if the grammar file cannot be read or parsed.
func CreateJSONLexer(source string) (*lexer.GrammarLexer, error) {
	return StartNew[*lexer.GrammarLexer]("jsonlexer.CreateJSONLexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			bytes, err := op.File.ReadFile(getGrammarPath())
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

// TokenizeJSON is a convenience function that tokenizes JSON text in a single
// call. It creates a lexer, runs tokenization, and returns the resulting token
// slice.
//
// Returns an error if the grammar file cannot be loaded.
func TokenizeJSON(source string) ([]lexer.Token, error) {
	jsonLexer, err := CreateJSONLexer(source)
	if err != nil {
		return nil, err
	}
	return jsonLexer.Tokenize(), nil
}

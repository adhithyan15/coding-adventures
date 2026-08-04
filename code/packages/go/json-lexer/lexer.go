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
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateJSONLexer returns a GrammarLexer configured with the JSON token
// grammar, ready to tokenize the given JSON text.
//
// The grammar is embedded at compile time as native Go in _grammar.go
// (TokenGrammarData); nothing is read from disk at run time. The lexer works
// unchanged when the package is built standalone and needs no filesystem
// capability. The error result is retained for API compatibility and is
// always nil.
func CreateJSONLexer(source string) (*lexer.GrammarLexer, error) {
	return lexer.NewGrammarLexer(source, TokenGrammarData), nil
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

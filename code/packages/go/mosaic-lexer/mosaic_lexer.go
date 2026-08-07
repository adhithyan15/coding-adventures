// Package mosaiclexer tokenizes Mosaic source text using a grammar-driven lexer.
//
// Mosaic is a component description language (CDL) for declaring UI component
// structure with named typed slots. A Mosaic file describes exactly one component
// with its data API (slots) and its visual tree (nodes with properties).
//
// This package is a thin wrapper around the generic grammar-driven lexer. It:
//  1. Uses the embedded mosaic.tokens grammar (TokenGrammarData in grammar_data.go)
//  2. Passes it to the GrammarLexer, which compiles the regex patterns
//  3. The GrammarLexer handles skip patterns (whitespace, comments) automatically
//
// The mosaic.tokens grammar defines:
//   - STRING: double-quoted strings with standard escapes
//   - DIMENSION: numbers with unit suffixes like 16dp, 1.5sp, 100%
//   - NUMBER: plain numeric literals (integers and decimals)
//   - COLOR_HEX: hex color literals (#rgb, #rrggbb, #rrggbbaa)
//   - KEYWORD: reserved words — component, slot, import, from, as, text,
//     number, bool, image, color, node, list, true, false, when, each
//   - NAME: identifiers, including hyphenated names like corner-radius
//   - Structural tokens: { } < > : ; , . = @
//   - skip: line comments, block comments, whitespace
//
// Usage:
//
//	// One-shot tokenization: Mosaic text in, token slice out
//	tokens, err := mosaiclexer.Tokenize(`component Button { slot label: text; Text {} }`)
//
//	// Or create a reusable lexer for more control
//	lex, err := mosaiclexer.CreateLexer(source)
//	tokens := lex.Tokenize()
package mosaiclexer

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateLexer returns a GrammarLexer configured with the Mosaic token grammar,
// ready to tokenize the given Mosaic source text.
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData); nothing is read from disk at run time. The returned lexer
// skips line comments (// ...), block comments (/* ... */), and all whitespace
// (spaces, tabs, newlines) between tokens.
//
// The error result is retained for API compatibility and is always nil.
func CreateLexer(source string) (*lexer.GrammarLexer, error) {
	return lexer.NewGrammarLexer(source, TokenGrammarData), nil
}

// Tokenize is a convenience function that tokenizes Mosaic source text in a
// single call. It creates a lexer, runs tokenization, and returns the token slice.
//
// Returns an error if the grammar cannot be loaded.
func Tokenize(source string) ([]lexer.Token, error) {
	l, err := CreateLexer(source)
	if err != nil {
		return nil, err
	}
	return l.Tokenize(), nil
}

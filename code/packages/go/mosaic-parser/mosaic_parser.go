// Package mosaicparser parses Mosaic source text into an Abstract Syntax Tree.
//
// Mosaic is a component description language (CDL) that declares UI component
// structure with named typed slots. This parser produces a generic AST using
// the grammar-driven parser engine.
//
// The parsing pipeline has two stages:
//
//  1. Lexing (mosaic-lexer): Mosaic source is tokenized into a stream of tokens.
//     The lexer handles string escapes, dimension literals, hex colors,
//     keyword reclassification, and whitespace/comment skipping.
//
//  2. Parsing (this package): The token stream is parsed according to the
//     Mosaic grammar rules using recursive descent with packrat memoization.
//     The grammar is embedded as Go data structures in _grammar.go (ParserGrammarData),
//     which ensures correct ordering of alternatives (e.g., list_type before KEYWORD
//     in slot_type, so `list<text>` is not consumed as a bare `list` keyword).
//
// Usage:
//
//	ast, err := mosaicparser.Parse(`
//	  component Label {
//	    slot text: text;
//	    Text { content: @text; }
//	  }
//	`)
package mosaicparser

import (
	mosaiclexer "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// CreateParser tokenizes the Mosaic source using the Mosaic lexer, then
// creates a GrammarParser using the embedded ParserGrammarData.
//
// Using the embedded grammar (rather than reading mosaic.grammar at runtime)
// ensures that the alternation order in slot_type is correct: list_type is
// tried before KEYWORD, so `list<text>` parses as a list_type rather than
// being consumed as a bare `list` keyword token.
//
// Returns an error if lexing fails.
func CreateParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source using the Mosaic lexer.
	tokens, err := mosaiclexer.Tokenize(source)
	if err != nil {
		return nil, err
	}

	// Step 2: Create the grammar-driven parser using the embedded grammar.
	// ParserGrammarData is defined in _grammar.go with list_type before KEYWORD
	// in the slot_type alternation, ensuring `list<T>` parses correctly.
	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
}

// Parse is a convenience function that parses Mosaic source text into an AST
// in a single call.
//
// Returns an error if lexing or parsing fails.
func Parse(source string) (*parser.ASTNode, error) {
	p, err := CreateParser(source)
	if err != nil {
		return nil, err
	}
	return p.Parse()
}

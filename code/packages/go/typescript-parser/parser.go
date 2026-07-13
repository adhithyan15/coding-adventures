// Package typescriptparser parses TypeScript source code into an Abstract
// Syntax Tree using versioned grammars. It supports TypeScript 1.0, 2.0, 3.0,
// 4.0, 5.0, and 5.8, each with its own parser grammar, plus a generic superset
// grammar for callers that do not care about a specific version.
//
// The parsing pipeline has two stages:
//
//  1. Lexing (typescript-lexer): source is tokenised with the token grammar for
//     the requested version. The version string is validated there; an unknown
//     version surfaces as an error before any parser grammar is selected.
//  2. Parsing (this package): the token stream is parsed with the parser grammar
//     for the same version.
//
// Both grammars are embedded at compile time as native Go data structures in
// grammar_data.go (VersionedParserGrammars, keyed by version string, plus
// ParserGrammarData for the generic path). Nothing is read from disk at run
// time, so the parser needs no filesystem capability and works unchanged when
// the package is built standalone.
//
// Usage:
//
//	ast, err := typescriptparser.ParseTypescript(source, "ts5.8")
//	ast, err := typescriptparser.ParseTypescript(source, "")  // generic grammar
package typescriptparser

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	typescriptlexer "github.com/adhithyan15/coding-adventures/code/packages/go/typescript-lexer"
)

// CreateTypescriptParser constructs a GrammarParser ready to parse the given
// TypeScript source string.
//
// version selects the TypeScript grammar pair:
//   - ""      — generic grammar (typescript.grammar / typescript.tokens);
//     same as pre-0.2.0 behaviour
//   - "ts1.0" through "ts5.8" — versioned grammar pair
//
// Both the lexer and parser grammars are selected by the same version string,
// guaranteeing that the token set and parse rules stay consistent. Grammars are
// read from the compiled-in grammar data; no grammar file is read at run time.
//
// An error is returned if the version string is unrecognised — that error comes
// from the lexer's tokenisation step, which validates the version first.
func CreateTypescriptParser(source string, version string) (*parser.GrammarParser, error) {
	// Tokenise first; a version error is surfaced here (the lexer owns version
	// validation) before we select the parser grammar.
	tokens, err := typescriptlexer.TokenizeTypescript(source, version)
	if err != nil {
		return nil, err
	}
	grammar := ParserGrammarData
	if version != "" {
		// Unknown versions were already rejected by TokenizeTypescript above,
		// so a present version is guaranteed to be in the map.
		grammar = VersionedParserGrammars[version]
	}
	return parser.NewGrammarParser(tokens, grammar), nil
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

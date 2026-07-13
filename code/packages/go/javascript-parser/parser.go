package javascriptparser

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	javascriptlexer "github.com/adhithyan15/coding-adventures/code/packages/go/javascript-lexer"
)

// CreateJavascriptParser tokenises the JavaScript source with the JavaScript
// lexer, then returns a GrammarParser ready to produce an AST.
//
// version selects the ECMAScript edition:
//   - ""      — generic grammar; same as pre-0.2.0 behaviour
//   - "es1", "es3", "es5" — classic ECMAScript editions
//   - "es2015" through "es2025" — modern ECMAScript yearly editions
//
// The parser grammar is embedded at compile time as native Go in grammar_data.go
// (ParserGrammarData); nothing is read from disk at run time, so the parser
// needs no filesystem capability and works when built standalone. Version
// validation (including the descriptive error for an unknown version such as
// "es99") is handled by the lexer's TokenizeJavascript step below.
func CreateJavascriptParser(source string, version string) (*parser.GrammarParser, error) {
	// Tokenise first; any version-error is surfaced here.
	tokens, err := javascriptlexer.TokenizeJavascript(source, version)
	if err != nil {
		return nil, err
	}
	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
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

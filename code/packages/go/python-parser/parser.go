// Package pythonparser parses Python source code into an Abstract Syntax Tree.
//
// The parsing pipeline has two stages:
//
//  1. Lexing (python-lexer): Python text is tokenized into a stream of tokens
//     for the requested language version.
//
//  2. Parsing (this package): The token stream is parsed according to the
//     Python parser grammar using the grammar-driven parser engine.
//
// The parser grammar is embedded at compile time as native Go in
// grammar_data.go (ParserGrammarData); nothing is read from disk at run time,
// so the parser needs no filesystem capability and works when built standalone.
package pythonparser

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	pythonlexer "github.com/adhithyan15/coding-adventures/code/packages/go/python-lexer"
)

// CreatePythonParser tokenizes the Python source using the Python lexer, then
// returns a GrammarParser configured with the embedded Python parser grammar,
// ready to produce an AST.
//
// The parser grammar is embedded at compile time as native Go in grammar_data.go
// (ParserGrammarData); nothing is read from disk at run time. The error result
// is retained for API compatibility; it is non-nil only when lexing fails.
func CreatePythonParser(source string) (*parser.GrammarParser, error) {
	tokens, err := pythonlexer.TokenizePython(source, "")
	if err != nil {
		return nil, err
	}
	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
}

// ParsePython parses Python source into an AST in a single call. It creates a
// parser, runs parsing, and returns the root AST node.
//
// Returns an error if lexing or parsing fails.
func ParsePython(source string) (*parser.ASTNode, error) {
	pythonParser, err := CreatePythonParser(source)
	if err != nil {
		return nil, err
	}
	return pythonParser.Parse()
}

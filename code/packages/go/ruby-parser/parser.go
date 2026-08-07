package rubyparser

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/ruby-lexer"
)

// CreateRubyParser tokenizes the Ruby source using the Ruby lexer, then returns
// a GrammarParser configured with the Ruby parser grammar, ready to produce an
// AST.
//
// The parser grammar is embedded at compile time as native Go in grammar_data.go
// (ParserGrammarData); nothing is read from disk at run time, so the parser
// needs no filesystem capability and works when built standalone. The error
// result is retained for API compatibility; it is non-nil only when lexing fails.
func CreateRubyParser(source string) (*parser.GrammarParser, error) {
	tokens, err := rubylexer.TokenizeRuby(source)
	if err != nil {
		return nil, err
	}
	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
}

func ParseRuby(source string) (*parser.ASTNode, error) {
	rubyParser, err := CreateRubyParser(source)
	if err != nil {
		return nil, err
	}
	return rubyParser.Parse()
}

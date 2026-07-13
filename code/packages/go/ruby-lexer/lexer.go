package rubylexer

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateRubyLexer returns a GrammarLexer configured with the Ruby token
// grammar, ready to tokenize the given Ruby source.
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData); nothing is read from disk at run time. The lexer works
// unchanged when the package is built standalone and needs no filesystem
// capability. The error result is retained for API compatibility and is
// always nil.
func CreateRubyLexer(source string) (*lexer.GrammarLexer, error) {
	return lexer.NewGrammarLexer(source, TokenGrammarData), nil
}

func TokenizeRuby(source string) ([]lexer.Token, error) {
	rubyLexer, err := CreateRubyLexer(source)
	if err != nil {
		return nil, err
	}
	return rubyLexer.Tokenize(), nil
}

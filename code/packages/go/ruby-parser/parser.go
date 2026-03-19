package rubyparser

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/ruby-lexer"
)

func ParseRuby(source string) (parser.Program, error) {
	tokens, err := rubylexer.TokenizeRuby(source)
	if err != nil {
		return parser.Program{}, err
	}
	p := parser.NewParser(tokens)
	ast := p.Parse()
	return ast, nil
}

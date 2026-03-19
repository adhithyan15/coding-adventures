package pythonparser

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/python-lexer"
)

func ParsePython(source string) (parser.Program, error) {
	tokens, err := pythonlexer.TokenizePython(source)
	if err != nil {
		return parser.Program{}, err
	}
	p := parser.NewParser(tokens)
	ast := p.Parse()
	return ast, nil
}

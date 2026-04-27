package nibparser

import (
	niblexer "github.com/adhithyan15/coding-adventures/code/packages/go/nib-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

func CreateNibParser(source string) *parser.GrammarParser {
	return parser.NewGrammarParser(niblexer.TokenizeNib(source), ParserGrammarData)
}

func ParseNib(source string) (*parser.ASTNode, error) {
	return CreateNibParser(source).Parse()
}

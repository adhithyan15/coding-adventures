// Package cssparser parses CSS text into a generic AST.
package cssparser

import (
	csslexer "github.com/adhithyan15/coding-adventures/code/packages/go/css-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

func convertToken(token csslexer.Token) lexer.Token {
	return lexer.Token{
		Type:     lexer.TokenName,
		TypeName: token.Type,
		Value:    token.Value,
		Line:     token.Line,
		Column:   token.Column,
	}
}

func convertTokens(tokens []csslexer.Token) []lexer.Token {
	converted := make([]lexer.Token, 0, len(tokens))
	for _, token := range tokens {
		converted = append(converted, convertToken(token))
	}
	return converted
}

// CreateCSSParser tokenizes source text and returns a configured grammar parser.
func CreateCSSParser(source string) (*parser.GrammarParser, error) {
	tokens, err := csslexer.Tokenize(source)
	if err != nil {
		return nil, err
	}
	return parser.NewGrammarParser(convertTokens(tokens), ParserGrammarData), nil
}

// ParseCSS parses source text into a generic AST.
func ParseCSS(source string) (*parser.ASTNode, error) {
	cssParser, err := CreateCSSParser(source)
	if err != nil {
		return nil, err
	}
	return cssParser.Parse()
}

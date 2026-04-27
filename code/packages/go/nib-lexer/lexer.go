package niblexer

import "github.com/adhithyan15/coding-adventures/code/packages/go/lexer"

func CreateNibLexer(source string) *lexer.GrammarLexer {
	return lexer.NewGrammarLexer(source, TokenGrammarData)
}

func TokenizeNib(source string) []lexer.Token {
	tokens := CreateNibLexer(source).Tokenize()
	for index := range tokens {
		if tokens[index].TypeName == "KEYWORD" {
			tokens[index].TypeName = tokens[index].Value
		}
	}
	return tokens
}

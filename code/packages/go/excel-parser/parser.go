package excelparser

import (
	excellexer "github.com/adhithyan15/coding-adventures/code/packages/go/excel-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// CreateExcelParser tokenizes the Excel formula using the Excel lexer, then
// returns a GrammarParser configured with the Excel parser grammar, ready to
// produce an AST.
//
// The parser grammar is embedded at compile time as native Go in grammar_data.go
// (ParserGrammarData); nothing is read from disk at run time, so the parser
// needs no filesystem capability and works when built standalone. The parser is
// wired with a pre-parse pass that normalizes NAME/NUMBER tokens adjacent to a
// colon into COLUMN_REF/ROW_REF reference tokens. The error result is retained
// for API compatibility; it is non-nil only when lexing fails.
func CreateExcelParser(source string) (*parser.GrammarParser, error) {
	tokens, err := excellexer.TokenizeExcelFormula(source)
	if err != nil {
		return nil, err
	}
	excelParser := parser.NewGrammarParser(tokens, ParserGrammarData)
	excelParser.AddPreParse(normalizeExcelReferenceTokens)
	return excelParser, nil
}

func previousSignificantToken(tokens []lexer.Token, index int) *lexer.Token {
	for i := index - 1; i >= 0; i-- {
		if tokens[i].EffectiveTypeName() != "SPACE" {
			return &tokens[i]
		}
	}
	return nil
}

func nextSignificantToken(tokens []lexer.Token, index int) *lexer.Token {
	for i := index + 1; i < len(tokens); i++ {
		if tokens[i].EffectiveTypeName() != "SPACE" {
			return &tokens[i]
		}
	}
	return nil
}

func normalizeExcelReferenceTokens(tokens []lexer.Token) []lexer.Token {
	normalized := make([]lexer.Token, len(tokens))
	copy(normalized, tokens)

	for index, token := range normalized {
		tokenType := token.EffectiveTypeName()
		if tokenType != "NAME" && tokenType != "NUMBER" {
			continue
		}

		previous := previousSignificantToken(normalized, index)
		next := nextSignificantToken(normalized, index)
		adjacentToColon := (previous != nil && previous.EffectiveTypeName() == "COLON") ||
			(next != nil && next.EffectiveTypeName() == "COLON")

		if tokenType == "NAME" && adjacentToColon {
			normalized[index].Type = lexer.TokenName
			normalized[index].TypeName = "COLUMN_REF"
			continue
		}

		if tokenType == "NUMBER" && adjacentToColon {
			normalized[index].Type = lexer.TokenName
			normalized[index].TypeName = "ROW_REF"
		}
	}

	return normalized
}

func ParseExcelFormula(source string) (*parser.ASTNode, error) {
	excelParser, err := CreateExcelParser(source)
	if err != nil {
		return nil, err
	}
	return excelParser.Parse()
}

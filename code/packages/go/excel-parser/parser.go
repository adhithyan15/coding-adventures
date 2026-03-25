package excelparser

import (
	"os"
	"path/filepath"
	"runtime"

	excellexer "github.com/adhithyan15/coding-adventures/code/packages/go/excel-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "excel.grammar")
}

func CreateExcelParser(source string) (*parser.GrammarParser, error) {
	tokens, err := excellexer.TokenizeExcelFormula(source)
	if err != nil {
		return nil, err
	}
	bytes, err := os.ReadFile(getGrammarPath())
	if err != nil {
		return nil, err
	}
	grammar, err := grammartools.ParseParserGrammar(string(bytes))
	if err != nil {
		return nil, err
	}
	excelParser := parser.NewGrammarParser(tokens, grammar)
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

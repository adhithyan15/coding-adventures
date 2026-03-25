package excellexer

import (
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "excel.tokens")
}

func CreateExcelLexer(source string) (*lexer.GrammarLexer, error) {
	bytes, err := os.ReadFile(getGrammarPath())
	if err != nil {
		return nil, err
	}
	grammar, err := grammartools.ParseTokenGrammar(string(bytes))
	if err != nil {
		return nil, err
	}
	for i, definition := range grammar.Definitions {
		if definition.Name == "FUNCTION_NAME" || definition.Name == "TABLE_NAME" ||
			definition.Name == "COLUMN_REF" || definition.Name == "ROW_REF" {
			grammar.Definitions[i].Pattern = "a^"
		}
	}
	excelLexer := lexer.NewGrammarLexer(source, grammar)
	excelLexer.SetOnToken(ExcelOnToken)
	return excelLexer, nil
}

func nextNonSpaceChar(ctx *lexer.LexerContext) string {
	for offset := 1; ; offset++ {
		ch := ctx.Peek(offset)
		if ch == "" || ch != " " {
			return ch
		}
	}
}

func ExcelOnToken(token lexer.Token, ctx *lexer.LexerContext) {
	if token.EffectiveTypeName() != "NAME" {
		return
	}

	nextChar := nextNonSpaceChar(ctx)
	if nextChar == "(" {
		ctx.Suppress()
		ctx.Emit(lexer.Token{
			Type:     token.Type,
			Value:    token.Value,
			Line:     token.Line,
			Column:   token.Column,
			TypeName: "FUNCTION_NAME",
		})
		return
	}

	if nextChar == "[" {
		ctx.Suppress()
		ctx.Emit(lexer.Token{
			Type:     token.Type,
			Value:    token.Value,
			Line:     token.Line,
			Column:   token.Column,
			TypeName: "TABLE_NAME",
		})
	}
}

func TokenizeExcelFormula(source string) ([]lexer.Token, error) {
	excelLexer, err := CreateExcelLexer(source)
	if err != nil {
		return nil, err
	}
	return excelLexer.Tokenize(), nil
}

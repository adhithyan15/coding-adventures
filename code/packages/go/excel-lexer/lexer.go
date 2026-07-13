package excellexer

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateExcelLexer returns a GrammarLexer configured with the Excel token
// grammar, ready to tokenize the given Excel formula.
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData); nothing is read from disk at run time. The embedded
// grammar already renders FUNCTION_NAME, TABLE_NAME, COLUMN_REF, and ROW_REF
// unmatchable (their patterns match nothing) so those token types are produced
// only via the ExcelOnToken reclassification hook and the parser's pre-parse
// normalization, never by the raw lexer. The lexer works unchanged when the
// package is built standalone and needs no filesystem capability. The error
// result is retained for API compatibility and is always nil.
func CreateExcelLexer(source string) (*lexer.GrammarLexer, error) {
	excelLexer := lexer.NewGrammarLexer(source, TokenGrammarData)
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

package typescriptlexer

import (
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "typescript.tokens")
}

func CreateTypescriptLexer(source string) (*lexer.GrammarLexer, error) {
	return StartNew[*lexer.GrammarLexer]("typescriptlexer.CreateTypescriptLexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}
			grammar, err := grammartools.ParseTokenGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
}

func TokenizeTypescript(source string) ([]lexer.Token, error) {
	typescriptLexer, err := CreateTypescriptLexer(source)
	if err != nil {
		return nil, err
	}
	return typescriptLexer.Tokenize(), nil
}

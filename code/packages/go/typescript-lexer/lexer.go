package typescriptlexer

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
	return filepath.Join(root, "typescript.tokens")
}

func CreateTypescriptLexer(source string) (*lexer.GrammarLexer, error) {
	bytes, err := os.ReadFile(getGrammarPath())
	if err != nil {
		return nil, err
	}
	grammar, err := grammartools.ParseTokenGrammar(string(bytes))
	if err != nil {
		return nil, err
	}
	return lexer.NewGrammarLexer(source, grammar), nil
}

func TokenizeTypescript(source string) ([]lexer.Token, error) {
	typescriptLexer, err := CreateTypescriptLexer(source)
	if err != nil {
		return nil, err
	}
	return typescriptLexer.Tokenize(), nil
}

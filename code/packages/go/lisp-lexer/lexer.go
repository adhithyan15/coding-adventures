package lisplexer

import (
	"os"
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func grammarRoot() string {
	_, filename, _, _ := runtime.Caller(0)
	return filepath.Join(filepath.Dir(filename), "..", "..", "..", "grammars")
}

func getTokensPath() string {
	return filepath.Join(grammarRoot(), "lisp.tokens")
}

// CreateLispLexer constructs a GrammarLexer configured for Lisp source.
func CreateLispLexer(source string) (*lexer.GrammarLexer, error) {
	bytes, err := os.ReadFile(getTokensPath())
	if err != nil {
		return nil, err
	}
	grammar, err := grammartools.ParseTokenGrammar(string(bytes))
	if err != nil {
		return nil, err
	}
	return lexer.NewGrammarLexer(source, grammar), nil
}

// TokenizeLisp tokenizes Lisp source with the repository grammar.
func TokenizeLisp(source string) ([]lexer.Token, error) {
	lispLexer, err := CreateLispLexer(source)
	if err != nil {
		return nil, err
	}
	return lispLexer.Tokenize(), nil
}

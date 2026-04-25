package lispparser

import (
	"os"
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	lisplexer "github.com/adhithyan15/coding-adventures/code/packages/go/lisp-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

func grammarRoot() string {
	_, filename, _, _ := runtime.Caller(0)
	return filepath.Join(filepath.Dir(filename), "..", "..", "..", "grammars")
}

func getGrammarPath() string {
	return filepath.Join(grammarRoot(), "lisp.grammar")
}

// CreateLispParser constructs a GrammarParser configured for Lisp source.
func CreateLispParser(source string) (*parser.GrammarParser, error) {
	tokens, err := lisplexer.TokenizeLisp(source)
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
	return parser.NewGrammarParser(tokens, grammar), nil
}

// ParseLisp parses Lisp source with the repository grammar.
func ParseLisp(source string) (*parser.ASTNode, error) {
	lispParser, err := CreateLispParser(source)
	if err != nil {
		return nil, err
	}
	return lispParser.Parse()
}

package rubyparser

import (
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/ruby-lexer"
)

func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "ruby.grammar")
}

func CreateRubyParser(source string) (*parser.GrammarParser, error) {
	tokens, err := rubylexer.TokenizeRuby(source)
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

func ParseRuby(source string) (*parser.ASTNode, error) {
	rubyParser, err := CreateRubyParser(source)
	if err != nil {
		return nil, err
	}
	return rubyParser.Parse()
}

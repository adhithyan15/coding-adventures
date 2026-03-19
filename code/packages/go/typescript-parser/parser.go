package typescriptparser

import (
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/typescript-lexer"
)

func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "typescript.grammar")
}

func CreateTypescriptParser(source string) (*parser.GrammarParser, error) {
	tokens, err := typescriptlexer.TokenizeTypescript(source)
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

func ParseTypescript(source string) (*parser.ASTNode, error) {
	typescriptParser, err := CreateTypescriptParser(source)
	if err != nil {
		return nil, err
	}
	return typescriptParser.Parse()
}

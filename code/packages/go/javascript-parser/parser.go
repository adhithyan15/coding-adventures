package javascriptparser

import (
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/javascript-lexer"
)

func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "javascript.grammar")
}

func CreateJavascriptParser(source string) (*parser.GrammarParser, error) {
	tokens, err := javascriptlexer.TokenizeJavascript(source)
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

func ParseJavascript(source string) (*parser.ASTNode, error) {
	javascriptParser, err := CreateJavascriptParser(source)
	if err != nil {
		return nil, err
	}
	return javascriptParser.Parse()
}

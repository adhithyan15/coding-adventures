package pythonparser

import (
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/python-lexer"
)

func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "python.grammar")
}

func CreatePythonParser(source string) (*parser.GrammarParser, error) {
	tokens, err := pythonlexer.TokenizePython(source)
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

func ParsePython(source string) (*parser.ASTNode, error) {
	pythonParser, err := CreatePythonParser(source)
	if err != nil {
		return nil, err
	}
	return pythonParser.Parse()
}

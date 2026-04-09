package pythonparser

import (
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
	tokens, err := pythonlexer.TokenizePython(source, "")
	if err != nil {
		return nil, err
	}
	return StartNew[*parser.GrammarParser]("pythonparser.CreatePythonParser", nil,
		func(op *Operation[*parser.GrammarParser], rf *ResultFactory[*parser.GrammarParser]) *OperationResult[*parser.GrammarParser] {
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}
			grammar, err := grammartools.ParseParserGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, parser.NewGrammarParser(tokens, grammar))
		}).GetResult()
}

func ParsePython(source string) (*parser.ASTNode, error) {
	pythonParser, err := CreatePythonParser(source)
	if err != nil {
		return nil, err
	}
	return pythonParser.Parse()
}

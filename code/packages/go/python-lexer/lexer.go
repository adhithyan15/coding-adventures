package pythonlexer

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
	return filepath.Join(root, "python.tokens")
}

func CreatePythonLexer(source string) (*lexer.GrammarLexer, error) {
	return StartNew[*lexer.GrammarLexer]("pythonlexer.CreatePythonLexer", nil,
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

func TokenizePython(source string) ([]lexer.Token, error) {
	pythonLexer, err := CreatePythonLexer(source)
	if err != nil {
		return nil, err
	}
	return pythonLexer.Tokenize(), nil
}

package typescriptparser

import (
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
	return StartNew[*parser.GrammarParser]("typescriptparser.CreateTypescriptParser", nil,
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

func ParseTypescript(source string) (*parser.ASTNode, error) {
	typescriptParser, err := CreateTypescriptParser(source)
	if err != nil {
		return nil, err
	}
	return typescriptParser.Parse()
}

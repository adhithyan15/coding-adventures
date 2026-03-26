package javascriptparser

import (
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
	return StartNew[*parser.GrammarParser]("javascriptparser.CreateJavascriptParser", nil,
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

func ParseJavascript(source string) (*parser.ASTNode, error) {
	javascriptParser, err := CreateJavascriptParser(source)
	if err != nil {
		return nil, err
	}
	return javascriptParser.Parse()
}

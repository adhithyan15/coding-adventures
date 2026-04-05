// Package ecmascriptes5parser parses ECMAScript 5 (2009) source code into ASTs.
//
// # What Is This Package?
//
// This is a thin wrapper around the grammar-driven parser engine. It reads
// the ES5 grammar definition from code/grammars/ecmascript/es5.grammar,
// tokenizes the source using the ES5 lexer, and produces an AST.
//
// # ES5 Grammar Additions Over ES3
//
// The ES5 grammar adds:
//   - debugger statement (`debugger;`)
//   - Getter/setter properties in object literals
//     (`{ get name() {}, set name(v) {} }`)
//
// The grammar is otherwise identical to ES3. The major ES5 innovations
// (strict mode, JSON, property descriptors) are semantic, not syntactic.
//
// # Usage
//
//	ast, err := ecmascriptes5parser.ParseEs5("debugger;")
//	if err != nil {
//	    log.Fatal(err)
//	}
package ecmascriptes5parser

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"

	ecmascriptes5lexer "github.com/adhithyan15/coding-adventures/code/packages/go/ecmascript-es5-lexer"
)

// getGrammarPath resolves the absolute path to the ES5 parser grammar file.
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "ecmascript", "es5.grammar")
}

// CreateEs5Parser constructs a GrammarParser configured for ECMAScript 5.
//
// Tokenizes the source using the ES5 lexer (which recognizes `debugger`
// as a keyword and has ES3's ===, !==, try/catch, instanceof), then
// loads the ES5 grammar and creates a parser.
func CreateEs5Parser(source string) (*parser.GrammarParser, error) {
	tokens, err := ecmascriptes5lexer.TokenizeEs5(source)
	if err != nil {
		return nil, err
	}

	return StartNew[*parser.GrammarParser]("ecmascriptes5parser.CreateEs5Parser", nil,
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

// ParseEs5 parses ECMAScript 5 source code into an AST in a single call.
//
// Example:
//
//	ast, err := ecmascriptes5parser.ParseEs5("debugger;")
//	// ast.RuleName == "program"
func ParseEs5(source string) (*parser.ASTNode, error) {
	es5Parser, err := CreateEs5Parser(source)
	if err != nil {
		return nil, err
	}
	return es5Parser.Parse()
}

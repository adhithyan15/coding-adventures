// Package ecmascriptes3parser parses ECMAScript 3 (1999) source code into ASTs.
//
// # What Is This Package?
//
// This is a thin wrapper around the grammar-driven parser engine. It reads
// the ES3 grammar definition from code/grammars/ecmascript/es3.grammar,
// tokenizes the source using the ES3 lexer, and produces an AST.
//
// # ES3 Grammar Additions Over ES1
//
// The ES3 grammar adds:
//   - try/catch/finally/throw statements (structured error handling)
//   - === and !== in equality expressions (strict equality)
//   - instanceof in relational expressions
//   - REGEX as a primary expression
//
// # Usage
//
//	ast, err := ecmascriptes3parser.ParseEs3("try { x === 1; } catch (e) {}")
//	if err != nil {
//	    log.Fatal(err)
//	}
package ecmascriptes3parser

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"

	ecmascriptes3lexer "github.com/adhithyan15/coding-adventures/code/packages/go/ecmascript-es3-lexer"
)

// getGrammarPath resolves the absolute path to the ES3 parser grammar file.
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "ecmascript", "es3.grammar")
}

// CreateEs3Parser constructs a GrammarParser configured for ECMAScript 3.
//
// Tokenizes the source using the ES3 lexer (which supports ===, !==,
// try/catch/finally/throw, instanceof, and regex literals), then loads
// the ES3 grammar and creates a parser.
func CreateEs3Parser(source string) (*parser.GrammarParser, error) {
	tokens, err := ecmascriptes3lexer.TokenizeEs3(source)
	if err != nil {
		return nil, err
	}

	return StartNew[*parser.GrammarParser]("ecmascriptes3parser.CreateEs3Parser", nil,
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

// ParseEs3 parses ECMAScript 3 source code into an AST in a single call.
//
// Example:
//
//	ast, err := ecmascriptes3parser.ParseEs3("try { x === 1; } catch (e) {}")
//	// ast.RuleName == "program"
func ParseEs3(source string) (*parser.ASTNode, error) {
	es3Parser, err := CreateEs3Parser(source)
	if err != nil {
		return nil, err
	}
	return es3Parser.Parse()
}

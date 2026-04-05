// Package ecmascriptes1parser parses ECMAScript 1 (1997) source code into ASTs.
//
// # What Is This Package?
//
// This is a thin wrapper around the grammar-driven parser engine. It reads
// the ES1 grammar definition from code/grammars/ecmascript/es1.grammar,
// tokenizes the source using the ES1 lexer, and produces an abstract syntax
// tree (AST) according to the ES1 grammar rules.
//
// # How It Works
//
// Parsing happens in two phases:
//
//  1. Lexing: The source code is tokenized by the ES1 lexer (ecmascript-es1-lexer).
//     This produces a flat list of tokens: keywords, operators, literals, etc.
//
//  2. Parsing: The token list is fed into the generic GrammarParser along with
//     the ES1 grammar rules. The parser uses PEG semantics with packrat
//     memoization to build an AST. Each node in the AST corresponds to a
//     grammar rule (program, statement, expression, etc.).
//
// # ES1 Grammar Overview
//
// The ES1 grammar supports:
//   - Variable declarations (var only — no let/const)
//   - Function declarations and expressions
//   - Control flow: if/else, while, do-while, for, for-in, switch
//   - Expressions: arithmetic, comparison, logical, bitwise, ternary
//   - Object and array literals
//   - Property access (dot and bracket notation)
//
// The ES1 grammar does NOT support:
//   - try/catch/finally/throw (added in ES3)
//   - === and !== operators (added in ES3)
//   - instanceof operator (added in ES3)
//   - debugger statement (added in ES5)
//
// # Usage
//
//	ast, err := ecmascriptes1parser.ParseEs1("var x = 1 + 2;")
//	if err != nil {
//	    log.Fatal(err)
//	}
//	// ast.RuleName == "program"
package ecmascriptes1parser

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"

	ecmascriptes1lexer "github.com/adhithyan15/coding-adventures/code/packages/go/ecmascript-es1-lexer"
)

// getGrammarPath resolves the absolute path to the ES1 parser grammar file.
// Uses runtime.Caller(0) to anchor the path relative to this source file,
// ensuring correct resolution regardless of working directory.
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "ecmascript", "es1.grammar")
}

// CreateEs1Parser constructs a GrammarParser configured for ECMAScript 1.
//
// The returned parser is ready to parse the token stream produced from the
// provided source string. It first tokenizes the source using the ES1 lexer,
// then loads the ES1 grammar rules and creates a parser instance.
//
// File system access is restricted by the capability cage to only the
// ES1 grammar file declared in required_capabilities.json.
func CreateEs1Parser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source using the ES1 lexer.
	// This produces a flat list of tokens according to ES1 lexical rules
	// (no ===, no try/catch, no regex literals).
	tokens, err := ecmascriptes1lexer.TokenizeEs1(source)
	if err != nil {
		return nil, err
	}

	// Step 2: Load the grammar and create the parser.
	return StartNew[*parser.GrammarParser]("ecmascriptes1parser.CreateEs1Parser", nil,
		func(op *Operation[*parser.GrammarParser], rf *ResultFactory[*parser.GrammarParser]) *OperationResult[*parser.GrammarParser] {
			// Read the ES1 grammar file from disk.
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Parse the grammar definition into structured form.
			grammar, err := grammartools.ParseParserGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Create a parser with the token stream and grammar rules.
			return rf.Generate(true, false, parser.NewGrammarParser(tokens, grammar))
		}).GetResult()
}

// ParseEs1 is a convenience function that parses ECMAScript 1 source code
// in a single call.
//
// It tokenizes the source, creates a parser, and runs it to completion,
// returning the root AST node. The root node always has RuleName "program".
//
// Example:
//
//	ast, err := ecmascriptes1parser.ParseEs1("var x = 1;")
//	// ast.RuleName == "program"
func ParseEs1(source string) (*parser.ASTNode, error) {
	es1Parser, err := CreateEs1Parser(source)
	if err != nil {
		return nil, err
	}
	return es1Parser.Parse()
}

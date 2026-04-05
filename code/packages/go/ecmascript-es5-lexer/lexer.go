// Package ecmascriptes5lexer tokenizes ECMAScript 5 (2009) source code.
//
// # What Is ECMAScript 5?
//
// ES5 landed in December 2009 — a full decade after ES3 (ES4 was abandoned
// after years of debate). The syntactic changes are modest: the real
// innovations were strict mode semantics, native JSON support, and property
// descriptors. Lexically, the main change is that `debugger` moves from
// future-reserved to a full keyword.
//
// # How This Lexer Works
//
// This package is a thin wrapper around the grammar-driven lexer engine.
// It reads the token grammar file at code/grammars/ecmascript/es5.tokens.
//
// # What ES5 Adds Over ES3
//
//   - `debugger` keyword (promoted from future-reserved in ES3)
//   - String line continuation (backslash before newline)
//   - Trailing commas in object literals
//   - Getter/setter syntax in object literals (grammar-level, not lexer-level)
//   - Reduced future-reserved word list (many ES3 reserved words freed)
//
// # What ES5 Does NOT Have
//
//   - No let/const (added in ES2015)
//   - No class syntax (added in ES2015)
//   - No arrow functions (added in ES2015)
//   - No template literals (added in ES2015)
//   - No modules (added in ES2015)
//
// # Usage
//
//	tokens, err := ecmascriptes5lexer.TokenizeEs5("debugger;")
//	if err != nil {
//	    log.Fatal(err)
//	}
package ecmascriptes5lexer

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// getGrammarPath resolves the absolute path to the ES5 token grammar file.
// Uses runtime.Caller(0) to anchor the path relative to this source file.
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "ecmascript", "es5.tokens")
}

// CreateEs5Lexer constructs a GrammarLexer configured for ECMAScript 5.
//
// The returned lexer supports all ES3 token types plus:
//   - `debugger` as a keyword (not just a future-reserved word)
//   - Reduced future-reserved word set compared to ES3
//
// File system access is restricted by the capability cage.
func CreateEs5Lexer(source string) (*lexer.GrammarLexer, error) {
	return StartNew[*lexer.GrammarLexer]("ecmascriptes5lexer.CreateEs5Lexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			// Step 1: Read the ES5 token grammar from disk.
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 2: Parse the grammar definition.
			grammar, err := grammartools.ParseTokenGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 3: Create the lexer.
			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
}

// TokenizeEs5 tokenizes ECMAScript 5 source code in a single call.
//
// Returns the full list of tokens ending with EOF.
//
// Example:
//
//	tokens, err := ecmascriptes5lexer.TokenizeEs5("debugger;")
//	// tokens: [KEYWORD("debugger"), SEMICOLON(";"), EOF("")]
func TokenizeEs5(source string) ([]lexer.Token, error) {
	es5Lexer, err := CreateEs5Lexer(source)
	if err != nil {
		return nil, err
	}
	return es5Lexer.Tokenize(), nil
}

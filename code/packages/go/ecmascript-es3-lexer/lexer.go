// Package ecmascriptes3lexer tokenizes ECMAScript 3 (1999) source code.
//
// # What Is ECMAScript 3?
//
// ES3 was published in December 1999 and is the version that made JavaScript
// a real, complete language. It added features that developers today consider
// fundamental: regular expressions, error handling, and strict equality.
//
// # How This Lexer Works
//
// This package is a thin wrapper around the grammar-driven lexer engine.
// It reads the token grammar file at code/grammars/ecmascript/es3.tokens,
// which defines every token pattern for the ES3 language.
//
// # What ES3 Adds Over ES1
//
//   - === and !== (strict equality — no type coercion)
//   - try/catch/finally/throw (structured error handling)
//   - Regular expression literals (/pattern/flags)
//   - `instanceof` operator
//   - Expanded future-reserved words (abstract, boolean, byte, etc.)
//
// # Regex vs Division Ambiguity
//
// The `/` character is ambiguous in JavaScript: it could start a regex
// literal or be the division operator. The grammar file defines REGEX
// as a token, but context-sensitive disambiguation (deciding whether `/`
// starts a regex or is division) would require the previousToken()
// callback mechanism described in the lexer-parser-extensions spec.
//
// # Usage
//
//	tokens, err := ecmascriptes3lexer.TokenizeEs3(`var x = /pattern/gi;`)
//	if err != nil {
//	    log.Fatal(err)
//	}
package ecmascriptes3lexer

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// getGrammarPath resolves the absolute path to the ES3 token grammar file.
// Uses runtime.Caller(0) to anchor the path relative to this source file,
// ensuring correct resolution regardless of working directory.
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "ecmascript", "es3.tokens")
}

// CreateEs3Lexer constructs a GrammarLexer configured for ECMAScript 3.
//
// The returned lexer is ready to tokenize the provided source string
// according to ES3 lexical rules, including support for:
//   - Strict equality operators (=== and !==)
//   - try/catch/finally/throw keywords
//   - instanceof keyword
//   - Regular expression literals
//
// File system access is restricted by the capability cage to only the
// ES3 grammar file declared in required_capabilities.json.
func CreateEs3Lexer(source string) (*lexer.GrammarLexer, error) {
	return StartNew[*lexer.GrammarLexer]("ecmascriptes3lexer.CreateEs3Lexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			// Step 1: Read the ES3 token grammar from disk.
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 2: Parse the grammar definition into structured form.
			grammar, err := grammartools.ParseTokenGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 3: Create and return the lexer.
			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
}

// TokenizeEs3 tokenizes ECMAScript 3 source code in a single call.
//
// Returns the full list of tokens ending with EOF.
//
// Example:
//
//	tokens, err := ecmascriptes3lexer.TokenizeEs3("try { x === 1; } catch (e) {}")
//	// tokens include: KEYWORD("try"), LBRACE, NAME("x"), STRICT_EQUALS("==="), ...
func TokenizeEs3(source string) ([]lexer.Token, error) {
	es3Lexer, err := CreateEs3Lexer(source)
	if err != nil {
		return nil, err
	}
	return es3Lexer.Tokenize(), nil
}

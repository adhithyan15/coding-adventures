// Package ecmascriptes1lexer tokenizes ECMAScript 1 (1997) source code.
//
// # What Is ECMAScript 1?
//
// ECMAScript 1 was the very first standardized version of JavaScript,
// published by ECMA International in June 1997. Brendan Eich created the
// language for Netscape Navigator in 1995; two years later, this spec
// formalized what "JavaScript" actually meant.
//
// # How This Lexer Works
//
// This package is a thin wrapper around the grammar-driven lexer engine.
// It reads the token grammar file at code/grammars/ecmascript/es1.tokens,
// which defines every token pattern (keywords, operators, literals, etc.)
// for the ES1 language. The generic GrammarLexer handles the actual
// tokenization — this package just wires up the right grammar file.
//
// # Key ES1 Lexical Features
//
//   - 23 keywords: break, case, continue, default, delete, do, else, for,
//     function, if, in, new, return, switch, this, typeof, var, void, while,
//     with, true, false, null
//   - No === or !== (strict equality was added in ES3)
//   - No try/catch/finally/throw (error handling was added in ES3)
//   - No regex literals (implementation-defined in ES1, formalized in ES3)
//   - The $ character is valid in identifiers (unusual for 1997)
//
// # Usage
//
//	tokens, err := ecmascriptes1lexer.TokenizeEs1("var x = 1 + 2;")
//	if err != nil {
//	    log.Fatal(err)
//	}
//	for _, tok := range tokens {
//	    fmt.Printf("%s %q\n", tok.Type, tok.Value)
//	}
package ecmascriptes1lexer

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// getGrammarPath resolves the absolute path to the ES1 token grammar file.
//
// Why runtime.Caller? Because Go packages can be imported from any working
// directory. We need the path relative to THIS source file, not relative
// to wherever `go test` or `go run` happens to be invoked from. The
// runtime.Caller(0) trick gives us the absolute path of this .go file,
// and we navigate from there to the grammar directory.
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "ecmascript", "es1.tokens")
}

// CreateEs1Lexer constructs a GrammarLexer configured for ECMAScript 1.
//
// The returned lexer is ready to tokenize the provided source string.
// It reads the ES1 token grammar from disk, parses the grammar definition,
// and creates a lexer instance that will match tokens according to ES1 rules.
//
// This function uses the Operation pattern (capability cage) to ensure
// file system access is limited to only the grammar file declared in
// required_capabilities.json.
func CreateEs1Lexer(source string) (*lexer.GrammarLexer, error) {
	return StartNew[*lexer.GrammarLexer]("ecmascriptes1lexer.CreateEs1Lexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			// Step 1: Read the grammar file from disk.
			// The Operation's File capability restricts us to only the paths
			// declared in required_capabilities.json — any other path would
			// cause a capability violation error.
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 2: Parse the token grammar definition.
			// The grammar file uses a custom format where each line defines
			// a token pattern (regex or literal) or a keyword/reserved word.
			grammar, err := grammartools.ParseTokenGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 3: Create the lexer with the parsed grammar.
			// The GrammarLexer will use the patterns from the grammar to
			// match and classify tokens in the source code.
			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
}

// TokenizeEs1 is a convenience function that tokenizes ECMAScript 1 source
// code in a single call.
//
// It creates an ES1 lexer and immediately runs it to completion, returning
// the full list of tokens. This is the simplest way to lex ES1 code when
// you don't need incremental/streaming tokenization.
//
// The returned token slice always ends with an EOF token.
//
// Example:
//
//	tokens, err := ecmascriptes1lexer.TokenizeEs1("var x = 42;")
//	// tokens: [KEYWORD("var"), NAME("x"), EQUALS("="), NUMBER("42"), SEMICOLON(";"), EOF("")]
func TokenizeEs1(source string) ([]lexer.Token, error) {
	es1Lexer, err := CreateEs1Lexer(source)
	if err != nil {
		return nil, err
	}
	return es1Lexer.Tokenize(), nil
}

// Package lisplexer tokenizes Lisp source using a grammar-driven lexer.
//
// Lisp's surface syntax is famously minimal: parentheses for lists, a handful
// of reader macros (quote, quasiquote, unquote), the dotted-pair marker, and
// atoms (symbols, numbers, strings). There are no reserved keywords at the
// lexical level — "define" and "lambda" are ordinary SYMBOL tokens that the
// parser and evaluator give meaning to.
//
// This package is a thin wrapper around the generic grammar-driven lexer. It:
//   1. Uses the Lisp token grammar embedded at compile time (grammar_data.go)
//   2. Passes it to the GrammarLexer, which compiles the regex patterns
//   3. The GrammarLexer handles skip patterns (whitespace, comments)
//      automatically based on the grammar definition
//
// The lisp.tokens grammar file defines tokens such as:
//   - LPAREN, RPAREN: list delimiters ( )
//   - QUOTE and related reader macros
//   - DOT: the dotted-pair marker .
//   - SYMBOL: identifiers and operators (define, +, *, x, ...)
//   - NUMBER: integer and signed numeric literals
//   - STRING: double-quoted strings with escape sequences
//   - skip: whitespace and line comments (; ...)
//
// Usage:
//
//   // One-shot tokenization: Lisp source in, token slice out
//   tokens, err := lisplexer.TokenizeLisp("(define x 42)")
//
//   // Or create a reusable lexer for more control
//   lex, err := lisplexer.CreateLispLexer("(+ 1 2)")
//   tokens := lex.Tokenize()
package lisplexer

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateLispLexer returns a GrammarLexer configured with the Lisp token
// grammar, ready to tokenize the given Lisp source.
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData); nothing is read from disk at run time. The lexer works
// unchanged when the package is built standalone and needs no filesystem
// capability. The error result is retained for API compatibility and is
// always nil.
func CreateLispLexer(source string) (*lexer.GrammarLexer, error) {
	return lexer.NewGrammarLexer(source, TokenGrammarData), nil
}

// TokenizeLisp is a convenience function that tokenizes Lisp source in a single
// call. It creates a lexer, runs tokenization, and returns the resulting token
// slice.
func TokenizeLisp(source string) ([]lexer.Token, error) {
	lispLexer, err := CreateLispLexer(source)
	if err != nil {
		return nil, err
	}
	return lispLexer.Tokenize(), nil
}

// Package lispparser parses Lisp source into an Abstract Syntax Tree (AST).
//
// Lisp's grammar is small and deeply recursive: a program is a sequence of
// forms, where a form is an atom (symbol, number, string) or a parenthesized
// list of forms, optionally prefixed by a reader macro (quote) and optionally
// containing a dotted pair. This parser produces a generic AST using the
// grammar-driven parser engine.
//
// The parsing pipeline has two stages:
//
//   1. Lexing (lisp-lexer): Lisp source is tokenized into a stream of tokens.
//      The lexer recognizes parentheses, reader macros, the dotted-pair DOT,
//      symbols, numbers, and strings, and skips whitespace and comments.
//
//   2. Parsing (this package): The token stream is parsed according to the
//      lisp.grammar rules using recursive descent with backtracking and
//      packrat memoization.
//
// The grammar file (lisp.grammar) uses EBNF notation:
//   - UPPERCASE names reference tokens from the lexer (SYMBOL, NUMBER, ...)
//   - lowercase names reference grammar rules (can be recursive)
//   - { x } means zero or more repetitions
//   - [ x ] means optional
//   - | means alternation (ordered choice)
//
// Usage:
//
//   // One-shot parsing: Lisp source in, AST out
//   ast, err := lispparser.ParseLisp("(define x 42)")
//
//   // Or create a reusable parser for more control
//   p, err := lispparser.CreateLispParser("'(a b c)")
//   ast, err := p.Parse()
package lispparser

import (
	lisplexer "github.com/adhithyan15/coding-adventures/code/packages/go/lisp-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// CreateLispParser tokenizes the Lisp source using the Lisp lexer, then returns
// a GrammarParser configured with the Lisp parser grammar, ready to produce an
// AST.
//
// The parser grammar is embedded at compile time as native Go in grammar_data.go
// (ParserGrammarData); nothing is read from disk at run time, so the parser
// needs no filesystem capability and works when built standalone. The error
// result is retained for API compatibility; it is non-nil only when lexing fails.
func CreateLispParser(source string) (*parser.GrammarParser, error) {
	tokens, err := lisplexer.TokenizeLisp(source)
	if err != nil {
		return nil, err
	}
	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
}

// ParseLisp is a convenience function that parses Lisp source into an AST in a
// single call. It creates a parser, runs parsing, and returns the root AST node.
//
// Returns an error if lexing or parsing fails.
func ParseLisp(source string) (*parser.ASTNode, error) {
	lispParser, err := CreateLispParser(source)
	if err != nil {
		return nil, err
	}
	return lispParser.Parse()
}

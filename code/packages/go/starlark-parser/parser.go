// Package starlarkparser parses Starlark source code into an Abstract Syntax Tree (AST).
//
// Starlark is a deterministic subset of Python designed for configuration files,
// most notably used in Bazel BUILD files. This parser produces a generic AST
// using the grammar-driven parser engine.
//
// The parsing pipeline has two stages:
//
//   1. Lexing (starlark-lexer): Source code is tokenized into a stream of tokens.
//      The lexer handles indentation tracking, keyword recognition, comment
//      skipping, and reserved keyword rejection.
//
//   2. Parsing (this package): The token stream is parsed according to the
//      starlark.grammar rules using recursive descent with backtracking and
//      packrat memoization. The grammar defines Starlark's full syntax including:
//        - Statements: assignment, return, break, continue, pass, load
//        - Compound statements: if/elif/else, for, def
//        - Expressions: full precedence chain from lambda down to primary
//        - Comprehensions: list, dict, and generator comprehensions
//        - Function calls with positional, keyword, *args, **kwargs arguments
//
// The grammar file (starlark.grammar) uses EBNF notation:
//   - UPPERCASE names reference tokens from the lexer (NAME, NUMBER, STRING, etc.)
//   - lowercase names reference grammar rules (can be recursive)
//   - { x } means zero or more repetitions
//   - [ x ] means optional
//   - | means alternation (ordered choice)
//   - "lit" matches a keyword or literal token value
//
// Usage:
//
//   // One-shot parsing: source code in, AST out
//   ast, err := starlarkparser.ParseStarlark(`x = 1 + 2`)
//
//   // Or create a reusable parser for more control
//   p, err := starlarkparser.CreateStarlarkParser(`def f():\n    return 1\n`)
//   ast, err := p.Parse()
package starlarkparser

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	starlarklexer "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-lexer"
)

// CreateStarlarkParser tokenizes the source code using the Starlark lexer,
// then returns a GrammarParser configured with the Starlark parser grammar,
// ready to produce an AST.
//
// The two-step process:
//   1. TokenizeStarlark(source) -- produces a token stream with INDENT/DEDENT
//   2. Create a GrammarParser from the tokens and the embedded grammar
//
// The parser grammar is embedded at compile time as native Go in grammar_data.go
// (ParserGrammarData); nothing is read from disk at run time, so the parser
// needs no filesystem capability and works when built standalone. The error
// result is retained for API compatibility; it is non-nil only when lexing fails.
//
// The GrammarParser uses recursive descent with packrat memoization. Each
// grammar rule becomes a parsing function. The memoization cache ensures that
// no (rule, position) pair is computed more than once, giving O(n) parsing
// for most practical grammars.
func CreateStarlarkParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source using the Starlark lexer.
	// This produces tokens with INDENT/DEDENT for indentation,
	// KEYWORD for recognized keywords, and panics on reserved keywords.
	tokens, err := starlarklexer.TokenizeStarlark(source)
	if err != nil {
		return nil, err
	}

	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
}

// ParseStarlark is a convenience function that parses Starlark source code
// into an AST in a single call. It creates a parser, runs parsing, and
// returns the root AST node.
//
// The returned ASTNode tree mirrors the grammar structure:
//   - node.RuleName is the grammar rule that matched (e.g., "file", "statement",
//     "expression", "if_stmt", "def_stmt", etc.)
//   - node.Children contains child ASTNodes and lexer.Token leaves
//   - Leaf nodes wrap individual tokens (identifiers, literals, operators)
//
// Example AST for `x = 1 + 2`:
//
//   file
//     statement
//       simple_stmt
//         assign_stmt
//           expression_list
//             expression
//               atom: NAME("x")
//           assign_op: EQUALS("=")
//           expression_list
//             expression
//               arith
//                 atom: INT("1")
//                 PLUS("+")
//                 atom: INT("2")
//
// Returns an error if lexing or parsing fails.
func ParseStarlark(source string) (*parser.ASTNode, error) {
	starlarkParser, err := CreateStarlarkParser(source)
	if err != nil {
		return nil, err
	}
	return starlarkParser.Parse()
}

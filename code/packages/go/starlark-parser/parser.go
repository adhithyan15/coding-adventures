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
	"os"
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	starlarklexer "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-lexer"
)

// getGrammarPath computes the absolute path to the starlark.grammar file.
//
// This uses the same runtime.Caller(0) technique as the starlark-lexer package.
// We navigate up 3 levels from this source file to reach the code/ directory,
// then down into grammars/.
//
// Directory structure:
//   code/
//     grammars/
//       starlark.grammar   <-- this is what we want
//     packages/
//       go/
//         starlark-parser/
//           parser.go      <-- we are here (3 levels below code/)
func getGrammarPath() string {
	// runtime.Caller(0) returns the file path of this source file at runtime.
	_, filename, _, _ := runtime.Caller(0)

	// Get the directory containing this file
	parent := filepath.Dir(filename)

	// Navigate up 3 levels to code/, then down to grammars/
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	return filepath.Join(root, "starlark.grammar")
}

// CreateStarlarkParser tokenizes the source code using the Starlark lexer,
// then loads the Starlark parser grammar and returns a configured GrammarParser
// ready to produce an AST.
//
// The two-step process:
//   1. TokenizeStarlark(source) -- produces a token stream with INDENT/DEDENT
//   2. Load starlark.grammar and create a GrammarParser from the tokens
//
// The GrammarParser uses recursive descent with packrat memoization. Each
// grammar rule becomes a parsing function. The memoization cache ensures that
// no (rule, position) pair is computed more than once, giving O(n) parsing
// for most practical grammars.
//
// Returns an error if lexing fails or the grammar file cannot be read/parsed.
func CreateStarlarkParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source using the Starlark lexer.
	// This produces tokens with INDENT/DEDENT for indentation,
	// KEYWORD for recognized keywords, and panics on reserved keywords.
	tokens, err := starlarklexer.TokenizeStarlark(source)
	if err != nil {
		return nil, err
	}

	// Step 2: Read the parser grammar file.
	// This file defines the syntax rules in EBNF notation.
	bytes, err := os.ReadFile(getGrammarPath())
	if err != nil {
		return nil, err
	}

	// Step 3: Parse the grammar file into a structured ParserGrammar object.
	// This extracts all rules, each with a name and a body (a tree of
	// grammar elements: sequences, alternations, repetitions, etc.).
	grammar, err := grammartools.ParseParserGrammar(string(bytes))
	if err != nil {
		return nil, err
	}

	// Step 4: Create the grammar-driven parser.
	// This builds a rule lookup table and initializes the memoization cache.
	// The first rule in the grammar (typically "file") becomes the entry point.
	return parser.NewGrammarParser(tokens, grammar), nil
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

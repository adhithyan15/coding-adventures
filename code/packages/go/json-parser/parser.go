// Package jsonparser parses JSON text into an Abstract Syntax Tree (AST).
//
// JSON (RFC 8259) is a lightweight data interchange format. This parser
// produces a generic AST using the grammar-driven parser engine.
//
// The parsing pipeline has two stages:
//
//   1. Lexing (json-lexer): JSON text is tokenized into a stream of tokens.
//      The lexer handles string escape processing, number recognition, and
//      whitespace skipping.
//
//   2. Parsing (this package): The token stream is parsed according to the
//      json.grammar rules using recursive descent with backtracking and
//      packrat memoization. The grammar defines JSON's complete syntax:
//        - value: the entry point (object | array | STRING | NUMBER | TRUE | FALSE | NULL)
//        - object: { [pair {, pair}] }
//        - pair: STRING : value
//        - array: [ [value {, value}] ]
//
// The grammar file (json.grammar) uses EBNF notation:
//   - UPPERCASE names reference tokens from the lexer (STRING, NUMBER, etc.)
//   - lowercase names reference grammar rules (can be recursive)
//   - { x } means zero or more repetitions
//   - [ x ] means optional
//   - | means alternation (ordered choice)
//
// JSON's grammar is recursive: value references object and array, which
// reference value again. This mutual recursion allows arbitrarily deep
// nesting like [{"a": [1, {"b": 2}]}].
//
// Usage:
//
//   // One-shot parsing: JSON text in, AST out
//   ast, err := jsonparser.ParseJSON(`{"name": "Alice", "age": 30}`)
//
//   // Or create a reusable parser for more control
//   p, err := jsonparser.CreateJSONParser(`[1, 2, 3]`)
//   ast, err := p.Parse()
package jsonparser

import (
	jsonlexer "github.com/adhithyan15/coding-adventures/code/packages/go/json-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// CreateJSONParser tokenizes the JSON text using the JSON lexer, then returns a
// GrammarParser configured with the JSON parser grammar, ready to produce an AST.
//
// The parser grammar is embedded at compile time as native Go in grammar_data.go
// (ParserGrammarData); nothing is read from disk at run time, so the parser
// needs no filesystem capability and works when built standalone. The error
// result is retained for API compatibility; it is non-nil only when lexing fails.
func CreateJSONParser(source string) (*parser.GrammarParser, error) {
	tokens, err := jsonlexer.TokenizeJSON(source)
	if err != nil {
		return nil, err
	}
	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
}

// ParseJSON is a convenience function that parses JSON text into an AST in a
// single call. It creates a parser, runs parsing, and returns the root AST node.
//
// The returned ASTNode tree mirrors the grammar structure:
//   - node.RuleName is the grammar rule that matched (e.g., "value", "object",
//     "array", "pair")
//   - node.Children contains child ASTNodes and lexer.Token leaves
//   - Leaf nodes wrap individual tokens (strings, numbers, literals, delimiters)
//
// Example AST for `{"name": "Alice"}`:
//
//   value
//     object
//       LBRACE("{")
//       pair
//         STRING("name")
//         COLON(":")
//         value
//           STRING("Alice")
//       RBRACE("}")
//
// Returns an error if lexing or parsing fails.
func ParseJSON(source string) (*parser.ASTNode, error) {
	jsonParser, err := CreateJSONParser(source)
	if err != nil {
		return nil, err
	}
	return jsonParser.Parse()
}

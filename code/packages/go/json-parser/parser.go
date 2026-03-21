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
	"os"
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	jsonlexer "github.com/adhithyan15/coding-adventures/code/packages/go/json-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// getGrammarPath computes the absolute path to the json.grammar file.
//
// This uses the same runtime.Caller(0) technique as the json-lexer package.
// We navigate up 3 levels from this source file to reach the code/ directory,
// then down into grammars/.
//
// Directory structure:
//   code/
//     grammars/
//       json.grammar       <-- this is what we want
//     packages/
//       go/
//         json-parser/
//           parser.go      <-- we are here (3 levels below code/)
func getGrammarPath() string {
	// runtime.Caller(0) returns the file path of this source file at runtime.
	_, filename, _, _ := runtime.Caller(0)

	// Get the directory containing this file
	parent := filepath.Dir(filename)

	// Navigate up 3 levels to code/, then down to grammars/
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	return filepath.Join(root, "json.grammar")
}

// CreateJSONParser tokenizes the JSON text using the JSON lexer, then loads
// the JSON parser grammar and returns a configured GrammarParser ready to
// produce an AST.
//
// The two-step process:
//   1. TokenizeJSON(source) -- produces a token stream (no indentation tracking)
//   2. Load json.grammar and create a GrammarParser from the tokens
//
// The GrammarParser uses recursive descent with packrat memoization. Each
// grammar rule becomes a parsing function. The memoization cache ensures that
// no (rule, position) pair is computed more than once, giving O(n) parsing
// for most practical grammars.
//
// Returns an error if lexing fails or the grammar file cannot be read/parsed.
func CreateJSONParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source using the JSON lexer.
	// This produces tokens with STRING values unquoted and escape-processed,
	// NUMBER values as their original text, and literal TRUE/FALSE/NULL tokens.
	tokens, err := jsonlexer.TokenizeJSON(source)
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
	// The first rule in the grammar ("value") becomes the entry point.
	return parser.NewGrammarParser(tokens, grammar), nil
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

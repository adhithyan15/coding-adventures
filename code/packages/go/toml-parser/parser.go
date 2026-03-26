// Package tomlparser parses TOML text into an Abstract Syntax Tree (AST).
//
// TOML (Tom's Obvious Minimal Language, v1.0.0 — https://toml.io/en/v1.0.0)
// is a configuration file format designed to be easy to read. This parser
// produces a generic AST using the grammar-driven parser engine.
//
// The parsing pipeline has two stages:
//
//   1. Lexing (toml-lexer): TOML text is tokenized into a stream of tokens.
//      The lexer handles four string types, date/time literals, numbers
//      (including hex/oct/bin), booleans, bare keys, and structural tokens.
//      The lexer emits NEWLINE tokens between lines (TOML is newline-sensitive).
//      String quotes are stripped but escape sequences are left as raw text
//      (escapes: none mode).
//
//   2. Parsing (this package): The token stream is parsed according to the
//      toml.grammar rules using recursive descent with backtracking and
//      packrat memoization. The grammar defines TOML's complete syntax:
//        - document: the entry point (sequence of expressions separated by newlines)
//        - expression: table_header | array_table_header | keyval
//        - keyval: key EQUALS value
//        - key: simple_key { DOT simple_key }
//        - value: strings | numbers | booleans | dates | array | inline_table
//        - array: LBRACKET array_values RBRACKET
//        - inline_table: LBRACE [ keyval { COMMA keyval } ] RBRACE
//
// The grammar file (toml.grammar) uses EBNF notation:
//   - UPPERCASE names reference tokens from the lexer (BARE_KEY, INTEGER, etc.)
//   - lowercase names reference grammar rules (can be recursive)
//   - { x } means zero or more repetitions
//   - [ x ] means optional
//   - | means alternation (ordered choice)
//
// TOML's grammar is more complex than JSON's because:
//   - It has table headers ([server]) and array-of-tables ([[products]])
//   - Keys can be dotted (a.b.c = val)
//   - Simple keys include all value tokens (true, 42, etc. can be key names)
//   - Arrays allow newlines between elements (multi-line arrays)
//   - Newlines delimit expressions
//
// Semantic constraints NOT enforced by the grammar (handled by a post-parse
// validation pass):
//   - Key uniqueness within a table
//   - Table path consistency
//   - Inline table immutability
//   - Inline tables must be single-line (no NEWLINE tokens between braces)
//
// Usage:
//
//   // One-shot parsing: TOML text in, AST out
//   ast, err := tomlparser.ParseTOML(`[server]\nhost = "localhost"`)
//
//   // Or create a reusable parser for more control
//   p, err := tomlparser.CreateTOMLParser(`name = "TOML"`)
//   ast, err := p.Parse()
package tomlparser

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	tomllexer "github.com/adhithyan15/coding-adventures/code/packages/go/toml-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// getGrammarPath computes the absolute path to the toml.grammar file.
//
// This uses the same runtime.Caller(0) technique as the toml-lexer package.
// We navigate up 3 levels from this source file to reach the code/ directory,
// then down into grammars/.
//
// Directory structure:
//
//	code/
//	  grammars/
//	    toml.grammar       <-- this is what we want
//	  packages/
//	    go/
//	      toml-parser/
//	        parser.go      <-- we are here (3 levels below code/)
func getGrammarPath() string {
	// runtime.Caller(0) returns the file path of this source file at runtime.
	_, filename, _, _ := runtime.Caller(0)

	// Get the directory containing this file
	parent := filepath.Dir(filename)

	// Navigate up 3 levels to code/, then down to grammars/
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	return filepath.Join(root, "toml.grammar")
}

// CreateTOMLParser tokenizes the TOML text using the TOML lexer, then loads
// the TOML parser grammar and returns a configured GrammarParser ready to
// produce an AST.
//
// The two-step process:
//   1. TokenizeTOML(source) -- produces a token stream with NEWLINE tokens
//   2. Load toml.grammar and create a GrammarParser from the tokens
//
// The GrammarParser uses recursive descent with packrat memoization. Each
// grammar rule becomes a parsing function. The memoization cache ensures that
// no (rule, position) pair is computed more than once, giving O(n) parsing
// for most practical grammars.
//
// Returns an error if lexing fails or the grammar file cannot be read/parsed.
func CreateTOMLParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source using the TOML lexer.
	// This produces tokens with string quotes stripped (escape sequences left
	// as raw text), NEWLINE tokens between lines, and all TOML token types.
	tokens, err := tomllexer.TokenizeTOML(source)
	if err != nil {
		return nil, err
	}

	return StartNew[*parser.GrammarParser]("tomlparser.CreateTOMLParser", nil,
		func(op *Operation[*parser.GrammarParser], rf *ResultFactory[*parser.GrammarParser]) *OperationResult[*parser.GrammarParser] {
			// Step 2: Read the parser grammar file.
			// This file defines the syntax rules in EBNF notation.
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 3: Parse the grammar file into a structured ParserGrammar object.
			// This extracts all rules, each with a name and a body (a tree of
			// grammar elements: sequences, alternations, repetitions, etc.).
			grammar, err := grammartools.ParseParserGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 4: Create the grammar-driven parser.
			// This builds a rule lookup table and initializes the memoization cache.
			// The first rule in the grammar ("document") becomes the entry point.
			return rf.Generate(true, false, parser.NewGrammarParser(tokens, grammar))
		}).GetResult()
}

// ParseTOML is a convenience function that parses TOML text into an AST in a
// single call. It creates a parser, runs parsing, and returns the root AST node.
//
// The returned ASTNode tree mirrors the grammar structure:
//   - node.RuleName is the grammar rule that matched (e.g., "document",
//     "expression", "keyval", "value", "array", "inline_table")
//   - node.Children contains child ASTNodes and lexer.Token leaves
//   - Leaf nodes wrap individual tokens (strings, numbers, booleans, etc.)
//
// Example AST for `name = "TOML"`:
//
//	document
//	  expression
//	    keyval
//	      key
//	        simple_key
//	          BARE_KEY("name")
//	      EQUALS("=")
//	      value
//	        BASIC_STRING("TOML")
//
// Returns an error if lexing or parsing fails.
func ParseTOML(source string) (*parser.ASTNode, error) {
	tomlParser, err := CreateTOMLParser(source)
	if err != nil {
		return nil, err
	}
	return tomlParser.Parse()
}

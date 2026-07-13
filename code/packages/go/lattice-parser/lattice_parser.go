// Package latticeparser parses Lattice CSS superset source into an AST.
//
// # What This Package Does
//
// This package is a thin wrapper around the grammar-driven GrammarParser.
// It wires together three things:
//
//  1. Tokenization: calls lattice-lexer to produce a token stream.
//  2. Grammar loading: reads the lattice.grammar file from grammars/.
//  3. Parsing: passes both to GrammarParser, which runs recursive descent
//     with packrat memoization to produce a generic ASTNode tree.
//
// # The Grammar Structure
//
// The lattice.grammar defines the Lattice language as an extended CSS grammar.
// Its top-level rule is "stylesheet", which contains a sequence of "rule"
// nodes. Each rule is one of:
//
//   - lattice_rule: Lattice-specific constructs that produce no CSS output
//     (variable_declaration, mixin_definition, function_definition, use_directive)
//   - at_rule: CSS @-rules (@media, @import, @keyframes, etc.)
//   - qualified_rule: CSS selector + block (h1 { color: red; })
//
// # The AST Shape
//
// The returned AST mirrors the grammar. Each ASTNode has:
//
//   - RuleName: the grammar rule that matched (e.g., "stylesheet", "declaration")
//   - Children: []interface{} containing *ASTNode or lexer.Token values
//
// Token leaves carry the actual text values. For example:
//
//	stylesheet
//	  rule
//	    lattice_rule
//	      variable_declaration
//	        VARIABLE("$primary")
//	        COLON(":")
//	        value_list
//	          value
//	            HASH("#4a90d9")
//	        SEMICOLON(";")
//	  rule
//	    qualified_rule
//	      selector_list
//	        ...
//	      block
//	        ...
//
// # Lattice-Specific Grammar Rules
//
// Beyond standard CSS rules, the grammar adds:
//
//   variable_declaration:  $name: value;
//   mixin_definition:      @mixin name($params) { ... }
//   include_directive:     @include name(args); or @include name;
//   if_directive:          @if expr { } @else if expr { } @else { }
//   for_directive:         @for $i from N through M { }
//   each_directive:        @each $x in a, b, c { }
//   function_definition:   @function name($params) { @return expr; }
//   return_directive:      @return expr;
//   use_directive:         @use "file" as alias;
//
// # The Compiled-In Grammar
//
// The parser grammar is embedded at compile time as native Go in
// grammar_data.go (ParserGrammarData); nothing is read from disk at run time.
//
// Usage:
//
//	// One-shot parsing: Lattice source text → AST
//	ast, err := latticeparser.ParseLattice(`$color: red; h1 { color: $color; }`)
//
//	// Or create a reusable parser for more control
//	p, err := latticeparser.CreateLatticeParser(source)
//	ast, err := p.Parse()
package latticeparser

import (
	latticelexer "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// CreateLatticeParser tokenizes the Lattice source using the Lattice lexer,
// then returns a configured GrammarParser ready to produce an AST.
//
// The two-step process mirrors the Python reference implementation:
//  1. lattice-lexer.TokenizeLatticeLexer(source) → []lexer.Token
//  2. NewGrammarParser(tokens, ParserGrammarData)
//
// The GrammarParser uses recursive descent with packrat memoization.
// Packrat guarantees that no (rule, position) pair is parsed more than once,
// giving O(n × rules) worst-case time — effectively linear for practical grammars.
//
// The parser grammar is embedded at compile time as native Go in grammar_data.go
// (ParserGrammarData); nothing is read from disk at run time, so the parser
// needs no filesystem capability and works when built standalone. The error
// result is retained for API compatibility; it is non-nil only when lexing fails.
func CreateLatticeParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source using the Lattice lexer.
	// This handles all Lattice and CSS tokens including $variables and
	// comparison operators (==, !=, >=, <=).
	tokens, err := latticelexer.TokenizeLatticeLexer(source)
	if err != nil {
		return nil, err
	}

	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
}

// ParseLattice is the main entry point: parse Lattice source text and return
// an AST rooted at a "stylesheet" node.
//
// The returned ASTNode tree has this overall structure:
//
//	stylesheet
//	  rule*
//	    (lattice_rule | at_rule | qualified_rule)
//
// Where each "rule" wrapper contains exactly one child. Lattice constructs
// (variable_declaration, mixin_definition, etc.) appear under lattice_rule.
// Standard CSS rules appear under at_rule or qualified_rule.
//
// The AST-to-CSS compiler (lattice-ast-to-css package) takes this AST and
// produces a clean CSS AST by expanding all Lattice nodes.
//
// Returns an error if:
//   - The grammar file cannot be loaded (FileNotFoundError)
//   - The source has lexical errors (unknown characters)
//   - The source has syntax errors (grammar rule mismatch)
func ParseLattice(source string) (*parser.ASTNode, error) {
	p, err := CreateLatticeParser(source)
	if err != nil {
		return nil, err
	}
	return p.Parse()
}

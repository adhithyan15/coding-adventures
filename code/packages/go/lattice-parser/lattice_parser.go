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
// # Locating the Grammar File
//
// The lattice.grammar file lives in code/grammars/ at the repository root.
// We locate it using runtime.Caller(0) to find this source file, then
// navigate up three levels to code/, then down into grammars/.
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
	"os"
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	latticelexer "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// getGrammarPath computes the absolute path to the lattice.grammar file.
//
// We use runtime.Caller(0) to find this source file's directory at runtime,
// then navigate up three levels to reach the code/ root directory,
// then descend into grammars/.
//
// Directory structure:
//
//	code/
//	  grammars/
//	    lattice.grammar     ← what we want
//	  packages/
//	    go/
//	      lattice-parser/
//	        lattice_parser.go  ← we are here (3 levels below code/)
func getGrammarPath() string {
	// runtime.Caller(0) returns the path of this source file at compile time.
	// We navigate up from it at runtime to find the grammar file.
	_, filename, _, _ := runtime.Caller(0)
	dir := filepath.Dir(filename)
	root := filepath.Join(dir, "..", "..", "..", "grammars")
	return filepath.Join(root, "lattice.grammar")
}

// CreateLatticeParser tokenizes the Lattice source using the Lattice lexer,
// loads the lattice.grammar file, and returns a configured GrammarParser
// ready to produce an AST.
//
// The two-step process mirrors the Python reference implementation:
//  1. lattice-lexer.TokenizeLatticeLexer(source) → []lexer.Token
//  2. ParseParserGrammar(grammarText) + NewGrammarParser(tokens, grammar)
//
// The GrammarParser uses recursive descent with packrat memoization.
// Packrat guarantees that no (rule, position) pair is parsed more than once,
// giving O(n × rules) worst-case time — effectively linear for practical grammars.
//
// Returns an error if the grammar file cannot be read/parsed, or if the
// lattice-lexer fails (bad grammar file path).
func CreateLatticeParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source using the Lattice lexer.
	// This handles all Lattice and CSS tokens including $variables and
	// comparison operators (==, !=, >=, <=).
	tokens, err := latticelexer.TokenizeLatticeLexer(source)
	if err != nil {
		return nil, err
	}

	// Step 2: Read the parser grammar file from disk.
	// This file defines the full Lattice/CSS syntax in EBNF-like notation.
	bytes, err := os.ReadFile(getGrammarPath())
	if err != nil {
		return nil, err
	}

	// Step 3: Parse the grammar file into a structured ParserGrammar object.
	// This extracts all rules with their names and bodies (sequences,
	// alternations, repetitions, optionals, and literals).
	grammar, err := grammartools.ParseParserGrammar(string(bytes))
	if err != nil {
		return nil, err
	}

	// Step 4: Create the grammar-driven parser.
	// The parser builds a rule lookup table and initializes its memoization
	// cache. The first rule in the grammar ("stylesheet") becomes the entry
	// point when Parse() is called.
	return parser.NewGrammarParser(tokens, grammar), nil
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

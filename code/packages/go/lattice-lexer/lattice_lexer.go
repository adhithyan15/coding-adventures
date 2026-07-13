// Package latticelexer tokenizes Lattice CSS superset source text.
//
// # What Is Lattice?
//
// Lattice is a CSS superset language — every valid CSS file is also a valid
// Lattice file. Lattice adds compile-time constructs that CSS lacks:
//
//   - Variables ($color, $font-size) — name a value once, use it everywhere
//   - Mixins (@mixin / @include) — reusable blocks of declarations
//   - Control flow (@if / @else, @for, @each) — conditional and looping
//   - Functions (@function / @return) — computed values
//   - Modules (@use) — split your styles across files
//
// None of these constructs survive to the browser. The Lattice compiler
// (lattice-transpiler) expands them all at compile time into plain CSS.
//
// # This Package's Role
//
// This package is a thin wrapper around the grammar-driven GrammarLexer.
// It uses the compiled-in lattice token grammar (TokenGrammarData, embedded
// as native Go in grammar_data.go) and passes it to the GrammarLexer, which
// handles:
//
//   - Skip patterns (whitespace, // line comments, /* */ block comments)
//   - Token ordering (VARIABLE before IDENT, multi-char ops before single-char)
//   - Type aliases (STRING_DQ and STRING_SQ both emit as STRING)
//
// # Five New Token Types Versus CSS
//
// The lattice.tokens grammar adds exactly 5 tokens not found in css.tokens:
//
//	VARIABLE        $color, $font-size-lg     (CSS never uses $)
//	EQUALS_EQUALS   ==                        (equality comparison in @if)
//	NOT_EQUALS      !=                        (inequality comparison)
//	GREATER_EQUALS  >=                        (greater-or-equal)
//	LESS_EQUALS     <=                        (less-or-equal)
//
// All other Lattice constructs (@mixin, @if, @for, etc.) reuse the existing
// AT_KEYWORD token type. The grammar (not the lexer) distinguishes @mixin from
// @media by literal matching on the token's text value.
//
// # Single-Line Comments (a CSS Extension)
//
// CSS supports only block comments (/* ... */). Lattice also supports
// single-line comments (// to end of line). Both are skip patterns — they
// are consumed and produce no tokens.
//
// Usage:
//
//	// One-shot tokenization
//	tokens, err := latticelexer.TokenizeLatticeLexer("$color: #4a90d9;")
//
//	// Or create a reusable lexer
//	lex, err := latticelexer.CreateLatticeLexer("h1 { color: $primary; }")
//	tokens := lex.Tokenize()
package latticelexer

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateLatticeLexer loads the Lattice token grammar and returns a configured
// GrammarLexer ready to tokenize the given Lattice source text.
//
// The lexer handles all Lattice token types automatically:
//
//   - VARIABLE:        $name tokens (dollar + identifier)
//   - AT_KEYWORD:      @mixin, @if, @function, @media, @use, etc.
//   - EQUALS_EQUALS:   == (must be matched before EQUALS)
//   - NOT_EQUALS:      != (must be matched before BANG)
//   - GREATER_EQUALS:  >= (must be matched before GREATER)
//   - LESS_EQUALS:     <= (must be matched before LESS — note: no LESS token)
//   - DIMENSION:       16px, 2em, 1.5rem (number + unit letters)
//   - PERCENTAGE:      50%, 100%
//   - NUMBER:          42, 3.14, -1 (must come after DIMENSION/PERCENTAGE)
//   - HASH:            #4a90d9, #fff (colors and id selectors)
//   - STRING:          "hello", 'world' (both quote styles → STRING alias)
//   - FUNCTION:        rgb(, calc(, name( (identifier immediately followed by ()
//   - IDENT:           red, bold, sans-serif, display, etc.
//
// Skip patterns silently consume:
//   - // single-line comments (Lattice extension, not in CSS)
//   - /* block comments (standard CSS)
//   - Whitespace (spaces, tabs, carriage returns, newlines)
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData); nothing is read from disk at run time. The lexer works
// unchanged when the package is built standalone and needs no filesystem
// capability. The error result is retained for API compatibility and is
// always nil.
func CreateLatticeLexer(source string) (*lexer.GrammarLexer, error) {
	return lexer.NewGrammarLexer(source, TokenGrammarData), nil
}

// TokenizeLatticeLexer is a convenience function that tokenizes Lattice source
// text in a single call. It creates a lexer, runs tokenization, and returns
// the resulting token slice.
//
// The returned slice always ends with an EOF token. Skip patterns (whitespace,
// comments) produce no tokens — they are consumed silently.
//
// Example token stream for "$color: #4a90d9;":
//
//	VARIABLE("$color")
//	COLON(":")
//	HASH("#4a90d9")
//	SEMICOLON(";")
//	EOF("")
//
// Use CreateLatticeLexer when you need access to the lexer object itself
// (e.g., for setting on-token callbacks or multiple tokenization passes).
//
// Returns an error if the grammar file cannot be loaded.
func TokenizeLatticeLexer(source string) ([]lexer.Token, error) {
	latticeLexer, err := CreateLatticeLexer(source)
	if err != nil {
		return nil, err
	}
	return latticeLexer.Tokenize(), nil
}

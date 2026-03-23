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
// It loads the lattice.tokens grammar file from the repository's grammars/
// directory and passes it to the GrammarLexer, which handles:
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
	"os"
	"path/filepath"
	"runtime"
	"strings"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// getGrammarPath computes the absolute path to the lattice.tokens grammar file.
//
// We use runtime.Caller(0) to discover this source file's location at runtime,
// then navigate up three directory levels to reach the repository's code/
// root, then down into grammars/.
//
// Directory tree:
//
//	code/
//	  grammars/
//	    lattice.tokens     ← what we want
//	  packages/
//	    go/
//	      lattice-lexer/
//	        lattice_lexer.go  ← we are here (3 levels below code/)
//
// Using runtime.Caller is more robust than os.Getwd() because the working
// directory varies depending on whether you run `go test`, `go build`, or the
// build tool — all from different locations.
func getGrammarPath() string {
	// runtime.Caller(0) returns the absolute path of this source file.
	// The three discarded return values are: program counter, line number, ok.
	_, filename, _, _ := runtime.Caller(0)

	// filepath.Dir strips the filename, leaving the containing directory.
	dir := filepath.Dir(filename)

	// Navigate up 3 levels: lattice-lexer → go → packages → code,
	// then descend into grammars/.
	root := filepath.Join(dir, "..", "..", "..", "grammars")
	return filepath.Join(root, "lattice.tokens")
}

// stripErrorsSection removes the "errors:" section from a .tokens grammar
// text before it is passed to ParseTokenGrammar.
//
// The Go grammar-tools parser recognizes "errors:" as a reserved section name
// (to prevent user groups from using that name) but does not yet parse it as
// a section header. The "errors:" section defines token patterns that cause
// the lexer to emit a lex error (bad strings, bad URLs). Since error recovery
// is outside the scope of the Lattice compiler, we simply strip this section.
//
// This is safe because:
//   1. The lattice compiler rejects malformed source before emitting CSS.
//   2. Bad strings/URLs would already fail to parse, so the error tokens
//      are only useful for IDEs / syntax highlighters, not compilers.
//
// The function strips from the first "errors:" line to the next non-indented
// non-blank line (or EOF), whichever comes first.
func stripErrorsSection(grammarText string) string {
	lines := strings.Split(grammarText, "\n")
	result := make([]string, 0, len(lines))
	inErrors := false

	for _, line := range lines {
		stripped := strings.TrimSpace(line)

		// Detect the start of the errors: section
		if stripped == "errors:" {
			inErrors = true
			continue
		}

		// Exit the errors: section when we hit a non-blank, non-indented,
		// non-comment line that isn't the section header itself.
		if inErrors {
			// Stay in the section for blank lines, comments, and indented lines
			if stripped == "" || strings.HasPrefix(stripped, "#") {
				continue
			}
			if len(line) > 0 && (line[0] == ' ' || line[0] == '\t') {
				continue // still inside the errors: section
			}
			// Non-indented content — exit errors: section and keep this line
			inErrors = false
		}

		result = append(result, line)
	}

	return strings.Join(result, "\n")
}

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
// Returns an error if the grammar file cannot be read or parsed.
func CreateLatticeLexer(source string) (*lexer.GrammarLexer, error) {
	// Read the grammar file from disk.
	// This file is part of the repository at code/grammars/lattice.tokens.
	bytes, err := os.ReadFile(getGrammarPath())
	if err != nil {
		return nil, err
	}

	// Strip the "errors:" section before parsing. The Go grammar-tools parser
	// does not yet handle "errors:" as a section header (it is reserved but
	// not implemented). The errors section defines lex-error patterns for
	// bad strings/URLs — useful for IDEs but not needed for compilation.
	grammarText := stripErrorsSection(string(bytes))

	// Parse the grammar file into a structured TokenGrammar object.
	// ParseTokenGrammar extracts: token definitions (with regex patterns),
	// skip patterns (for whitespace/comments), and type aliases
	// (e.g., STRING_DQ -> STRING).
	grammar, err := grammartools.ParseTokenGrammar(grammarText)
	if err != nil {
		return nil, err
	}

	// Create the grammar-driven lexer with the Lattice token definitions.
	// The GrammarLexer compiles all regexes at construction time, so
	// repeated calls to Tokenize() are efficient.
	return lexer.NewGrammarLexer(source, grammar), nil
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

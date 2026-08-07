// Package algollexer tokenizes ALGOL 60 source text using a grammar-driven lexer.
//
// # ALGOL 60: The Language That Changed Everything
//
// ALGOL 60 (ALGOrithmic Language, 1960) was designed by an international committee
// including John Backus, Peter Naur, John McCarthy, and Edsger Dijkstra. It was
// the first language formally specified using BNF (Backus-Naur Form) — the notation
// used to describe virtually every programming language since.
//
// ALGOL 60 introduced concepts that are now considered fundamental:
//   - Block structure: begin...end blocks with lexical scoping
//   - Recursive procedures: functions that can call themselves
//   - The call stack: runtime activation records for nested calls
//   - Free-format source: indentation is for humans, not the compiler
//   - Formal grammar specification: a language described by its own syntax rules
//
// Every mainstream language today — C, Java, Python, Go, Rust — is an ALGOL
// descendant in spirit if not in syntax. C derives from BCPL via CPL via ALGOL.
// Pascal was a direct descendant. Simula (the first OOP language) extended ALGOL.
// Java copied Pascal. The lineage is clear.
//
// # ALGOL 60 Token Structure
//
// ALGOL 60's token set is deliberately minimal. Unlike C or Java, it uses words
// for boolean operators (and, or, not) instead of symbols. Assignment uses := to
// distinguish it from equality =. Exponentiation uses ** or ^ (no symbol existed
// on 1960s printers for ↑, the character used in the original report).
//
// Token categories:
//   - Keywords: begin end if then else for do step until while goto
//              switch procedure own array label value integer real boolean string
//              true false not and or impl eqv div mod
//   - Operators: := (ASSIGN), ** or ^ (POWER/CARET), <= >= != (relational),
//                + - * / = < > (arithmetic/comparison)
//   - Literals: INTEGER_LIT (digits), REAL_LIT (decimal/exponent), STRING_LIT ('quoted')
//   - Identifiers: IDENT (letter followed by letters/digits)
//   - Delimiters: ( ) [ ] ; , :
//
// Comment syntax is unique: the word "comment" followed by any text up to ";".
// This is a statement-level construct, not a line comment like // in C.
// Example: comment this explains the next line;
//
// # Grammar-Driven Approach
//
// This package is a thin wrapper around the generic grammar-driven lexer. It:
//  1. Loads the ALGOL token grammar from algol.tokens
//  2. Passes it to GrammarLexer, which compiles the regex patterns into a DFA
//  3. The GrammarLexer handles skip patterns (whitespace, comments) automatically
//
// The algol.tokens file encodes all of the above structure declaratively. Adding
// a new token type requires only an edit to algol.tokens — no code change here.
//
// Usage:
//
//	// One-shot tokenization: ALGOL source in, token slice out
//	tokens, err := algollexer.TokenizeAlgol("begin integer x; x := 42 end")
//
//	// Or create a reusable lexer for more control
//	lex, err := algollexer.CreateAlgolLexer("begin real pi; pi := 3.14159 end")
//	tokens := lex.Tokenize()
package algollexer

import (
	"fmt"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// validAlgolVersions is the set of ALGOL versions with an embedded grammar.
// ALGOL 60 is the only version we support; the version parameter is retained
// on the public API so callers can pass a version explicitly and receive a
// clear error for anything unrecognized.
var validAlgolVersions = map[string]bool{
	"algol60": true,
}

// resolveVersion normalizes the optional version argument. An empty string
// defaults to "algol60". It returns an error for any unrecognized version,
// preserving the historical unknown-version behavior even though the grammar
// is now compiled in rather than read from disk.
func resolveVersion(version string) (string, error) {
	if version == "" {
		version = "algol60"
	}
	if !validAlgolVersions[version] {
		return "", fmt.Errorf("unknown ALGOL version %q: valid versions are algol60", version)
	}
	return version, nil
}

// CreateAlgolLexer returns a GrammarLexer configured with the ALGOL 60 token
// grammar, ready to tokenize the given ALGOL 60 source text.
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData); nothing is read from disk at run time, so the lexer needs
// no filesystem capability and works unchanged when the package is built
// standalone. The returned lexer operates in default mode (no indentation
// tracking). ALGOL 60's whitespace is handled by skip patterns: spaces, tabs,
// carriage returns, and newlines are all consumed silently between tokens.
// Comments (the keyword "comment" through the next ";") are also consumed
// silently.
//
// An error is returned only when an unrecognized version is requested.
func CreateAlgolLexer(source string, version ...string) (*lexer.GrammarLexer, error) {
	effectiveVersion := ""
	if len(version) > 0 {
		effectiveVersion = version[0]
	}
	if _, err := resolveVersion(effectiveVersion); err != nil {
		return nil, err
	}
	return lexer.NewGrammarLexer(source, TokenGrammarData), nil
}

// TokenizeAlgol is a convenience function that tokenizes ALGOL 60 source text
// in a single call. It creates a lexer, runs tokenization, and returns the
// resulting token slice.
//
// The token stream includes all meaningful tokens (keywords, identifiers,
// literals, operators, delimiters) and a final EOF token. Whitespace and
// comments are silently consumed and do not appear in the output.
//
// Example token stream for "begin integer x; x := 42 end":
//
//	BEGIN("begin")
//	INTEGER("integer")
//	IDENT("x")
//	SEMICOLON(";")
//	IDENT("x")
//	ASSIGN(":=")
//	INTEGER_LIT("42")
//	END("end")
//	EOF("")
//
// Returns an error if the grammar file cannot be loaded.
func TokenizeAlgol(source string, version ...string) ([]lexer.Token, error) {
	algolLexer, err := CreateAlgolLexer(source, version...)
	if err != nil {
		return nil, err
	}
	tokens := algolLexer.Tokenize()
	for i := range tokens {
		if tokens[i].TypeName == "KEYWORD" {
			tokens[i].TypeName = strings.ToUpper(tokens[i].Value)
		}
	}
	return tokens, nil
}

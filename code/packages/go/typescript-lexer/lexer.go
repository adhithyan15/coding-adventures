// Package typescriptlexer tokenizes TypeScript source code using versioned
// grammars. It supports TypeScript 1.0, 2.0, 3.0, 4.0, 5.0, and 5.8, each with
// its own token grammar, plus a generic superset grammar for callers that do
// not care about a specific version.
//
// The grammars are embedded at compile time as native Go data structures in
// grammar_data.go (VersionedTokenGrammars, keyed by version string, plus
// TokenGrammarData for the generic path). Nothing is read from disk at run
// time, so the lexer needs no filesystem capability and works unchanged when
// the package is built standalone.
//
// Usage:
//
//	tokens, err := typescriptlexer.TokenizeTypescript(source, "ts5.8")
//	tokens, err := typescriptlexer.TokenizeTypescript(source, "")  // generic grammar
package typescriptlexer

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// Supported TypeScript version strings and the grammar each selects:
//
//	""     — generic superset grammar (TokenGrammarData); pre-0.2.0 behaviour
//	ts1.0  — TypeScript 1.0  (April 2014)     first public release
//	ts2.0  — TypeScript 2.0  (September 2016)  strict null checks era
//	ts3.0  — TypeScript 3.0  (July 2018)       project references era
//	ts4.0  — TypeScript 4.0  (August 2020)     variadic tuple types era
//	ts5.0  — TypeScript 5.0  (March 2023)      decorators era
//	ts5.8  — TypeScript 5.8  (February 2025)   latest stable
//
// The versioned keys are exactly the keys of VersionedTokenGrammars.

// CreateTypescriptLexer constructs a GrammarLexer ready to tokenise the given
// TypeScript source string.
//
// version selects the TypeScript grammar:
//   - ""      — generic grammar (TokenGrammarData); same as pre-0.2.0 behaviour
//   - "ts1.0" through "ts5.8" — versioned grammar from VersionedTokenGrammars
//
// The grammar is selected from the compiled-in grammar data; no grammar file is
// read at run time. An unrecognised non-empty version returns a descriptive
// error so that typos produce actionable messages.
func CreateTypescriptLexer(source string, version string) (*lexer.GrammarLexer, error) {
	if version == "" {
		return lexer.NewGrammarLexer(source, TokenGrammarData), nil
	}
	grammar, ok := VersionedTokenGrammars[version]
	if !ok {
		return nil, fmt.Errorf("unknown TypeScript version %q: valid versions are ts1.0, ts2.0, ts3.0, ts4.0, ts5.0, ts5.8", version)
	}
	return lexer.NewGrammarLexer(source, grammar), nil
}

// TokenizeTypescript is the main entry point for lexing TypeScript source code.
//
// It tokenises source using the grammar for the given TypeScript version and
// returns the flat token slice produced by the underlying GrammarLexer.
// Pass version="" to use the generic grammar, which covers the superset of all
// supported versions and is the best choice when version is unknown.
//
// Example — tokenise with the generic grammar:
//
//	tokens, err := TokenizeTypescript("let x = 1;", "")
//
// Example — tokenise with a specific version:
//
//	tokens, err := TokenizeTypescript("const x: string = 'hi';", "ts5.8")
func TokenizeTypescript(source string, version string) ([]lexer.Token, error) {
	typescriptLexer, err := CreateTypescriptLexer(source, version)
	if err != nil {
		return nil, err
	}
	return typescriptLexer.Tokenize(), nil
}

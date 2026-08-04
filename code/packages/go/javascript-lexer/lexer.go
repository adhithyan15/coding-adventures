package javascriptlexer

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateJavascriptLexer constructs a GrammarLexer ready to tokenise the given
// JavaScript source string.
//
// version selects the ECMAScript edition, each of which has its own token
// grammar capturing the exact token set for that edition:
//   - ""      — generic grammar (TokenGrammarData); same as pre-0.2.0 behaviour
//   - "es1", "es3", "es5" — classic ECMAScript editions
//   - "es2015" through "es2025" — modern ECMAScript yearly editions
//
// The token grammars are embedded at compile time as native Go in
// grammar_data.go: TokenGrammarData is the generic grammar and
// VersionedTokenGrammars maps each version string to its grammar. Nothing is
// read from disk at run time, so the lexer needs no filesystem capability and
// works when built standalone. A non-empty version with no embedded grammar
// (e.g. "es99") returns a descriptive error rather than silently succeeding.
func CreateJavascriptLexer(source string, version string) (*lexer.GrammarLexer, error) {
	if version == "" {
		return lexer.NewGrammarLexer(source, TokenGrammarData), nil
	}
	grammar, ok := VersionedTokenGrammars[version]
	if !ok {
		return nil, fmt.Errorf(
			"unknown JavaScript version %q: valid versions are es1, es3, es5, es2015–es2025",
			version,
		)
	}
	return lexer.NewGrammarLexer(source, grammar), nil
}

// TokenizeJavascript is the main entry point for lexing JavaScript source code.
//
// It tokenises source using the grammar for the given ECMAScript version and
// returns the flat token slice produced by the underlying GrammarLexer.
// Pass version="" to use the generic grammar, which covers the superset of all
// supported versions and is the best choice when version is unknown.
//
// Example — tokenise with the generic grammar:
//
//	tokens, err := TokenizeJavascript("let x = 1;", "")
//
// Example — tokenise with a specific version:
//
//	tokens, err := TokenizeJavascript("const x = 1;", "es2022")
func TokenizeJavascript(source string, version string) ([]lexer.Token, error) {
	javascriptLexer, err := CreateJavascriptLexer(source, version)
	if err != nil {
		return nil, err
	}
	return javascriptLexer.Tokenize(), nil
}

// Package javalexer tokenizes Java source code using versioned grammars.
// It supports Java 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, and 21, each with its
// own token grammar that captures the exact token set for that release.
//
// The grammars are embedded at compile time as native Go data structures in
// grammar_data.go (VersionedTokenGrammars, keyed by version string). Nothing
// is read from disk at run time, so the lexer needs no filesystem capability
// and works unchanged when the package is built standalone.
//
// Java's version numbering changed over the years. Early releases used a
// "1.x" scheme (Java 1.0 through Java 1.4). Starting with Java 5, Sun dropped
// the "1." prefix for marketing but kept it internally. From Java 9 onward the
// short form became the only official version number, and the release cadence
// shifted to every six months.
//
// The supported versions correspond to the embedded grammars:
//
//	1.0  — Java 1.0   (January 1996)   the original release
//	1.1  — Java 1.1   (February 1997)  inner classes, JDBC, RMI
//	1.4  — Java 1.4   (February 2002)  assertions, NIO, regex
//	5    — Java 5     (September 2004) generics, annotations, enums, autoboxing
//	7    — Java 7     (July 2011)      diamond operator, try-with-resources
//	8    — Java 8     (March 2014)     lambdas, streams, default methods
//	10   — Java 10    (March 2018)     local-variable type inference (var)
//	14   — Java 14    (March 2020)     switch expressions, records (preview)
//	17   — Java 17    (September 2021) sealed classes, pattern matching (LTS)
//	21   — Java 21    (September 2023) virtual threads, record patterns (LTS)
package javalexer

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// DefaultVersion is the Java version used when no version is specified.
// Java 21 is the latest long-term support (LTS) release and the most
// widely deployed modern version.
const DefaultVersion = "21"

// CreateJavaLexer constructs a GrammarLexer ready to tokenise the given
// Java source string.
//
// version selects the Java grammar:
//   - ""     — uses DefaultVersion ("21"), the latest LTS release
//   - "1.0", "1.1", "1.4" — classic Java releases
//   - "5", "7", "8" — pre-modular Java releases
//   - "10", "14", "17", "21" — modern Java releases
//
// The grammar is selected from the compiled-in VersionedTokenGrammars map;
// no grammar file is read at run time. An error is returned only when the
// requested version has no embedded grammar.
func CreateJavaLexer(source string, version string) (*lexer.GrammarLexer, error) {
	// Default to the latest LTS version when no version is specified.
	if version == "" {
		version = DefaultVersion
	}
	grammar, ok := VersionedTokenGrammars[version]
	if !ok {
		return nil, fmt.Errorf(
			"unknown Java version %q: valid versions are 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, 21",
			version,
		)
	}
	return lexer.NewGrammarLexer(source, grammar), nil
}

// TokenizeJava is the main entry point for lexing Java source code.
//
// It tokenises source using the grammar for the given Java version and
// returns the flat token slice produced by the underlying GrammarLexer.
// Pass version="" to use the default grammar (Java 21), which is the best
// choice when version is unknown.
//
// Example — tokenise with the default grammar:
//
//	tokens, err := TokenizeJava("int x = 1;", "")
//
// Example — tokenise with a specific version:
//
//	tokens, err := TokenizeJava("var x = 1;", "10")
//
// Example — tokenise classic Java:
//
//	tokens, err := TokenizeJava("int x = 1;", "1.0")
func TokenizeJava(source string, version string) ([]lexer.Token, error) {
	javaLexer, err := CreateJavaLexer(source, version)
	if err != nil {
		return nil, err
	}
	return javaLexer.Tokenize(), nil
}

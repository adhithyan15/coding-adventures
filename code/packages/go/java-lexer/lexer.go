package javalexer

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// validVersions is the set of Java version strings the lexer recognises.
// Each maps to a versioned grammar file under code/grammars/java/.
//
// Java's version numbering changed over the years. Early releases used a
// "1.x" scheme (Java 1.0 through Java 1.4). Starting with Java 5, Sun
// dropped the "1." prefix for marketing but kept it internally. From Java 9
// onward the short form became the only official version number, and the
// release cadence shifted to every six months.
//
// The versions here correspond to the grammar files available in the
// code/grammars/java/ directory:
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
var validVersions = map[string]bool{
	"1.0": true,
	"1.1": true,
	"1.4": true,
	"5":   true,
	"7":   true,
	"8":   true,
	"10":  true,
	"14":  true,
	"17":  true,
	"21":  true,
}

// DefaultVersion is the Java version used when no version is specified.
// Java 21 is the latest long-term support (LTS) release and the most
// widely deployed modern version.
const DefaultVersion = "21"

// getGrammarPath resolves the absolute path to the .tokens grammar file for
// the given Java version string.
//
// When version is "" (empty string), the DefaultVersion ("21") is used.
// This provides a sensible default for callers that do not care about a
// specific version.
//
// Any other non-empty string must appear in validVersions; an unknown version
// returns a descriptive error so that typos produce actionable messages.
//
// The path is resolved relative to this source file using runtime.Caller(0),
// which ensures the grammar directory is found regardless of the working
// directory at runtime.
func getGrammarPath(version string) (string, error) {
	// runtime.Caller(0) returns the path to *this* source file at compile
	// time. We navigate up three directories to reach code/grammars/.
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	// Default to the latest LTS version when no version is specified.
	if version == "" {
		version = DefaultVersion
	}

	if !validVersions[version] {
		return "", fmt.Errorf(
			"unknown Java version %q: valid versions are 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, 21",
			version,
		)
	}

	// Grammar files follow the pattern: java/java{version}.tokens
	// For example: java/java21.tokens, java/java1.0.tokens
	return filepath.Join(root, "java", "java"+version+".tokens"), nil
}

// CreateJavaLexer constructs a GrammarLexer ready to tokenise the given
// Java source string.
//
// version selects the Java grammar file:
//   - ""     — uses DefaultVersion ("21"), the latest LTS release
//   - "1.0", "1.1", "1.4" — classic Java releases
//   - "5", "7", "8" — pre-modular Java releases
//   - "10", "14", "17", "21" — modern Java releases
//
// An error is returned if the version string is unrecognised or if the grammar
// file cannot be read.
func CreateJavaLexer(source string, version string) (*lexer.GrammarLexer, error) {
	grammarPath, err := getGrammarPath(version)
	if err != nil {
		return nil, err
	}
	bytes, err := os.ReadFile(grammarPath)
	if err != nil {
		return nil, err
	}
	grammar, err := grammartools.ParseTokenGrammar(string(bytes))
	if err != nil {
		return nil, err
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

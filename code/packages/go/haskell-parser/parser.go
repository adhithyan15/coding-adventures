// Package haskellparser parses Haskell source code into an AST using versioned
// grammars. It supports Haskell 1.0, 1.1, 1.2, 1.3, 1.4, 98, and 2010, each
// with its own parser grammar.
//
// The grammars are embedded at compile time as native Go data structures in
// grammar_data.go (VersionedParserGrammars, keyed by version string). Nothing
// is read from disk at run time, so the parser needs no filesystem capability
// and works unchanged when the package is built standalone.
//
// Usage:
//
//	ast, err := haskellparser.ParseHaskell(source, "2010")
//	ast, err := haskellparser.ParseHaskell(source, "")  // defaults to 2010
package haskellparser

import (
	"fmt"
	"sort"
	"strings"

	haskelllexer "github.com/adhithyan15/coding-adventures/code/packages/go/haskell-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// DefaultVersion is used when callers pass version="".
const DefaultVersion = "2010"

var validVersions = map[string]bool{
	"1.0":  true,
	"1.1":  true,
	"1.2":  true,
	"1.3":  true,
	"1.4":  true,
	"98":   true,
	"2010": true,
}

// ValidVersions returns the supported Haskell grammar versions in stable order.
func ValidVersions() []string {
	versions := make([]string, 0, len(validVersions))
	for version := range validVersions {
		versions = append(versions, version)
	}
	sort.Strings(versions)
	return versions
}

// normalizeVersion resolves the empty version to DefaultVersion and rejects any
// version that has no embedded grammar, mirroring the original error text.
func normalizeVersion(version string) (string, error) {
	if version == "" {
		return DefaultVersion, nil
	}
	if validVersions[version] {
		return version, nil
	}
	return "", fmt.Errorf("unknown Haskell version %q: valid versions are %s", version, strings.Join(ValidVersions(), ", "))
}

// CreateHaskellParser tokenizes the source with the Haskell lexer, then returns
// a GrammarParser configured for the selected Haskell version. If version is
// empty, DefaultVersion ("2010") is used.
//
// The parser grammar is selected from the compiled-in VersionedParserGrammars
// map; no grammar file is read at run time. An error is returned when lexing
// fails or when the requested version is not a supported Haskell version.
func CreateHaskellParser(source string, version string) (*parser.GrammarParser, error) {
	tokens, err := haskelllexer.TokenizeHaskell(source, version)
	if err != nil {
		return nil, err
	}
	effectiveVersion, err := normalizeVersion(version)
	if err != nil {
		return nil, err
	}
	// Fail closed if the validator's version set ever drifts from the embedded
	// grammar map, rather than deferring a nil grammar to the parser engine.
	grammar, ok := VersionedParserGrammars[effectiveVersion]
	if !ok {
		return nil, fmt.Errorf("no embedded Haskell parser grammar for version %q", effectiveVersion)
	}
	return parser.NewGrammarParser(tokens, grammar), nil
}

// ParseHaskell parses source with the selected Haskell grammar.
func ParseHaskell(source string, version string) (*parser.ASTNode, error) {
	haskellParser, err := CreateHaskellParser(source, version)
	if err != nil {
		return nil, err
	}
	return haskellParser.Parse()
}

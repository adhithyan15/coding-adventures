// Package haskelllexer tokenizes Haskell source code using versioned grammars.
// It supports Haskell 1.0, 1.1, 1.2, 1.3, 1.4, 98, and 2010, each with its own
// token grammar that captures the layout-aware lexical structure for that
// version.
//
// The grammars are embedded at compile time as native Go data structures in
// grammar_data.go (VersionedTokenGrammars, keyed by version string). Nothing
// is read from disk at run time, so the lexer needs no filesystem capability
// and works unchanged when the package is built standalone.
//
// Usage:
//
//	tokens, err := haskelllexer.TokenizeHaskell(source, "2010")
//	tokens, err := haskelllexer.TokenizeHaskell(source, "")  // defaults to 2010
package haskelllexer

import (
	"fmt"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
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

// CreateHaskellLexer constructs a shared GrammarLexer for the selected Haskell
// version. If version is empty, DefaultVersion ("2010") is used.
//
// The grammar is selected from the compiled-in VersionedTokenGrammars map; no
// grammar file is read at run time. An error is returned only when the
// requested version is not a supported Haskell version.
func CreateHaskellLexer(source string, version string) (*lexer.GrammarLexer, error) {
	effectiveVersion, err := normalizeVersion(version)
	if err != nil {
		return nil, err
	}
	grammar := VersionedTokenGrammars[effectiveVersion]
	return lexer.NewGrammarLexer(source, grammar), nil
}

// TokenizeHaskell tokenizes source with the selected Haskell grammar.
func TokenizeHaskell(source string, version string) ([]lexer.Token, error) {
	haskellLexer, err := CreateHaskellLexer(source, version)
	if err != nil {
		return nil, err
	}
	return haskellLexer.Tokenize(), nil
}

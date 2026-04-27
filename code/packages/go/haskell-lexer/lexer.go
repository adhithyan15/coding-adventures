package haskelllexer

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
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

func normalizeVersion(version string) (string, error) {
	if version == "" {
		return DefaultVersion, nil
	}
	if validVersions[version] {
		return version, nil
	}
	return "", fmt.Errorf("unknown Haskell version %q: valid versions are %s", version, strings.Join(ValidVersions(), ", "))
}

func grammarRoot() string {
	_, filename, _, _ := runtime.Caller(0)
	return filepath.Join(filepath.Dir(filename), "..", "..", "..", "grammars")
}

func getTokensPath(version string) (string, error) {
	effectiveVersion, err := normalizeVersion(version)
	if err != nil {
		return "", err
	}
	return filepath.Join(grammarRoot(), "haskell", "haskell"+effectiveVersion+".tokens"), nil
}

// CreateHaskellLexer constructs a shared GrammarLexer for the selected Haskell version.
func CreateHaskellLexer(source string, version string) (*lexer.GrammarLexer, error) {
	tokensPath, err := getTokensPath(version)
	if err != nil {
		return nil, err
	}
	bytes, err := os.ReadFile(tokensPath)
	if err != nil {
		return nil, err
	}
	grammar, err := grammartools.ParseTokenGrammar(string(bytes))
	if err != nil {
		return nil, err
	}
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

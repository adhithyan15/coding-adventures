package haskellparser

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
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

func getGrammarPath(version string) (string, error) {
	effectiveVersion, err := normalizeVersion(version)
	if err != nil {
		return "", err
	}
	return filepath.Join(grammarRoot(), "haskell", "haskell"+effectiveVersion+".grammar"), nil
}

// CreateHaskellParser constructs a shared GrammarParser for the selected Haskell version.
func CreateHaskellParser(source string, version string) (*parser.GrammarParser, error) {
	tokens, err := haskelllexer.TokenizeHaskell(source, version)
	if err != nil {
		return nil, err
	}
	grammarPath, err := getGrammarPath(version)
	if err != nil {
		return nil, err
	}
	bytes, err := os.ReadFile(grammarPath)
	if err != nil {
		return nil, err
	}
	grammar, err := grammartools.ParseParserGrammar(string(bytes))
	if err != nil {
		return nil, err
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

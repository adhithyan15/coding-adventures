// Package pythonlexer tokenizes Python source code using versioned grammars.
// It supports Python 2.7, 3.0, 3.6, 3.8, 3.10, and 3.12, each with its own
// token grammar that captures the exact token set for that version.
//
// The grammars are embedded at compile time as native Go data structures in
// grammar_data.go (VersionedTokenGrammars, keyed by version string). Nothing
// is read from disk at run time, so the lexer needs no filesystem capability
// and works unchanged when the package is built standalone.
//
// Usage:
//
//	tokens, err := pythonlexer.TokenizePython(source, "3.12")
//	tokens, err := pythonlexer.TokenizePython(source, "")  // defaults to 3.12
package pythonlexer

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// DefaultVersion is the Python version used when no version is specified.
// We default to the latest grammar we have.
const DefaultVersion = "3.12"

// SupportedVersions lists all Python versions with embedded grammars.
var SupportedVersions = []string{"2.7", "3.0", "3.6", "3.8", "3.10", "3.12"}

// resolveVersion returns the version string to use. If version is empty,
// it returns DefaultVersion.
func resolveVersion(version string) string {
	if version == "" {
		return DefaultVersion
	}
	return version
}

// CreatePythonLexer creates a GrammarLexer configured for the given Python
// version. If version is empty, DefaultVersion ("3.12") is used.
//
//	lexer, err := CreatePythonLexer(source, "3.8")
//	lexer, err := CreatePythonLexer(source, "")  // defaults to 3.12
//
// The grammar is selected from the compiled-in VersionedTokenGrammars map;
// no grammar file is read at run time. An error is returned only when the
// requested version has no embedded grammar.
func CreatePythonLexer(source string, version string) (*lexer.GrammarLexer, error) {
	v := resolveVersion(version)
	grammar, ok := VersionedTokenGrammars[v]
	if !ok {
		return nil, fmt.Errorf("unsupported Python version %q: no embedded grammar", v)
	}
	return lexer.NewGrammarLexer(source, grammar), nil
}

// TokenizePython tokenizes Python source code using the grammar for the
// specified version. If version is empty, DefaultVersion ("3.12") is used.
//
//	tokens, err := TokenizePython("x = 1\n", "3.12")
//	tokens, err := TokenizePython("x = 1\n", "")  // defaults to 3.12
func TokenizePython(source string, version string) ([]lexer.Token, error) {
	pythonLexer, err := CreatePythonLexer(source, version)
	if err != nil {
		return nil, err
	}
	return pythonLexer.Tokenize(), nil
}

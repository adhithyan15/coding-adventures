// Package pythonlexer tokenizes Python source code using versioned grammar
// files. It supports Python 2.7, 3.0, 3.6, 3.8, 3.10, and 3.12, each with
// its own .tokens grammar that captures the exact token set for that version.
//
// The grammar files live at code/grammars/python/pythonX.Y.tokens and are
// loaded at runtime by the grammar-driven lexer. A per-version cache avoids
// re-parsing the grammar on every call.
//
// Usage:
//
//	tokens, err := pythonlexer.TokenizePython(source, "3.12")
//	tokens, err := pythonlexer.TokenizePython(source, "")  // defaults to 3.12
//
// Future: these grammar files will be compiled into Go source (like the
// existing _grammar.go pattern) so that no file I/O is needed at runtime.
// The public API will remain the same — only the loading internals change.
package pythonlexer

import (
	"fmt"
	"path/filepath"
	"runtime"
	"sync"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// DefaultVersion is the Python version used when no version is specified.
// We default to the latest grammar we have.
const DefaultVersion = "3.12"

// SupportedVersions lists all Python versions with grammar files.
var SupportedVersions = []string{"2.7", "3.0", "3.6", "3.8", "3.10", "3.12"}

// grammarCache stores parsed TokenGrammar objects keyed by version string.
// Once a grammar is parsed, it is reused for all subsequent calls with that
// version. This is safe for concurrent use because TokenGrammar is read-only
// after construction.
var (
	grammarCache = make(map[string]*grammartools.TokenGrammar)
	cacheMu      sync.Mutex
)

// grammarsDir returns the absolute path to the grammars directory.
// It navigates from this source file's location:
//
//	code/packages/go/python-lexer/lexer.go
//	 → code/packages/go/          (parent)
//	 → code/packages/             (parent)
//	 → code/                      (parent)
//	 → code/grammars/             (join)
func grammarsDir() string {
	_, filename, _, _ := runtime.Caller(0)
	return filepath.Join(filepath.Dir(filename), "..", "..", "..", "grammars")
}

// grammarPath returns the path to the .tokens file for the given version.
//
//	grammarPath("3.12") → ".../code/grammars/python/python3.12.tokens"
func grammarPath(version string) string {
	return filepath.Join(grammarsDir(), "python", fmt.Sprintf("python%s.tokens", version))
}

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
func CreatePythonLexer(source string, version string) (*lexer.GrammarLexer, error) {
	v := resolveVersion(version)

	// Check cache first (fast path).
	cacheMu.Lock()
	if grammar, ok := grammarCache[v]; ok {
		cacheMu.Unlock()
		return lexer.NewGrammarLexer(source, grammar), nil
	}
	cacheMu.Unlock()

	// Cache miss — load grammar via Operation framework.
	return StartNew[*lexer.GrammarLexer]("pythonlexer.CreatePythonLexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			bytes, err := op.File.ReadFile(grammarPath(v))
			if err != nil {
				return rf.Fail(nil, fmt.Errorf("failed to read grammar for Python %s: %w", v, err))
			}
			grammar, err := grammartools.ParseTokenGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, fmt.Errorf("failed to parse grammar for Python %s: %w", v, err))
			}

			// Cache the parsed grammar for future calls.
			cacheMu.Lock()
			grammarCache[v] = grammar
			cacheMu.Unlock()

			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
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

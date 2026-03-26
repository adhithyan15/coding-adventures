// Package starlarklexer tokenizes Starlark source code using a grammar-driven lexer.
//
// Starlark is a deterministic subset of Python designed for configuration files,
// most notably used in Bazel BUILD files. It uses significant indentation (like
// Python), meaning the lexer must track indentation levels and emit synthetic
// INDENT/DEDENT tokens to delimit blocks.
//
// This package is a thin wrapper around the generic grammar-driven lexer. It:
//   1. Loads the Starlark token grammar from the starlark.tokens file
//   2. Passes it to the GrammarLexer, which compiles the regex patterns
//   3. The GrammarLexer handles indentation mode, skip patterns, reserved
//      keywords, and type aliases automatically based on the grammar file
//
// The starlark.tokens grammar file defines:
//   - mode: indentation  -- enables Python-style INDENT/DEDENT/NEWLINE tracking
//   - skip: patterns     -- comments (#...) and inline whitespace are discarded
//   - reserved: keywords -- Python keywords not in Starlark (class, while, etc.)
//                           cause a panic if encountered, giving clear error messages
//   - -> TYPE aliases    -- multiple string patterns (e.g., triple-quoted strings)
//                           all emit the same STRING token type
//
// Usage:
//
//   // One-shot tokenization: source code in, token slice out
//   tokens, err := starlarklexer.TokenizeStarlark(`x = 1 + 2`)
//
//   // Or create a reusable lexer for more control
//   lex, err := starlarklexer.CreateStarlarkLexer(`def f():\n    return 1\n`)
//   tokens := lex.Tokenize()
package starlarklexer

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// getGrammarPath computes the absolute path to the starlark.tokens grammar file.
//
// We use runtime.Caller(0) to find the directory of this Go source file at
// runtime, then navigate up three levels (starlark-lexer -> go -> packages ->
// code) to reach the grammars directory. This approach works regardless of the
// working directory, which is important because tests and the build tool may
// run from different locations.
//
// Directory structure:
//   code/
//     grammars/
//       starlark.tokens    <-- this is what we want
//     packages/
//       go/
//         starlark-lexer/
//           lexer.go       <-- we are here (3 levels below code/)
func getGrammarPath() string {
	// runtime.Caller(0) returns the file path of the current source file.
	// The underscore variables are: program counter, line number, and ok bool.
	_, filename, _, _ := runtime.Caller(0)

	// filepath.Dir gives us the directory containing lexer.go
	parent := filepath.Dir(filename)

	// Navigate up 3 levels: starlark-lexer -> go -> packages -> code,
	// then down into grammars/
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	return filepath.Join(root, "starlark.tokens")
}

// CreateStarlarkLexer loads the Starlark token grammar and returns a configured
// GrammarLexer ready to tokenize the given source code.
//
// The returned lexer operates in indentation mode, meaning it will:
//   - Track indentation levels using a stack (starting at [0])
//   - Emit INDENT tokens when indentation increases
//   - Emit DEDENT tokens when indentation decreases
//   - Emit NEWLINE tokens at logical line boundaries
//   - Suppress INDENT/DEDENT/NEWLINE inside brackets ((), [], {})
//   - Reject tab characters in leading whitespace
//   - Skip comments (# to end of line)
//   - Panic on reserved keywords (class, while, import, etc.)
//
// Returns an error if the grammar file cannot be read or parsed.
func CreateStarlarkLexer(source string) (*lexer.GrammarLexer, error) {
	return StartNew[*lexer.GrammarLexer]("starlarklexer.CreateStarlarkLexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			// Read the grammar file from disk. This file defines all token patterns,
			// keywords, reserved words, skip patterns, and the indentation mode flag.
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Parse the grammar file into a structured TokenGrammar object.
			// This extracts keywords, reserved keywords, token definitions (with
			// regex patterns and type aliases), skip definitions, and the mode.
			grammar, err := grammartools.ParseTokenGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Create the grammar-driven lexer. The GrammarLexer constructor compiles
			// all regex patterns, builds keyword/reserved-keyword lookup sets, and
			// initializes the indentation stack if mode is "indentation".
			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
}

// TokenizeStarlark is a convenience function that tokenizes Starlark source code
// in a single call. It creates a lexer, runs tokenization, and returns the
// resulting token slice.
//
// This is the simplest way to tokenize Starlark code. For repeated tokenization
// or when you need access to the lexer object itself, use CreateStarlarkLexer
// instead.
//
// The returned tokens include:
//   - KEYWORD tokens for Starlark keywords (def, if, for, return, etc.)
//   - NAME tokens for identifiers
//   - INT, FLOAT tokens for numeric literals
//   - STRING tokens for string literals (all quote styles unified)
//   - Operator tokens (PLUS, STAR, DOUBLE_STAR, FLOOR_DIV, etc.)
//   - INDENT/DEDENT tokens for indentation changes
//   - NEWLINE tokens at logical line boundaries
//   - EOF token at the end
//
// Returns an error if the grammar file cannot be loaded.
func TokenizeStarlark(source string) ([]lexer.Token, error) {
	starlarkLexer, err := CreateStarlarkLexer(source)
	if err != nil {
		return nil, err
	}
	return starlarkLexer.Tokenize(), nil
}

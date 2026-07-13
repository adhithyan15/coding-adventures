// Package starlarklexer tokenizes Starlark source code using a grammar-driven lexer.
//
// Starlark is a deterministic subset of Python designed for configuration files,
// most notably used in Bazel BUILD files. It uses significant indentation (like
// Python), meaning the lexer must track indentation levels and emit synthetic
// INDENT/DEDENT tokens to delimit blocks.
//
// This package is a thin wrapper around the generic grammar-driven lexer. It:
//   1. Uses the Starlark token grammar embedded at compile time as native Go
//      in grammar_data.go (TokenGrammarData)
//   2. Passes it to the GrammarLexer, which compiles the regex patterns
//   3. The GrammarLexer handles indentation mode, skip patterns, reserved
//      keywords, and type aliases automatically based on the grammar
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
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateStarlarkLexer returns a GrammarLexer configured with the Starlark token
// grammar, ready to tokenize the given source code.
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData); nothing is read from disk at run time. The lexer works
// unchanged when the package is built standalone and needs no filesystem
// capability. The error result is retained for API compatibility and is
// always nil.
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
func CreateStarlarkLexer(source string) (*lexer.GrammarLexer, error) {
	return lexer.NewGrammarLexer(source, TokenGrammarData), nil
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

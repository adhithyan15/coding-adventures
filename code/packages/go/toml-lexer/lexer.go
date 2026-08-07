// Package tomllexer tokenizes TOML text using a grammar-driven lexer.
//
// TOML (Tom's Obvious Minimal Language, v1.0.0 — https://toml.io/en/v1.0.0)
// is a configuration file format designed to be easy to read. Unlike JSON,
// TOML is newline-sensitive: key-value pairs are terminated by newlines, so
// the lexer emits NEWLINE tokens that the parser grammar uses to delimit
// expressions.
//
// This package is a thin wrapper around the generic grammar-driven lexer. It:
//   1. Uses the TOML token grammar embedded at compile time (TokenGrammarData)
//   2. Passes it to the GrammarLexer, which compiles the regex patterns
//   3. The GrammarLexer handles skip patterns (comments and whitespace)
//      automatically based on the grammar file
//
// The toml.tokens grammar file defines:
//   - ML_BASIC_STRING, ML_LITERAL_STRING: triple-quoted multi-line strings
//   - BASIC_STRING: double-quoted strings with escape sequences
//   - LITERAL_STRING: single-quoted strings with no escape processing
//   - OFFSET_DATETIME, LOCAL_DATETIME, LOCAL_DATE, LOCAL_TIME: date/time literals
//   - FLOAT: decimal, scientific notation, inf, nan
//   - INTEGER: decimal, hexadecimal (0x), octal (0o), binary (0b)
//   - TRUE, FALSE: boolean literals
//   - BARE_KEY: unquoted key names ([A-Za-z0-9_-]+)
//   - Structural tokens: = . , [ ] { }
//   - skip: comments (#...) and whitespace (spaces, tabs only)
//
// TOML uses `escapes: none` in its grammar file. This tells the lexer to
// strip quotes from strings but leave escape sequences as raw text. TOML has
// four string types with different escape semantics:
//   - Basic strings ("..."): support \n, \t, \\, \", \uXXXX, \UXXXXXXXX
//   - Multi-line basic strings ("""..."""): same escapes + line-ending backslash
//   - Literal strings ('...'): NO escape processing at all
//   - Multi-line literal strings ('''...'''): NO escape processing at all
//
// Because the generic lexer can only apply one escape mode to all strings,
// we disable lexer-level escape processing entirely. The toml-parser's
// semantic layer handles type-specific escape processing after parsing.
//
// Usage:
//
//   // One-shot tokenization: TOML text in, token slice out
//   tokens, err := tomllexer.TokenizeTOML(`[server]\nhost = "localhost"`)
//
//   // Or create a reusable lexer for more control
//   lex, err := tomllexer.CreateTOMLLexer(`name = "TOML"`)
//   tokens := lex.Tokenize()
package tomllexer

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateTOMLLexer returns a GrammarLexer configured with the TOML token
// grammar, ready to tokenize the given TOML text.
//
// The returned lexer operates in standard mode (not indentation mode). TOML
// is newline-sensitive — newlines delimit key-value pairs — so the standard
// tokenizer emits NEWLINE tokens for every line break. The grammar uses these
// NEWLINE tokens to separate expressions.
//
// Skip patterns consume:
//   - Comments: # to end of line (the newline itself becomes a NEWLINE token)
//   - Whitespace: spaces and tabs (NOT newlines — those are significant)
//
// The lexer produces these token types:
//   - ML_BASIC_STRING: triple-double-quoted multi-line strings
//   - ML_LITERAL_STRING: triple-single-quoted multi-line strings
//   - BASIC_STRING: double-quoted strings (quotes stripped, escapes raw)
//   - LITERAL_STRING: single-quoted strings (quotes stripped)
//   - OFFSET_DATETIME: full datetime with timezone (2024-01-15T10:30:00Z)
//   - LOCAL_DATETIME: datetime without timezone (2024-01-15T10:30:00)
//   - LOCAL_DATE: date only (2024-01-15)
//   - LOCAL_TIME: time only (10:30:00)
//   - FLOAT: floating-point numbers, inf, nan
//   - INTEGER: decimal, hex, octal, binary integers
//   - TRUE, FALSE: boolean literals
//   - BARE_KEY: unquoted key names
//   - EQUALS, DOT, COMMA: delimiters
//   - LBRACKET, RBRACKET: bracket delimiters (used for tables and arrays)
//   - LBRACE, RBRACE: brace delimiters (used for inline tables)
//   - NEWLINE: line breaks (significant in TOML)
//   - EOF: end of input
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData); nothing is read from disk at run time. The lexer works
// unchanged when the package is built standalone and needs no filesystem
// capability. The error result is retained for API compatibility and is
// always nil.
func CreateTOMLLexer(source string) (*lexer.GrammarLexer, error) {
	return lexer.NewGrammarLexer(source, TokenGrammarData), nil
}

// TokenizeTOML is a convenience function that tokenizes TOML text in a single
// call. It creates a lexer, runs tokenization, and returns the resulting token
// slice.
//
// This is the simplest way to tokenize TOML. For repeated tokenization or when
// you need access to the lexer object itself, use CreateTOMLLexer instead.
//
// The returned tokens include:
//   - String tokens for all four string types (quotes stripped)
//   - Number tokens for integers and floats (including special values)
//   - Date/time tokens for all four date/time types
//   - Boolean tokens (TRUE, FALSE)
//   - BARE_KEY tokens for unquoted key names
//   - Structural tokens (EQUALS, DOT, COMMA, brackets, braces)
//   - NEWLINE tokens between lines
//   - EOF token at the end
//
// Returns an error if the grammar file cannot be loaded.
func TokenizeTOML(source string) ([]lexer.Token, error) {
	tomlLexer, err := CreateTOMLLexer(source)
	if err != nil {
		return nil, err
	}
	return tomlLexer.Tokenize(), nil
}

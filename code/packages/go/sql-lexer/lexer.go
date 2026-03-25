// Package sqllexer tokenizes SQL text using a grammar-driven lexer.
//
// SQL (Structured Query Language) is the lingua franca of relational databases.
// This package implements tokenization for an ANSI SQL subset covering DQL
// (SELECT), DML (INSERT, UPDATE, DELETE), and DDL (CREATE TABLE, DROP TABLE).
//
// Unlike JSON, SQL has case-insensitive keywords: SELECT, select, and Select are
// all the same keyword. This is handled automatically by the grammar-tools magic
// comment `# @case_insensitive true` in sql.tokens. The GrammarLexer reads this
// flag from the parsed grammar struct and stores keywords as uppercase, normalizing
// all keyword token values to uppercase on emit. No extra API call is needed.
//
// This package is a thin wrapper around the generic grammar-driven lexer. It:
//  1. Loads the SQL token grammar from the sql.tokens file
//  2. Parses it into a TokenGrammar struct (with CaseInsensitive=true)
//  3. Passes it to the GrammarLexer, which configures case-insensitive mode
//
// The sql.tokens grammar defines:
//   - NAME: identifiers ([a-zA-Z_][a-zA-Z0-9_]*)
//   - NUMBER: integer and decimal literals
//   - STRING: single-quoted strings (alias of STRING_SQ)
//   - KEYWORD: SQL keywords normalized to uppercase (SELECT, FROM, WHERE, ...)
//   - Operators: =, !=, <>, <, >, <=, >=, +, -, *, /, %
//   - Punctuation: ( ) , ; .
//   - skip: whitespace, -- line comments, /* block comments */
//
// Token aliases:
//   - STRING_SQ → STRING (single-quoted strings become STRING tokens)
//   - QUOTED_ID → NAME  (backtick-quoted identifiers become NAME tokens)
//   - NEQ_ANSI (<>) → NOT_EQUALS (both spellings of ≠ produce NOT_EQUALS)
//
// Usage:
//
//	// One-shot tokenization: SQL text in, token slice out
//	tokens, err := sqllexer.TokenizeSQL("SELECT id, name FROM users WHERE active = TRUE")
//
//	// Or create a reusable lexer for more control
//	lex, err := sqllexer.CreateSQLLexer("SELECT * FROM orders")
//	tokens := lex.Tokenize()
package sqllexer

import (
	"path/filepath"
	"runtime"

	cage "github.com/adhithyan15/coding-adventures/code/packages/go/capability-cage"
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// sqlTokensPath is the path to the sql.tokens file. It defaults to "" which
// triggers automatic path discovery via runtime.Caller(0). Tests can override
// this variable to exercise error-handling code paths (e.g., by pointing at a
// non-existent file or a file with invalid grammar syntax).
var sqlTokensPath = ""

// getGrammarPath computes the absolute path to the sql.tokens grammar file.
//
// If sqlTokensPath is non-empty (e.g., overridden by a test), that value is
// returned directly. Otherwise, runtime.Caller(0) locates the source file and
// navigates up three levels (sql-lexer → go → packages → code) to reach the
// grammars directory.
//
// Directory structure:
//
//	code/
//	  grammars/
//	    sql.tokens        <-- this is what we want
//	  packages/
//	    go/
//	      sql-lexer/
//	        lexer.go      <-- we are here (3 levels below code/)
func getGrammarPath() string {
	if sqlTokensPath != "" {
		return sqlTokensPath
	}

	// runtime.Caller(0) returns the file path of this source file at runtime.
	// The underscore variables are: program counter, line number, and ok bool.
	_, filename, _, _ := runtime.Caller(0)

	// filepath.Dir gives us the directory containing lexer.go
	parent := filepath.Dir(filename)

	// Navigate up 3 levels: sql-lexer → go → packages → code,
	// then down into grammars/
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	return filepath.Join(root, "sql.tokens")
}

// CreateSQLLexer loads the SQL token grammar and returns a configured
// GrammarLexer ready to tokenize the given SQL text.
//
// The grammar file contains `# @case_insensitive true`, so the returned lexer
// automatically:
//   - Stores all keywords in uppercase internally
//   - Accepts SELECT, select, Select (any casing) as the same keyword
//   - Emits KEYWORD tokens with uppercase values (e.g., "SELECT" not "select")
//
// This ensures that the grammar literals like "SELECT" in sql.grammar match
// regardless of how the user typed the keyword.
//
// Returns an error if the grammar file cannot be read or parsed.
func CreateSQLLexer(source string) (*lexer.GrammarLexer, error) {
	// Read the grammar file via the capability cage. This ensures the operation
	// is covered by the declared fs:read capability in gen_capabilities.go.
	bytes, err := cage.ReadFileAt(Manifest, "code/grammars/sql.tokens", getGrammarPath())
	if err != nil {
		return nil, err
	}

	// Parse the grammar file into a structured TokenGrammar object.
	// The magic comment `# @case_insensitive true` sets grammar.CaseInsensitive
	// to true, which the GrammarLexer reads to enable case-folding on keywords.
	grammar, err := grammartools.ParseTokenGrammar(string(bytes))
	if err != nil {
		return nil, err
	}

	// Create the grammar-driven lexer. Because grammar.CaseInsensitive is true,
	// the constructor stores all keywords as uppercase and sets up the
	// case-folding lookup. SQL uses no indentation mode (no INDENT/DEDENT tokens).
	return lexer.NewGrammarLexer(source, grammar), nil
}

// TokenizeSQL is a convenience function that tokenizes SQL text in a single
// call. It creates a lexer, runs tokenization, and returns the resulting token
// slice.
//
// This is the simplest way to tokenize SQL. For repeated tokenization or when
// you need access to the lexer object itself, use CreateSQLLexer instead.
//
// The returned tokens include (non-exhaustive):
//   - KEYWORD tokens for SQL keywords (always uppercase: "SELECT", "FROM", …)
//   - NAME tokens for table names, column names, and other identifiers
//   - NUMBER tokens for integer and decimal literals
//   - STRING tokens for single-quoted string literals (quotes stripped)
//   - NOT_EQUALS tokens for both != and <>
//   - STAR tokens for * (used in SELECT *, COUNT(*), and multiplication)
//   - LPAREN/RPAREN tokens for parentheses
//   - COMMA/SEMICOLON/DOT tokens for punctuation
//   - EOF token at the end
//
// Returns an error if the grammar file cannot be loaded.
func TokenizeSQL(source string) ([]lexer.Token, error) {
	sqlLexer, err := CreateSQLLexer(source)
	if err != nil {
		return nil, err
	}
	return sqlLexer.Tokenize(), nil
}

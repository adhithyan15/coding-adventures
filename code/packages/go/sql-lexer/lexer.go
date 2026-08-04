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
//  1. Uses the SQL token grammar embedded at compile time (TokenGrammarData,
//     built with CaseInsensitive=true)
//  2. Passes it to the GrammarLexer, which configures case-insensitive mode
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
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateSQLLexer returns a GrammarLexer configured with the SQL token grammar,
// ready to tokenize the given SQL text.
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData); nothing is read from disk at run time. Because the
// embedded grammar was built with CaseInsensitive=true, the returned lexer
// automatically:
//   - Stores all keywords in uppercase internally
//   - Accepts SELECT, select, Select (any casing) as the same keyword
//   - Emits KEYWORD tokens with uppercase values (e.g., "SELECT" not "select")
//
// This ensures that the grammar literals like "SELECT" in sql.grammar match
// regardless of how the user typed the keyword. The lexer works unchanged when
// the package is built standalone and needs no filesystem capability. The error
// result is retained for API compatibility and is always nil.
func CreateSQLLexer(source string) (*lexer.GrammarLexer, error) {
	return lexer.NewGrammarLexer(source, TokenGrammarData), nil
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

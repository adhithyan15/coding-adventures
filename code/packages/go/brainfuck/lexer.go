// Package brainfuck tokenizes and parses Brainfuck source code.
//
// Brainfuck is an esoteric programming language created by Urban Müller in
// 1993. The entire language consists of exactly eight commands, each
// represented by a single ASCII character:
//
//	>   increment data pointer (move right one cell)
//	<   decrement data pointer (move left one cell)
//	+   increment byte at data pointer
//	-   decrement byte at data pointer
//	.   output byte at data pointer as ASCII character
//	,   accept one byte of input, store at data pointer
//	[   if byte at data pointer is zero, jump forward past matching ]
//	]   if byte at data pointer is nonzero, jump back past matching [
//
// Everything else in a Brainfuck source file is a comment. The language has
// no dedicated comment syntax — any character that is not one of the eight
// commands is simply ignored. This means Brainfuck programs can be annotated
// with natural language text placed anywhere in the source.
//
// # Tokenization Pipeline
//
// The tokenizer converts raw Brainfuck source text into a flat list of tokens:
//
//	Source text  →  Lexer  →  []Token
//	"++[>+<-]"      rules     [INC INC LOOP_START RIGHT INC LEFT DEC LOOP_END EOF]
//
// The lexer uses a grammar-driven engine whose token grammar is embedded at
// compile time as native Go data (TokenGrammarData in grammar_data.go). The
// grammar specifies:
//   - 8 literal token types (one per command character)
//   - 2 skip patterns: whitespace and comments (non-command characters)
//
// Skip patterns are consumed silently and never appear in the token stream.
// This keeps the parser grammar clean: it describes only the 8 commands and
// the loop structure, never worrying about comments.
//
// # Line and Column Tracking
//
// The lexer tracks line and column numbers as it scans. The WHITESPACE skip
// pattern is defined separately from the COMMENT skip pattern specifically to
// preserve accurate line counting: the lexer increments the line counter when
// it sees a newline character, and this works correctly because whitespace
// (including \n) is a distinct pattern.
//
// # Usage
//
//	// One-shot tokenization: Brainfuck source in, token slice out
//	tokens, err := brainfuck.TokenizeBrainfuck(`++ increment > move right`)
//
//	// Or create a reusable lexer for more control
//	lex, err := brainfuck.CreateBrainfuckLexer(`[>+<-]`)
//	tokens := lex.Tokenize()
package brainfuck

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// CreateBrainfuckLexer returns a GrammarLexer configured with the Brainfuck
// token grammar, ready to tokenize the given Brainfuck source text.
//
// The grammar is embedded at compile time as native Go in grammar_data.go
// (TokenGrammarData); nothing is read from disk at run time. The lexer
// compiles the grammar's regex patterns once here, then reuses them during
// Tokenize(). Line and column tracking start at (1, 1).
//
// The returned lexer operates in default scanning mode. Whitespace and
// non-command characters (comments) are discarded automatically by the
// skip: patterns in the grammar — they are consumed but never emitted.
//
// The error result is retained for API compatibility and is always nil: with
// the grammar compiled in, there is no file to read and nothing to parse at
// run time, so the capability-scoped file system is no longer needed.
func CreateBrainfuckLexer(source string) (*lexer.GrammarLexer, error) {
	return lexer.NewGrammarLexer(source, TokenGrammarData), nil
}

// TokenizeBrainfuck is a convenience function that tokenizes Brainfuck source
// text in a single call. It creates a lexer, runs tokenization, and returns
// the resulting token slice.
//
// The token slice always ends with an EOF token. Skip tokens (whitespace and
// comments) are not included — only the eight command tokens appear.
//
// Example:
//
//	tokens, err := TokenizeBrainfuck("++ increment cell")
//	// tokens = [INC(1,1) INC(1,2) EOF(1,19)]
//	// "increment cell" is consumed as a COMMENT skip and never emitted
//
// Returns an error if the grammar file cannot be loaded.
func TokenizeBrainfuck(source string) ([]lexer.Token, error) {
	bfLexer, err := CreateBrainfuckLexer(source)
	if err != nil {
		return nil, err
	}
	return bfLexer.Tokenize(), nil
}

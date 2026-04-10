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
// The lexer uses a grammar-driven engine loaded from brainfuck.tokens. The
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
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// getTokensGrammarPath computes the absolute path to the brainfuck.tokens
// grammar file.
//
// We use runtime.Caller(0) to find the directory of this Go source file at
// runtime, then navigate up three levels (brainfuck -> go -> packages ->
// code) to reach the grammars directory. This approach works regardless of
// the working directory, which matters because tests and the build tool may
// run from different locations.
//
// Directory structure:
//
//	code/
//	  grammars/
//	    brainfuck.tokens    <-- this is what we want
//	  packages/
//	    go/
//	      brainfuck/
//	        lexer.go        <-- we are here (3 levels below code/)
func getTokensGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "brainfuck.tokens")
}

// CreateBrainfuckLexer loads the Brainfuck token grammar and returns a
// configured GrammarLexer ready to tokenize the given Brainfuck source text.
//
// The pipeline:
//  1. Open brainfuck.tokens via the capability-scoped file system
//  2. Parse the grammar file into a TokenGrammar structure
//  3. Construct a GrammarLexer configured with that grammar
//
// The returned lexer operates in default scanning mode. Whitespace and
// non-command characters (comments) are discarded automatically by the
// skip: patterns in the grammar — they are consumed but never emitted.
//
// Returns an error if the grammar file cannot be read or parsed.
func CreateBrainfuckLexer(source string) (*lexer.GrammarLexer, error) {
	return StartNew[*lexer.GrammarLexer]("brainfuck.CreateBrainfuckLexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			// Read the grammar file through the capability-enforced file accessor.
			// This ensures only the declared paths (brainfuck.tokens and
			// brainfuck.grammar) can be opened — any other path causes a
			// capability violation error rather than a silent success.
			bytes, err := op.File.ReadFile(getTokensGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Parse the raw grammar text into structured token definitions.
			// ParseTokenGrammar extracts the token names, patterns (literal or
			// regex), and skip definitions into a TokenGrammar value.
			grammar, err := grammartools.ParseTokenGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Construct the generic grammar-driven lexer with the Brainfuck
			// grammar. The lexer compiles regex patterns once here, then reuses
			// them during Tokenize(). Line and column tracking start at (1, 1).
			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
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

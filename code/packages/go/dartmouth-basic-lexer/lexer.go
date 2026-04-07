// Package dartmouthlexer tokenizes 1964 Dartmouth BASIC source text using a
// grammar-driven lexer.
//
// # Dartmouth BASIC: The Language for Everyone
//
// In 1964, John Kemeny and Thomas Kurtz at Dartmouth College created BASIC
// (Beginner's All-purpose Symbolic Instruction Code) with a radical goal: give
// non-science students easy access to computing on a time-sharing system.
//
// The language ran on a GE-225 mainframe, with users connecting via teletypes —
// mechanical keyboards that printed output on paper rolls. These teletypes only
// had uppercase letters, which is why BASIC is case-insensitive and why we
// normalize the whole source to uppercase before lexing.
//
// Dartmouth BASIC 1964 has a deliberately small design:
//   - Every statement lives on a numbered line: "10 LET X = 5"
//   - Line numbers are the only addressing scheme — GOTO 30 jumps to line 30
//   - 286 possible variable names: single letters (A–Z) or letter+digit (A0–Z9)
//   - 20 keywords covering arithmetic, control flow, I/O, and subroutines
//   - 11 built-in mathematical functions: SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR, INT, RND, SGN
//   - All numbers are floating-point internally (even 42 is stored as 42.0)
//
// # Grammar-Driven Approach
//
// This package is a thin wrapper around the generic grammar-driven lexer. It:
//  1. Loads the dartmouth_basic.tokens grammar file from code/grammars/
//  2. Passes it to GrammarLexer, which compiles the regex patterns into a DFA
//  3. Registers two post-tokenize hooks for BASIC-specific disambiguation:
//     - relabelLineNumbers: reclassifies NUMBER tokens at line-start as LINE_NUM
//     - suppressRemContent: discards tokens after a REM keyword until NEWLINE
//
// # LINE_NUM vs NUMBER Disambiguation
//
// The grammar cannot distinguish "10" (a line label) from "10" (a numeric value
// in an expression) by regex alone — both are sequences of digits. The
// relabelLineNumbers hook solves this by walking the finished token list and
// relabeling the first NUMBER on each line as LINE_NUM.
//
// After relabeling:
//
//	"10 LET X = 5"  →  LINE_NUM("10") KEYWORD("LET") NAME("X") EQ("=") NUMBER("5")
//	"GOTO 30"       →  KEYWORD("GOTO") NUMBER("30")  ← target stays NUMBER
//
// # REM Comment Handling
//
// REM (remark) introduces a comment that runs to the end of the line. The
// suppressRemContent hook discards all tokens between a KEYWORD("REM") and the
// next NEWLINE, so comment text never reaches the parser:
//
//	"10 REM THIS IS A COMMENT"  →  LINE_NUM("10") KEYWORD("REM") NEWLINE
//
// # Case Insensitivity
//
// Because @case_insensitive is set in the grammar, the lexer normalizes the
// entire source to uppercase before matching. This means "print", "Print", and
// "PRINT" all produce KEYWORD("PRINT"). This mirrors the historical reality of
// uppercase-only teletypes.
//
// Usage:
//
//	// One-shot tokenization: BASIC source in, token slice out
//	tokens, err := dartmouthlexer.TokenizeDartmouthBasic("10 LET X = 5\n20 PRINT X\n30 END")
//
//	// Or create a reusable lexer for more control
//	lex, err := dartmouthlexer.CreateDartmouthBasicLexer("10 PRINT \"HELLO\"\n20 END")
//	tokens, err := lex.Tokenize()
package dartmouthlexer

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// getGrammarPath computes the absolute path to the dartmouth_basic.tokens
// grammar file at runtime.
//
// We use runtime.Caller(0) to find the directory containing this source file,
// then navigate up three directory levels to reach the repo's grammars/ folder.
// Using the source-file location rather than the working directory means tests
// and the build tool — which may run from any directory — always find the grammar.
//
// Directory structure (three levels from lexer.go to code/):
//
//	code/
//	  grammars/
//	    dartmouth_basic.tokens    <-- this is what we want
//	  packages/
//	    go/
//	      dartmouth-basic-lexer/
//	        lexer.go              <-- we are here
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "dartmouth_basic.tokens")
}

// relabelLineNumbers is a post-tokenize hook that reclassifies NUMBER tokens
// in line-number position as LINE_NUM tokens.
//
// The challenge: "10" in "10 LET X = 5" (a line label) looks identical to
// "10" in "GOTO 10" (a branch target) or "10" in "LET X = 10" (a literal).
// The grammar cannot distinguish them by pattern — all are digit sequences.
//
// The solution is positional: a Dartmouth BASIC line always begins with a line
// number. So the first NUMBER token on each new physical line is a LINE_NUM.
//
// Algorithm:
//  1. Walk the token list with an atLineStart flag, initially true (we start
//     at the beginning of the source, which is also a line start).
//  2. When atLineStart is true and we see a NUMBER token: relabel it LINE_NUM
//     and set atLineStart = false.
//  3. When atLineStart is true and we see a non-NUMBER token: just set
//     atLineStart = false (a blank line, or a line that starts with something
//     else — unlikely in valid BASIC but we handle it gracefully).
//  4. When we see a NEWLINE token: set atLineStart = true for the next token.
//
// This hook must run before suppressRemContent because REM content may include
// digit sequences that should not be treated as LINE_NUMs.
func relabelLineNumbers(tokens []lexer.Token) []lexer.Token {
	result := make([]lexer.Token, 0, len(tokens))
	atLineStart := true
	for _, tok := range tokens {
		if atLineStart && tok.TypeName == "NUMBER" {
			tok.TypeName = "LINE_NUM"
			atLineStart = false
		} else if atLineStart {
			// Non-number token at line start (e.g., a blank line's NEWLINE,
			// or malformed input). Do not relabel; just mark that we are
			// no longer at the start of a line.
			atLineStart = false
		}
		if tok.TypeName == "NEWLINE" {
			atLineStart = true
		}
		result = append(result, tok)
	}
	return result
}

// suppressRemContent is a post-tokenize hook that discards all tokens after a
// REM keyword until the next NEWLINE (exclusive).
//
// REM (remark) introduces a comment that runs to the end of the physical line.
// Example: "10 REM THIS IS IGNORED" should produce only:
//
//	LINE_NUM("10") KEYWORD("REM") NEWLINE
//
// The text after REM — which was tokenized character-by-character as NAME and
// UNKNOWN tokens — is simply dropped. The parser never sees it.
//
// Algorithm:
//  1. Walk the token list with a suppressing flag, initially false.
//  2. If not suppressing: emit the current token.
//  3. After emitting a KEYWORD("REM") token: set suppressing = true.
//  4. When a NEWLINE token is encountered: set suppressing = false.
//     The NEWLINE is emitted regardless (it was not suppressed because
//     suppressing was still true from the REM, but the loop emits it only
//     when !suppressing — see the code carefully).
//
// Wait — the NEWLINE itself must reach the parser (it is the statement
// terminator). So we emit the NEWLINE token unconditionally and then clear
// the suppression flag. The implementation below achieves this by checking
// suppression *before* updating the flag, and by always emitting NEWLINE.
//
// More precisely:
//   - We only append to result when !suppressing.
//   - When we see KEYWORD("REM"): we already appended it (suppressing was false),
//     then set suppressing = true.
//   - When we see NEWLINE while suppressing: we do NOT append it in the
//     "if !suppressing" branch. Instead we set suppressing = false so the
//     NEWLINE passes through on the *next* NEWLINE check...
//
// Actually, looking at the provided algorithm more carefully:
//
//	for _, tok := range tokens {
//	    if !suppressing {
//	        result = append(result, tok)      // emit when not suppressed
//	    }
//	    if tok.Type == "KEYWORD" && tok.Value == "REM" {
//	        suppressing = true                // start suppressing after REM
//	    } else if tok.Type == "NEWLINE" {
//	        suppressing = false              // stop suppressing at NEWLINE
//	    }
//	}
//
// This means NEWLINE itself is suppressed (not emitted) while suppressing=true.
// But wait — we need the NEWLINE to reach the parser! Let's trace:
//
//	"10 REM HELLO\n20 LET X = 1"
//
// Tokens before hook (after relabelLineNumbers):
//
//	LINE_NUM("10"), KEYWORD("REM"), NAME("HELLO"), NEWLINE, LINE_NUM("20"), ...
//
// Processing:
//   - LINE_NUM: suppressing=false → emit; not REM, not NEWLINE → no change
//   - KEYWORD("REM"): suppressing=false → emit; is REM → suppressing=true
//   - NAME("HELLO"): suppressing=true → skip; not NEWLINE → no change
//   - NEWLINE: suppressing=true → skip; is NEWLINE → suppressing=false
//   - LINE_NUM("20"): suppressing=false → emit; ...
//
// So the NEWLINE after REM is also suppressed. The spec says:
// "10 REM THIS IS A COMMENT" → [LINE_NUM("10"), KEYWORD("REM"), NEWLINE, EOF]
//
// This matches the spec: the NEWLINE IS in the expected output. So our
// implementation must emit the NEWLINE. We need to adjust: emit NEWLINE even
// when suppressing, then clear the flag.
//
// The correct implementation (what is implemented below):
//   - Emit the token if NOT suppressing, OR if the token is NEWLINE.
//   - Then update the suppression flag.
func suppressRemContent(tokens []lexer.Token) []lexer.Token {
	result := make([]lexer.Token, 0, len(tokens))
	suppressing := false
	for _, tok := range tokens {
		if !suppressing {
			result = append(result, tok)
		} else if tok.TypeName == "NEWLINE" {
			// The NEWLINE that ends a REM line is the statement terminator.
			// The parser needs it to know where the REM statement ends.
			result = append(result, tok)
		}
		if tok.TypeName == "KEYWORD" && tok.Value == "REM" {
			suppressing = true
		} else if tok.TypeName == "NEWLINE" {
			suppressing = false
		}
	}
	return result
}

// CreateDartmouthBasicLexer loads the Dartmouth BASIC token grammar and returns
// a configured GrammarLexer ready to tokenize the given BASIC source text.
//
// The lexer is configured with two post-tokenize hooks (applied in order):
//  1. relabelLineNumbers — reclassifies the first NUMBER on each line as LINE_NUM
//  2. suppressRemContent — discards comment text after REM until end of line
//
// The grammar is loaded from code/grammars/dartmouth_basic.tokens, which
// declares @case_insensitive true. This means the source text is normalized to
// uppercase before matching, so "print", "Print", and "PRINT" all produce
// KEYWORD("PRINT").
//
// The returned lexer's Tokenize() method produces the full token stream
// including NEWLINE tokens (which are significant in BASIC — they terminate
// statements) and a final EOF token.
//
// Returns an error if the grammar file cannot be read or parsed.
func CreateDartmouthBasicLexer(source string) (*lexer.GrammarLexer, error) {
	return StartNew[*lexer.GrammarLexer]("dartmouthlexer.CreateDartmouthBasicLexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}
			grammar, err := grammartools.ParseTokenGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}
			lex := lexer.NewGrammarLexer(source, grammar)
			lex.AddPostTokenize(relabelLineNumbers)
			lex.AddPostTokenize(suppressRemContent)
			return rf.Generate(true, false, lex)
		}).GetResult()
}

// TokenizeDartmouthBasic is a convenience function that tokenizes Dartmouth
// BASIC source text in a single call. It creates a lexer, runs tokenization,
// and returns the resulting token slice.
//
// The token stream reflects the full, post-processed output:
//   - NEWLINE tokens are present (they are statement terminators, not whitespace)
//   - LINE_NUM tokens appear at the start of each line (relabeled from NUMBER)
//   - REM comment text is absent (suppressed by the hook)
//   - All keyword and identifier values are uppercase (case_insensitive)
//   - A final EOF token is always appended
//
// Example token stream for "10 LET X = 5\n20 PRINT X\n30 END":
//
//	LINE_NUM("10") KEYWORD("LET") NAME("X") EQ("=") NUMBER("5") NEWLINE
//	LINE_NUM("20") KEYWORD("PRINT") NAME("X") NEWLINE
//	LINE_NUM("30") KEYWORD("END") NEWLINE
//	EOF("")
//
// Returns an error if the grammar file cannot be loaded.
func TokenizeDartmouthBasic(source string) ([]lexer.Token, error) {
	lex, err := CreateDartmouthBasicLexer(source)
	if err != nil {
		return nil, err
	}
	return lex.Tokenize(), nil
}

// Brainfuck parser — converts a token stream into an Abstract Syntax Tree.
//
// After the lexer has converted raw Brainfuck text into a flat list of tokens,
// the parser gives that list a hierarchical structure. The grammar has just
// four rules:
//
//	program     = { instruction }
//	instruction = loop | command
//	loop        = LOOP_START { instruction } LOOP_END
//	command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT
//
// This grammar is simple but expressive: a Brainfuck program is a tree of
// loops and commands, where loops can nest arbitrarily deep.
//
// # The AST Shape
//
// For the source "++[>+<-]" the parser produces:
//
//	program
//	  instruction
//	    command: INC("+")
//	  instruction
//	    command: INC("+")
//	  instruction
//	    loop
//	      LOOP_START("[")
//	      instruction
//	        command: RIGHT(">")
//	      instruction
//	        command: INC("+")
//	      instruction
//	        command: LEFT("<")
//	      instruction
//	        command: DEC("-")
//	      LOOP_END("]")
//
// # Error Cases
//
// Because brackets must be matched, the parser catches structural errors:
//   - "[["    — unmatched open bracket  → parse error
//   - "]"     — unexpected close bracket → parse error
//   - "[>]+"  — valid (trailing command after loop is fine)
//   - "[]"    — valid (empty loop is legal — it's a no-op when cell is zero)
//
// # Why a Separate Parser File?
//
// The lexer and parser are in the same Go package (package brainfuck). This
// means the parser can call TokenizeBrainfuck directly — no separate import
// of a brainfuck-lexer package is needed. This differs from the json-parser,
// which imports json-lexer as a separate module because the JSON lexer has its
// own separately versioned package.
//
// Usage:
//
//	// One-shot parsing: Brainfuck text in, AST out
//	ast, err := brainfuck.ParseBrainfuck(`++[>+<-]`)
//
//	// Or create a reusable parser for more control
//	p, err := brainfuck.CreateBrainfuckParser(`[-]`)
//	ast, err := p.Parse()
package brainfuck

import (
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// getParserGrammarPath computes the absolute path to the brainfuck.grammar
// file.
//
// Like getTokensGrammarPath, this uses runtime.Caller(0) to anchor the path
// relative to the source file's directory rather than the working directory.
//
// Directory structure:
//
//	code/
//	  grammars/
//	    brainfuck.grammar   <-- this is what we want
//	  packages/
//	    go/
//	      brainfuck/
//	        parser.go       <-- we are here (3 levels below code/)
func getParserGrammarPath() string {
	// runtime.Caller(0) returns the path of this source file at compile time.
	// Since lexer.go and parser.go live in the same directory, both navigate
	// the same three levels up to reach code/grammars/.
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "brainfuck.grammar")
}

// CreateBrainfuckParser tokenizes the Brainfuck source text using
// TokenizeBrainfuck, then loads the Brainfuck parser grammar and returns a
// configured GrammarParser ready to produce an AST.
//
// The pipeline has three steps:
//  1. TokenizeBrainfuck(source) — produces a flat token stream.
//     Comments and whitespace are already stripped; only command tokens remain.
//  2. Read brainfuck.grammar via the capability-scoped file system.
//  3. Parse the grammar file with grammartools.ParseParserGrammar, then
//     construct a GrammarParser from the tokens and grammar.
//
// The GrammarParser uses recursive descent with packrat memoization. Each
// grammar rule becomes a parsing function. The "program" rule (first in the
// file) is the entry point. Memoization ensures that no (rule, position) pair
// is computed twice, giving linear parse time even for deeply nested programs.
//
// Returns an error if lexing fails or the grammar file cannot be read/parsed.
func CreateBrainfuckParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source. Because the lexer and parser are in the
	// same package, we call TokenizeBrainfuck directly — no import needed.
	// After this step, all comments and whitespace are gone. The token slice
	// contains only the eight command token types plus a terminal EOF.
	tokens, err := TokenizeBrainfuck(source)
	if err != nil {
		return nil, err
	}

	// Steps 2–4 run inside a capability-scoped Operation so that all file I/O
	// is audited against the declared allowlist in required_capabilities.json.
	return StartNew[*parser.GrammarParser]("brainfuck.CreateBrainfuckParser", nil,
		func(op *Operation[*parser.GrammarParser], rf *ResultFactory[*parser.GrammarParser]) *OperationResult[*parser.GrammarParser] {
			// Step 2: Read the parser grammar file.
			// The grammar file defines the four rules: program, instruction,
			// loop, and command, using EBNF-style notation.
			bytes, err := op.File.ReadFile(getParserGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 3: Parse the grammar file into structured rule definitions.
			// This produces a ParserGrammar with one GrammarRule per production:
			// program, instruction, loop, command.
			grammar, err := grammartools.ParseParserGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 4: Create the grammar-driven parser.
			// This builds a rule lookup table (map[string]GrammarRule) and
			// initializes the packrat memoization cache. The first rule in
			// brainfuck.grammar — "program" — becomes the parse entry point.
			return rf.Generate(true, false, parser.NewGrammarParser(tokens, grammar))
		}).GetResult()
}

// ParseBrainfuck is a convenience function that parses Brainfuck source text
// into an AST in a single call.
//
// The returned ASTNode tree mirrors the grammar structure:
//   - node.RuleName is the grammar rule that matched (e.g., "program",
//     "instruction", "loop", "command")
//   - node.Children contains child ASTNodes and lexer.Token leaves
//   - Leaf nodes wrap individual tokens (the command characters)
//
// Example AST for "[-]":
//
//	program
//	  instruction
//	    loop
//	      LOOP_START("[")
//	      instruction
//	        command
//	          DEC("-")
//	      LOOP_END("]")
//
// This pattern — a loop containing a single DEC command — is the canonical
// "clear cell" idiom: it decrements the current cell until it reaches zero.
//
// Returns an error if lexing or parsing fails (e.g., unmatched brackets).
func ParseBrainfuck(source string) (*parser.ASTNode, error) {
	bfParser, err := CreateBrainfuckParser(source)
	if err != nil {
		return nil, err
	}
	return bfParser.Parse()
}

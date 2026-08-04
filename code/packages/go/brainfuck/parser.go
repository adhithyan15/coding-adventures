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
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// CreateBrainfuckParser tokenizes the Brainfuck source text using
// TokenizeBrainfuck, then returns a configured GrammarParser ready to produce
// an AST.
//
// The pipeline has two steps:
//  1. TokenizeBrainfuck(source) — produces a flat token stream.
//     Comments and whitespace are already stripped; only command tokens remain.
//  2. Construct a GrammarParser from the tokens and the parser grammar.
//
// The parser grammar is embedded at compile time as native Go in
// parser_grammar_data.go (ParserGrammarData); nothing is read from disk at run
// time. The GrammarParser uses recursive descent with packrat memoization.
// Each grammar rule becomes a parsing function, and the first rule in the
// grammar — "program" — is the entry point. Memoization ensures that no
// (rule, position) pair is computed twice, giving linear parse time even for
// deeply nested programs.
//
// The error result carries lexing failures from step 1; grammar loading can no
// longer fail because it is compiled in rather than read from disk.
func CreateBrainfuckParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source. Because the lexer and parser are in the
	// same package, we call TokenizeBrainfuck directly — no import needed.
	// After this step, all comments and whitespace are gone. The token slice
	// contains only the eight command token types plus a terminal EOF.
	tokens, err := TokenizeBrainfuck(source)
	if err != nil {
		return nil, err
	}

	// Step 2: Create the grammar-driven parser. This builds a rule lookup
	// table (map[string]GrammarRule) and initializes the packrat memoization
	// cache from the compiled-in ParserGrammarData.
	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
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

// Package dartmouthbasicparser parses Dartmouth BASIC 1964 source code into
// an Abstract Syntax Tree (AST).
//
// # A Brief History: Dartmouth BASIC and the Grammar-Driven Parser
//
// When John Kemeny and Thomas Kurtz created BASIC at Dartmouth College in 1964,
// the parser for the language had to be hand-crafted. Every compiler written in
// that era was a custom, bespoke affair — no parser generators, no EBNF tools.
// The BASIC parser on the GE-225 mainframe was a relatively simple top-down
// scan because BASIC's grammar was deliberately unambiguous: every statement
// starts with a distinct keyword, and expressions use the standard arithmetic
// precedence cascade.
//
// Today we take a different approach: the grammar is described declaratively in
// `dartmouth_basic.grammar` using EBNF notation, and a reusable parser engine
// interprets that grammar at runtime. This package is a thin adapter that wires
// together:
//
//  1. The Dartmouth BASIC lexer (dartmouth-basic-lexer): turns raw BASIC text
//     into a typed token stream (LINE_NUM, KEYWORD, NAME, NUMBER, etc.).
//
//  2. The grammar-driven parser engine (parser package): applies the
//     dartmouth_basic grammar — embedded at compile time as native Go in
//     grammar_data.go (ParserGrammarData) — to the token stream using
//     recursive descent with packrat memoization.
//
// The result is an AST rooted at the "program" rule.
//
// # The Grammar at a Glance
//
// Dartmouth BASIC 1964 has 17 statement types. Each begins with a distinct
// keyword, so the parser never has to guess which rule to try:
//
//	LET    — variable assignment:       10 LET X = 5
//	PRINT  — output:                    10 PRINT X, Y
//	INPUT  — read from user:            10 INPUT A, B
//	IF     — conditional branch:        10 IF X > 0 THEN 100
//	GOTO   — unconditional jump:        10 GOTO 50
//	GOSUB  — subroutine call:           10 GOSUB 200
//	RETURN — return from subroutine:   200 RETURN
//	FOR    — loop start:                10 FOR I = 1 TO 10
//	NEXT   — loop end:                  30 NEXT I
//	END    — normal termination:        99 END
//	STOP   — halt with message:         99 STOP
//	REM    — comment:                   10 REM A COMMENT
//	READ   — read from data pool:       10 READ X, Y
//	DATA   — declare data pool values:  20 DATA 1, 2, 3
//	RESTORE — reset data pool pointer:  30 RESTORE
//	DIM    — dimension an array:        10 DIM A(100)
//	DEF    — define a function:         10 DEF FNA(X) = X * X
//
// Expressions use a precedence cascade:
//
//	expr  (lowest:  + −)
//	  └── term (mid:    * /)
//	        └── power (high:   ^ right-assoc)
//	              └── unary (−)
//	                    └── primary (atoms: NUMBER, FN(expr), variable, (expr))
//
// # Parsing Pipeline
//
//	BASIC source text
//	      │
//	      ▼ dartmouthlexer.TokenizeDartmouthBasic(source)
//	  token stream [LINE_NUM, KEYWORD, NAME, ...]
//	      │
//	      ▼ parser.NewGrammarParser(tokens, grammar)
//	  GrammarParser (recursive descent + packrat memoization)
//	      │
//	      ▼ grammarParser.Parse()
//	  *parser.ASTNode{RuleName: "program", ...}
//
// # Usage
//
//	// One-shot: BASIC source → AST
//	ast, err := dartmouthbasicparser.ParseDartmouthBasic("10 PRINT \"HELLO\"\n20 END\n")
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Println(ast.RuleName) // "program"
//
//	// Two-step: create parser, then parse
//	p, err := dartmouthbasicparser.CreateDartmouthBasicParser("10 LET X = 5\n")
//	if err != nil {
//	    log.Fatal(err)
//	}
//	ast, err := p.Parse()
package dartmouthbasicparser

import (
	dartmouthlexer "github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// CreateDartmouthBasicParser tokenizes the BASIC source text using the Dartmouth
// BASIC lexer, then loads the parser grammar and returns a configured
// GrammarParser ready to produce an AST.
//
// The two-stage pipeline:
//
//  1. TokenizeDartmouthBasic(source) — scans the source and produces a token
//     stream. The lexer applies three post-tokenize hooks:
//     (a) relabelLineNumbers — promotes the first NUMBER on each line to LINE_NUM
//     (b) suppressRemContent — drops all tokens between REM and NEWLINE
//     (c) upcaseIdentifiers  — upcases NAME, BUILTIN_FN, USER_FN values
//
//  2. Create a GrammarParser from the tokens and the parser grammar embedded at
//     compile time as native Go in grammar_data.go (ParserGrammarData).
//     The GrammarParser uses recursive descent with packrat memoization.
//     Packrat memoization ensures no (rule, position) pair is computed more
//     than once, giving O(n × rules) time for most practical inputs.
//
// The grammar's "program" rule is the entry point (first rule in the file).
//
// The parser grammar is embedded at compile time; nothing is read from disk at
// run time, so the parser needs no filesystem capability and works when built
// standalone. Returns an error only when lexing fails.
func CreateDartmouthBasicParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the BASIC source.
	// The lexer normalises the token stream: line-number labels become LINE_NUM,
	// REM comments are suppressed, and identifiers are uppercased to match the
	// uppercase-only convention of 1964 Dartmouth BASIC teletypes.
	tokens, err := dartmouthlexer.TokenizeDartmouthBasic(source)
	if err != nil {
		return nil, err
	}

	// Step 2: Create the GrammarParser.
	// NewGrammarParser builds a rule-name → rule lookup table and initialises
	// the packrat memoization cache. ParserGrammarData is the embedded EBNF
	// grammar for Dartmouth BASIC 1964: all 17 statement types, the expression
	// precedence cascade, the variable rule (scalar and array forms), and helper
	// rules like relop, print_list, dim_decl, etc. The first rule ("program")
	// becomes the implicit entry point, called by GrammarParser.Parse().
	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
}

// ParseDartmouthBasic is a convenience function that parses Dartmouth BASIC
// source text into an AST in a single call.
//
// It creates a parser via CreateDartmouthBasicParser, runs Parse(), and returns
// the root AST node. The returned node always has RuleName == "program".
//
// The AST mirrors the grammar structure. For example, parsing:
//
//	10 LET X = 5
//	20 PRINT X
//	30 END
//
// produces an AST like:
//
//	program
//	  line
//	    LINE_NUM("10")
//	    statement
//	      let_stmt
//	        KEYWORD("LET")
//	        variable → NAME("X")
//	        EQ("=")
//	        expr → term → power → unary → primary → NUMBER("5")
//	    NEWLINE
//	  line
//	    LINE_NUM("20")
//	    statement
//	      print_stmt
//	        KEYWORD("PRINT")
//	        print_list → print_item → expr → ... → NAME("X")
//	    NEWLINE
//	  line
//	    LINE_NUM("30")
//	    statement
//	      end_stmt → KEYWORD("END")
//	    NEWLINE
//
// Returns an error if lexing or parsing fails. Common error cases:
//   - Missing "=" in LET:          "10 LET X 5\n"
//   - Missing "THEN" in IF:        "10 IF X > 0 100\n"
//   - Incomplete FOR (no TO):      "10 FOR I = 1\n"
//   - Unrecognised character in source
func ParseDartmouthBasic(source string) (*parser.ASTNode, error) {
	basicParser, err := CreateDartmouthBasicParser(source)
	if err != nil {
		return nil, err
	}
	return basicParser.Parse()
}

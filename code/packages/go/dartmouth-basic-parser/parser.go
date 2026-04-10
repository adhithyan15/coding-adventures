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
//  2. The grammar-driven parser engine (parser package): reads
//     dartmouth_basic.grammar and applies it to the token stream using
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
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	dartmouthlexer "github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// getGrammarPath computes the absolute path to the dartmouth_basic.grammar file.
//
// This function uses runtime.Caller(0) to locate this source file at runtime,
// then navigates up the directory tree to find the shared grammars directory.
//
// Directory layout:
//
//	code/
//	  grammars/
//	    dartmouth_basic.grammar   <-- what we want
//	  packages/
//	    go/
//	      dartmouth-basic-parser/
//	        parser.go             <-- we are here (3 levels below code/)
//
// Three "../" steps from the package directory reach code/; then we descend
// into grammars/.
func getGrammarPath() string {
	// runtime.Caller(0) returns the source path of THIS file, even at runtime.
	// This is Go's way of embedding a file-relative path — no os.Getwd() needed.
	_, filename, _, _ := runtime.Caller(0)

	// Navigate from the file's directory up 3 levels to code/
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	return filepath.Join(root, "dartmouth_basic.grammar")
}

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
//  2. Load dartmouth_basic.grammar, create a GrammarParser from the tokens.
//     The GrammarParser uses recursive descent with packrat memoization.
//     Packrat memoization ensures no (rule, position) pair is computed more
//     than once, giving O(n × rules) time for most practical inputs.
//
// The grammar's "program" rule is the entry point (first rule in the file).
//
// Returns an error if lexing fails or the grammar file cannot be read/parsed.
func CreateDartmouthBasicParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the BASIC source.
	// The lexer normalises the token stream: line-number labels become LINE_NUM,
	// REM comments are suppressed, and identifiers are uppercased to match the
	// uppercase-only convention of 1964 Dartmouth BASIC teletypes.
	tokens, err := dartmouthlexer.TokenizeDartmouthBasic(source)
	if err != nil {
		return nil, err
	}

	// Steps 2–4 run inside a capability-scoped Operation so that all file I/O
	// is audited against the declared allowlist in required_capabilities.json.
	// Only the exact path ../../../grammars/dartmouth_basic.grammar is permitted.
	return StartNew[*parser.GrammarParser]("dartmouthbasicparser.CreateDartmouthBasicParser", nil,
		func(op *Operation[*parser.GrammarParser], rf *ResultFactory[*parser.GrammarParser]) *OperationResult[*parser.GrammarParser] {
			// Step 2: Read the grammar file.
			// This EBNF grammar defines the full syntax of Dartmouth BASIC 1964:
			// all 17 statement types, the expression precedence cascade, the
			// variable rule (scalar and array forms), and helper rules like relop,
			// print_list, dim_decl, etc.
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 3: Parse the grammar file into a structured ParserGrammar.
			// ParseParserGrammar extracts each rule (name + body), where the body
			// is a tree of grammar elements: sequences, alternations (|), repetitions
			// ({ }), options ([ ]), token references (UPPERCASE), and rule references
			// (lowercase). Literal strings like "LET" match keyword tokens by value.
			grammar, err := grammartools.ParseParserGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 4: Create the GrammarParser.
			// NewGrammarParser builds a rule-name → rule lookup table and
			// initialises the packrat memoization cache. The first rule in the
			// grammar file ("program") becomes the implicit entry point, called
			// by GrammarParser.Parse().
			return rf.Generate(true, false, parser.NewGrammarParser(tokens, grammar))
		}).GetResult()
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

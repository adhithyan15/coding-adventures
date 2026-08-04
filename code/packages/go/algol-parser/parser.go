// Package algolparser parses ALGOL 60 source text into an Abstract Syntax Tree (AST).
//
// # ALGOL 60: The Language That Invented BNF
//
// ALGOL 60 (ALGOrithmic Language, 1960) holds a unique place in computer science
// history: it was the first programming language whose complete syntax was formally
// specified using BNF (Backus-Naur Form). The BNF notation was invented specifically
// to describe ALGOL 60's grammar, by John Backus and Peter Naur (the "B" and "N").
//
// Before ALGOL 60, language definitions were informal English prose. Ambiguities
// were common, implementations differed, and programmers had to read the source
// code of the compiler to understand what was legal. ALGOL changed all that by
// providing a mathematical description of syntax. Today, every major language
// standard (C, Java, Python, Go, Rust) includes a formal grammar in BNF or a
// close variant (EBNF, PEG).
//
// # ALGOL 60 Grammar Structure
//
// The grammar has these key levels:
//
//   program      → block
//   block        → BEGIN { declaration ; } statement { ; statement } END
//   declaration  → type_decl | array_decl | switch_decl | procedure_decl
//   statement    → assign_stmt | goto_stmt | proc_stmt | cond_stmt | for_stmt | ...
//   expression   → arith_expr | bool_expr
//
// Key design decisions encoded in the grammar:
//
//  1. Declaration-before-use at block level: all declarations come first in a block,
//     followed by all statements. This is enforced by the grammar, not checked
//     separately. Modern languages like Go and Java allow interleaving.
//
//  2. Dangling else is resolved by grammar: the then-branch uses unlabeled_stmt
//     (which cannot contain conditionals) while the else-branch uses statement
//     (which can). This requires begin...end to nest if-then-else chains.
//     C and Java resolve the dangling else by convention (else binds to nearest if).
//
//  3. Call-by-name is the default: procedure arguments are re-evaluated on each
//     use inside the procedure body. Only parameters listed in a VALUE declaration
//     are passed by value (evaluated once at call time). This rule led to the famous
//     "Jensen's device" — a legitimate ALGOL 60 technique that abuses call-by-name
//     to implement general summation procedures.
//
//  4. Exponentiation is left-associative: 2^3^4 = (2^3)^4 = 4096, not 2^(3^4) = 2^81.
//     This follows the original ALGOL 60 report, and differs from most modern languages
//     and mathematical convention (which use right-associativity).
//
// # Parsing Pipeline
//
// The two-stage pipeline:
//
//   1. Lexing (algol-lexer): ALGOL source text → token stream
//      Handles whitespace, comments (comment...;), keyword reclassification,
//      and multi-character operators (:=, **, <=, >=, !=).
//
//   2. Parsing (this package): token stream → AST
//      Applies algol.grammar rules using recursive descent with backtracking
//      and packrat memoization. The grammar is complex (30+ rules) but the
//      generic GrammarParser handles all of it from the grammar file.
//
// The packrat memoization cache ensures O(n) parsing time for most inputs.
// Each (rule, position) pair is computed at most once, stored in the cache,
// and reused on subsequent attempts. This is especially important for ALGOL's
// grammar, which has significant ambiguity between expressions and statement forms.
//
// Usage:
//
//	// One-shot parsing: ALGOL source in, AST out
//	ast, err := algolparser.ParseAlgol("begin integer x; x := 42 end")
//
//	// Or create a reusable parser for more control
//	p, err := algolparser.CreateAlgolParser("begin real pi; pi := 3.14159 end")
//	ast, parseErr := p.Parse()
package algolparser

import (
	"fmt"

	algollexer "github.com/adhithyan15/coding-adventures/code/packages/go/algol-lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// validAlgolVersions is the set of ALGOL versions with an embedded grammar.
// ALGOL 60 is the only version we support; the version parameter is retained
// on the public API so callers can pass a version explicitly and receive a
// clear error for anything unrecognized.
var validAlgolVersions = map[string]bool{
	"algol60": true,
}

// resolveVersion normalizes the optional version argument. An empty string
// defaults to "algol60". It returns an error for any unrecognized version,
// preserving the historical unknown-version behavior even though the grammar
// is now compiled in rather than read from disk.
func resolveVersion(version string) (string, error) {
	if version == "" {
		version = "algol60"
	}
	if !validAlgolVersions[version] {
		return "", fmt.Errorf("unknown ALGOL version %q: valid versions are algol60", version)
	}
	return version, nil
}

// CreateAlgolParser tokenizes the ALGOL 60 source using the ALGOL lexer, then
// returns a GrammarParser configured with the ALGOL 60 parser grammar, ready
// to produce an AST.
//
// The two-step process:
//  1. TokenizeAlgol(source) — produces a token stream with comments and
//     whitespace already stripped
//  2. Create a GrammarParser from the tokens and the embedded parser grammar
//
// The parser grammar is embedded at compile time as native Go in grammar_data.go
// (ParserGrammarData); nothing is read from disk at run time, so the parser
// needs no filesystem capability and works when built standalone. The
// GrammarParser uses recursive descent with packrat memoization. Each grammar
// rule (program, block, declaration, statement, expression, ...) becomes a
// parsing function. The memoization cache prevents exponential blowup when
// backtracking across ALGOL's many ambiguous rules.
//
// Returns an error if lexing fails or an unrecognized version is requested.
func CreateAlgolParser(source string, version ...string) (*parser.GrammarParser, error) {
	effectiveVersion := ""
	if len(version) > 0 {
		effectiveVersion = version[0]
	}
	if _, err := resolveVersion(effectiveVersion); err != nil {
		return nil, err
	}
	// Step 1: Tokenize the ALGOL source.
	// The lexer strips whitespace and comments (comment...;), reclassifies
	// keywords from identifiers, and handles multi-character operators.
	tokens, err := algollexer.TokenizeAlgol(source, effectiveVersion)
	if err != nil {
		return nil, err
	}

	// Step 2: Create the grammar-driven parser from the embedded grammar.
	// The first rule in the grammar ("program") becomes the entry point.
	// The memoization cache is initialized empty and populated on demand.
	return parser.NewGrammarParser(tokens, ParserGrammarData), nil
}

// ParseAlgol is a convenience function that parses ALGOL 60 source text into
// an AST in a single call. It creates a parser, runs parsing, and returns the
// root AST node.
//
// The returned ASTNode tree mirrors the grammar structure:
//   - node.RuleName is the grammar rule that matched ("program", "block",
//     "declaration", "statement", "assign_stmt", "arith_expr", etc.)
//   - node.Children contains child ASTNodes and lexer.Token leaves
//   - Leaf nodes wrap individual tokens
//
// Example AST for "begin integer x; x := 42 end":
//
//	program
//	  block
//	    BEGIN("begin")
//	    declaration
//	      type_decl
//	        type
//	          INTEGER("integer")
//	        ident_list
//	          IDENT("x")
//	    SEMICOLON(";")
//	    statement
//	      unlabeled_stmt
//	        assign_stmt
//	          left_part
//	            variable
//	              IDENT("x")
//	            ASSIGN(":=")
//	          expression
//	            arith_expr
//	              simple_arith
//	                term
//	                  factor
//	                    primary
//	                      INTEGER_LIT("42")
//	    END("end")
//
// Returns an error if lexing or parsing fails.
func ParseAlgol(source string, version ...string) (*parser.ASTNode, error) {
	algolParser, err := CreateAlgolParser(source, version...)
	if err != nil {
		return nil, err
	}
	return algolParser.Parse()
}

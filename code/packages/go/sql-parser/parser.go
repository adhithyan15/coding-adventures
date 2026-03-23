// Package sqlparser parses SQL text into an Abstract Syntax Tree (AST).
//
// SQL (Structured Query Language) is the lingua franca of relational databases.
// This parser processes an ANSI SQL subset covering:
//
//   - DQL: SELECT with FROM, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET,
//     JOINs (INNER, LEFT/RIGHT/FULL OUTER, CROSS), subexpressions, DISTINCT
//   - DML: INSERT INTO ... VALUES, UPDATE ... SET, DELETE FROM
//   - DDL: CREATE TABLE (with column constraints), DROP TABLE
//
// The parsing pipeline has two stages:
//
//  1. Lexing (sql-lexer): SQL text is tokenized into a stream of tokens.
//     Keywords are case-insensitive: select == SELECT == Select.
//     The grammar file's `# @case_insensitive true` handles this automatically.
//
//  2. Parsing (this package): The token stream is parsed according to the
//     sql.grammar rules using recursive descent with packrat memoization.
//     The grammar defines SQL's expression hierarchy:
//
//     program → statement { ";" statement } [ ";" ]
//     statement → select_stmt | insert_stmt | update_stmt | delete_stmt
//               | create_table_stmt | drop_table_stmt
//
// The grammar file (sql.grammar) uses EBNF notation:
//   - UPPERCASE names reference token types from the lexer (NAME, NUMBER, STAR, …)
//   - lowercase names reference grammar rules (can be recursive)
//   - { x } means zero or more repetitions
//   - [ x ] means optional
//   - | means alternation (ordered choice)
//   - "literal" matches a KEYWORD or PUNCTUATION token with that exact value
//
// Expression precedence is implemented via rule chaining:
//
//	expr → or_expr → and_expr → not_expr → comparison
//	     → additive → multiplicative → unary → primary
//
// This chains ensure + and - bind tighter than AND/OR, which bind tighter
// than NOT, which binds tighter than comparisons.
//
// Usage:
//
//	// One-shot parsing: SQL text in, AST out
//	ast, err := sqlparser.ParseSQL("SELECT id, name FROM users WHERE active = TRUE")
//
//	// Or create a reusable parser for more control
//	p, err := sqlparser.CreateSQLParser("SELECT * FROM orders")
//	ast, err := p.Parse()
package sqlparser

import (
	"os"
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	sqllexer "github.com/adhithyan15/coding-adventures/code/packages/go/sql-lexer"
)

// getGrammarPath computes the absolute path to the sql.grammar file.
//
// This uses the same runtime.Caller(0) technique as the sql-lexer package.
// We navigate up 3 levels from this source file to reach the code/ directory,
// then down into grammars/.
//
// Directory structure:
//
//	code/
//	  grammars/
//	    sql.grammar      <-- this is what we want
//	  packages/
//	    go/
//	      sql-parser/
//	        parser.go    <-- we are here (3 levels below code/)
func getGrammarPath() string {
	// runtime.Caller(0) returns the file path of this source file at runtime.
	_, filename, _, _ := runtime.Caller(0)

	// Get the directory containing this file
	parent := filepath.Dir(filename)

	// Navigate up 3 levels to code/, then down to grammars/
	root := filepath.Join(parent, "..", "..", "..", "grammars")

	return filepath.Join(root, "sql.grammar")
}

// CreateSQLParser tokenizes the SQL text using the SQL lexer, then loads the
// SQL parser grammar and returns a configured GrammarParser ready to produce
// an AST.
//
// The two-step process:
//  1. TokenizeSQL(source) -- produces a token stream with case-normalized keywords
//  2. Load sql.grammar and create a GrammarParser from the tokens
//
// The GrammarParser uses recursive descent with packrat memoization. Each
// grammar rule becomes a parsing function. The memoization cache ensures that
// no (rule, position) pair is computed more than once, giving O(n) parsing
// for most practical inputs.
//
// Returns an error if lexing fails or the grammar file cannot be read/parsed.
func CreateSQLParser(source string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the source using the SQL lexer.
	// Case-insensitive keyword mode is active (from `# @case_insensitive true`
	// in sql.tokens), so SELECT/select/Select all produce KEYWORD("SELECT").
	tokens, err := sqllexer.TokenizeSQL(source)
	if err != nil {
		return nil, err
	}

	// Step 2: Read the parser grammar file.
	// This file defines the SQL syntax rules in EBNF notation, including
	// statement types, expression precedence hierarchy, and DDL/DML forms.
	bytes, err := os.ReadFile(getGrammarPath())
	if err != nil {
		return nil, err
	}

	// Step 3: Parse the grammar file into a structured ParserGrammar object.
	// This extracts all rules (select_stmt, where_clause, expr, etc.) and
	// builds a rule lookup table for the recursive descent parser.
	grammar, err := grammartools.ParseParserGrammar(string(bytes))
	if err != nil {
		return nil, err
	}

	// Step 4: Create the grammar-driven parser.
	// The first rule in sql.grammar ("program") becomes the entry point.
	// The packrat memoization cache is initialized here.
	return parser.NewGrammarParser(tokens, grammar), nil
}

// ParseSQL is a convenience function that parses SQL text into an AST in a
// single call. It creates a parser, runs parsing, and returns the root AST node.
//
// The returned ASTNode tree mirrors the grammar structure:
//   - node.RuleName is the grammar rule that matched (e.g., "program", "select_stmt",
//     "where_clause", "expr")
//   - node.Children contains child ASTNodes and lexer.Token leaves
//   - Leaf nodes wrap individual tokens (keywords, names, numbers, operators)
//
// Example AST for `SELECT name FROM users WHERE id = 1`:
//
//	program
//	  statement
//	    select_stmt
//	      KEYWORD("SELECT")
//	      select_list
//	        select_item
//	          expr → or_expr → … → primary → column_ref
//	            NAME("name")
//	      KEYWORD("FROM")
//	      table_ref
//	        table_name
//	          NAME("users")
//	      where_clause
//	        KEYWORD("WHERE")
//	        expr → … → comparison
//	          additive → … → primary → column_ref
//	            NAME("id")
//	          cmp_op → EQUALS("=")
//	          additive → … → primary → NUMBER("1")
//
// Returns an error if lexing or parsing fails.
func ParseSQL(source string) (*parser.ASTNode, error) {
	sqlParser, err := CreateSQLParser(source)
	if err != nil {
		return nil, err
	}
	return sqlParser.Parse()
}

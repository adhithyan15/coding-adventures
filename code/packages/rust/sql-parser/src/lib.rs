//! # SQL Parser — parsing SQL source text into an AST.
//!
//! This crate is the second half of the SQL front-end pipeline. Where the
//! `sql-lexer` crate breaks source text into tokens, this crate arranges
//! those tokens into a tree that reflects the **structure** of the SQL —
//! an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! ```text
//! Source text  ("SELECT name FROM users WHERE age > 18")
//!       |
//!       v
//! sql-lexer            → Vec<Token>
//!       |                [Keyword("SELECT"), Name("name"), Keyword("FROM"),
//!       |                 Name("users"), Keyword("WHERE"), Name("age"),
//!       |                 GreaterThan(">"), Number("18"), EOF]
//!       v
//! sql.grammar          → ParserGrammar (rules: program, statement, ...)
//!       |
//!       v
//! GrammarParser        → GrammarASTNode tree
//!       |
//!       |                program
//!       |                  └── statement
//!       |                        └── select_stmt
//!       |                              ├── Keyword("SELECT")
//!       |                              ├── select_list
//!       |                              │     └── select_item
//!       |                              │           └── expr
//!       |                              │                 └── Name("name")
//!       |                              ├── Keyword("FROM")
//!       |                              ├── table_ref
//!       |                              │     └── Name("users")
//!       |                              └── where_clause
//!       |                                    └── expr
//!       |                                          └── comparison
//!       v
//! [application logic consumes the AST]
//! ```
//!
//! # Grammar-driven parsing
//!
//! The `GrammarParser` is a **recursive descent parser with backtracking and
//! packrat memoization**. The SQL grammar covers:
//!
//! - `program` — the start symbol: one or more semicolon-separated statements
//! - `statement` — SELECT | INSERT | UPDATE | DELETE | CREATE TABLE | DROP TABLE
//! - `select_stmt` — full SELECT including joins, WHERE, GROUP BY, HAVING, ORDER, LIMIT
//! - `insert_stmt` — INSERT INTO ... VALUES (...)
//! - `update_stmt` — UPDATE ... SET ... WHERE ...
//! - `delete_stmt` — DELETE FROM ... WHERE ...
//! - `create_table_stmt` — CREATE TABLE with column definitions and constraints
//! - `drop_table_stmt` — DROP TABLE [IF EXISTS]
//! - `expr` / `or_expr` / `and_expr` / ... — full expression hierarchy with
//!   precedence via nested rules (OR → AND → NOT → comparison → additive →
//!   multiplicative → unary → primary)
//!
//! # Why SQL?
//!
//! SQL is an excellent target for demonstrating the grammar-driven parser because:
//!
//! 1. **Universally known** — every backend developer has written SQL queries.
//! 2. **Rich grammar** — 20+ grammar rules covering real-world constructs.
//! 3. **Operator precedence** — the expression hierarchy shows how grammars
//!    encode precedence without explicit priority declarations.
//! 4. **Case-insensitivity** — SQL keywords are normalized by the lexer to
//!    uppercase, so the grammar file uses uppercase quoted strings.
//! 5. **Recursive expressions** — subqueries and nested expressions exercise
//!    the parser's backtracking and memoization.

use coding_adventures_sql_lexer::tokenize_sql;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for SQL source text.
///
/// This function performs three major steps:
///
/// 1. **Tokenization** — uses `tokenize_sql` from the sql-lexer crate to
///    break the source into tokens. Keywords are normalized to uppercase.
///
/// 2. **Grammar loading** — reads and parses the `sql.grammar` file, which
///    defines ~25 rules covering the full ANSI SQL subset.
///
/// 3. **Parser construction** — creates a `GrammarParser` with packrat
///    memoization for efficient backtracking.
///
/// # Errors
///
/// Returns `Err(String)` if:
/// - The source text fails tokenization (unexpected character).
/// - The `sql.grammar` file cannot be read or parsed.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_sql_parser::create_sql_parser;
///
/// let mut parser = create_sql_parser("SELECT id FROM users").unwrap();
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_sql_parser(source: &str) -> Result<GrammarParser, String> {
    // Step 1: Tokenize the source using the sql-lexer.
    //
    // tokenize_sql returns Result<Vec<Token>, String>. Keywords are normalized
    // to uppercase (e.g., "select" → Keyword("SELECT")) because sql.tokens
    // declares # @case_insensitive true.
    let tokens = tokenize_sql(source)?;

    // Step 2: Load the compiled parser grammar.
    let grammar = _grammar::parser_grammar();

    // Step 4: Create the parser.
    //
    // The GrammarParser takes ownership of both the tokens and the grammar.
    // It uses packrat memoization to avoid redundant re-parsing of the same
    // position, which is important for SQL's expression grammar which requires
    // backtracking.
    Ok(GrammarParser::new(tokens, grammar))
}

/// Parse SQL source text into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the SQL grammar), with children corresponding to the
/// SQL statements in the source.
///
/// # Errors
///
/// Returns `Err(String)` if tokenization fails, the grammar file is
/// missing/invalid, or the source text has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_sql_parser::parse_sql;
///
/// let ast = parse_sql("SELECT id FROM users").unwrap();
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_sql(source: &str) -> Result<GrammarASTNode, String> {
    // Create a parser wired to the SQL grammar and tokens.
    let mut sql_parser = create_sql_parser(source)?;

    // Parse and propagate any GrammarParseError as a String.
    sql_parser
        .parse()
        .map_err(|e| format!("SQL parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Assert that parsing succeeds and the root rule name is "program".
    ///
    /// All valid SQL documents parse to a root "program" node because
    /// that is the start symbol of sql.grammar.
    fn assert_program_root(source: &str) -> GrammarASTNode {
        let result = parse_sql(source);
        assert!(
            result.is_ok(),
            "Expected successful parse for {:?}, got: {:?}",
            source,
            result.err()
        );
        let ast = result.unwrap();
        assert_eq!(
            ast.rule_name, "program",
            "Expected root rule 'program', got '{}'",
            ast.rule_name
        );
        ast
    }

    /// Recursively search the AST for a node with the given rule name.
    /// Returns true if found anywhere in the tree.
    fn find_rule(node: &GrammarASTNode, target_rule: &str) -> bool {
        if node.rule_name == target_rule {
            return true;
        }
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                if find_rule(child_node, target_rule) {
                    return true;
                }
            }
        }
        false
    }

    // -----------------------------------------------------------------------
    // Test 1: SELECT * FROM table
    // -----------------------------------------------------------------------

    /// The simplest SELECT: star projection from a named table.
    /// Exercises: select_stmt, select_list (STAR), table_ref.
    ///
    /// `SELECT * FROM users`
    #[test]
    fn test_parse_select_star() {
        let ast = assert_program_root("SELECT * FROM users");
        assert!(
            find_rule(&ast, "select_stmt"),
            "Expected select_stmt in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: SELECT with column list
    // -----------------------------------------------------------------------

    /// SELECT with explicit column names.
    /// Exercises: select_list with multiple select_item nodes.
    ///
    /// `SELECT id, name, age FROM users`
    #[test]
    fn test_parse_select_columns() {
        let ast = assert_program_root("SELECT id, name, age FROM users");
        assert!(find_rule(&ast, "select_stmt"), "Expected select_stmt");
        assert!(find_rule(&ast, "select_list"), "Expected select_list");
    }

    // -----------------------------------------------------------------------
    // Test 3: SELECT with WHERE clause
    // -----------------------------------------------------------------------

    /// WHERE clause exercises the comparison sub-expression grammar.
    ///
    /// `SELECT id FROM users WHERE age > 18`
    #[test]
    fn test_parse_select_where() {
        let ast = assert_program_root("SELECT id FROM users WHERE age > 18");
        assert!(find_rule(&ast, "where_clause"), "Expected where_clause");
        assert!(find_rule(&ast, "comparison"), "Expected comparison in expr");
    }

    // -----------------------------------------------------------------------
    // Test 4: SELECT with ORDER BY
    // -----------------------------------------------------------------------

    /// ORDER BY exercises the order_clause and order_item rules.
    ///
    /// `SELECT id FROM users ORDER BY age DESC`
    #[test]
    fn test_parse_select_order_by() {
        let ast = assert_program_root("SELECT id FROM users ORDER BY age DESC");
        assert!(find_rule(&ast, "order_clause"), "Expected order_clause");
    }

    // -----------------------------------------------------------------------
    // Test 5: SELECT with LIMIT and OFFSET
    // -----------------------------------------------------------------------

    /// LIMIT / OFFSET clauses exercise the limit_clause rule.
    ///
    /// `SELECT id FROM users LIMIT 10 OFFSET 20`
    #[test]
    fn test_parse_select_limit_offset() {
        let ast = assert_program_root("SELECT id FROM users LIMIT 10 OFFSET 20");
        assert!(find_rule(&ast, "limit_clause"), "Expected limit_clause");
    }

    // -----------------------------------------------------------------------
    // Test 6: SELECT with GROUP BY and HAVING
    // -----------------------------------------------------------------------

    /// GROUP BY and HAVING exercise the group_clause and having_clause rules.
    ///
    /// `SELECT dept, COUNT(*) FROM employees GROUP BY dept HAVING COUNT(*) > 5`
    #[test]
    fn test_parse_select_group_having() {
        let ast = assert_program_root(
            "SELECT dept, COUNT(*) FROM employees GROUP BY dept HAVING COUNT(*) > 5",
        );
        assert!(find_rule(&ast, "group_clause"), "Expected group_clause");
        assert!(find_rule(&ast, "having_clause"), "Expected having_clause");
    }

    // -----------------------------------------------------------------------
    // Test 7: SELECT with alias
    // -----------------------------------------------------------------------

    /// AS alias on a column exercises the `select_item = expr [ AS NAME ]` rule.
    ///
    /// `SELECT age AS years FROM users`
    #[test]
    fn test_parse_select_alias() {
        let ast = assert_program_root("SELECT age AS years FROM users");
        assert!(find_rule(&ast, "select_item"), "Expected select_item");
    }

    // -----------------------------------------------------------------------
    // Test 8: INSERT statement
    // -----------------------------------------------------------------------

    /// INSERT INTO exercises insert_stmt and row_value rules.
    ///
    /// `INSERT INTO users (name, age) VALUES ('Alice', 30)`
    #[test]
    fn test_parse_insert() {
        let ast = assert_program_root("INSERT INTO users (name, age) VALUES ('Alice', 30)");
        assert!(find_rule(&ast, "insert_stmt"), "Expected insert_stmt");
        assert!(find_rule(&ast, "row_value"), "Expected row_value");
    }

    // -----------------------------------------------------------------------
    // Test 9: UPDATE statement
    // -----------------------------------------------------------------------

    /// UPDATE ... SET ... WHERE exercises update_stmt and assignment rules.
    ///
    /// `UPDATE users SET name = 'Bob' WHERE id = 1`
    #[test]
    fn test_parse_update() {
        let ast = assert_program_root("UPDATE users SET name = 'Bob' WHERE id = 1");
        assert!(find_rule(&ast, "update_stmt"), "Expected update_stmt");
        assert!(find_rule(&ast, "assignment"), "Expected assignment");
    }

    // -----------------------------------------------------------------------
    // Test 10: DELETE statement
    // -----------------------------------------------------------------------

    /// DELETE FROM exercises delete_stmt rule.
    ///
    /// `DELETE FROM users WHERE id = 1`
    #[test]
    fn test_parse_delete() {
        let ast = assert_program_root("DELETE FROM users WHERE id = 1");
        assert!(find_rule(&ast, "delete_stmt"), "Expected delete_stmt");
    }

    // -----------------------------------------------------------------------
    // Test 11: CREATE TABLE statement
    // -----------------------------------------------------------------------

    /// CREATE TABLE exercises create_table_stmt, col_def, col_constraint.
    ///
    /// `CREATE TABLE users (id INTEGER PRIMARY KEY, name VARCHAR NOT NULL)`
    #[test]
    fn test_parse_create_table() {
        let ast = assert_program_root(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name VARCHAR NOT NULL)",
        );
        assert!(
            find_rule(&ast, "create_table_stmt"),
            "Expected create_table_stmt"
        );
        assert!(find_rule(&ast, "col_def"), "Expected col_def");
        assert!(find_rule(&ast, "col_constraint"), "Expected col_constraint");
    }

    // -----------------------------------------------------------------------
    // Test 12: CREATE TABLE IF NOT EXISTS
    // -----------------------------------------------------------------------

    /// IF NOT EXISTS is an optional qualifier in the CREATE TABLE rule.
    #[test]
    fn test_parse_create_table_if_not_exists() {
        let ast = assert_program_root("CREATE TABLE IF NOT EXISTS t (id INTEGER)");
        assert!(
            find_rule(&ast, "create_table_stmt"),
            "Expected create_table_stmt"
        );
    }

    // -----------------------------------------------------------------------
    // Test 13: DROP TABLE statement
    // -----------------------------------------------------------------------

    /// DROP TABLE exercises drop_table_stmt rule.
    ///
    /// `DROP TABLE users`
    #[test]
    fn test_parse_drop_table() {
        let ast = assert_program_root("DROP TABLE users");
        assert!(
            find_rule(&ast, "drop_table_stmt"),
            "Expected drop_table_stmt"
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: DROP TABLE IF EXISTS
    // -----------------------------------------------------------------------

    /// IF EXISTS is an optional qualifier in the DROP TABLE rule.
    #[test]
    fn test_parse_drop_table_if_exists() {
        let ast = assert_program_root("DROP TABLE IF EXISTS users");
        assert!(
            find_rule(&ast, "drop_table_stmt"),
            "Expected drop_table_stmt"
        );
    }

    // -----------------------------------------------------------------------
    // Test 15: Case-insensitive keywords in parser
    // -----------------------------------------------------------------------

    /// Because the sql-lexer normalizes keywords to uppercase, the parser
    /// sees the same token stream regardless of the source casing.
    ///
    /// `select * from users` should parse identically to `SELECT * FROM users`.
    #[test]
    fn test_parse_case_insensitive_keywords() {
        let ast_lower = assert_program_root("select * from users");
        let ast_upper = assert_program_root("SELECT * FROM users");

        // Both should contain select_stmt and table_ref.
        assert!(
            find_rule(&ast_lower, "select_stmt"),
            "Lowercase: Expected select_stmt"
        );
        assert!(
            find_rule(&ast_upper, "select_stmt"),
            "Uppercase: Expected select_stmt"
        );
    }

    // -----------------------------------------------------------------------
    // Test 16: Expression precedence (arithmetic in WHERE)
    // -----------------------------------------------------------------------

    /// Expressions in WHERE clauses exercise the additive/multiplicative
    /// precedence hierarchy.
    ///
    /// `SELECT x FROM t WHERE x + 1 > 5 * 2`
    #[test]
    fn test_parse_arithmetic_expression() {
        let ast = assert_program_root("SELECT x FROM t WHERE x + 1 > 5 * 2");
        assert!(
            find_rule(&ast, "additive"),
            "Expected additive in expression"
        );
        assert!(find_rule(&ast, "multiplicative"), "Expected multiplicative");
    }

    // -----------------------------------------------------------------------
    // Test 17: NOT expression
    // -----------------------------------------------------------------------

    /// NOT prefix exercises the not_expr grammar rule.
    ///
    /// `SELECT x FROM t WHERE NOT x = 1`
    #[test]
    fn test_parse_not_expression() {
        let ast = assert_program_root("SELECT x FROM t WHERE NOT x = 1");
        assert!(find_rule(&ast, "not_expr"), "Expected not_expr");
    }

    // -----------------------------------------------------------------------
    // Test 18: AND / OR in WHERE
    // -----------------------------------------------------------------------

    /// Logical operators AND/OR exercise or_expr and and_expr rules.
    ///
    /// `SELECT x FROM t WHERE a = 1 AND b = 2 OR c = 3`
    #[test]
    fn test_parse_and_or() {
        let ast = assert_program_root("SELECT x FROM t WHERE a = 1 AND b = 2 OR c = 3");
        assert!(find_rule(&ast, "or_expr"), "Expected or_expr");
        assert!(find_rule(&ast, "and_expr"), "Expected and_expr");
    }

    // -----------------------------------------------------------------------
    // Test 19: LIKE and IS NULL
    // -----------------------------------------------------------------------

    /// LIKE and IS NULL are special comparison forms.
    ///
    /// `SELECT x FROM t WHERE name LIKE 'A%' AND addr IS NULL`
    #[test]
    fn test_parse_like_is_null() {
        let ast = assert_program_root("SELECT x FROM t WHERE name LIKE 'A%' AND addr IS NULL");
        assert!(find_rule(&ast, "comparison"), "Expected comparison");
    }

    // -----------------------------------------------------------------------
    // Test 20: BETWEEN expression
    // -----------------------------------------------------------------------

    /// BETWEEN exercises the special comparison form `x BETWEEN a AND b`.
    ///
    /// `SELECT x FROM t WHERE age BETWEEN 18 AND 65`
    #[test]
    fn test_parse_between() {
        let ast = assert_program_root("SELECT x FROM t WHERE age BETWEEN 18 AND 65");
        assert!(
            find_rule(&ast, "comparison"),
            "Expected comparison with BETWEEN"
        );
    }

    // -----------------------------------------------------------------------
    // Test 21: IN expression
    // -----------------------------------------------------------------------

    /// IN exercises the comparison form `x IN (v1, v2, ...)`.
    ///
    /// `SELECT x FROM t WHERE id IN (1, 2, 3)`
    #[test]
    fn test_parse_in() {
        let ast = assert_program_root("SELECT x FROM t WHERE id IN (1, 2, 3)");
        assert!(find_rule(&ast, "comparison"), "Expected comparison with IN");
        assert!(find_rule(&ast, "value_list"), "Expected value_list");
    }

    // -----------------------------------------------------------------------
    // Test 22: Function call in expression
    // -----------------------------------------------------------------------

    /// Function calls like `COUNT(*)` exercise the function_call rule.
    ///
    /// `SELECT COUNT(*) FROM users`
    #[test]
    fn test_parse_function_call() {
        let ast = assert_program_root("SELECT COUNT(*) FROM users");
        assert!(find_rule(&ast, "function_call"), "Expected function_call");
    }

    // -----------------------------------------------------------------------
    // Test 23: Qualified column reference (table.column)
    // -----------------------------------------------------------------------

    /// Qualified references exercise `column_ref = NAME [ DOT NAME ]`.
    /// The table alias requires the AS keyword per the grammar:
    /// `table_ref = table_name [ "AS" NAME ]`
    ///
    /// `SELECT u.name FROM users AS u`
    #[test]
    fn test_parse_qualified_column() {
        let ast = assert_program_root("SELECT u.name FROM users AS u");
        assert!(find_rule(&ast, "column_ref"), "Expected column_ref");
    }

    // -----------------------------------------------------------------------
    // Test 24: Multiple statements separated by semicolons
    // -----------------------------------------------------------------------

    /// The `program` rule handles multiple statements:
    /// `program = statement { ";" statement } [ ";" ]`
    ///
    /// `SELECT 1; SELECT 2`
    #[test]
    fn test_parse_multiple_statements() {
        let ast = assert_program_root("SELECT 1 FROM a; SELECT 2 FROM b");
        // Both select_stmts should be in the AST.
        assert!(find_rule(&ast, "select_stmt"), "Expected select_stmt");
    }

    // -----------------------------------------------------------------------
    // Test 25: Trailing semicolon
    // -----------------------------------------------------------------------

    /// A trailing semicolon after the last statement is optional.
    /// `program = statement { ";" statement } [ ";" ]`
    #[test]
    fn test_parse_trailing_semicolon() {
        let ast = assert_program_root("SELECT * FROM t;");
        assert!(
            find_rule(&ast, "select_stmt"),
            "Expected select_stmt with trailing semicolon"
        );
    }

    // -----------------------------------------------------------------------
    // Test 26: NULL, TRUE, FALSE literals in expressions
    // -----------------------------------------------------------------------

    /// NULL, TRUE, FALSE are SQL keywords that appear as primary expressions.
    ///
    /// `SELECT NULL, TRUE, FALSE FROM t`
    #[test]
    fn test_parse_null_true_false() {
        let ast = assert_program_root("SELECT NULL, TRUE, FALSE FROM t");
        assert!(
            find_rule(&ast, "primary"),
            "Expected primary node for literals"
        );
    }

    // -----------------------------------------------------------------------
    // Test 27: factory function create_sql_parser
    // -----------------------------------------------------------------------

    /// `create_sql_parser` returns `Ok(GrammarParser)` on valid SQL and
    /// the parser can be used directly.
    #[test]
    fn test_create_sql_parser() {
        let result = create_sql_parser("SELECT 1 FROM t");
        assert!(
            result.is_ok(),
            "Expected Ok parser, got: {:?}",
            result.err()
        );
        let mut parser = result.unwrap();
        let parse_result = parser.parse();
        assert!(parse_result.is_ok(), "Expected successful parse");
        assert_eq!(parse_result.unwrap().rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 28: Error path — invalid SQL
    // -----------------------------------------------------------------------

    /// A syntactically invalid SQL statement should return `Err`.
    ///
    /// | Input         | Result |
    /// |---------------|--------|
    /// | "SELECT FROM" | Err(_) |
    ///
    /// This is not a valid SELECT because there is no select_list before FROM.
    #[test]
    fn test_parse_invalid_sql() {
        // "SELECT FROM" is missing the select_list — should fail to parse.
        let result = parse_sql("SELECT FROM");
        assert!(
            result.is_err(),
            "Expected Err for invalid SQL 'SELECT FROM'"
        );
    }

    // -----------------------------------------------------------------------
    // Test 29: Error path — tokenization failure propagates
    // -----------------------------------------------------------------------

    /// A source string that cannot be tokenized should cause `create_sql_parser`
    /// to return Err before even attempting to build the grammar.
    ///
    /// The `@` character is not a valid SQL token, so tokenization fails.
    #[test]
    fn test_parse_tokenization_error_propagates() {
        // "@" is not in the sql.tokens grammar — tokenization should fail.
        let result = parse_sql("SELECT @ FROM t");
        assert!(
            result.is_err(),
            "Expected Err when source has an invalid character"
        );
    }

    // -----------------------------------------------------------------------
    // Test 30: INSERT with multiple rows
    // -----------------------------------------------------------------------

    /// INSERT with multiple row_value clauses.
    ///
    /// `INSERT INTO t VALUES (1), (2), (3)`
    #[test]
    fn test_parse_insert_multiple_rows() {
        let ast = assert_program_root("INSERT INTO t VALUES (1), (2), (3)");
        assert!(find_rule(&ast, "insert_stmt"), "Expected insert_stmt");
        assert!(find_rule(&ast, "row_value"), "Expected row_value");
    }

    // -----------------------------------------------------------------------
    // Test 31: UPDATE with multiple assignments
    // -----------------------------------------------------------------------

    /// SET clause with multiple column assignments.
    ///
    /// `UPDATE t SET a = 1, b = 2 WHERE id = 3`
    #[test]
    fn test_parse_update_multiple_assignments() {
        let ast = assert_program_root("UPDATE t SET a = 1, b = 2 WHERE id = 3");
        assert!(find_rule(&ast, "update_stmt"), "Expected update_stmt");
        assert!(find_rule(&ast, "assignment"), "Expected assignment");
    }

    // -----------------------------------------------------------------------
    // Test 32: SELECT DISTINCT
    // -----------------------------------------------------------------------

    /// DISTINCT is an optional qualifier in the select_stmt rule:
    /// `select_stmt = SELECT [ DISTINCT | ALL ] select_list ...`
    ///
    /// `SELECT DISTINCT name FROM users`
    #[test]
    fn test_parse_select_distinct() {
        let ast = assert_program_root("SELECT DISTINCT name FROM users");
        assert!(
            find_rule(&ast, "select_stmt"),
            "Expected select_stmt with DISTINCT"
        );
    }

    // -----------------------------------------------------------------------
    // Test 33: Parenthesized expression
    // -----------------------------------------------------------------------

    /// Parenthesized expressions exercise `primary = "(" expr ")"`.
    ///
    /// `SELECT (a + b) * c FROM t`
    #[test]
    fn test_parse_parenthesized_expr() {
        let ast = assert_program_root("SELECT (a + b) * c FROM t");
        assert!(find_rule(&ast, "multiplicative"), "Expected multiplicative");
    }
}

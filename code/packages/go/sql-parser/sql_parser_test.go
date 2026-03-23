package sqlparser

import (
	"os"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// =============================================================================
// TestParseSimpleSelect
// =============================================================================
//
// Verifies that a basic SELECT statement parses into an AST rooted at "program".
// This is the entry point rule in sql.grammar. Any valid SQL input should
// produce a root node named "program".
func TestParseSimpleSelect(t *testing.T) {
	source := "SELECT id FROM users"
	program, err := ParseSQL(source)
	if err != nil {
		t.Fatalf("ParseSQL failed: %v", err)
	}

	if program.RuleName != "program" {
		t.Fatalf("Expected root rule 'program', got %q", program.RuleName)
	}
	if len(program.Children) == 0 {
		t.Error("Expected program node to have children")
	}
}

// =============================================================================
// TestParseCaseInsensitiveKeywords
// =============================================================================
//
// SQL keywords must be accepted in any case. The grammar uses KEYWORD tokens
// whose values are normalized to uppercase by the lexer. This means grammar
// literals like "SELECT" match select/SELECT/Select equally.
//
// This test verifies that lowercase SQL parses identically to uppercase SQL.
func TestParseCaseInsensitiveKeywords(t *testing.T) {
	// All three should parse without error and produce identical root rules
	sources := []string{
		"SELECT id FROM users",
		"select id from users",
		"Select Id From Users",
	}

	for _, source := range sources {
		program, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
		if program.RuleName != "program" {
			t.Errorf("Expected root rule 'program' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseSelectStar
// =============================================================================
//
// SELECT * is a common shorthand that selects all columns. The STAR token is
// used in three different contexts in SQL: SELECT *, COUNT(*), and as the
// multiplication operator. The grammar disambiguates by position.
func TestParseSelectStar(t *testing.T) {
	source := "SELECT * FROM orders"
	_, err := ParseSQL(source)
	if err != nil {
		t.Fatalf("ParseSQL(%q) failed: %v", source, err)
	}
}

// =============================================================================
// TestParseSelectWithWhere
// =============================================================================
//
// Verifies that a WHERE clause is parsed. The WHERE clause contains an
// expression that can include comparisons, boolean logic, and arithmetic.
func TestParseSelectWithWhere(t *testing.T) {
	source := "SELECT id FROM users WHERE active = TRUE"
	_, err := ParseSQL(source)
	if err != nil {
		t.Fatalf("ParseSQL(%q) failed: %v", source, err)
	}
}

// =============================================================================
// TestParseSelectWithMultipleColumns
// =============================================================================
//
// Verifies that multiple comma-separated columns in the select list are parsed
// correctly. The grammar uses `select_item { "," select_item }` which allows
// an arbitrary number of columns.
func TestParseSelectWithMultipleColumns(t *testing.T) {
	source := "SELECT id, name, email FROM users"
	_, err := ParseSQL(source)
	if err != nil {
		t.Fatalf("ParseSQL(%q) failed: %v", source, err)
	}
}

// =============================================================================
// TestParseSelectWithAlias
// =============================================================================
//
// Verifies that column and table aliases (AS) are parsed. Aliases are optional
// and appear after expressions in select_item and after table_name in table_ref.
func TestParseSelectWithAlias(t *testing.T) {
	sources := []string{
		"SELECT id AS user_id FROM users",
		"SELECT u.name FROM users AS u",
		"SELECT u.name AS n FROM users AS u",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseSelectWithOrderBy
// =============================================================================
//
// Verifies that ORDER BY clauses are parsed. ORDER BY can have multiple items,
// each optionally followed by ASC or DESC. The default direction when omitted
// is ASC (standard SQL), but the grammar accepts either.
func TestParseSelectWithOrderBy(t *testing.T) {
	sources := []string{
		"SELECT id FROM users ORDER BY id",
		"SELECT id FROM users ORDER BY name ASC",
		"SELECT id FROM users ORDER BY created DESC",
		"SELECT id FROM users ORDER BY name ASC, id DESC",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseSelectWithLimit
// =============================================================================
//
// Verifies that LIMIT and LIMIT ... OFFSET clauses are parsed. These restrict
// the number of rows returned and are used for pagination.
func TestParseSelectWithLimit(t *testing.T) {
	sources := []string{
		"SELECT id FROM users LIMIT 10",
		"SELECT id FROM users LIMIT 10 OFFSET 20",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseSelectWithGroupBy
// =============================================================================
//
// Verifies that GROUP BY and HAVING clauses are parsed. GROUP BY collapses
// rows with the same group key; HAVING filters aggregated groups.
func TestParseSelectWithGroupBy(t *testing.T) {
	sources := []string{
		"SELECT dept, COUNT(*) FROM employees GROUP BY dept",
		"SELECT dept, COUNT(*) FROM employees GROUP BY dept HAVING COUNT(*) > 5",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseSelectWithJoin
// =============================================================================
//
// Verifies that JOIN clauses are parsed. SQL supports multiple join types:
// INNER JOIN, LEFT [OUTER] JOIN, RIGHT [OUTER] JOIN, FULL [OUTER] JOIN, CROSS JOIN.
// Each join requires an ON condition (except CROSS JOIN which takes no ON).
func TestParseSelectWithJoin(t *testing.T) {
	sources := []string{
		"SELECT u.name, o.total FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id",
		"SELECT u.name FROM users AS u LEFT JOIN orders AS o ON u.id = o.user_id",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseInsert
// =============================================================================
//
// Verifies that INSERT INTO ... VALUES statements are parsed. The INSERT
// statement supports an optional column list before the VALUES keyword.
func TestParseInsert(t *testing.T) {
	sources := []string{
		"INSERT INTO users VALUES ('Alice', 30)",
		"INSERT INTO users (name, age) VALUES ('Bob', 25)",
		"INSERT INTO users (name) VALUES ('Carol'), ('Dave')",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseUpdate
// =============================================================================
//
// Verifies that UPDATE ... SET ... WHERE statements are parsed. The WHERE
// clause is optional; without it, all rows are updated.
func TestParseUpdate(t *testing.T) {
	sources := []string{
		"UPDATE users SET active = FALSE",
		"UPDATE users SET name = 'Alice', age = 31 WHERE id = 1",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseDelete
// =============================================================================
//
// Verifies that DELETE FROM statements are parsed. The WHERE clause is optional.
func TestParseDelete(t *testing.T) {
	sources := []string{
		"DELETE FROM users",
		"DELETE FROM users WHERE id = 1",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseCreateTable
// =============================================================================
//
// Verifies that CREATE TABLE statements are parsed. Column definitions can
// include type names and constraints (NOT NULL, NULL, PRIMARY KEY, UNIQUE,
// DEFAULT <value>).
func TestParseCreateTable(t *testing.T) {
	sources := []string{
		"CREATE TABLE users (id INTEGER, name VARCHAR)",
		"CREATE TABLE IF NOT EXISTS orders (id INTEGER PRIMARY KEY, total NUMBER NOT NULL)",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseDropTable
// =============================================================================
//
// Verifies that DROP TABLE statements are parsed. The IF EXISTS guard prevents
// errors when the table does not exist.
func TestParseDropTable(t *testing.T) {
	sources := []string{
		"DROP TABLE users",
		"DROP TABLE IF EXISTS temp_data",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseExpressions
// =============================================================================
//
// Verifies that complex SQL expressions are parsed. SQL expressions include
// arithmetic, comparisons, boolean logic (AND/OR/NOT), and special forms
// like BETWEEN, IN, LIKE, and IS NULL.
func TestParseExpressions(t *testing.T) {
	sources := []string{
		// Arithmetic
		"SELECT a + b * c FROM t",
		"SELECT (a + b) * c FROM t",
		// Comparisons
		"SELECT id FROM t WHERE x != y",
		"SELECT id FROM t WHERE x <> y",
		"SELECT id FROM t WHERE x <= 100",
		// Boolean logic
		"SELECT id FROM t WHERE x = 1 AND y = 2",
		"SELECT id FROM t WHERE x = 1 OR y = 2",
		"SELECT id FROM t WHERE NOT active = TRUE",
		// BETWEEN
		"SELECT id FROM t WHERE age BETWEEN 18 AND 65",
		// IN
		"SELECT id FROM t WHERE status IN (1, 2, 3)",
		// LIKE
		"SELECT id FROM t WHERE name LIKE '%alice%'",
		// IS NULL
		"SELECT id FROM t WHERE email IS NULL",
		"SELECT id FROM t WHERE email IS NOT NULL",
		// Function call
		"SELECT COUNT(*) FROM orders",
		"SELECT MAX(price) FROM products",
		// Unary minus
		"SELECT -price FROM products",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v\n", source, err)
		}
	}
}

// =============================================================================
// TestParseMultipleStatements
// =============================================================================
//
// Verifies that multiple SQL statements separated by semicolons are parsed.
// The grammar rule `program = statement { ";" statement } [ ";" ]` allows
// this. A trailing semicolon is optional.
func TestParseMultipleStatements(t *testing.T) {
	sources := []string{
		"SELECT * FROM a; SELECT * FROM b",
		"SELECT * FROM a; SELECT * FROM b;",
		"INSERT INTO log VALUES (1); DELETE FROM temp WHERE id = 1",
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err != nil {
			t.Fatalf("ParseSQL(%q) failed: %v", source, err)
		}
	}
}

// =============================================================================
// TestParseInvalidSQL
// =============================================================================
//
// Verifies that invalid SQL input returns an error instead of silently producing
// a malformed AST. The parser should reject syntactically incorrect input.
func TestParseInvalidSQL(t *testing.T) {
	sources := []string{
		"SELECT FROM",          // missing select list
		"INSERT users VALUES", // missing INTO
		"",                    // empty input
	}

	for _, source := range sources {
		_, err := ParseSQL(source)
		if err == nil {
			t.Errorf("ParseSQL(%q) should have failed but returned nil error", source)
		}
	}
}

// =============================================================================
// TestCreateSQLParser
// =============================================================================
//
// Verifies that CreateSQLParser returns a non-nil parser and that Parse()
// can be called on the returned parser.
func TestCreateSQLParser(t *testing.T) {
	sqlParser, err := CreateSQLParser("SELECT 1 FROM dual")
	if err != nil {
		t.Fatalf("CreateSQLParser failed: %v", err)
	}
	if sqlParser == nil {
		t.Fatal("CreateSQLParser returned nil")
	}

	ast, err := sqlParser.Parse()
	if err != nil {
		t.Fatalf("Parse() failed: %v", err)
	}
	if ast == nil {
		t.Fatal("Parse() returned nil AST")
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected root rule 'program', got %q", ast.RuleName)
	}
}

// =============================================================================
// TestCreateSQLParserErrorMissingGrammarFile
// =============================================================================
//
// Verifies that CreateSQLParser returns an error when the grammar file cannot
// be found. This exercises the os.ReadFile error path that is otherwise
// unreachable in normal operation.
//
// We use the package-level sqlGrammarPath override to point at a non-existent
// file, then restore it after the test with defer.
func TestCreateSQLParserErrorMissingGrammarFile(t *testing.T) {
	original := sqlGrammarPath
	sqlGrammarPath = "/does/not/exist/sql.grammar"
	defer func() { sqlGrammarPath = original }()

	_, err := CreateSQLParser("SELECT 1")
	if err == nil {
		t.Error("Expected error for missing grammar file, got nil")
	}
}

// =============================================================================
// TestParseSQLErrorPropagates
// =============================================================================
//
// Verifies that ParseSQL propagates errors from CreateSQLParser. This covers
// the `if err != nil { return nil, err }` path inside ParseSQL, which is
// otherwise unreachable because the grammar file is always present.
func TestParseSQLErrorPropagates(t *testing.T) {
	original := sqlGrammarPath
	sqlGrammarPath = "/does/not/exist/sql.grammar"
	defer func() { sqlGrammarPath = original }()

	_, err := ParseSQL("SELECT 1")
	if err == nil {
		t.Error("Expected error for missing grammar file, got nil")
	}
}

// =============================================================================
// TestCreateSQLParserErrorInvalidGrammar
// =============================================================================
//
// Verifies that CreateSQLParser returns an error when the grammar file exists
// but contains invalid EBNF. This covers the ParseParserGrammar error path.
func TestCreateSQLParserErrorInvalidGrammar(t *testing.T) {
	tmp := t.TempDir()
	badPath := tmp + "/bad.grammar"
	if err := os.WriteFile(badPath, []byte("not valid EBNF %%%\n"), 0o644); err != nil {
		t.Fatalf("Failed to write temp grammar: %v", err)
	}

	original := sqlGrammarPath
	sqlGrammarPath = badPath
	defer func() { sqlGrammarPath = original }()

	_, err := CreateSQLParser("SELECT 1")
	if err == nil {
		t.Error("Expected error for invalid grammar content, got nil")
	}
}

// =============================================================================
// TestASTStructure
// =============================================================================
//
// Verifies that the AST produced by ParseSQL has the expected shape for a
// simple SELECT query. We don't check every node, but verify that key rules
// appear in the tree, confirming that the parse correctly constructs the
// hierarchical AST.
func TestASTStructure(t *testing.T) {
	source := "SELECT id FROM users WHERE id = 1"
	program, err := ParseSQL(source)
	if err != nil {
		t.Fatalf("ParseSQL failed: %v", err)
	}

	// The AST should contain certain rule names somewhere in the tree.
	// findRule recurses the *parser.ASTNode tree using the exported RuleName field.
	requiredRules := []string{"program", "select_stmt", "where_clause"}
	for _, rule := range requiredRules {
		if !findRule(program, rule) {
			t.Errorf("AST should contain rule %q but it was not found", rule)
		}
	}
}

// =============================================================================
// Helpers
// =============================================================================

// findRule performs a depth-first search of the AST looking for a node with
// the given rule name. Returns true if found, false otherwise.
//
// ASTNode.Children is []interface{} where each element is either *parser.ASTNode
// or a lexer.Token leaf. We recurse only into *parser.ASTNode children.
func findRule(node *parser.ASTNode, ruleName string) bool {
	if node == nil {
		return false
	}
	if node.RuleName == ruleName {
		return true
	}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			if findRule(childNode, ruleName) {
				return true
			}
		}
	}
	return false
}

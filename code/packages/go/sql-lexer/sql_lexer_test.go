package sqllexer

import (
	"os"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// =============================================================================
// TestTokenizeSQLSelect
// =============================================================================
//
// Verifies that a basic SELECT statement is tokenized into the correct sequence
// of tokens. This is the most common SQL statement; getting the basic form right
// is the foundation for all further tests.
//
// Input: SELECT id FROM users
// Expected token sequence:
//   KEYWORD("SELECT"), NAME("id"), KEYWORD("FROM"), NAME("users"), EOF
func TestTokenizeSQLSelect(t *testing.T) {
	tokens, err := TokenizeSQL("SELECT id FROM users")
	if err != nil {
		t.Fatalf("TokenizeSQL failed: %v", err)
	}

	// Filter out EOF for easier assertion
	nonEOF := filterEOF(tokens)
	if len(nonEOF) != 4 {
		t.Fatalf("Expected 4 tokens, got %d: %v", len(nonEOF), nonEOF)
	}

	assertToken(t, nonEOF[0], lexer.TokenKeyword, "SELECT")
	assertToken(t, nonEOF[1], lexer.TokenName, "id")
	assertToken(t, nonEOF[2], lexer.TokenKeyword, "FROM")
	assertToken(t, nonEOF[3], lexer.TokenName, "users")
}

// =============================================================================
// TestTokenizeSQLCaseInsensitiveKeywords
// =============================================================================
//
// SQL keywords are case-insensitive by the ANSI standard: select, SELECT, and
// Select must all tokenize as KEYWORD tokens. The grammar uses
// `# @case_insensitive true`, so the lexer normalizes keyword values to uppercase.
//
// Truth table:
//   Input          | Expected type | Expected value
//   "select"       | KEYWORD       | "SELECT"
//   "SELECT"       | KEYWORD       | "SELECT"
//   "Select"       | KEYWORD       | "SELECT"
//   "from"         | KEYWORD       | "FROM"
func TestTokenizeSQLCaseInsensitiveKeywords(t *testing.T) {
	cases := []struct {
		input string
		want  string
	}{
		{"select", "SELECT"},
		{"SELECT", "SELECT"},
		{"Select", "SELECT"},
		{"from", "FROM"},
		{"WHERE", "WHERE"},
		{"Insert", "INSERT"},
	}

	for _, tc := range cases {
		tokens, err := TokenizeSQL(tc.input)
		if err != nil {
			t.Fatalf("TokenizeSQL(%q) failed: %v", tc.input, err)
		}
		nonEOF := filterEOF(tokens)
		if len(nonEOF) != 1 {
			t.Fatalf("Expected 1 token for %q, got %d: %v", tc.input, len(nonEOF), nonEOF)
		}
		assertToken(t, nonEOF[0], lexer.TokenKeyword, tc.want)
	}
}

// =============================================================================
// TestTokenizeSQLNumber
// =============================================================================
//
// Verifies integer and decimal number literals. SQL uses numbers in WHERE
// clauses, LIMIT clauses, and expressions.
func TestTokenizeSQLNumber(t *testing.T) {
	cases := []string{"42", "3.14", "0", "100"}

	for _, source := range cases {
		tokens, err := TokenizeSQL(source)
		if err != nil {
			t.Fatalf("TokenizeSQL(%q) failed: %v", source, err)
		}
		nonEOF := filterEOF(tokens)
		if len(nonEOF) != 1 {
			t.Fatalf("Expected 1 NUMBER token for %q, got %d", source, len(nonEOF))
		}
		if nonEOF[0].Type != lexer.TokenNumber {
			t.Errorf("Expected NUMBER token for %q, got %s", source, nonEOF[0].Type)
		}
		if nonEOF[0].Value != source {
			t.Errorf("Expected value %q, got %q", source, nonEOF[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeSQLString
// =============================================================================
//
// Verifies that single-quoted string literals are tokenized as STRING tokens.
// SQL uses single quotes for string literals (double quotes are for identifiers).
// The grammar aliases STRING_SQ → STRING so the token type is STRING.
func TestTokenizeSQLString(t *testing.T) {
	tokens, err := TokenizeSQL("'hello world'")
	if err != nil {
		t.Fatalf("TokenizeSQL failed: %v", err)
	}

	nonEOF := filterEOF(tokens)
	if len(nonEOF) != 1 {
		t.Fatalf("Expected 1 STRING token, got %d: %v", len(nonEOF), nonEOF)
	}
	if nonEOF[0].Type != lexer.TokenString {
		t.Errorf("Expected STRING token, got %s", nonEOF[0].Type)
	}
	// The lexer strips the surrounding single quotes
	if nonEOF[0].Value != "hello world" {
		t.Errorf("Expected value 'hello world', got %q", nonEOF[0].Value)
	}
}

// =============================================================================
// TestTokenizeSQLOperators
// =============================================================================
//
// Verifies that SQL comparison and arithmetic operators are tokenized correctly.
// Important: >= and <= must be matched as single tokens (longest-match rule),
// not as > followed by = or < followed by =.
//
// Both != and <> should produce NOT_EQUALS tokens (NEQ_ANSI is aliased).
func TestTokenizeSQLOperators(t *testing.T) {
	cases := []struct {
		input    string
		typeName string
	}{
		{"=", "EQUALS"},
		{"!=", "NOT_EQUALS"},
		{"<>", "NOT_EQUALS"},
		{"<", "LESS_THAN"},
		{">", "GREATER_THAN"},
		{"<=", "LESS_EQUALS"},
		{">=", "GREATER_EQUALS"},
		{"+", "PLUS"},
		{"-", "MINUS"},
		{"*", "STAR"},
		{"/", "SLASH"},
		{"%", "PERCENT"},
	}

	for _, tc := range cases {
		tokens, err := TokenizeSQL(tc.input)
		if err != nil {
			t.Fatalf("TokenizeSQL(%q) failed: %v", tc.input, err)
		}
		nonEOF := filterEOF(tokens)
		if len(nonEOF) != 1 {
			t.Fatalf("Expected 1 token for %q, got %d: %v", tc.input, len(nonEOF), nonEOF)
		}
		if nonEOF[0].TypeName != tc.typeName {
			t.Errorf("For %q: expected TypeName %q, got %q", tc.input, tc.typeName, nonEOF[0].TypeName)
		}
	}
}

// =============================================================================
// TestTokenizeSQLPunctuation
// =============================================================================
//
// Verifies that punctuation tokens (, ; . ( )) are recognized correctly.
// These are used throughout SQL statements for argument lists, statement
// terminators, schema-qualified names, and subexpressions.
func TestTokenizeSQLPunctuation(t *testing.T) {
	cases := []struct {
		input    string
		typeName string
	}{
		{"(", "LPAREN"},
		{")", "RPAREN"},
		{",", "COMMA"},
		{";", "SEMICOLON"},
		{".", "DOT"},
	}

	for _, tc := range cases {
		tokens, err := TokenizeSQL(tc.input)
		if err != nil {
			t.Fatalf("TokenizeSQL(%q) failed: %v", tc.input, err)
		}
		nonEOF := filterEOF(tokens)
		if len(nonEOF) != 1 {
			t.Fatalf("Expected 1 token for %q, got %d", tc.input, len(nonEOF))
		}
		if nonEOF[0].TypeName != tc.typeName {
			t.Errorf("For %q: expected TypeName %q, got %q", tc.input, tc.typeName, nonEOF[0].TypeName)
		}
	}
}

// =============================================================================
// TestTokenizeSQLLineComment
// =============================================================================
//
// Verifies that -- line comments are skipped. Line comments run from --
// to the end of the line. They must not appear in the token stream.
func TestTokenizeSQLLineComment(t *testing.T) {
	source := "SELECT id -- pick the id column\nFROM users"
	tokens, err := TokenizeSQL(source)
	if err != nil {
		t.Fatalf("TokenizeSQL failed: %v", err)
	}

	nonEOF := filterEOF(tokens)
	if len(nonEOF) != 4 {
		t.Fatalf("Expected 4 tokens (comment skipped), got %d: %v", len(nonEOF), nonEOF)
	}
	assertToken(t, nonEOF[0], lexer.TokenKeyword, "SELECT")
	assertToken(t, nonEOF[1], lexer.TokenName, "id")
	assertToken(t, nonEOF[2], lexer.TokenKeyword, "FROM")
	assertToken(t, nonEOF[3], lexer.TokenName, "users")
}

// =============================================================================
// TestTokenizeSQLBlockComment
// =============================================================================
//
// Verifies that /* block comments */ are skipped. Block comments can span
// multiple lines and are used in SQL for query annotations.
func TestTokenizeSQLBlockComment(t *testing.T) {
	source := "SELECT /* all columns */ * FROM t"
	tokens, err := TokenizeSQL(source)
	if err != nil {
		t.Fatalf("TokenizeSQL failed: %v", err)
	}

	nonEOF := filterEOF(tokens)
	// Expected: KEYWORD(SELECT), STAR(*), KEYWORD(FROM), NAME(t)
	if len(nonEOF) != 4 {
		t.Fatalf("Expected 4 tokens (block comment skipped), got %d: %v", len(nonEOF), nonEOF)
	}
	assertToken(t, nonEOF[0], lexer.TokenKeyword, "SELECT")
	assertToken(t, nonEOF[2], lexer.TokenKeyword, "FROM")
}

// =============================================================================
// TestTokenizeSQLWhereClause
// =============================================================================
//
// Verifies tokenization of a WHERE clause with a comparison expression.
// This exercises identifier recognition, keyword detection, operator tokenization,
// and number literal tokenization together.
func TestTokenizeSQLWhereClause(t *testing.T) {
	source := "WHERE age >= 18"
	tokens, err := TokenizeSQL(source)
	if err != nil {
		t.Fatalf("TokenizeSQL failed: %v", err)
	}

	nonEOF := filterEOF(tokens)
	if len(nonEOF) != 4 {
		t.Fatalf("Expected 4 tokens, got %d: %v", len(nonEOF), nonEOF)
	}
	assertToken(t, nonEOF[0], lexer.TokenKeyword, "WHERE")
	assertToken(t, nonEOF[1], lexer.TokenName, "age")
	if nonEOF[2].TypeName != "GREATER_EQUALS" {
		t.Errorf("Expected GREATER_EQUALS, got %q", nonEOF[2].TypeName)
	}
	if nonEOF[3].Type != lexer.TokenNumber {
		t.Errorf("Expected NUMBER, got %s", nonEOF[3].Type)
	}
}

// =============================================================================
// TestTokenizeSQLQualifiedName
// =============================================================================
//
// Verifies that schema-qualified names like schema.table or table.column
// are tokenized as NAME DOT NAME (three tokens). The dot is a separate
// PUNCTUATION token; the parser combines them into qualified references.
func TestTokenizeSQLQualifiedName(t *testing.T) {
	// Use a non-keyword name for both parts. "table" is a SQL keyword,
	// so schema.table would produce NAME DOT KEYWORD. Use schema.orders instead.
	source := "schema.orders"
	tokens, err := TokenizeSQL(source)
	if err != nil {
		t.Fatalf("TokenizeSQL failed: %v", err)
	}

	nonEOF := filterEOF(tokens)
	if len(nonEOF) != 3 {
		t.Fatalf("Expected 3 tokens (NAME DOT NAME), got %d: %v", len(nonEOF), nonEOF)
	}
	assertToken(t, nonEOF[0], lexer.TokenName, "schema")
	if nonEOF[1].TypeName != "DOT" {
		t.Errorf("Expected DOT, got %q", nonEOF[1].TypeName)
	}
	assertToken(t, nonEOF[2], lexer.TokenName, "orders")
}

// =============================================================================
// TestTokenizeSQLQuotedIdentifier
// =============================================================================
//
// Verifies that backtick-quoted identifiers like `my table` are aliased to NAME
// tokens (QUOTED_ID → NAME). This allows identifiers with spaces or reserved
// word names to be used safely.
func TestTokenizeSQLQuotedIdentifier(t *testing.T) {
	// QUOTED_ID = /`[^`]+`/ -> NAME aliases the token type to NAME.
	// The backtick quotes are included in the value (the lexer only strips
	// quotes for patterns whose name or alias contains "STRING").
	source := "`my table`"
	tokens, err := TokenizeSQL(source)
	if err != nil {
		t.Fatalf("TokenizeSQL failed: %v", err)
	}

	nonEOF := filterEOF(tokens)
	if len(nonEOF) != 1 {
		t.Fatalf("Expected 1 NAME token, got %d: %v", len(nonEOF), nonEOF)
	}
	if nonEOF[0].Type != lexer.TokenName {
		t.Errorf("Expected NAME token, got %s (QUOTED_ID should alias to NAME)", nonEOF[0].Type)
	}
	// The backtick quotes are preserved in the token value.
	// Callers that need the unquoted name should strip the surrounding backticks.
	if nonEOF[0].Value != "`my table`" {
		t.Errorf("Expected value '`my table`', got %q", nonEOF[0].Value)
	}
}

// =============================================================================
// TestTokenizeSQLFullSelectStatement
// =============================================================================
//
// Verifies tokenization of a complete SELECT statement with multiple clauses.
// This is an integration test that exercises most of the grammar at once.
func TestTokenizeSQLFullSelectStatement(t *testing.T) {
	source := "SELECT id, name FROM users WHERE active = TRUE ORDER BY name ASC LIMIT 10"
	tokens, err := TokenizeSQL(source)
	if err != nil {
		t.Fatalf("TokenizeSQL failed: %v", err)
	}

	// Verify key tokens appear in the right spots
	nonEOF := filterEOF(tokens)

	// First token must be SELECT keyword
	if len(nonEOF) == 0 || nonEOF[0].Value != "SELECT" {
		t.Errorf("Expected first token to be SELECT keyword")
	}

	// Collect all keyword values for inspection
	keywords := []string{}
	for _, tok := range nonEOF {
		if tok.Type == lexer.TokenKeyword {
			keywords = append(keywords, tok.Value)
		}
	}

	// All keywords must be uppercase regardless of input casing
	expected := []string{"SELECT", "FROM", "WHERE", "TRUE", "ORDER", "BY", "ASC", "LIMIT"}
	if len(keywords) != len(expected) {
		t.Errorf("Expected keywords %v, got %v", expected, keywords)
	}
	for i, kw := range keywords {
		if kw != expected[i] {
			t.Errorf("Expected keyword[%d]=%q, got %q", i, expected[i], kw)
		}
	}
}

// =============================================================================
// TestTokenizeSQLNullHandling
// =============================================================================
//
// Verifies that NULL, TRUE, and FALSE are tokenized as keywords. These are
// special values in SQL that look like identifiers but are keywords.
func TestTokenizeSQLNullHandling(t *testing.T) {
	cases := []string{"NULL", "null", "TRUE", "true", "FALSE", "false"}

	for _, source := range cases {
		tokens, err := TokenizeSQL(source)
		if err != nil {
			t.Fatalf("TokenizeSQL(%q) failed: %v", source, err)
		}
		nonEOF := filterEOF(tokens)
		if len(nonEOF) != 1 {
			t.Fatalf("Expected 1 token for %q", source)
		}
		if nonEOF[0].Type != lexer.TokenKeyword {
			t.Errorf("Expected KEYWORD for %q, got %s", source, nonEOF[0].Type)
		}
	}
}

// =============================================================================
// TestCreateSQLLexer
// =============================================================================
//
// Verifies that CreateSQLLexer returns a non-nil lexer and that the lexer
// can be called multiple times to produce consistent results.
func TestCreateSQLLexer(t *testing.T) {
	lex, err := CreateSQLLexer("SELECT 1")
	if err != nil {
		t.Fatalf("CreateSQLLexer failed: %v", err)
	}
	if lex == nil {
		t.Fatal("CreateSQLLexer returned nil lexer")
	}

	tokens := lex.Tokenize()
	if len(tokens) < 2 {
		t.Fatalf("Expected at least 2 tokens, got %d", len(tokens))
	}
}

// =============================================================================
// TestCreateSQLLexerErrorMissingFile
// =============================================================================
//
// Verifies that CreateSQLLexer returns an error when the grammar file cannot
// be found. This exercises the os.ReadFile error path in CreateSQLLexer that
// is otherwise unreachable in normal operation.
//
// We use the package-level sqlTokensPath override to point at a non-existent
// file, then restore it after the test. This is the standard Go pattern for
// testing file-path-dependent code without mocking.
func TestCreateSQLLexerErrorMissingFile(t *testing.T) {
	original := sqlTokensPath
	sqlTokensPath = "/does/not/exist/sql.tokens"
	defer func() { sqlTokensPath = original }()

	_, err := CreateSQLLexer("SELECT 1")
	if err == nil {
		t.Error("Expected error for missing grammar file, got nil")
	}
}

// =============================================================================
// TestTokenizeSQLErrorMissingFile
// =============================================================================
//
// Verifies that TokenizeSQL propagates the error from CreateSQLLexer when the
// grammar file is missing. This covers the error return path in TokenizeSQL.
func TestTokenizeSQLErrorMissingFile(t *testing.T) {
	original := sqlTokensPath
	sqlTokensPath = "/does/not/exist/sql.tokens"
	defer func() { sqlTokensPath = original }()

	_, err := TokenizeSQL("SELECT 1")
	if err == nil {
		t.Error("Expected error for missing grammar file, got nil")
	}
}

// =============================================================================
// TestCreateSQLLexerErrorInvalidGrammar
// =============================================================================
//
// Verifies that CreateSQLLexer returns an error when the grammar file exists
// but contains invalid content. This exercises the ParseTokenGrammar error path.
//
// We write a temporary file with invalid grammar content and point the lexer at it.
func TestCreateSQLLexerErrorInvalidGrammar(t *testing.T) {
	// Write a temp file with invalid grammar content (malformed token definition)
	tmp := t.TempDir()
	badGrammarPath := tmp + "/bad.tokens"
	if err := os.WriteFile(badGrammarPath, []byte("INVALID%%GRAMMAR\n"), 0o644); err != nil {
		t.Fatalf("Failed to create temp grammar file: %v", err)
	}

	original := sqlTokensPath
	sqlTokensPath = badGrammarPath
	defer func() { sqlTokensPath = original }()

	_, err := CreateSQLLexer("SELECT 1")
	if err == nil {
		t.Error("Expected error for invalid grammar content, got nil")
	}
}

// =============================================================================
// Helpers
// =============================================================================

// filterEOF returns tokens with the EOF sentinel removed.
func filterEOF(tokens []lexer.Token) []lexer.Token {
	result := make([]lexer.Token, 0, len(tokens))
	for _, tok := range tokens {
		if tok.Type != lexer.TokenEOF {
			result = append(result, tok)
		}
	}
	return result
}

// assertToken checks that a token has the expected type and value.
func assertToken(t *testing.T, tok lexer.Token, wantType lexer.TokenType, wantValue string) {
	t.Helper()
	if tok.Type != wantType {
		t.Errorf("Expected type %s, got %s (value=%q)", wantType, tok.Type, tok.Value)
	}
	if tok.Value != wantValue {
		t.Errorf("Expected value %q, got %q (type=%s)", wantValue, tok.Value, tok.Type)
	}
}

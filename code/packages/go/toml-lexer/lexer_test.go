package tomllexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// =============================================================================
// TestTokenizeTOMLBasicString
// =============================================================================
//
// Verifies that a basic (double-quoted) string is tokenized correctly.
// TOML basic strings support escape sequences (\n, \t, \\, \", \uXXXX,
// \UXXXXXXXX), but the lexer operates in escapes: none mode — it strips
// the surrounding quotes but leaves escape sequences as raw text for the
// parser's semantic layer to process.
func TestTokenizeTOMLBasicString(t *testing.T) {
	source := `key = "hello world"`
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize TOML basic string: %v", err)
	}

	foundString := false
	for _, tok := range tokens {
		if tok.TypeName == "BASIC_STRING" {
			foundString = true
			if tok.Value != "hello world" {
				t.Errorf("Expected string value 'hello world', got %q", tok.Value)
			}
			break
		}
	}
	if !foundString {
		t.Errorf("No BASIC_STRING token found in tokens: %v", tokens)
	}
}

// =============================================================================
// TestTokenizeTOMLBasicStringWithEscapes
// =============================================================================
//
// Verifies that escape sequences in basic strings are preserved as raw text.
// The lexer uses escapes: none mode, so \n stays as the two characters \ and n
// rather than being converted to a real newline character.
func TestTokenizeTOMLBasicStringWithEscapes(t *testing.T) {
	source := `key = "hello\nworld"`
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize TOML string with escapes: %v", err)
	}

	foundString := false
	for _, tok := range tokens {
		if tok.TypeName == "BASIC_STRING" {
			foundString = true
			// In escapes: none mode, \n stays as raw backslash-n (2 chars)
			if tok.Value != `hello\nworld` {
				t.Errorf("Expected raw escape 'hello\\nworld', got %q", tok.Value)
			}
			break
		}
	}
	if !foundString {
		t.Errorf("No BASIC_STRING token found in tokens: %v", tokens)
	}
}

// =============================================================================
// TestTokenizeTOMLLiteralString
// =============================================================================
//
// Verifies that a literal (single-quoted) string is tokenized correctly.
// TOML literal strings do not support any escape sequences — what you see
// is what you get. Backslashes are literal characters, not escape prefixes.
func TestTokenizeTOMLLiteralString(t *testing.T) {
	source := `path = 'C:\Users\docs'`
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize TOML literal string: %v", err)
	}

	foundString := false
	for _, tok := range tokens {
		if tok.TypeName == "LITERAL_STRING" {
			foundString = true
			if tok.Value != `C:\Users\docs` {
				t.Errorf("Expected literal value 'C:\\Users\\docs', got %q", tok.Value)
			}
			break
		}
	}
	if !foundString {
		t.Errorf("No LITERAL_STRING token found in tokens: %v", tokens)
	}
}

// =============================================================================
// TestTokenizeTOMLMultiLineBasicString
// =============================================================================
//
// Verifies that triple-double-quoted multi-line basic strings are tokenized.
// The lexer strips the triple quotes but leaves escape sequences as raw text.
func TestTokenizeTOMLMultiLineBasicString(t *testing.T) {
	source := "key = \"\"\"hello\nworld\"\"\""
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize TOML multi-line basic string: %v", err)
	}

	foundString := false
	for _, tok := range tokens {
		if tok.TypeName == "ML_BASIC_STRING" {
			foundString = true
			if tok.Value != "hello\nworld" {
				t.Errorf("Expected 'hello\\nworld', got %q", tok.Value)
			}
			break
		}
	}
	if !foundString {
		t.Errorf("No ML_BASIC_STRING token found in tokens: %v", tokens)
	}
}

// =============================================================================
// TestTokenizeTOMLMultiLineLiteralString
// =============================================================================
//
// Verifies that triple-single-quoted multi-line literal strings are tokenized.
// Multi-line literal strings can span multiple lines but do NOT support escape
// sequences. Backslashes are literal characters.
func TestTokenizeTOMLMultiLineLiteralString(t *testing.T) {
	source := "key = '''hello\nworld'''"
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize TOML multi-line literal string: %v", err)
	}

	foundString := false
	for _, tok := range tokens {
		if tok.TypeName == "ML_LITERAL_STRING" {
			foundString = true
			if tok.Value != "hello\nworld" {
				t.Errorf("Expected 'hello\\nworld', got %q", tok.Value)
			}
			break
		}
	}
	if !foundString {
		t.Errorf("No ML_LITERAL_STRING token found in tokens: %v", tokens)
	}
}

// =============================================================================
// TestTokenizeTOMLInteger
// =============================================================================
//
// Verifies that decimal integers are tokenized correctly. TOML integers can
// have optional leading +/- signs and underscore separators between digits.
func TestTokenizeTOMLInteger(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"val = 42", "42"},
		{"val = 0", "0"},
		{"val = +99", "+99"},
		{"val = -17", "-17"},
		{"val = 1_000", "1_000"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeTOML(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize integer in %q: %v", tc.source, err)
		}

		foundInt := false
		for _, tok := range tokens {
			if tok.TypeName == "INTEGER" {
				foundInt = true
				if tok.Value != tc.value {
					t.Errorf("For %q: expected value %q, got %q", tc.source, tc.value, tok.Value)
				}
				break
			}
		}
		if !foundInt {
			t.Errorf("No INTEGER token found in %q, tokens: %v", tc.source, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLHexOctBinInteger
// =============================================================================
//
// Verifies that hexadecimal (0x), octal (0o), and binary (0b) integers are
// tokenized with the INTEGER type name (via -> alias in the grammar).
func TestTokenizeTOMLHexOctBinInteger(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"val = 0xDEAD_BEEF", "0xDEAD_BEEF"},
		{"val = 0o755", "0o755"},
		{"val = 0b1101_0110", "0b1101_0110"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeTOML(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tc.source, err)
		}

		foundInt := false
		for _, tok := range tokens {
			if tok.TypeName == "INTEGER" {
				foundInt = true
				if tok.Value != tc.value {
					t.Errorf("For %q: expected value %q, got %q", tc.source, tc.value, tok.Value)
				}
				break
			}
		}
		if !foundInt {
			t.Errorf("No INTEGER token found in %q, tokens: %v", tc.source, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLFloat
// =============================================================================
//
// Verifies that floating-point numbers are tokenized correctly. TOML floats
// include decimal notation, scientific notation, and special values (inf, nan).
func TestTokenizeTOMLFloat(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"val = 3.14", "3.14"},
		{"val = -0.01", "-0.01"},
		{"val = 5e+22", "5e+22"},
		{"val = 1e06", "1e06"},
		{"val = -2E-2", "-2E-2"},
		{"val = 6.626e-34", "6.626e-34"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeTOML(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize float in %q: %v", tc.source, err)
		}

		foundFloat := false
		for _, tok := range tokens {
			if tok.TypeName == "FLOAT" {
				foundFloat = true
				if tok.Value != tc.value {
					t.Errorf("For %q: expected value %q, got %q", tc.source, tc.value, tok.Value)
				}
				break
			}
		}
		if !foundFloat {
			t.Errorf("No FLOAT token found in %q, tokens: %v", tc.source, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLSpecialFloats
// =============================================================================
//
// Verifies that special float values (inf, nan) with optional signs are
// tokenized as FLOAT tokens.
func TestTokenizeTOMLSpecialFloats(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"val = inf", "inf"},
		{"val = +inf", "+inf"},
		{"val = -inf", "-inf"},
		{"val = nan", "nan"},
		{"val = +nan", "+nan"},
		{"val = -nan", "-nan"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeTOML(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize special float in %q: %v", tc.source, err)
		}

		foundFloat := false
		for _, tok := range tokens {
			if tok.TypeName == "FLOAT" {
				foundFloat = true
				if tok.Value != tc.value {
					t.Errorf("For %q: expected value %q, got %q", tc.source, tc.value, tok.Value)
				}
				break
			}
		}
		if !foundFloat {
			t.Errorf("No FLOAT token found in %q, tokens: %v", tc.source, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLBooleans
// =============================================================================
//
// Verifies that boolean literals true and false are tokenized correctly.
func TestTokenizeTOMLBooleans(t *testing.T) {
	testCases := []struct {
		source   string
		typeName string
		value    string
	}{
		{"flag = true", "TRUE", "true"},
		{"flag = false", "FALSE", "false"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeTOML(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize boolean in %q: %v", tc.source, err)
		}

		foundBool := false
		for _, tok := range tokens {
			if tok.TypeName == tc.typeName {
				foundBool = true
				if tok.Value != tc.value {
					t.Errorf("For %q: expected value %q, got %q", tc.source, tc.value, tok.Value)
				}
				break
			}
		}
		if !foundBool {
			t.Errorf("No %s token found in %q, tokens: %v", tc.typeName, tc.source, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLOffsetDatetime
// =============================================================================
//
// Verifies that offset datetime literals are tokenized correctly.
func TestTokenizeTOMLOffsetDatetime(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"ts = 1979-05-27T07:32:00Z", "1979-05-27T07:32:00Z"},
		{"ts = 1979-05-27T00:32:00-07:00", "1979-05-27T00:32:00-07:00"},
		{"ts = 1979-05-27T00:32:00.999999-07:00", "1979-05-27T00:32:00.999999-07:00"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeTOML(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize offset datetime in %q: %v", tc.source, err)
		}

		found := false
		for _, tok := range tokens {
			if tok.TypeName == "OFFSET_DATETIME" {
				found = true
				if tok.Value != tc.value {
					t.Errorf("For %q: expected value %q, got %q", tc.source, tc.value, tok.Value)
				}
				break
			}
		}
		if !found {
			t.Errorf("No OFFSET_DATETIME token found in %q, tokens: %v", tc.source, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLLocalDatetime
// =============================================================================
//
// Verifies that local datetime literals are tokenized correctly.
func TestTokenizeTOMLLocalDatetime(t *testing.T) {
	source := "ts = 1979-05-27T07:32:00"
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize local datetime: %v", err)
	}

	found := false
	for _, tok := range tokens {
		if tok.TypeName == "LOCAL_DATETIME" {
			found = true
			if tok.Value != "1979-05-27T07:32:00" {
				t.Errorf("Expected '1979-05-27T07:32:00', got %q", tok.Value)
			}
			break
		}
	}
	if !found {
		t.Errorf("No LOCAL_DATETIME token found, tokens: %v", tokens)
	}
}

// =============================================================================
// TestTokenizeTOMLLocalDate
// =============================================================================
//
// Verifies that local date literals are tokenized correctly.
func TestTokenizeTOMLLocalDate(t *testing.T) {
	source := "date = 1979-05-27"
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize local date: %v", err)
	}

	found := false
	for _, tok := range tokens {
		if tok.TypeName == "LOCAL_DATE" {
			found = true
			if tok.Value != "1979-05-27" {
				t.Errorf("Expected '1979-05-27', got %q", tok.Value)
			}
			break
		}
	}
	if !found {
		t.Errorf("No LOCAL_DATE token found, tokens: %v", tokens)
	}
}

// =============================================================================
// TestTokenizeTOMLLocalTime
// =============================================================================
//
// Verifies that local time literals are tokenized correctly.
func TestTokenizeTOMLLocalTime(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"time = 07:32:00", "07:32:00"},
		{"time = 00:32:00.999999", "00:32:00.999999"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeTOML(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize local time in %q: %v", tc.source, err)
		}

		found := false
		for _, tok := range tokens {
			if tok.TypeName == "LOCAL_TIME" {
				found = true
				if tok.Value != tc.value {
					t.Errorf("For %q: expected value %q, got %q", tc.source, tc.value, tok.Value)
				}
				break
			}
		}
		if !found {
			t.Errorf("No LOCAL_TIME token found in %q, tokens: %v", tc.source, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLBareKey
// =============================================================================
//
// Verifies that bare keys (unquoted key names) are tokenized correctly.
func TestTokenizeTOMLBareKey(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"server = 1", "server"},
		{"my-key = 1", "my-key"},
		{"key_name = 1", "key_name"},
		{"key123 = 1", "key123"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeTOML(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize bare key in %q: %v", tc.source, err)
		}

		if tokens[0].TypeName != "BARE_KEY" {
			t.Errorf("For %q: expected first token BARE_KEY, got %s(%q)", tc.source, tokens[0].TypeName, tokens[0].Value)
		}
		if tokens[0].Value != tc.value {
			t.Errorf("For %q: expected value %q, got %q", tc.source, tc.value, tokens[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLStructuralTokens
// =============================================================================
//
// Verifies that all structural tokens are tokenized correctly.
func TestTokenizeTOMLStructuralTokens(t *testing.T) {
	testCases := []struct {
		source   string
		tokType  lexer.TokenType
		typeName string
	}{
		{"=", lexer.TokenEquals, "EQUALS"},
		{".", lexer.TokenDot, "DOT"},
		{",", lexer.TokenComma, "COMMA"},
		{"[", lexer.TokenLBracket, "LBRACKET"},
		{"]", lexer.TokenRBracket, "RBRACKET"},
		{"{", lexer.TokenLBrace, "LBRACE"},
		{"}", lexer.TokenRBrace, "RBRACE"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeTOML(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tc.source, err)
		}

		if tokens[0].Type != tc.tokType {
			t.Errorf("Expected type %v for %q, got %v", tc.tokType, tc.source, tokens[0].Type)
		}
		if tokens[0].TypeName != tc.typeName {
			t.Errorf("Expected TypeName %q for %q, got %q",
				tc.typeName, tc.source, tokens[0].TypeName)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLCommentSkipped
// =============================================================================
//
// Verifies that comments are silently consumed.
func TestTokenizeTOMLCommentSkipped(t *testing.T) {
	source := "# this is a comment\nkey = 1"
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize TOML with comment: %v", err)
	}

	for _, tok := range tokens {
		if tok.TypeName == "COMMENT" {
			t.Error("Comment should be skipped, not tokenized")
		}
	}

	foundKey := false
	for _, tok := range tokens {
		if tok.TypeName == "BARE_KEY" && tok.Value == "key" {
			foundKey = true
			break
		}
	}
	if !foundKey {
		t.Errorf("Expected BARE_KEY 'key' after comment, tokens: %v", tokens)
	}
}

// =============================================================================
// TestTokenizeTOMLInlineComment
// =============================================================================
//
// Verifies that inline comments are handled correctly.
func TestTokenizeTOMLInlineComment(t *testing.T) {
	source := "key = 42 # the answer"
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize TOML with inline comment: %v", err)
	}

	foundInt := false
	for _, tok := range tokens {
		if tok.TypeName == "INTEGER" && tok.Value == "42" {
			foundInt = true
			break
		}
	}
	if !foundInt {
		t.Errorf("Expected INTEGER '42' before inline comment, tokens: %v", tokens)
	}
}

// =============================================================================
// TestTokenizeTOMLNewlineSignificance
// =============================================================================
//
// Verifies that newlines produce NEWLINE tokens.
func TestTokenizeTOMLNewlineSignificance(t *testing.T) {
	source := "a = 1\nb = 2"
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize multi-line TOML: %v", err)
	}

	newlineCount := 0
	for _, tok := range tokens {
		if tok.Type == lexer.TokenNewline {
			newlineCount++
		}
	}

	if newlineCount < 1 {
		t.Errorf("Expected at least 1 NEWLINE token, got %d", newlineCount)
	}
}

// =============================================================================
// TestTokenizeTOMLTableHeader
// =============================================================================
//
// Verifies that a table header [table-name] is tokenized correctly.
func TestTokenizeTOMLTableHeader(t *testing.T) {
	source := "[server]"
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize table header: %v", err)
	}

	expected := []struct {
		typeName string
		value    string
	}{
		{"LBRACKET", "["},
		{"BARE_KEY", "server"},
		{"RBRACKET", "]"},
		{"EOF", ""},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(tokens), tokens)
	}

	for i, exp := range expected {
		if tokens[i].TypeName != exp.typeName {
			t.Errorf("Token %d: expected TypeName %q, got %q", i, exp.typeName, tokens[i].TypeName)
		}
		if tokens[i].Value != exp.value {
			t.Errorf("Token %d: expected value %q, got %q", i, exp.value, tokens[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLArrayOfTablesHeader
// =============================================================================
//
// Verifies that [[table-name]] is tokenized as four bracket tokens plus a key.
func TestTokenizeTOMLArrayOfTablesHeader(t *testing.T) {
	source := "[[products]]"
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize array-of-tables header: %v", err)
	}

	expected := []struct {
		typeName string
		value    string
	}{
		{"LBRACKET", "["},
		{"LBRACKET", "["},
		{"BARE_KEY", "products"},
		{"RBRACKET", "]"},
		{"RBRACKET", "]"},
		{"EOF", ""},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(tokens), tokens)
	}

	for i, exp := range expected {
		if tokens[i].TypeName != exp.typeName {
			t.Errorf("Token %d: expected TypeName %q, got %q", i, exp.typeName, tokens[i].TypeName)
		}
		if tokens[i].Value != exp.value {
			t.Errorf("Token %d: expected value %q, got %q", i, exp.value, tokens[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLDottedKey
// =============================================================================
//
// Verifies that dotted keys produce the expected token sequence.
func TestTokenizeTOMLDottedKey(t *testing.T) {
	source := `physical.color = "orange"`
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize dotted key: %v", err)
	}

	if tokens[0].TypeName != "BARE_KEY" || tokens[0].Value != "physical" {
		t.Errorf("Expected BARE_KEY('physical'), got %s(%q)", tokens[0].TypeName, tokens[0].Value)
	}
	if tokens[1].TypeName != "DOT" {
		t.Errorf("Expected DOT, got %s(%q)", tokens[1].TypeName, tokens[1].Value)
	}
	if tokens[2].TypeName != "BARE_KEY" || tokens[2].Value != "color" {
		t.Errorf("Expected BARE_KEY('color'), got %s(%q)", tokens[2].TypeName, tokens[2].Value)
	}
	if tokens[3].TypeName != "EQUALS" {
		t.Errorf("Expected EQUALS, got %s(%q)", tokens[3].TypeName, tokens[3].Value)
	}
}

// =============================================================================
// TestTokenizeTOMLInlineTable
// =============================================================================
//
// Verifies that inline tables are tokenized correctly.
func TestTokenizeTOMLInlineTable(t *testing.T) {
	source := `point = { x = 1, y = 2 }`
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize inline table: %v", err)
	}

	foundLBrace := false
	foundRBrace := false
	for _, tok := range tokens {
		if tok.TypeName == "LBRACE" {
			foundLBrace = true
		}
		if tok.TypeName == "RBRACE" {
			foundRBrace = true
		}
	}
	if !foundLBrace {
		t.Error("Expected LBRACE token in inline table")
	}
	if !foundRBrace {
		t.Error("Expected RBRACE token in inline table")
	}
}

// =============================================================================
// TestTokenizeTOMLArray
// =============================================================================
//
// Verifies that arrays are tokenized correctly.
func TestTokenizeTOMLArray(t *testing.T) {
	source := `colors = ["red", "green", "blue"]`
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize array: %v", err)
	}

	stringCount := 0
	for _, tok := range tokens {
		if tok.TypeName == "BASIC_STRING" {
			stringCount++
		}
	}
	if stringCount != 3 {
		t.Errorf("Expected 3 BASIC_STRING tokens, got %d", stringCount)
	}
}

// =============================================================================
// TestTokenizeTOMLKeyValuePair
// =============================================================================
//
// Verifies the fundamental key = value pair token sequence.
func TestTokenizeTOMLKeyValuePair(t *testing.T) {
	source := `name = "TOML"`
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize key-value pair: %v", err)
	}

	expected := []struct {
		typeName string
		value    string
	}{
		{"BARE_KEY", "name"},
		{"EQUALS", "="},
		{"BASIC_STRING", "TOML"},
		{"EOF", ""},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(tokens), tokens)
	}

	for i, exp := range expected {
		if tokens[i].TypeName != exp.typeName {
			t.Errorf("Token %d: expected TypeName %q, got %q", i, exp.typeName, tokens[i].TypeName)
		}
		if tokens[i].Value != exp.value {
			t.Errorf("Token %d: expected value %q, got %q", i, exp.value, tokens[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLComplexDocument
// =============================================================================
//
// Verifies tokenization of a realistic TOML document.
func TestTokenizeTOMLComplexDocument(t *testing.T) {
	source := "# Server configuration\n[server]\nhost = \"localhost\"\nport = 8080\nenabled = true\n\n[database]\nname = \"mydb\"\ntimeout = 30.5\ncreated = 1979-05-27\n\n[[products]]\nname = \"Hammer\"\nprice = 9.99\n\n[[products]]\nname = \"Nail\"\nprice = 0.05\n"
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize complex TOML document: %v", err)
	}

	if len(tokens) < 20 {
		t.Fatalf("Expected at least 20 tokens for complex document, got %d", len(tokens))
	}

	bareKeys := 0
	lbrackets := 0
	strings := 0
	for _, tok := range tokens {
		switch tok.TypeName {
		case "BARE_KEY":
			bareKeys++
		case "LBRACKET":
			lbrackets++
		case "BASIC_STRING":
			strings++
		}
	}

	if bareKeys < 8 {
		t.Errorf("Expected at least 8 BARE_KEY tokens, got %d", bareKeys)
	}
	if lbrackets < 4 {
		t.Errorf("Expected at least 4 LBRACKET tokens, got %d", lbrackets)
	}
	if strings < 4 {
		t.Errorf("Expected at least 4 BASIC_STRING tokens, got %d", strings)
	}
}

// =============================================================================
// TestCreateTOMLLexer
// =============================================================================
//
// Verifies that the factory function returns a valid GrammarLexer instance.
func TestCreateTOMLLexer(t *testing.T) {
	source := `key = 42`
	tomlLexer, err := CreateTOMLLexer(source)
	if err != nil {
		t.Fatalf("Failed to create TOML lexer: %v", err)
	}

	if tomlLexer == nil {
		t.Fatal("CreateTOMLLexer returned nil lexer")
	}

	tokens := tomlLexer.Tokenize()

	if len(tokens) < 2 {
		t.Fatalf("Expected at least 2 tokens, got %d: %v", len(tokens), tokens)
	}

	lastToken := tokens[len(tokens)-1]
	if lastToken.Type != lexer.TokenEOF {
		t.Errorf("Expected last token to be EOF, got %s", lastToken.Type)
	}
}

// =============================================================================
// TestTokenizeTOMLEOFToken
// =============================================================================
//
// Verifies that every tokenized TOML input ends with an EOF token.
func TestTokenizeTOMLEOFToken(t *testing.T) {
	inputs := []string{
		`key = "hello"`,
		`val = 42`,
		`flag = true`,
		`[table]`,
		`arr = [1, 2, 3]`,
	}

	for _, input := range inputs {
		tokens, err := TokenizeTOML(input)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", input, err)
		}

		last := tokens[len(tokens)-1]
		if last.Type != lexer.TokenEOF {
			t.Errorf("Input %q: expected last token EOF, got %s(%q)",
				input, last.TypeName, last.Value)
		}
	}
}

// =============================================================================
// TestTokenizeTOMLLineAndColumn
// =============================================================================
//
// Verifies that tokens have correct line and column information.
func TestTokenizeTOMLLineAndColumn(t *testing.T) {
	source := `key = 1`
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if tokens[0].Line != 1 {
		t.Errorf("Expected line 1, got %d", tokens[0].Line)
	}
	if tokens[0].Column != 1 {
		t.Errorf("Expected column 1, got %d", tokens[0].Column)
	}
}

// =============================================================================
// TestTokenizeTOMLQuotedKey
// =============================================================================
//
// Verifies that quoted keys are tokenized correctly.
func TestTokenizeTOMLQuotedKey(t *testing.T) {
	source := `"my key" = 1`
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize quoted key: %v", err)
	}

	if tokens[0].TypeName != "BASIC_STRING" {
		t.Errorf("Expected BASIC_STRING for quoted key, got %s", tokens[0].TypeName)
	}
	if tokens[0].Value != "my key" {
		t.Errorf("Expected 'my key', got %q", tokens[0].Value)
	}
}

// =============================================================================
// TestTokenizeTOMLWhitespaceHandling
// =============================================================================
//
// Verifies that spaces and tabs are consumed without producing tokens.
func TestTokenizeTOMLWhitespaceHandling(t *testing.T) {
	source := "  \tkey  \t=  \t42  \t"
	tokens, err := TokenizeTOML(source)
	if err != nil {
		t.Fatalf("Failed to tokenize with whitespace: %v", err)
	}

	nonEOF := 0
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			nonEOF++
		}
	}
	if nonEOF != 3 {
		t.Errorf("Expected 3 non-EOF tokens, got %d: %v", nonEOF, tokens)
	}
}

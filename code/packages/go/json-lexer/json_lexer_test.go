package jsonlexer

import (
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// =============================================================================
// Escape-processing grammar helper
// =============================================================================
//
// The real JSON grammar (json.tokens) uses "escapes: none", which tells the
// lexer to strip surrounding quotes from STRING tokens but leave escape
// sequences raw. This is intentional: escape decoding is the JSON parser's
// responsibility, not the lexer's.
//
// TestTokenizeJSONStringWithEscapes needs a grammar that DOES process escape
// sequences so we can verify the lexer engine handles JSON escapes correctly.
// This constant is identical to json.tokens except it omits "escapes: none".
const escapeProcessingGrammarSrc = `
STRING   = /"([^"\\]|\\["\\\x2fbfnrt]|\\u[0-9a-fA-F]{4})*"/
NUMBER   = /-?[0-9]+\.?[0-9]*[eE]?[-+]?[0-9]*/
TRUE     = "true"
FALSE    = "false"
NULL     = "null"
LBRACE   = "{"
RBRACE   = "}"
LBRACKET = "["
RBRACKET = "]"
COLON    = ":"
COMMA    = ","
skip:
  WHITESPACE = /[ \t\r\n]+/
`

// tokenizeWithEscapeProcessing tokenizes source using the escape-processing
// grammar (no "escapes: none"), so escape sequences are decoded by the lexer.
func tokenizeWithEscapeProcessing(source string) ([]lexer.Token, error) {
	grammar, err := grammartools.ParseTokenGrammar(escapeProcessingGrammarSrc)
	if err != nil {
		return nil, err
	}
	l := lexer.NewGrammarLexer(source, grammar)
	return l.Tokenize(), nil
}

// =============================================================================
// TestTokenizeJSONSimpleString
// =============================================================================
//
// Verifies that a simple double-quoted string is tokenized correctly.
// JSON strings are always double-quoted (single quotes are not valid JSON).
// The lexer should strip the quotes and return the inner value.
func TestTokenizeJSONSimpleString(t *testing.T) {
	source := `"hello"`
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize JSON string: %v", err)
	}

	// Expected tokens: STRING("hello"), EOF
	if len(tokens) < 2 {
		t.Fatalf("Expected at least 2 tokens, got %d: %v", len(tokens), tokens)
	}

	// Verify the STRING token
	if tokens[0].Type != lexer.TokenString {
		t.Errorf("Expected STRING token, got %s", tokens[0].Type)
	}
	if tokens[0].Value != "hello" {
		t.Errorf("Expected string value 'hello', got %q", tokens[0].Value)
	}
	if tokens[0].TypeName != "STRING" {
		t.Errorf("Expected TypeName 'STRING', got %q", tokens[0].TypeName)
	}
}

// =============================================================================
// TestTokenizeJSONStringWithEscapes
// =============================================================================
//
// Verifies that JSON escape sequences inside strings are handled correctly
// by the lexer engine. JSON supports: \" \\ \/ \b \f \n \r \t \uXXXX
//
// NOTE: The real JSON grammar (json.tokens) uses "escapes: none", which
// intentionally leaves escape sequences as raw text for the parser to decode.
// This test uses the escape-processing grammar defined in this file (which
// omits "escapes: none") to verify that the lexer engine itself handles
// JSON escape sequences correctly. This mirrors the approach used in the
// Python json-lexer tests.
func TestTokenizeJSONStringWithEscapes(t *testing.T) {
	source := `"hello\nworld"`
	tokens, err := tokenizeWithEscapeProcessing(source)
	if err != nil {
		t.Fatalf("Failed to tokenize JSON string with escapes: %v", err)
	}

	// The STRING token should contain the string with processed escapes
	found := false
	for _, tok := range tokens {
		if tok.Type == lexer.TokenString {
			found = true
			// The escape-processing grammar decodes \n into a real newline
			if tok.Value != "hello\nworld" {
				t.Errorf("Expected 'hello\\nworld' (with real newline), got %q", tok.Value)
			}
			break
		}
	}
	if !found {
		t.Errorf("No STRING token found in tokens: %v", tokens)
	}
}

// =============================================================================
// TestTokenizeJSONEmptyString
// =============================================================================
//
// Verifies that an empty string "" is tokenized correctly. Edge case: the regex
// must match zero characters between the quotes.
func TestTokenizeJSONEmptyString(t *testing.T) {
	source := `""`
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize empty JSON string: %v", err)
	}

	if tokens[0].Type != lexer.TokenString {
		t.Errorf("Expected STRING token, got %s", tokens[0].Type)
	}
	if tokens[0].Value != "" {
		t.Errorf("Expected empty string value, got %q", tokens[0].Value)
	}
}

// =============================================================================
// TestTokenizeJSONIntegerNumber
// =============================================================================
//
// Verifies that simple integer numbers are tokenized as NUMBER tokens.
// JSON numbers have no leading zeros (except 0 itself) and the minus sign
// is part of the number token, not a separate operator.
func TestTokenizeJSONIntegerNumber(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"0", "0"},
		{"42", "42"},
		{"123456789", "123456789"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeJSON(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize number %q: %v", tc.source, err)
		}

		if tokens[0].Type != lexer.TokenNumber {
			t.Errorf("Expected NUMBER for %q, got %s", tc.source, tokens[0].Type)
		}
		if tokens[0].Value != tc.value {
			t.Errorf("Expected value %q, got %q", tc.value, tokens[0].Value)
		}
		if tokens[0].TypeName != "NUMBER" {
			t.Errorf("Expected TypeName 'NUMBER', got %q", tokens[0].TypeName)
		}
	}
}

// =============================================================================
// TestTokenizeJSONNegativeNumber
// =============================================================================
//
// Verifies that negative numbers are tokenized as a single NUMBER token.
// In JSON, -42 is one token (unlike many programming languages where the
// minus is a separate unary operator). The NUMBER regex starts with an
// optional - sign: /-?(0|[1-9][0-9]*).../
func TestTokenizeJSONNegativeNumber(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"-1", "-1"},
		{"-42", "-42"},
		{"-0", "-0"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeJSON(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize negative number %q: %v", tc.source, err)
		}

		if tokens[0].Type != lexer.TokenNumber {
			t.Errorf("Expected NUMBER for %q, got %s", tc.source, tokens[0].Type)
		}
		if tokens[0].Value != tc.value {
			t.Errorf("Expected value %q, got %q", tc.value, tokens[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeJSONDecimalNumber
// =============================================================================
//
// Verifies that decimal (floating-point) numbers are tokenized correctly.
// JSON decimals have the form: integer-part.fractional-part
// The fractional part must have at least one digit: 3. is NOT valid JSON
// (unlike Starlark where "5." is a valid float).
func TestTokenizeJSONDecimalNumber(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"3.14", "3.14"},
		{"0.5", "0.5"},
		{"-0.001", "-0.001"},
		{"100.0", "100.0"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeJSON(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize decimal %q: %v", tc.source, err)
		}

		if tokens[0].Type != lexer.TokenNumber {
			t.Errorf("Expected NUMBER for %q, got %s", tc.source, tokens[0].Type)
		}
		if tokens[0].Value != tc.value {
			t.Errorf("Expected value %q, got %q", tc.value, tokens[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeJSONExponentNumber
// =============================================================================
//
// Verifies that numbers with exponents (scientific notation) are tokenized
// correctly. JSON supports both 'e' and 'E', with optional + or - signs:
//   1e10, 1E10, 1.5e-3, 2.998e+8
func TestTokenizeJSONExponentNumber(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"1e10", "1e10"},
		{"1E10", "1E10"},
		{"1.5e-3", "1.5e-3"},
		{"2.998e+8", "2.998e+8"},
		{"-1e5", "-1e5"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeJSON(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize exponent number %q: %v", tc.source, err)
		}

		if tokens[0].Type != lexer.TokenNumber {
			t.Errorf("Expected NUMBER for %q, got %s", tc.source, tokens[0].Type)
		}
		if tokens[0].Value != tc.value {
			t.Errorf("Expected value %q, got %q", tc.value, tokens[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeJSONLiterals
// =============================================================================
//
// Verifies that the three JSON literal values (true, false, null) are
// tokenized correctly. Unlike programming languages where these might be
// keywords reclassified from NAME tokens, JSON has no NAME token at all.
// These are defined as literal token patterns in json.tokens, each producing
// its own token type (TRUE, FALSE, NULL).
func TestTokenizeJSONLiterals(t *testing.T) {
	testCases := []struct {
		source   string
		typeName string
	}{
		{"true", "TRUE"},
		{"false", "FALSE"},
		{"null", "NULL"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeJSON(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize literal %q: %v", tc.source, err)
		}

		if tokens[0].TypeName != tc.typeName {
			t.Errorf("Expected TypeName %q for %q, got %q",
				tc.typeName, tc.source, tokens[0].TypeName)
		}
		if tokens[0].Value != tc.source {
			t.Errorf("Expected value %q, got %q", tc.source, tokens[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeJSONStructuralTokens
// =============================================================================
//
// Verifies that all six structural tokens ({ } [ ] : ,) are tokenized with
// the correct types. These delimiters organize JSON data into objects and
// arrays. Each maps to a well-known TokenType in the lexer package.
func TestTokenizeJSONStructuralTokens(t *testing.T) {
	testCases := []struct {
		source   string
		tokType  lexer.TokenType
		typeName string
	}{
		{"{", lexer.TokenLBrace, "LBRACE"},
		{"}", lexer.TokenRBrace, "RBRACE"},
		{"[", lexer.TokenLBracket, "LBRACKET"},
		{"]", lexer.TokenRBracket, "RBRACKET"},
		{":", lexer.TokenColon, "COLON"},
		{",", lexer.TokenComma, "COMMA"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeJSON(tc.source)
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
// TestTokenizeJSONWhitespaceSkipped
// =============================================================================
//
// Verifies that inline whitespace (spaces, tabs, carriage returns) is silently
// consumed and produces no tokens. The standard tokenizer emits NEWLINE tokens
// for line breaks (this is a fixed behavior in the lexer engine), but these
// are harmless because the JSON parser grammar never references NEWLINE tokens
// -- they are simply skipped during parsing.
func TestTokenizeJSONWhitespaceSkipped(t *testing.T) {
	// Spaces and tabs around a value -- should produce just NUMBER + EOF
	source := "  \t  42  \t  "
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize with whitespace: %v", err)
	}

	// Should have exactly 2 tokens: NUMBER("42") and EOF
	// Inline whitespace (spaces, tabs) is consumed by the standard tokenizer.
	nonEOF := 0
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			nonEOF++
			if tok.Type != lexer.TokenNumber || tok.Value != "42" {
				t.Errorf("Expected only NUMBER(42), got %s(%q)", tok.TypeName, tok.Value)
			}
		}
	}
	if nonEOF != 1 {
		t.Errorf("Expected 1 non-EOF token, got %d: %v", nonEOF, tokens)
	}
}

// =============================================================================
// TestTokenizeJSONNewlinesProduceTokens
// =============================================================================
//
// Verifies that newlines are consumed by the skip pattern and do NOT produce
// NEWLINE tokens. The JSON grammar's skip pattern includes \n in its character
// class (WHITESPACE = /[ \t\r\n]+/), which intentionally suppresses NEWLINE
// tokens. This is correct for JSON — the parser grammar never references
// NEWLINE tokens, and newlines are just whitespace.
func TestTokenizeJSONNewlinesProduceTokens(t *testing.T) {
	source := "\n42\n"
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize with newlines: %v", err)
	}

	// Skip pattern consumes \n, so only NUMBER and EOF remain.
	foundNumber := false
	newlineCount := 0
	for _, tok := range tokens {
		if tok.Type == lexer.TokenNumber && tok.Value == "42" {
			foundNumber = true
		}
		if tok.Type == lexer.TokenNewline {
			newlineCount++
		}
	}

	if !foundNumber {
		t.Error("Expected NUMBER(42) token")
	}
	if newlineCount != 0 {
		t.Errorf("Expected 0 NEWLINE tokens (consumed by skip pattern), got %d", newlineCount)
	}
}

// =============================================================================
// TestTokenizeJSONSimpleObject
// =============================================================================
//
// Verifies that a simple JSON object with one key-value pair is tokenized
// into the expected token stream. This is the most common JSON pattern:
//   {"key": "value"}
func TestTokenizeJSONSimpleObject(t *testing.T) {
	source := `{"name": "Alice"}`
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize JSON object: %v", err)
	}

	// Expected tokens:
	//   LBRACE, STRING("name"), COLON, STRING("Alice"), RBRACE, EOF
	expected := []struct {
		typeName string
		value    string
	}{
		{"LBRACE", "{"},
		{"STRING", "name"},
		{"COLON", ":"},
		{"STRING", "Alice"},
		{"RBRACE", "}"},
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
// TestTokenizeJSONSimpleArray
// =============================================================================
//
// Verifies that a simple JSON array is tokenized correctly.
//   [1, 2, 3]
func TestTokenizeJSONSimpleArray(t *testing.T) {
	source := `[1, 2, 3]`
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize JSON array: %v", err)
	}

	// Expected tokens:
	//   LBRACKET, NUMBER("1"), COMMA, NUMBER("2"), COMMA, NUMBER("3"), RBRACKET, EOF
	expected := []struct {
		typeName string
		value    string
	}{
		{"LBRACKET", "["},
		{"NUMBER", "1"},
		{"COMMA", ","},
		{"NUMBER", "2"},
		{"COMMA", ","},
		{"NUMBER", "3"},
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
// TestTokenizeJSONComplexObject
// =============================================================================
//
// Verifies tokenization of a multi-key object with mixed value types.
// This exercises all value token types in a single object:
//   {"name": "Bob", "age": 25, "active": true, "score": null}
func TestTokenizeJSONComplexObject(t *testing.T) {
	source := `{"name": "Bob", "age": 25, "active": true, "score": null}`
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize complex JSON object: %v", err)
	}

	// Verify we got at least the structural tokens plus values
	// LBRACE + 4*(STRING COLON value COMMA) - 1 COMMA + RBRACE + EOF
	// = 1 + 4*3 + 3*1 + 1 + 1 = 18 tokens
	if len(tokens) < 10 {
		t.Fatalf("Expected at least 10 tokens for complex object, got %d", len(tokens))
	}

	// First should be LBRACE, last non-EOF should be RBRACE
	if tokens[0].TypeName != "LBRACE" {
		t.Errorf("Expected first token LBRACE, got %q", tokens[0].TypeName)
	}

	// Find the TRUE and NULL tokens
	foundTrue := false
	foundNull := false
	for _, tok := range tokens {
		if tok.TypeName == "TRUE" && tok.Value == "true" {
			foundTrue = true
		}
		if tok.TypeName == "NULL" && tok.Value == "null" {
			foundNull = true
		}
	}
	if !foundTrue {
		t.Error("Expected TRUE token in complex object")
	}
	if !foundNull {
		t.Error("Expected NULL token in complex object")
	}
}

// =============================================================================
// TestTokenizeJSONNestedStructures
// =============================================================================
//
// Verifies tokenization of nested objects and arrays. JSON's power comes from
// its recursive structure: values can contain objects, which contain arrays,
// which contain objects, to arbitrary depth.
//
//   {"users": [{"name": "Alice"}, {"name": "Bob"}]}
func TestTokenizeJSONNestedStructures(t *testing.T) {
	source := `{"users": [{"name": "Alice"}, {"name": "Bob"}]}`
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize nested JSON: %v", err)
	}

	// Count structural tokens to verify nesting
	lbraces := 0
	rbraces := 0
	lbrackets := 0
	rbrackets := 0
	for _, tok := range tokens {
		switch tok.TypeName {
		case "LBRACE":
			lbraces++
		case "RBRACE":
			rbraces++
		case "LBRACKET":
			lbrackets++
		case "RBRACKET":
			rbrackets++
		}
	}

	// Should have 3 opening braces (outer + 2 inner) and matching closing braces
	if lbraces != 3 {
		t.Errorf("Expected 3 LBRACE tokens, got %d", lbraces)
	}
	if rbraces != 3 {
		t.Errorf("Expected 3 RBRACE tokens, got %d", rbraces)
	}
	// Should have 1 opening bracket and 1 closing bracket
	if lbrackets != 1 {
		t.Errorf("Expected 1 LBRACKET token, got %d", lbrackets)
	}
	if rbrackets != 1 {
		t.Errorf("Expected 1 RBRACKET token, got %d", rbrackets)
	}
}

// =============================================================================
// TestTokenizeJSONEmptyObject
// =============================================================================
//
// Verifies that an empty object {} produces just structural tokens.
// This is an important edge case: the grammar must handle the empty case
// via the optional pattern [ pair { COMMA pair } ].
func TestTokenizeJSONEmptyObject(t *testing.T) {
	source := `{}`
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize empty object: %v", err)
	}

	// Expected: LBRACE, RBRACE, EOF
	if len(tokens) != 3 {
		t.Fatalf("Expected 3 tokens for {}, got %d: %v", len(tokens), tokens)
	}
	if tokens[0].TypeName != "LBRACE" {
		t.Errorf("Expected LBRACE, got %q", tokens[0].TypeName)
	}
	if tokens[1].TypeName != "RBRACE" {
		t.Errorf("Expected RBRACE, got %q", tokens[1].TypeName)
	}
	if tokens[2].TypeName != "EOF" {
		t.Errorf("Expected EOF, got %q", tokens[2].TypeName)
	}
}

// =============================================================================
// TestTokenizeJSONEmptyArray
// =============================================================================
//
// Verifies that an empty array [] produces just structural tokens.
func TestTokenizeJSONEmptyArray(t *testing.T) {
	source := `[]`
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize empty array: %v", err)
	}

	// Expected: LBRACKET, RBRACKET, EOF
	if len(tokens) != 3 {
		t.Fatalf("Expected 3 tokens for [], got %d: %v", len(tokens), tokens)
	}
	if tokens[0].TypeName != "LBRACKET" {
		t.Errorf("Expected LBRACKET, got %q", tokens[0].TypeName)
	}
	if tokens[1].TypeName != "RBRACKET" {
		t.Errorf("Expected RBRACKET, got %q", tokens[1].TypeName)
	}
}

// =============================================================================
// TestCreateJSONLexer
// =============================================================================
//
// Verifies that the factory function CreateJSONLexer returns a valid
// GrammarLexer instance that can be used for tokenization. This tests the
// two-step API (create lexer, then call Tokenize) as opposed to the
// one-shot TokenizeJSON convenience function.
func TestCreateJSONLexer(t *testing.T) {
	source := `42`
	jsonLexer, err := CreateJSONLexer(source)
	if err != nil {
		t.Fatalf("Failed to create JSON lexer: %v", err)
	}

	// The lexer should not be nil
	if jsonLexer == nil {
		t.Fatal("CreateJSONLexer returned nil lexer")
	}

	// Tokenize using the created lexer instance
	tokens := jsonLexer.Tokenize()

	// Should produce at least: NUMBER("42"), EOF
	if len(tokens) < 2 {
		t.Fatalf("Expected at least 2 tokens, got %d: %v", len(tokens), tokens)
	}

	// Verify the last token is EOF
	lastToken := tokens[len(tokens)-1]
	if lastToken.Type != lexer.TokenEOF {
		t.Errorf("Expected last token to be EOF, got %s", lastToken.Type)
	}
}

// =============================================================================
// TestTokenizeJSONMultilineFormatting
// =============================================================================
//
// Verifies that multi-line (pretty-printed) JSON is tokenized correctly.
// JSON whitespace is insignificant, so indentation and newlines between
// tokens should be silently consumed by the skip pattern.
func TestTokenizeJSONMultilineFormatting(t *testing.T) {
	source := `{
  "name": "Alice",
  "age": 30
}`
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize multi-line JSON: %v", err)
	}

	// Should produce the same tokens as the single-line version.
	// Verify key structural elements are present.
	if tokens[0].TypeName != "LBRACE" {
		t.Errorf("Expected LBRACE first, got %q", tokens[0].TypeName)
	}

	// Find the STRING "name" and NUMBER "30"
	foundName := false
	foundAge := false
	for _, tok := range tokens {
		if tok.Type == lexer.TokenString && tok.Value == "name" {
			foundName = true
		}
		if tok.Type == lexer.TokenNumber && tok.Value == "30" {
			foundAge = true
		}
	}
	if !foundName {
		t.Error("Expected STRING 'name' in multi-line JSON")
	}
	if !foundAge {
		t.Error("Expected NUMBER '30' in multi-line JSON")
	}
}

// =============================================================================
// TestTokenizeJSONEOFToken
// =============================================================================
//
// Verifies that every tokenized JSON input ends with an EOF token.
// The EOF token signals the end of input to the parser. Without it,
// the parser would not know when to stop consuming tokens.
func TestTokenizeJSONEOFToken(t *testing.T) {
	inputs := []string{
		`"hello"`,
		`42`,
		`true`,
		`null`,
		`{}`,
		`[]`,
	}

	for _, input := range inputs {
		tokens, err := TokenizeJSON(input)
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
// TestTokenizeJSONLineAndColumn
// =============================================================================
//
// Verifies that tokens have correct line and column information. This is
// important for error reporting in the parser. The first character of the
// input is at line 1, column 1.
func TestTokenizeJSONLineAndColumn(t *testing.T) {
	source := `{"a": 1}`
	tokens, err := TokenizeJSON(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// First token LBRACE should be at line 1, column 1
	if tokens[0].Line != 1 {
		t.Errorf("Expected line 1, got %d", tokens[0].Line)
	}
	if tokens[0].Column != 1 {
		t.Errorf("Expected column 1, got %d", tokens[0].Column)
	}
}


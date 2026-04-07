package dartmouthlexer

import (
	"errors"
	"fmt"
	"path/filepath"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// =============================================================================
// Token value conventions in case_sensitive: false mode
// =============================================================================
//
// When case_sensitive: false is set in the grammar, the GrammarLexer lowercases
// the source before matching. The following value conventions apply:
//
//   KEYWORD:    uppercase   (normalized by @case_insensitive true + KEYWORD logic)
//   BUILTIN_FN: lowercase   (matches lowercase patterns; not a KEYWORD so not uppercased)
//   USER_FN:    lowercase   (same as BUILTIN_FN)
//   NAME:       lowercase   (source is lowercased, NAME is not a KEYWORD)
//   NUMBER:     lowercase-e ("1.5E3" in source becomes "1.5e3" after lowercasing)
//   STRING:     original case, NO surrounding quotes (lexer strips quotes)
//   NEWLINE:    "\\n"       (hardcoded literal in GrammarLexer.tokenizeStandard)
//   EQ, PLUS, etc.: unchanged (ASCII symbols have no case)
//   EOF:        ""
//
// Test expectations are written to match this actual behavior.

// =============================================================================
// Helper: tokenTypes extracts just the TypeName from each token.
// =============================================================================

func tokenTypes(tokens []lexer.Token) []string {
	result := make([]string, len(tokens))
	for i, tok := range tokens {
		result[i] = tok.TypeName
	}
	return result
}

// tokenTypeValues returns "TYPE(value)" strings for readable test diffs.
func tokenTypeValues(tokens []lexer.Token) []string {
	result := make([]string, len(tokens))
	for i, tok := range tokens {
		result[i] = tok.TypeName + "(" + tok.Value + ")"
	}
	return result
}

// =============================================================================
// TestTokenizeDartmouthBasic_LetStatement
// =============================================================================
//
// Verifies the simplest complete Dartmouth BASIC statement: LET.
//
//	10 LET X = 5
//
// Expected token stream:
//   LINE_NUM("10")  -- relabeled from NUMBER by hook
//   KEYWORD("LET")  -- uppercase (normalized)
//   NAME("x")       -- lowercase (source is lowercased)
//   EQ("=")
//   NUMBER("5")
//   NEWLINE("\\n")  -- GrammarLexer hardcodes "\\n" as the NEWLINE value
//   EOF("")
func TestTokenizeDartmouthBasic_LetStatement(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 LET X = 5\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expected := []struct {
		typeName string
		value    string
	}{
		{"LINE_NUM", "10"},
		{"KEYWORD", "LET"},
		{"NAME", "x"},
		{"EQ", "="},
		{"NUMBER", "5"},
		{"NEWLINE", "\\n"},
		{"EOF", ""},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(tokens), tokenTypeValues(tokens))
	}
	for i, exp := range expected {
		if tokens[i].TypeName != exp.typeName {
			t.Errorf("Token %d: expected TypeName %q, got %q (value=%q)",
				i, exp.typeName, tokens[i].TypeName, tokens[i].Value)
		}
		if tokens[i].Value != exp.value {
			t.Errorf("Token %d: expected value %q, got %q",
				i, exp.value, tokens[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_PrintStatement
// =============================================================================
//
// Verifies that PRINT with multiple arguments and COMMA separator tokenizes
// correctly.
//
//	20 PRINT X, Y
func TestTokenizeDartmouthBasic_PrintStatement(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("20 PRINT X, Y\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expected := []struct {
		typeName string
		value    string
	}{
		{"LINE_NUM", "20"},
		{"KEYWORD", "PRINT"},
		{"NAME", "x"},
		{"COMMA", ","},
		{"NAME", "y"},
		{"NEWLINE", "\\n"},
		{"EOF", ""},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(tokens), tokenTypeValues(tokens))
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
// TestTokenizeDartmouthBasic_GotoStatement
// =============================================================================
//
// Verifies that GOTO produces a NUMBER (not LINE_NUM) for the branch target.
//
//	30 GOTO 10
//
// The LINE_NUM hook only relabels the FIRST NUMBER on each line. "10" in
// "GOTO 10" is not at line start, so it remains NUMBER.
func TestTokenizeDartmouthBasic_GotoStatement(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("30 GOTO 10\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	types := tokenTypes(tokens)
	expected := []string{"LINE_NUM", "KEYWORD", "NUMBER", "NEWLINE", "EOF"}

	if len(types) != len(expected) {
		t.Fatalf("Expected %v, got %v", expected, types)
	}
	for i, exp := range expected {
		if types[i] != exp {
			t.Errorf("Token %d: expected %q, got %q", i, exp, types[i])
		}
	}

	if tokens[1].Value != "GOTO" {
		t.Errorf("Expected KEYWORD value \"GOTO\", got %q", tokens[1].Value)
	}
	// GOTO target stays NUMBER
	if tokens[2].TypeName != "NUMBER" {
		t.Errorf("GOTO target should be NUMBER, got %q", tokens[2].TypeName)
	}
	if tokens[2].Value != "10" {
		t.Errorf("GOTO target value should be \"10\", got %q", tokens[2].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_IfThenStatement
// =============================================================================
//
// Verifies the IF...THEN conditional statement.
//
//	40 IF X > 0 THEN 100
func TestTokenizeDartmouthBasic_IfThenStatement(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("40 IF X > 0 THEN 100\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expected := []struct {
		typeName string
		value    string
	}{
		{"LINE_NUM", "40"},
		{"KEYWORD", "IF"},
		{"NAME", "x"},
		{"GT", ">"},
		{"NUMBER", "0"},
		{"KEYWORD", "THEN"},
		{"NUMBER", "100"},
		{"NEWLINE", "\\n"},
		{"EOF", ""},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(tokens), tokenTypeValues(tokens))
	}
	for i, exp := range expected {
		if tokens[i].TypeName != exp.typeName {
			t.Errorf("Token %d: expected TypeName %q, got %q (value=%q)",
				i, exp.typeName, tokens[i].TypeName, tokens[i].Value)
		}
		if tokens[i].Value != exp.value {
			t.Errorf("Token %d: expected value %q, got %q",
				i, exp.value, tokens[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_ForStatement
// =============================================================================
//
// Verifies the FOR...TO...STEP loop statement.
//
//	50 FOR I = 1 TO 10 STEP 2
func TestTokenizeDartmouthBasic_ForStatement(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("50 FOR I = 1 TO 10 STEP 2\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expectedTypes := []string{
		"LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER",
		"KEYWORD", "NUMBER", "KEYWORD", "NUMBER", "NEWLINE", "EOF",
	}
	expectedValues := []string{
		"50", "FOR", "i", "=", "1",
		"TO", "10", "STEP", "2", "\\n", "",
	}

	if len(tokens) != len(expectedTypes) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expectedTypes), len(tokens), tokenTypeValues(tokens))
	}
	for i := range expectedTypes {
		if tokens[i].TypeName != expectedTypes[i] {
			t.Errorf("Token %d: expected TypeName %q, got %q", i, expectedTypes[i], tokens[i].TypeName)
		}
		if tokens[i].Value != expectedValues[i] {
			t.Errorf("Token %d: expected value %q, got %q", i, expectedValues[i], tokens[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_DefStatement
// =============================================================================
//
// Verifies DEF with a user-defined function.
//
//	60 DEF FNA(X) = X * X
//
// USER_FN values are lowercase ("fna") because they are matched by a lowercase
// pattern and not normalized like KEYWORD tokens.
func TestTokenizeDartmouthBasic_DefStatement(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("60 DEF FNA(X) = X * X\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expectedTypes := []string{
		"LINE_NUM", "KEYWORD", "USER_FN", "LPAREN", "NAME", "RPAREN",
		"EQ", "NAME", "STAR", "NAME", "NEWLINE", "EOF",
	}

	if len(tokens) != len(expectedTypes) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expectedTypes), len(tokens), tokenTypeValues(tokens))
	}
	for i, exp := range expectedTypes {
		if tokens[i].TypeName != exp {
			t.Errorf("Token %d: expected TypeName %q, got %q (value=%q)",
				i, exp, tokens[i].TypeName, tokens[i].Value)
		}
	}
	// USER_FN value is lowercase because case_sensitive: false lowercases source
	if tokens[2].Value != "fna" {
		t.Errorf("USER_FN value: expected \"fna\", got %q", tokens[2].Value)
	}
	// KEYWORD value is uppercase (normalized)
	if tokens[1].Value != "DEF" {
		t.Errorf("KEYWORD value: expected \"DEF\", got %q", tokens[1].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_BuiltinFunctions
// =============================================================================
//
// Verifies that all 11 built-in mathematical functions are recognized as
// BUILTIN_FN tokens. Values are lowercase due to case_sensitive: false.
func TestTokenizeDartmouthBasic_BuiltinFunctions(t *testing.T) {
	// Map from uppercase function name to expected lowercase token value
	builtins := map[string]string{
		"SIN": "sin", "COS": "cos", "TAN": "tan", "ATN": "atn",
		"EXP": "exp", "LOG": "log", "ABS": "abs", "SQR": "sqr",
		"INT": "int", "RND": "rnd", "SGN": "sgn",
	}
	for upperName, lowerValue := range builtins {
		source := "10 LET Y = " + upperName + "(X)\n"
		tokens, err := TokenizeDartmouthBasic(source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", source, err)
		}
		// Find the BUILTIN_FN token with the expected lowercase value
		found := false
		for _, tok := range tokens {
			if tok.TypeName == "BUILTIN_FN" && tok.Value == lowerValue {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("Expected BUILTIN_FN(%q) in token stream, got: %v",
				lowerValue, tokenTypeValues(tokens))
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_BuiltinFunctionsInExpression
// =============================================================================
//
// Verifies a full expression with two built-in functions.
//
//	70 LET Y = SIN(X) + COS(X)
func TestTokenizeDartmouthBasic_BuiltinFunctionsInExpression(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("70 LET Y = SIN(X) + COS(X)\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expectedTypes := []string{
		"LINE_NUM", "KEYWORD", "NAME", "EQ",
		"BUILTIN_FN", "LPAREN", "NAME", "RPAREN",
		"PLUS",
		"BUILTIN_FN", "LPAREN", "NAME", "RPAREN",
		"NEWLINE", "EOF",
	}

	if len(tokens) != len(expectedTypes) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expectedTypes), len(tokens), tokenTypeValues(tokens))
	}
	for i, exp := range expectedTypes {
		if tokens[i].TypeName != exp {
			t.Errorf("Token %d: expected %q, got %q (value=%q)",
				i, exp, tokens[i].TypeName, tokens[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_CaseInsensitivity
// =============================================================================
//
// Verifies that keywords are recognized regardless of input case.
//
// Because case_sensitive: false lowercases the source, "print", "PRINT",
// and "Print" all produce KEYWORD("PRINT"). Variable names like "X" and "x"
// both produce NAME("x") (lowercase in output).
func TestTokenizeDartmouthBasic_CaseInsensitivity(t *testing.T) {
	// Verify that lowercase and uppercase source produce identical token streams
	testCases := []struct {
		lower string
		upper string
	}{
		{"10 print x\n", "10 PRINT X\n"},
		{"20 let a = 1\n", "20 LET A = 1\n"},
		{"30 goto 20\n", "30 GOTO 20\n"},
	}

	for _, tc := range testCases {
		tokensLower, err := TokenizeDartmouthBasic(tc.lower)
		if err != nil {
			t.Fatalf("Failed to tokenize lowercase %q: %v", tc.lower, err)
		}
		tokensUpper, err := TokenizeDartmouthBasic(tc.upper)
		if err != nil {
			t.Fatalf("Failed to tokenize uppercase %q: %v", tc.upper, err)
		}

		if len(tokensLower) != len(tokensUpper) {
			t.Errorf("Case mismatch: %q produced %d tokens, %q produced %d tokens",
				tc.lower, len(tokensLower), tc.upper, len(tokensUpper))
			continue
		}
		for i := range tokensLower {
			if tokensLower[i].TypeName != tokensUpper[i].TypeName {
				t.Errorf("Case mismatch at token %d: lower=%q, upper=%q",
					i, tokensLower[i].TypeName, tokensUpper[i].TypeName)
			}
			if tokensLower[i].Value != tokensUpper[i].Value {
				t.Errorf("Case value mismatch at token %d: lower=%q, upper=%q",
					i, tokensLower[i].Value, tokensUpper[i].Value)
			}
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_MultiCharOperators
// =============================================================================
//
// Verifies that the three two-character comparison operators are lexed as
// single tokens, not as two separate single-character tokens.
//
//   <= must be LE, not LT + EQ
//   >= must be GE, not GT + EQ
//   <> must be NE, not LT + GT
func TestTokenizeDartmouthBasic_MultiCharOperators(t *testing.T) {
	testCases := []struct {
		source   string
		typeName string
		value    string
	}{
		{"<=", "LE", "<="},
		{">=", "GE", ">="},
		{"<>", "NE", "<>"},
	}

	for _, tc := range testCases {
		source := "10 IF X " + tc.source + " Y THEN 50\n"
		tokens, err := TokenizeDartmouthBasic(source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", source, err)
		}
		found := false
		for _, tok := range tokens {
			if tok.TypeName == tc.typeName {
				found = true
				if tok.Value != tc.value {
					t.Errorf("Operator %q: expected value %q, got %q",
						tc.source, tc.value, tok.Value)
				}
				break
			}
		}
		if !found {
			t.Errorf("Operator %q: expected token type %q in stream, got: %v",
				tc.source, tc.typeName, tokenTypeValues(tokens))
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_SingleCharOperators
// =============================================================================
//
// Verifies all single-character arithmetic and comparison operators.
func TestTokenizeDartmouthBasic_SingleCharOperators(t *testing.T) {
	testCases := []struct {
		source   string
		typeName string
		value    string
	}{
		{"+", "PLUS", "+"},
		{"-", "MINUS", "-"},
		{"*", "STAR", "*"},
		{"/", "SLASH", "/"},
		{"^", "CARET", "^"},
		{"=", "EQ", "="},
		{"<", "LT", "<"},
		{">", "GT", ">"},
		{"(", "LPAREN", "("},
		{")", "RPAREN", ")"},
		{",", "COMMA", ","},
		{";", "SEMICOLON", ";"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeDartmouthBasic(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize operator %q: %v", tc.source, err)
		}
		if len(tokens) < 1 {
			t.Fatalf("No tokens for operator %q", tc.source)
		}
		if tokens[0].TypeName != tc.typeName {
			t.Errorf("Operator %q: expected TypeName %q, got %q",
				tc.source, tc.typeName, tokens[0].TypeName)
		}
		if tokens[0].Value != tc.value {
			t.Errorf("Operator %q: expected value %q, got %q",
				tc.source, tc.value, tokens[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_NumberFormats
// =============================================================================
//
// Verifies all numeric literal formats supported by Dartmouth BASIC 1964.
//
// Because case_sensitive: false lowercases the source, 'E' in scientific
// notation becomes 'e': "1.5E3" → "1.5e3" in the token value.
func TestTokenizeDartmouthBasic_NumberFormats(t *testing.T) {
	testCases := []struct {
		source string
		value  string // expected token value (lowercase e due to source lowercasing)
	}{
		{"42", "42"},
		{"3.14", "3.14"},
		{".5", ".5"},
		{"1.5E3", "1.5e3"},   // E becomes e after lowercasing
		{"1.5E-3", "1.5e-3"}, // E becomes e after lowercasing
		{"1E10", "1e10"},     // E becomes e after lowercasing
	}

	for _, tc := range testCases {
		// Wrap in a LET statement so the number is not at line-start (not LINE_NUM)
		source := "10 LET X = " + tc.source + "\n"
		tokens, err := TokenizeDartmouthBasic(source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", source, err)
		}
		found := false
		for _, tok := range tokens {
			if tok.TypeName == "NUMBER" {
				found = true
				if tok.Value != tc.value {
					t.Errorf("Number %q: expected value %q, got %q",
						tc.source, tc.value, tok.Value)
				}
				break
			}
		}
		if !found {
			t.Errorf("Number %q: no NUMBER token in stream: %v",
				tc.source, tokenTypeValues(tokens))
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_StringLiteral
// =============================================================================
//
// Verifies that string literals produce STRING tokens.
//
// The GrammarLexer strips surrounding double quotes from string tokens.
// The STRING token value is the content between the quotes, preserving
// original case (via originalSource in case_sensitive: false mode).
//
// Note: Unlike the Elixir lexer (which preserves quotes), the Go GrammarLexer
// strips them. The value of STRING("HELLO WORLD") does not include quotes.
func TestTokenizeDartmouthBasic_StringLiteral(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 PRINT \"HELLO WORLD\"\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expectedTypes := []string{"LINE_NUM", "KEYWORD", "STRING", "NEWLINE", "EOF"}
	if len(tokens) != len(expectedTypes) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expectedTypes), len(tokens), tokenTypeValues(tokens))
	}
	for i, exp := range expectedTypes {
		if tokens[i].TypeName != exp {
			t.Errorf("Token %d: expected %q, got %q", i, exp, tokens[i].TypeName)
		}
	}
	// The STRING token value is the content without surrounding quotes.
	// Original case is preserved (GrammarLexer uses originalSource for strings).
	if tokens[2].Value != "HELLO WORLD" {
		t.Errorf("STRING value: expected \"HELLO WORLD\", got %q", tokens[2].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_REMSuppression
// =============================================================================
//
// Verifies that REM comment content is suppressed from the token stream.
//
// The suppressRemContent hook checks for KEYWORD("REM"). Since keywords are
// normalized to uppercase, the hook correctly identifies REM tokens.
//
//	10 REM THIS IS A COMMENT
//	→ [LINE_NUM("10"), KEYWORD("REM"), NEWLINE("\\n"), EOF("")]
func TestTokenizeDartmouthBasic_REMSuppression(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 REM THIS IS A COMMENT\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expectedTypes := []string{"LINE_NUM", "KEYWORD", "NEWLINE", "EOF"}
	expectedValues := []string{"10", "REM", "\\n", ""}

	if len(tokens) != len(expectedTypes) {
		t.Fatalf("Expected %d tokens (REM content suppressed), got %d: %v",
			len(expectedTypes), len(tokens), tokenTypeValues(tokens))
	}
	for i, exp := range expectedTypes {
		if tokens[i].TypeName != exp {
			t.Errorf("Token %d: expected TypeName %q, got %q", i, exp, tokens[i].TypeName)
		}
		if tokens[i].Value != expectedValues[i] {
			t.Errorf("Token %d: expected value %q, got %q", i, expectedValues[i], tokens[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_REMFollowedByCode
// =============================================================================
//
// Verifies that REM suppression stops at NEWLINE and does not affect
// subsequent lines.
func TestTokenizeDartmouthBasic_REMFollowedByCode(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 REM\n20 LET X = 1\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expectedTypes := []string{
		"LINE_NUM", "KEYWORD", "NEWLINE",
		"LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE",
		"EOF",
	}

	if len(tokens) != len(expectedTypes) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expectedTypes), len(tokens), tokenTypeValues(tokens))
	}
	for i, exp := range expectedTypes {
		if tokens[i].TypeName != exp {
			t.Errorf("Token %d: expected %q, got %q (value=%q)",
				i, exp, tokens[i].TypeName, tokens[i].Value)
		}
	}
	// Line 2 should have LINE_NUM("20"), not NUMBER("20")
	if tokens[3].TypeName != "LINE_NUM" || tokens[3].Value != "20" {
		t.Errorf("Second line number: expected LINE_NUM(\"20\"), got %s(%q)",
			tokens[3].TypeName, tokens[3].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_REMWithContent
// =============================================================================
//
// Verifies that all REM content (including keywords and numbers) is suppressed.
//
//	10 REM GOTO 20 MEANS JUMP
//
// "GOTO" and "20" appear after REM but should not reach the token stream.
func TestTokenizeDartmouthBasic_REMWithContent(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 REM GOTO 20 MEANS JUMP\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Should only have LINE_NUM, KEYWORD(REM), NEWLINE, EOF
	expectedTypes := []string{"LINE_NUM", "KEYWORD", "NEWLINE", "EOF"}
	if len(tokens) != len(expectedTypes) {
		t.Fatalf("REM content not fully suppressed. Expected %d tokens, got %d: %v",
			len(expectedTypes), len(tokens), tokenTypeValues(tokens))
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_MultiLineProgram
// =============================================================================
//
// Verifies that a complete multi-line BASIC program tokenizes correctly.
//
//	10 LET X = 1
//	20 PRINT X
//	30 END
func TestTokenizeDartmouthBasic_MultiLineProgram(t *testing.T) {
	source := "10 LET X = 1\n20 PRINT X\n30 END\n"
	tokens, err := TokenizeDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expected := []struct {
		typeName string
		value    string
	}{
		{"LINE_NUM", "10"},
		{"KEYWORD", "LET"},
		{"NAME", "x"},
		{"EQ", "="},
		{"NUMBER", "1"},
		{"NEWLINE", "\\n"},
		{"LINE_NUM", "20"},
		{"KEYWORD", "PRINT"},
		{"NAME", "x"},
		{"NEWLINE", "\\n"},
		{"LINE_NUM", "30"},
		{"KEYWORD", "END"},
		{"NEWLINE", "\\n"},
		{"EOF", ""},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(tokens), tokenTypeValues(tokens))
	}
	for i, exp := range expected {
		if tokens[i].TypeName != exp.typeName {
			t.Errorf("Token %d: expected TypeName %q, got %q (value=%q)",
				i, exp.typeName, tokens[i].TypeName, tokens[i].Value)
		}
		if tokens[i].Value != exp.value {
			t.Errorf("Token %d: expected value %q, got %q",
				i, exp.value, tokens[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_VariableNames
// =============================================================================
//
// Verifies that variable names are recognized as NAME tokens.
// Due to case_sensitive: false, NAME values are lowercase.
func TestTokenizeDartmouthBasic_VariableNames(t *testing.T) {
	testCases := []struct {
		source      string
		expectedVar string // lowercase expected value
		desc        string
	}{
		{"10 LET X = 1\n", "x", "single letter"},
		{"10 LET A1 = 2\n", "a1", "letter + digit"},
		{"10 LET Z9 = 3\n", "z9", "letter + digit (max)"},
		{"10 LET B0 = 4\n", "b0", "letter + zero"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeDartmouthBasic(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q (%s): %v", tc.source, tc.desc, err)
		}
		found := false
		for _, tok := range tokens {
			if tok.TypeName == "NAME" && tok.Value == tc.expectedVar {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("Variable %q (%s): NAME(%q) not found in stream: %v",
				tc.source, tc.desc, tc.expectedVar, tokenTypeValues(tokens))
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_LineNumberHook
// =============================================================================
//
// Verifies the LINE_NUM disambiguation hook in detail.
//
// Only the FIRST NUMBER on each line becomes LINE_NUM. Numbers elsewhere
// (GOTO targets, literal values) remain as NUMBER tokens.
func TestTokenizeDartmouthBasic_LineNumberHook(t *testing.T) {
	source := "10 LET X = 99\n20 GOTO 10\n"
	tokens, err := TokenizeDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Check index 0: line 1 label
	if tokens[0].TypeName != "LINE_NUM" || tokens[0].Value != "10" {
		t.Errorf("Expected LINE_NUM(10) at index 0, got %s(%q)",
			tokens[0].TypeName, tokens[0].Value)
	}
	// "99" is a literal — should be NUMBER
	if tokens[4].TypeName != "NUMBER" || tokens[4].Value != "99" {
		t.Errorf("Expected NUMBER(99) at index 4, got %s(%q)",
			tokens[4].TypeName, tokens[4].Value)
	}
	// Line 2 label
	if tokens[6].TypeName != "LINE_NUM" || tokens[6].Value != "20" {
		t.Errorf("Expected LINE_NUM(20) at index 6, got %s(%q)",
			tokens[6].TypeName, tokens[6].Value)
	}
	// GOTO target stays NUMBER
	if tokens[8].TypeName != "NUMBER" || tokens[8].Value != "10" {
		t.Errorf("Expected NUMBER(10) at GOTO target (index 8), got %s(%q)",
			tokens[8].TypeName, tokens[8].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_PrintSeparators
// =============================================================================
//
// Verifies the two PRINT separator tokens: COMMA and SEMICOLON.
func TestTokenizeDartmouthBasic_PrintSeparators(t *testing.T) {
	// Test COMMA separator
	tokens, err := TokenizeDartmouthBasic("10 PRINT X, Y\n")
	if err != nil {
		t.Fatalf("Failed to tokenize COMMA: %v", err)
	}
	found := false
	for _, tok := range tokens {
		if tok.TypeName == "COMMA" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("Expected COMMA token in PRINT X, Y: %v", tokenTypeValues(tokens))
	}

	// Test SEMICOLON separator
	tokens, err = TokenizeDartmouthBasic("10 PRINT X; Y\n")
	if err != nil {
		t.Fatalf("Failed to tokenize SEMICOLON: %v", err)
	}
	found = false
	for _, tok := range tokens {
		if tok.TypeName == "SEMICOLON" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("Expected SEMICOLON token in PRINT X; Y: %v", tokenTypeValues(tokens))
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_NewlineSignificance
// =============================================================================
//
// Verifies that NEWLINE tokens appear in the stream (not skipped).
// NEWLINE is the statement terminator in Dartmouth BASIC.
func TestTokenizeDartmouthBasic_NewlineSignificance(t *testing.T) {
	source := "10 LET X = 1\n20 LET Y = 2\n"
	tokens, err := TokenizeDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	newlineCount := 0
	for _, tok := range tokens {
		if tok.TypeName == "NEWLINE" {
			newlineCount++
		}
	}
	if newlineCount != 2 {
		t.Errorf("Expected 2 NEWLINE tokens (one per statement), got %d", newlineCount)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_WhitespaceSkipped
// =============================================================================
//
// Verifies that horizontal whitespace is consumed silently.
// "10 LET X=5" and "10  LET  X  =  5" should produce identical token streams.
func TestTokenizeDartmouthBasic_WhitespaceSkipped(t *testing.T) {
	tokensDense, err := TokenizeDartmouthBasic("10 LET X=5\n")
	if err != nil {
		t.Fatalf("Failed to tokenize dense: %v", err)
	}

	tokensSpaced, err := TokenizeDartmouthBasic("10  LET  X  =  5\n")
	if err != nil {
		t.Fatalf("Failed to tokenize spaced: %v", err)
	}

	if len(tokensDense) != len(tokensSpaced) {
		t.Fatalf("Dense has %d tokens, spaced has %d tokens",
			len(tokensDense), len(tokensSpaced))
	}
	for i := range tokensDense {
		if tokensDense[i].TypeName != tokensSpaced[i].TypeName {
			t.Errorf("Token %d type: dense=%q, spaced=%q",
				i, tokensDense[i].TypeName, tokensSpaced[i].TypeName)
		}
		if tokensDense[i].Value != tokensSpaced[i].Value {
			t.Errorf("Token %d value: dense=%q, spaced=%q",
				i, tokensDense[i].Value, tokensSpaced[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_EOFToken
// =============================================================================
//
// Verifies that every tokenized source ends with an EOF token.
func TestTokenizeDartmouthBasic_EOFToken(t *testing.T) {
	inputs := []string{
		"10 LET X = 1\n",
		"10 PRINT X\n",
		"10 END\n",
		"10 REM COMMENT\n",
		"10 GOTO 10\n",
	}

	for _, input := range inputs {
		tokens, err := TokenizeDartmouthBasic(input)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", input, err)
		}
		last := tokens[len(tokens)-1]
		if last.TypeName != "EOF" {
			t.Errorf("Input %q: expected last token EOF, got %s(%q)",
				input, last.TypeName, last.Value)
		}
		if last.Value != "" {
			t.Errorf("Input %q: EOF token should have empty value, got %q", input, last.Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_AllKeywords
// =============================================================================
//
// Verifies that all 20 Dartmouth BASIC 1964 keywords are recognized.
// Keyword values are normalized to uppercase by @case_insensitive true.
func TestTokenizeDartmouthBasic_AllKeywords(t *testing.T) {
	keywords := []string{
		"LET", "PRINT", "INPUT", "IF", "THEN", "GOTO", "GOSUB", "RETURN",
		"FOR", "TO", "STEP", "NEXT", "END", "STOP", "REM",
		"READ", "DATA", "RESTORE", "DIM", "DEF",
	}

	for _, kw := range keywords {
		source := "10 " + kw + "\n"
		tokens, err := TokenizeDartmouthBasic(source)
		if err != nil {
			t.Fatalf("Failed to tokenize keyword %q: %v", kw, err)
		}
		if len(tokens) < 2 {
			t.Fatalf("Too few tokens for keyword %q: %v", kw, tokenTypeValues(tokens))
		}
		if tokens[1].TypeName != "KEYWORD" {
			t.Errorf("Keyword %q: expected KEYWORD at index 1, got %s(%q)",
				kw, tokens[1].TypeName, tokens[1].Value)
		}
		// Keyword value should be uppercase (normalized)
		if tokens[1].Value != kw {
			t.Errorf("Keyword %q: expected uppercase value %q, got %q",
				kw, kw, tokens[1].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_UserDefinedFunctions
// =============================================================================
//
// Verifies that user-defined function names (FNA through FNZ) are recognized
// as USER_FN tokens. Values are lowercase due to case_sensitive: false.
func TestTokenizeDartmouthBasic_UserDefinedFunctions(t *testing.T) {
	// Map from uppercase name to expected lowercase value
	userFunctions := map[string]string{
		"FNA": "fna", "FNB": "fnb", "FNC": "fnc",
		"FNX": "fnx", "FNY": "fny", "FNZ": "fnz",
	}

	for upperName, lowerValue := range userFunctions {
		source := "10 LET Y = " + upperName + "(X)\n"
		tokens, err := TokenizeDartmouthBasic(source)
		if err != nil {
			t.Fatalf("Failed to tokenize USER_FN %q: %v", upperName, err)
		}
		found := false
		for _, tok := range tokens {
			if tok.TypeName == "USER_FN" && tok.Value == lowerValue {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("USER_FN %q: expected USER_FN(%q) in stream, got: %v",
				upperName, lowerValue, tokenTypeValues(tokens))
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_ExponentiationOperator
// =============================================================================
//
// Verifies the exponentiation operator (^).
// In Dartmouth BASIC, ^ means "raise to the power of": 2^3 = 8.
func TestTokenizeDartmouthBasic_ExponentiationOperator(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 LET Y = 2^3\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	found := false
	for _, tok := range tokens {
		if tok.TypeName == "CARET" && tok.Value == "^" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("Expected CARET(^) in stream, got: %v", tokenTypeValues(tokens))
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_WindowsLineEndings
// =============================================================================
//
// Verifies that Windows line endings (\r\n) produce NEWLINE tokens.
// The original Dartmouth BASIC ran on teletypes that used \r\n.
func TestTokenizeDartmouthBasic_WindowsLineEndings(t *testing.T) {
	source := "10 LET X = 1\r\n20 PRINT X\r\n"
	tokens, err := TokenizeDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to tokenize Windows line endings: %v", err)
	}

	// Should produce NEWLINE tokens (at least 2)
	newlineCount := 0
	for _, tok := range tokens {
		if tok.TypeName == "NEWLINE" {
			newlineCount++
		}
	}
	if newlineCount < 2 {
		t.Errorf("Expected at least 2 NEWLINE tokens with \\r\\n endings, got %d", newlineCount)
	}

	// Both LINE_NUMs should be correctly identified
	if tokens[0].TypeName != "LINE_NUM" || tokens[0].Value != "10" {
		t.Errorf("Expected LINE_NUM(10) at start, got %s(%q)", tokens[0].TypeName, tokens[0].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_GosubReturn
// =============================================================================
//
// Verifies GOSUB and RETURN keywords.
// GOSUB is BASIC's subroutine call; RETURN pops the return address.
func TestTokenizeDartmouthBasic_GosubReturn(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 GOSUB 100\n")
	if err != nil {
		t.Fatalf("Failed to tokenize GOSUB: %v", err)
	}

	if tokens[1].TypeName != "KEYWORD" || tokens[1].Value != "GOSUB" {
		t.Errorf("Expected KEYWORD(GOSUB), got %s(%q)", tokens[1].TypeName, tokens[1].Value)
	}

	tokens, err = TokenizeDartmouthBasic("200 RETURN\n")
	if err != nil {
		t.Fatalf("Failed to tokenize RETURN: %v", err)
	}
	if tokens[1].TypeName != "KEYWORD" || tokens[1].Value != "RETURN" {
		t.Errorf("Expected KEYWORD(RETURN), got %s(%q)", tokens[1].TypeName, tokens[1].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_LineAndColumn
// =============================================================================
//
// Verifies that tokens carry correct line and column positions.
func TestTokenizeDartmouthBasic_LineAndColumn(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 LET X = 1\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// First token should be at line 1, column 1
	if tokens[0].Line != 1 {
		t.Errorf("Expected line 1 for first token, got %d", tokens[0].Line)
	}
	if tokens[0].Column != 1 {
		t.Errorf("Expected column 1 for first token, got %d", tokens[0].Column)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_ComplexProgram
// =============================================================================
//
// End-to-end test of a complete Dartmouth BASIC program.
//
// This program computes the first 10 squares:
//
//	10 REM COMPUTE SQUARES
//	20 FOR I = 1 TO 10
//	30 LET S = I ^ 2
//	40 PRINT I, S
//	50 NEXT I
//	60 END
func TestTokenizeDartmouthBasic_ComplexProgram(t *testing.T) {
	source := "10 REM COMPUTE SQUARES\n20 FOR I = 1 TO 10\n30 LET S = I ^ 2\n40 PRINT I, S\n50 NEXT I\n60 END\n"
	tokens, err := TokenizeDartmouthBasic(source)
	if err != nil {
		t.Fatalf("Failed to tokenize complex program: %v", err)
	}

	// Verify all six LINE_NUM tokens
	lineNums := []string{}
	for _, tok := range tokens {
		if tok.TypeName == "LINE_NUM" {
			lineNums = append(lineNums, tok.Value)
		}
	}
	expectedLineNums := []string{"10", "20", "30", "40", "50", "60"}
	if len(lineNums) != len(expectedLineNums) {
		t.Fatalf("Expected %d LINE_NUM tokens, got %d: %v", len(expectedLineNums), len(lineNums), lineNums)
	}
	for i, v := range expectedLineNums {
		if lineNums[i] != v {
			t.Errorf("LINE_NUM %d: expected %q, got %q", i, v, lineNums[i])
		}
	}

	// Verify REM content is gone (no NAME("compute") token after REM)
	// "compute" would be NAME if not suppressed
	for _, tok := range tokens {
		if tok.TypeName == "NAME" && tok.Value == "compute" {
			t.Errorf("REM content 'compute' should be suppressed but appears in stream")
		}
	}

	// Verify last token is EOF
	last := tokens[len(tokens)-1]
	if last.TypeName != "EOF" {
		t.Errorf("Expected EOF at end, got %s(%q)", last.TypeName, last.Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_CreateLexerAPI
// =============================================================================
//
// Verifies that CreateDartmouthBasicLexer and TokenizeDartmouthBasic produce
// identical results.
func TestTokenizeDartmouthBasic_CreateLexerAPI(t *testing.T) {
	source := "10 LET X = 42\n"

	// Using CreateDartmouthBasicLexer + Tokenize()
	lex, err := CreateDartmouthBasicLexer(source)
	if err != nil {
		t.Fatalf("CreateDartmouthBasicLexer failed: %v", err)
	}
	tokensCreate := lex.Tokenize()

	// Using TokenizeDartmouthBasic directly
	tokensDirect, err := TokenizeDartmouthBasic(source)
	if err != nil {
		t.Fatalf("TokenizeDartmouthBasic failed: %v", err)
	}

	// Both should produce identical results
	if len(tokensCreate) != len(tokensDirect) {
		t.Fatalf("API mismatch: Create+Tokenize produced %d tokens, TokenizeDartmouthBasic produced %d",
			len(tokensCreate), len(tokensDirect))
	}
	for i := range tokensCreate {
		if tokensCreate[i].TypeName != tokensDirect[i].TypeName {
			t.Errorf("Token %d TypeName: Create=%q, Direct=%q",
				i, tokensCreate[i].TypeName, tokensDirect[i].TypeName)
		}
		if tokensCreate[i].Value != tokensDirect[i].Value {
			t.Errorf("Token %d value: Create=%q, Direct=%q",
				i, tokensCreate[i].Value, tokensDirect[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_StopKeyword
// =============================================================================
//
// Verifies the STOP keyword (terminates execution immediately).
func TestTokenizeDartmouthBasic_StopKeyword(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("999 STOP\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if tokens[0].TypeName != "LINE_NUM" || tokens[0].Value != "999" {
		t.Errorf("Expected LINE_NUM(999), got %s(%q)", tokens[0].TypeName, tokens[0].Value)
	}
	if tokens[1].TypeName != "KEYWORD" || tokens[1].Value != "STOP" {
		t.Errorf("Expected KEYWORD(STOP), got %s(%q)", tokens[1].TypeName, tokens[1].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_DataReadRestore
// =============================================================================
//
// Verifies DATA, READ, and RESTORE keywords.
func TestTokenizeDartmouthBasic_DataReadRestore(t *testing.T) {
	keywords := []string{"DATA", "READ", "RESTORE"}
	for _, kw := range keywords {
		source := "10 " + kw + "\n"
		tokens, err := TokenizeDartmouthBasic(source)
		if err != nil {
			t.Fatalf("Failed to tokenize keyword %q: %v", kw, err)
		}
		if tokens[1].TypeName != "KEYWORD" || tokens[1].Value != kw {
			t.Errorf("Expected KEYWORD(%s), got %s(%q)", kw, tokens[1].TypeName, tokens[1].Value)
		}
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_DimKeyword
// =============================================================================
//
// Verifies the DIM keyword (array dimension declaration).
func TestTokenizeDartmouthBasic_DimKeyword(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 DIM A(10)\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if tokens[1].TypeName != "KEYWORD" || tokens[1].Value != "DIM" {
		t.Errorf("Expected KEYWORD(DIM), got %s(%q)", tokens[1].TypeName, tokens[1].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_InputKeyword
// =============================================================================
//
// Verifies the INPUT keyword (read from user).
func TestTokenizeDartmouthBasic_InputKeyword(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 INPUT X\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	expectedTypes := []string{"LINE_NUM", "KEYWORD", "NAME", "NEWLINE", "EOF"}
	if len(tokens) != len(expectedTypes) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expectedTypes), len(tokens), tokenTypeValues(tokens))
	}
	if tokens[1].Value != "INPUT" {
		t.Errorf("Expected KEYWORD(INPUT), got %s(%q)", tokens[1].TypeName, tokens[1].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_NextKeyword
// =============================================================================
//
// Verifies the NEXT keyword (end of FOR loop body).
func TestTokenizeDartmouthBasic_NextKeyword(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("50 NEXT I\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if tokens[1].TypeName != "KEYWORD" || tokens[1].Value != "NEXT" {
		t.Errorf("Expected KEYWORD(NEXT), got %s(%q)", tokens[1].TypeName, tokens[1].Value)
	}
	// NAME value is lowercase
	if tokens[2].TypeName != "NAME" || tokens[2].Value != "i" {
		t.Errorf("Expected NAME(i), got %s(%q)", tokens[2].TypeName, tokens[2].Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_NewlineValue
// =============================================================================
//
// Verifies that NEWLINE tokens have the value "\\n" (backslash-n, two bytes).
//
// The GrammarLexer.tokenizeStandard hardcodes `Value: "\\n"` for newline
// tokens. This is a representation choice: the value is the two-character
// escape sequence, not the raw newline character.
func TestTokenizeDartmouthBasic_NewlineValue(t *testing.T) {
	tokens, err := TokenizeDartmouthBasic("10 END\n")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	var newlineTok *lexer.Token
	for i := range tokens {
		if tokens[i].TypeName == "NEWLINE" {
			newlineTok = &tokens[i]
			break
		}
	}
	if newlineTok == nil {
		t.Fatal("No NEWLINE token found")
	}
	// The NEWLINE value is "\\n" (two bytes: backslash + n)
	if newlineTok.Value != "\\n" {
		t.Errorf("NEWLINE value: expected \"\\\\n\", got %q", newlineTok.Value)
	}
}

// =============================================================================
// TestTokenizeDartmouthBasic_KeywordNotSplit
// =============================================================================
//
// Verifies that keywords are not split across multiple NAME tokens.
//
// This test confirms the maximal-munch behavior: "PRINT" must not become
// NAME("p") + BUILTIN_FN("rint") or similar incorrect splits. The NAME
// regex /[a-z][a-z0-9]*/ matches the whole word "print" and then the
// keyword promotion mechanism converts it to KEYWORD("PRINT").
func TestTokenizeDartmouthBasic_KeywordNotSplit(t *testing.T) {
	testCases := []string{
		"PRINT", "GOTO", "GOSUB", "RETURN", "RESTORE",
	}

	for _, kw := range testCases {
		source := "10 " + kw + "\n"
		tokens, err := TokenizeDartmouthBasic(source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", kw, err)
		}
		// Index 1 should be a KEYWORD (the whole word), not multiple NAME tokens
		if tokens[1].TypeName != "KEYWORD" {
			t.Errorf("%q: expected single KEYWORD token at index 1, got %s(%q); full stream: %v",
				kw, tokens[1].TypeName, tokens[1].Value, tokenTypeValues(tokens))
		}
		if tokens[1].Value != kw {
			t.Errorf("%q: expected KEYWORD value %q, got %q", kw, kw, tokens[1].Value)
		}
	}
}

// =============================================================================
// Capability cage tests (gen_capabilities.go coverage)
// =============================================================================
//
// The capability cage pattern wraps OS operations in typed boundaries. These
// tests exercise the error-path branches that are not reached by normal usage.

// TestCapabilityCage_CapabilityViolationError verifies that attempting to read
// a file not declared in required_capabilities.json returns a properly-formatted
// capability violation error.
//
// The _capabilityViolationError.Error() method formats a diagnostic message
// pointing to the undeclared path and explaining how to add it.
func TestCapabilityCage_CapabilityViolationError(t *testing.T) {
	fc := &_FileCapabilities{}
	_, err := fc.ReadFile("/etc/passwd")
	if err == nil {
		t.Fatal("expected capability violation error, got nil")
	}
	errMsg := err.Error()
	if len(errMsg) == 0 {
		t.Fatal("capability violation error message must not be empty")
	}
	// The error message must mention the undeclared path. The error formatter
	// uses %q on the filepath.Clean'd path, which on Windows converts forward
	// slashes to backslashes and then escapes them. We reconstruct the expected
	// substring the same way so the check is cross-platform.
	//   Linux:   filepath.Clean("/etc/passwd")  → "/etc/passwd"
	//            fmt.Sprintf("%q", …)            → `"/etc/passwd"`  → inner: /etc/passwd
	//   Windows: filepath.Clean("/etc/passwd")  → `\etc\passwd`
	//            fmt.Sprintf("%q", …)            → `"\\etc\\passwd"` → inner: \\etc\\passwd
	cleanedPath := filepath.Clean("/etc/passwd")
	quotedPath := fmt.Sprintf("%q", cleanedPath)
	// Strip the surrounding double-quotes that %q adds.
	innerPath := quotedPath[1 : len(quotedPath)-1]
	if !containsSubstr(errMsg, innerPath) {
		t.Errorf("expected error to mention requested path, got: %q", errMsg)
	}
}

// containsSubstr is a minimal substring check to avoid importing strings in tests.
func containsSubstr(s, sub string) bool {
	for i := 0; i+len(sub) <= len(s); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

// TestCapabilityCage_OperationFail verifies that ResultFactory.Fail produces
// an OperationResult with DidSucceed=false and the error propagated through
// GetResult as-is (preserving the dynamic type for errors.As).
func TestCapabilityCage_OperationFail(t *testing.T) {
	// Use errors.New to create a sentinel error. We verify the error is non-nil
	// and that its message is preserved (Fail does not wrap the error).
	sentinel := errors.New("expected failure sentinel")

	op := StartNew[int]("test.Fail", 0,
		func(o *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Fail(0, sentinel)
		})

	_, err := op.GetResult()
	if err == nil {
		t.Fatal("expected non-nil error from Fail, got nil")
	}
	// The typed error must be preserved exactly (not wrapped).
	if !errors.Is(err, sentinel) {
		t.Errorf("expected sentinel error to be returned as-is; got %T: %v", err, err)
	}
}

// TestCapabilityCage_OperationAddProperty verifies that AddProperty stores
// key-value metadata on the operation without panicking. This covers the
// method body which populates the property bag used for structured logging.
func TestCapabilityCage_OperationAddProperty(t *testing.T) {
	var capturedOp *Operation[string]

	op := StartNew[string]("test.AddProperty", "",
		func(o *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			capturedOp = o
			o.AddProperty("key", "value")
			o.AddProperty("count", 42)
			return rf.Generate(true, false, "ok")
		})

	val, err := op.GetResult()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if val != "ok" {
		t.Errorf("expected \"ok\", got %q", val)
	}
	// The property bag is private; we verify AddProperty ran by checking the
	// captured operation's bag length via reflection is not needed — the fact
	// that no panic occurred is the observable guarantee.
	_ = capturedOp
}

// TestCapabilityCage_PanicCaught verifies that a panic inside the operation
// callback is caught and converted to an unexpected-failure error (when
// PanicOnUnexpected is NOT set). The fallback value is returned.
func TestCapabilityCage_PanicCaught(t *testing.T) {
	op := StartNew[int]("test.PanicCaught", 99,
		func(o *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			panic("deliberate test panic")
		})

	val, err := op.GetResult()
	if err == nil {
		t.Fatal("expected error from panicking callback, got nil")
	}
	if val != 99 {
		t.Errorf("expected fallback value 99, got %d", val)
	}
}

// TestCapabilityCage_PanicOnUnexpected verifies that PanicOnUnexpected() causes
// a panic from the callback to be re-panicked (not swallowed).
func TestCapabilityCage_PanicOnUnexpected(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Error("expected PanicOnUnexpected to re-panic, but no panic occurred")
		}
	}()

	op := StartNew[int]("test.PanicOnUnexpected", 0,
		func(o *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			panic("deliberate re-panic")
		}).PanicOnUnexpected()

	op.GetResult() //nolint:errcheck
}

// TestCapabilityCage_UnexpectedFailureWithoutPanic verifies that returning
// DidFailUnexpectedly=true from the callback (without a panic) produces an
// error via GetResult.
func TestCapabilityCage_UnexpectedFailureWithoutPanic(t *testing.T) {
	op := StartNew[int]("test.UnexpectedFailure", 0,
		func(o *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(false, true, 0)
		})

	_, err := op.GetResult()
	if err == nil {
		t.Fatal("expected error for unexpected failure, got nil")
	}
}

// TestCapabilityCage_ExpectedFailureNoErr verifies that returning DidSucceed=false
// with no typed Err (nil) falls through to the generic "operation failed" error.
func TestCapabilityCage_ExpectedFailureNoErr(t *testing.T) {
	op := StartNew[int]("test.ExpectedFailureNoErr", 0,
		func(o *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			// Generate with didSucceed=false, didFailUnexpectedly=false, no Err set
			return &OperationResult[int]{
				DidSucceed:          false,
				DidFailUnexpectedly: false,
				ReturnValue:         0,
				Err:                 nil,
			}
		})

	_, err := op.GetResult()
	if err == nil {
		t.Fatal("expected error, got nil")
	}
}

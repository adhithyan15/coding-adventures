package algollexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// =============================================================================
// TestTokenizeAlgol_Keywords
// =============================================================================
//
// Verifies that all ALGOL 60 keywords are correctly recognized and produce
// keyword tokens rather than IDENT tokens.
//
// ALGOL 60 has a rich keyword vocabulary compared to, say, JSON (which has
// none). Keywords cover control flow (if/then/else/for/do), block structure
// (begin/end), type names (integer/real/boolean/string), and boolean operators
// (and/or/not/impl/eqv). All keywords are case-insensitive per the ALGOL spec.
func TestTokenizeAlgol_Keywords(t *testing.T) {
	testCases := []struct {
		source   string
		typeName string
	}{
		{"begin", "BEGIN"},
		{"end", "END"},
		{"if", "IF"},
		{"then", "THEN"},
		{"else", "ELSE"},
		{"for", "FOR"},
		{"do", "DO"},
		{"step", "STEP"},
		{"until", "UNTIL"},
		{"while", "WHILE"},
		{"goto", "GOTO"},
		{"integer", "INTEGER"},
		{"real", "REAL"},
		{"boolean", "BOOLEAN"},
		{"procedure", "PROCEDURE"},
		{"array", "ARRAY"},
		{"switch", "SWITCH"},
		{"own", "OWN"},
		{"label", "LABEL"},
		{"value", "VALUE"},
		{"true", "TRUE"},
		{"false", "FALSE"},
		{"not", "NOT"},
		{"and", "AND"},
		{"or", "OR"},
		{"impl", "IMPL"},
		{"eqv", "EQV"},
		{"div", "DIV"},
		{"mod", "MOD"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeAlgol(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize keyword %q: %v", tc.source, err)
		}

		if len(tokens) < 1 {
			t.Fatalf("Expected at least 1 token for %q, got 0", tc.source)
		}

		if tokens[0].TypeName != tc.typeName {
			t.Errorf("Keyword %q: expected TypeName %q, got %q",
				tc.source, tc.typeName, tokens[0].TypeName)
		}
		// The token value should be the original source text
		if tokens[0].Value != tc.source {
			t.Errorf("Keyword %q: expected value %q, got %q",
				tc.source, tc.source, tokens[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeAlgol_AssignVsEquality
// =============================================================================
//
// Verifies that := (ASSIGN) and = (EQ) are correctly distinguished.
//
// This is one of ALGOL 60's most important design choices. By requiring := for
// assignment, ALGOL makes it impossible to accidentally write an assignment
// where you meant a comparison. In C, writing "if (x = 0)" is valid but
// assigns 0 to x instead of testing for equality — a bug that has caused
// countless production failures. ALGOL's := prevents this class of error
// entirely at the lexer level.
//
// Historical note: Donald Knuth has said the use of = for equality (not
// assignment) in ALGOL was one of the language's best decisions.
func TestTokenizeAlgol_AssignVsEquality(t *testing.T) {
	// := is a single ASSIGN token, not COLON then EQ
	tokens, err := TokenizeAlgol(":=")
	if err != nil {
		t.Fatalf("Failed to tokenize :=: %v", err)
	}
	if tokens[0].TypeName != "ASSIGN" {
		t.Errorf("Expected ASSIGN for :=, got %q", tokens[0].TypeName)
	}
	if tokens[0].Value != ":=" {
		t.Errorf("Expected value :=, got %q", tokens[0].Value)
	}

	// = alone is EQ (equality comparison, not assignment)
	tokens, err = TokenizeAlgol("=")
	if err != nil {
		t.Fatalf("Failed to tokenize =: %v", err)
	}
	if tokens[0].TypeName != "EQ" {
		t.Errorf("Expected EQ for =, got %q", tokens[0].TypeName)
	}

	// In context: "x := y = z" should lex as IDENT ASSIGN IDENT EQ IDENT
	tokens, err = TokenizeAlgol("x := y = z")
	if err != nil {
		t.Fatalf("Failed to tokenize assignment with equality: %v", err)
	}
	expected := []string{"NAME", "ASSIGN", "NAME", "EQ", "NAME", "EOF"}
	for i, typeName := range expected {
		if i >= len(tokens) {
			t.Fatalf("Too few tokens: expected %d, got %d", len(expected), len(tokens))
		}
		if tokens[i].TypeName != typeName {
			t.Errorf("Token %d: expected %q, got %q", i, typeName, tokens[i].TypeName)
		}
	}
}

// =============================================================================
// TestTokenizeAlgol_Operators
// =============================================================================
//
// Verifies that all ALGOL 60 operators are lexed correctly.
//
// ALGOL 60's operator set mixes symbol and keyword operators:
//   - Arithmetic: + - * / ** ^ (the last two are exponentiation)
//   - Relational: = < > <= >= !=
//   - Boolean: and or not impl eqv (all keywords, not symbols)
//   - Assignment: :=
//
// The multi-character operators (**  <=  >=  !=  :=) must be tried before
// their single-character prefixes (* < > ! :) — the lexer grammar enforces
// this ordering via declaration order in algol.tokens.
func TestTokenizeAlgol_Operators(t *testing.T) {
	testCases := []struct {
		source   string
		typeName string
		value    string
	}{
		{"+", "PLUS", "+"},
		{"-", "MINUS", "-"},
		{"*", "STAR", "*"},
		{"/", "SLASH", "/"},
		{"**", "POWER", "**"},
		{"^", "CARET", "^"},
		{"=", "EQ", "="},
		{"<", "LT", "<"},
		{">", "GT", ">"},
		{"<=", "LEQ", "<="},
		{">=", "GEQ", ">="},
		{"!=", "NEQ", "!="},
		{":=", "ASSIGN", ":="},
		{"(", "LPAREN", "("},
		{")", "RPAREN", ")"},
		{"[", "LBRACKET", "["},
		{"]", "RBRACKET", "]"},
		{";", "SEMICOLON", ";"},
		{",", "COMMA", ","},
		{":", "COLON", ":"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeAlgol(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize operator %q: %v", tc.source, err)
		}

		if len(tokens) < 1 {
			t.Fatalf("Expected at least 1 token for %q", tc.source)
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
// TestTokenizeAlgol_IntegerLit
// =============================================================================
//
// Verifies that integer literals (sequences of decimal digits) are tokenized
// correctly.
//
// ALGOL 60 integer literals are simple sequences of one or more decimal digits.
// Unlike C, there is no 0x prefix for hex, no 0 prefix for octal, and no
// trailing type suffixes (L, U, etc.). What you see is what you get.
func TestTokenizeAlgol_IntegerLit(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"0", "0"},
		{"1", "1"},
		{"42", "42"},
		{"1000", "1000"},
		{"999999", "999999"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeAlgol(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize integer %q: %v", tc.source, err)
		}

		if tokens[0].TypeName != "INTEGER_LIT" {
			t.Errorf("Integer %q: expected INTEGER_LIT, got %q",
				tc.source, tokens[0].TypeName)
		}
		if tokens[0].Value != tc.value {
			t.Errorf("Integer %q: expected value %q, got %q",
				tc.source, tc.value, tokens[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeAlgol_RealLit
// =============================================================================
//
// Verifies that real (floating-point) literals are tokenized correctly.
//
// ALGOL 60 real literals have several forms:
//   3.14        — integer part + fractional part
//   1.5E3       — decimal + exponent (= 1500.0)
//   1.5E-3      — decimal + negative exponent (= 0.0015)
//   100E2       — integer + exponent, no decimal point (= 10000.0)
//
// The grammar requires that REAL_LIT be tried before INTEGER_LIT so that
// "3.14" doesn't match as INTEGER_LIT("3") followed by a lone dot.
//
// ALGOL 60 used the notation 1.5×10³ in the original report. Implementations
// adopted the E notation (from Fortran) as the ASCII equivalent.
func TestTokenizeAlgol_RealLit(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"3.14", "3.14"},
		{"0.5", "0.5"},
		{"1.0", "1.0"},
		{"1.5E3", "1.5E3"},
		{"1.5E-3", "1.5E-3"},
		{"100E2", "100E2"},
		{"2.998e8", "2.998e8"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeAlgol(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize real %q: %v", tc.source, err)
		}

		if tokens[0].TypeName != "REAL_LIT" {
			t.Errorf("Real %q: expected REAL_LIT, got %q",
				tc.source, tokens[0].TypeName)
		}
		if tokens[0].Value != tc.value {
			t.Errorf("Real %q: expected value %q, got %q",
				tc.source, tc.value, tokens[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeAlgol_StringLit
// =============================================================================
//
// Verifies that string literals (single-quoted) are tokenized correctly.
//
// ALGOL 60 uses single quotes for string literals: 'hello world'
// There are no escape sequences — a single quote cannot appear inside a string.
// This is simpler than C's backslash escaping, but means strings cannot contain
// apostrophes without a workaround (some implementations doubled the quote: '').
//
// Example: 'hello world' → STRING_LIT token with value (including quotes)
func TestTokenizeAlgol_StringLit(t *testing.T) {
	testCases := []struct {
		source string
	}{
		{"'hello world'"},
		{"'x'"},
		{"''"},
		{"'ALGOL 60'"},
		{"'this is a string'"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeAlgol(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize string %q: %v", tc.source, err)
		}

		if tokens[0].TypeName != "STRING_LIT" {
			t.Errorf("String %q: expected STRING_LIT, got %q",
				tc.source, tokens[0].TypeName)
		}
	}
}

// =============================================================================
// TestTokenizeAlgol_CommentSkipping
// =============================================================================
//
// Verifies that ALGOL 60 comments are silently consumed and produce no tokens.
//
// ALGOL 60 uses a unique comment syntax: the keyword "comment" followed by
// arbitrary text up to and including the next semicolon. The comment and the
// terminating semicolon are consumed together — they produce no tokens.
//
// Example:
//   comment this is ignored; x := 1
//   → IDENT("x") ASSIGN(":=") INTEGER_LIT("1") EOF
//
// This differs from line comments (// in C) and block comments (/* */ in C).
// ALGOL comments are statement-level constructs: they appear where a statement
// could appear and are terminated by the statement terminator ";".
func TestTokenizeAlgol_CommentSkipping(t *testing.T) {
	// "comment ignored;" should be silently consumed
	// Only "x := 1" remains
	source := "comment ignored; x := 1"
	tokens, err := TokenizeAlgol(source)
	if err != nil {
		t.Fatalf("Failed to tokenize with comment: %v", err)
	}

	// Should yield: IDENT("x"), ASSIGN(":="), INTEGER_LIT("1"), EOF
	// The comment keyword, its body, and the semicolon are all gone.
	nonEOF := make([]lexer.Token, 0)
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			nonEOF = append(nonEOF, tok)
		}
	}

	if len(nonEOF) != 3 {
		t.Fatalf("Expected 3 non-EOF tokens after comment skip, got %d: %v", len(nonEOF), nonEOF)
	}

	expectedTypes := []string{"NAME", "ASSIGN", "INTEGER_LIT"}
	for i, typeName := range expectedTypes {
		if nonEOF[i].TypeName != typeName {
			t.Errorf("Token %d: expected %q, got %q (value=%q)",
				i, typeName, nonEOF[i].TypeName, nonEOF[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeAlgol_KeywordBoundary
// =============================================================================
//
// Verifies that keywords embedded in longer identifiers are NOT reclassified.
//
// This is the "keyword boundary" or "maximal munch" property. The identifier
// "beginning" contains "begin" but should be tokenized as a single IDENT,
// not as BEGIN + IDENT("ning"). Similarly "endif", "forloop", "integer_val"
// (wait, ALGOL doesn't allow underscores, but "integers" is a valid IDENT).
//
// This works because the IDENT pattern matches the longest possible sequence
// of letters and digits, and keyword reclassification only applies to exact
// matches of the whole token value.
func TestTokenizeAlgol_KeywordBoundary(t *testing.T) {
	testCases := []struct {
		source   string
		expected string // expected TypeName
	}{
		{"beginning", "NAME"},  // contains "begin" but is not BEGIN
		{"endif", "NAME"},      // contains "end" and "if" but is IDENT
		{"forloop", "NAME"},    // contains "for" but is IDENT
		{"integers", "NAME"},   // contains "integer" but is IDENT
		{"notfound", "NAME"},   // contains "not" but is IDENT
		{"andmore", "NAME"},    // contains "and" but is IDENT
		{"trueish", "NAME"},    // contains "true" but is IDENT
		{"begin", "BEGIN"},      // exact match IS a keyword
		{"integer", "INTEGER"},  // exact match IS a keyword
	}

	for _, tc := range testCases {
		tokens, err := TokenizeAlgol(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tc.source, err)
		}

		if len(tokens) < 1 {
			t.Fatalf("No tokens for %q", tc.source)
		}

		if tokens[0].TypeName != tc.expected {
			t.Errorf("Boundary %q: expected %q, got %q",
				tc.source, tc.expected, tokens[0].TypeName)
		}
	}
}

// =============================================================================
// TestTokenizeAlgol_FullExpression
// =============================================================================
//
// Verifies that a full arithmetic expression tokenizes into the expected stream.
//
// The expression "x := 1 + 2 * 3" is a classic test for operator parsing.
// The lexer does not care about precedence — that is the parser's job.
// The lexer simply produces tokens in left-to-right order.
//
// Expected token stream:
//   IDENT("x") ASSIGN(":=") INTEGER_LIT("1") PLUS("+") INTEGER_LIT("2")
//   STAR("*") INTEGER_LIT("3") EOF
func TestTokenizeAlgol_FullExpression(t *testing.T) {
	source := "x := 1 + 2 * 3"
	tokens, err := TokenizeAlgol(source)
	if err != nil {
		t.Fatalf("Failed to tokenize expression: %v", err)
	}

	expected := []struct {
		typeName string
		value    string
	}{
		{"NAME", "x"},
		{"ASSIGN", ":="},
		{"INTEGER_LIT", "1"},
		{"PLUS", "+"},
		{"INTEGER_LIT", "2"},
		{"STAR", "*"},
		{"INTEGER_LIT", "3"},
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
// TestTokenizeAlgol_Whitespace
// =============================================================================
//
// Verifies that whitespace between tokens produces no tokens of its own.
//
// ALGOL 60 is free-format: whitespace (spaces, tabs, newlines) between
// tokens is always insignificant. "x:=1" and "x   :=   1" produce identical
// token streams. This was a deliberate improvement over Fortran, which had
// strict column-based formatting inherited from punched cards.
//
// This test verifies that the skip pattern in algol.tokens correctly consumes
// all whitespace variants.
func TestTokenizeAlgol_Whitespace(t *testing.T) {
	// Dense version: no spaces
	tokensCompact, err := TokenizeAlgol("x:=1")
	if err != nil {
		t.Fatalf("Failed to tokenize compact form: %v", err)
	}

	// Spaced version: spaces around all tokens
	tokensSpaced, err := TokenizeAlgol("x := 1")
	if err != nil {
		t.Fatalf("Failed to tokenize spaced form: %v", err)
	}

	// Both should produce the same token types and values
	if len(tokensCompact) != len(tokensSpaced) {
		t.Fatalf("Compact has %d tokens, spaced has %d tokens",
			len(tokensCompact), len(tokensSpaced))
	}

	for i := range tokensCompact {
		if tokensCompact[i].TypeName != tokensSpaced[i].TypeName {
			t.Errorf("Token %d: compact=%q, spaced=%q",
				i, tokensCompact[i].TypeName, tokensSpaced[i].TypeName)
		}
		if tokensCompact[i].Value != tokensSpaced[i].Value {
			t.Errorf("Token %d value: compact=%q, spaced=%q",
				i, tokensCompact[i].Value, tokensSpaced[i].Value)
		}
	}
}

// =============================================================================
// TestTokenizeAlgol_EOFToken
// =============================================================================
//
// Verifies that every tokenized ALGOL source ends with an EOF token.
// The EOF token signals to the parser that no more input is available.
// Without it, a parser would read past the end of the token stream.
func TestTokenizeAlgol_EOFToken(t *testing.T) {
	inputs := []string{
		"begin end",
		"x := 42",
		"integer x",
		"true",
		"3.14",
		"'hello'",
	}

	for _, input := range inputs {
		tokens, err := TokenizeAlgol(input)
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
// TestTokenizeAlgol_MinimalProgram
// =============================================================================
//
// Verifies that a minimal but complete ALGOL 60 program tokenizes correctly.
//
// The smallest useful ALGOL 60 program declares a variable and assigns it:
//   begin integer x; x := 42 end
//
// Note the semicolon after the declaration — ALGOL uses semicolons as
// separators between statements, not terminators. There is no semicolon
// after the last statement before end.
func TestTokenizeAlgol_MinimalProgram(t *testing.T) {
	source := "begin integer x; x := 42 end"
	tokens, err := TokenizeAlgol(source)
	if err != nil {
		t.Fatalf("Failed to tokenize minimal program: %v", err)
	}

	expected := []struct {
		typeName string
		value    string
	}{
		{"BEGIN", "begin"},
		{"INTEGER", "integer"},
		{"NAME", "x"},
		{"SEMICOLON", ";"},
		{"NAME", "x"},
		{"ASSIGN", ":="},
		{"INTEGER_LIT", "42"},
		{"END", "end"},
		{"EOF", ""},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(tokens), tokens)
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
// TestTokenizeAlgol_LineAndColumn
// =============================================================================
//
// Verifies that tokens carry correct line and column positions.
// Accurate position information is essential for error messages: the parser
// uses line/column to say "unexpected token at line 3, column 12".
func TestTokenizeAlgol_LineAndColumn(t *testing.T) {
	source := "begin integer x"
	tokens, err := TokenizeAlgol(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// First token "begin" should be at line 1, column 1
	if tokens[0].Line != 1 {
		t.Errorf("Expected line 1 for first token, got %d", tokens[0].Line)
	}
	if tokens[0].Column != 1 {
		t.Errorf("Expected column 1 for first token, got %d", tokens[0].Column)
	}
}

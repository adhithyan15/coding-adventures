package starlarklexer

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// =============================================================================
// TestTokenizeStarlarkSimple
// =============================================================================
//
// Verifies that a basic arithmetic assignment `x = 1 + 2` produces the
// expected token stream. This is the simplest possible Starlark program
// and exercises: NAME, EQUALS, INT (number), and PLUS tokens.
func TestTokenizeStarlarkSimple(t *testing.T) {
	source := `x = 1 + 2`
	tokens, err := TokenizeStarlark(source)
	if err != nil {
		t.Fatalf("Failed to tokenize Starlark source: %v", err)
	}

	// Expected tokens:
	//   NAME("x"), EQUALS("="), INT("1"), PLUS("+"), INT("2"),
	//   NEWLINE, EOF
	//
	// The NEWLINE is emitted at EOF because indentation mode always ensures
	// a trailing NEWLINE before the final EOF token.
	if len(tokens) < 5 {
		t.Fatalf("Expected at least 5 tokens, got %d: %v", len(tokens), tokens)
	}

	// Verify the first token is the identifier "x"
	if tokens[0].Type != lexer.TokenName || tokens[0].Value != "x" {
		t.Errorf("Expected NAME 'x', got %s %q", tokens[0].Type, tokens[0].Value)
	}

	// Verify the equals sign
	if tokens[1].Type != lexer.TokenEquals || tokens[1].Value != "=" {
		t.Errorf("Expected EQUALS '=', got %s %q", tokens[1].Type, tokens[1].Value)
	}

	// Verify the integer literal "1"
	if tokens[2].Value != "1" {
		t.Errorf("Expected value '1', got %q", tokens[2].Value)
	}

	// Verify the plus operator
	if tokens[3].Type != lexer.TokenPlus || tokens[3].Value != "+" {
		t.Errorf("Expected PLUS '+', got %s %q", tokens[3].Type, tokens[3].Value)
	}

	// Verify the integer literal "2"
	if tokens[4].Value != "2" {
		t.Errorf("Expected value '2', got %q", tokens[4].Value)
	}
}

// =============================================================================
// TestTokenizeStarlarkKeywords
// =============================================================================
//
// Verifies that Starlark keywords are recognized and emitted as KEYWORD tokens
// rather than NAME tokens. The keyword list includes: def, return, if, elif,
// else, for, in, and, or, not, pass, break, continue, load, lambda, True,
// False, None.
func TestTokenizeStarlarkKeywords(t *testing.T) {
	// Each keyword on its own line so they are separated by NEWLINEs.
	// We check each keyword individually to ensure the lexer's keyword
	// reclassification logic works for every entry in the keywords: section.
	keywords := []string{
		"def", "return", "if", "elif", "else", "for", "in",
		"and", "or", "not", "pass", "break", "continue", "load",
		"lambda", "True", "False", "None",
	}

	for _, kw := range keywords {
		tokens, err := TokenizeStarlark(kw)
		if err != nil {
			t.Fatalf("Failed to tokenize keyword %q: %v", kw, err)
		}

		// The first token should be a KEYWORD with the keyword value.
		// After that we expect NEWLINE and EOF.
		if tokens[0].Type != lexer.TokenKeyword {
			t.Errorf("Expected %q to be KEYWORD, got %s", kw, tokens[0].Type)
		}
		if tokens[0].Value != kw {
			t.Errorf("Expected keyword value %q, got %q", kw, tokens[0].Value)
		}
	}
}

// =============================================================================
// TestTokenizeStarlarkReservedKeyword
// =============================================================================
//
// Verifies that reserved keywords (Python keywords not in Starlark) cause a
// panic. The starlark.tokens grammar reserves words like `class`, `while`,
// `import`, etc. When the lexer encounters one as a NAME, it panics with a
// clear error message rather than silently producing a misleading token.
//
// We test `class` specifically because it is the most common source of
// confusion for Python programmers writing Starlark for the first time.
func TestTokenizeStarlarkReservedKeyword(t *testing.T) {
	reservedWords := []string{"class", "while", "import", "try", "except", "raise", "yield"}

	for _, word := range reservedWords {
		// Use defer/recover to catch the expected panic.
		// The lexer panics (rather than returning an error) for reserved keywords
		// because this is a hard failure — the source code is invalid Starlark.
		func() {
			defer func() {
				r := recover()
				if r == nil {
					t.Errorf("Expected panic for reserved keyword %q, but no panic occurred", word)
					return
				}
				// Verify the panic message mentions the reserved word
				msg, ok := r.(string)
				if !ok {
					t.Errorf("Expected string panic for %q, got %T: %v", word, r, r)
					return
				}
				if !strings.Contains(msg, word) {
					t.Errorf("Panic message for %q doesn't mention the word: %s", word, msg)
				}
			}()

			// This should panic because "class" (etc.) is a reserved keyword
			TokenizeStarlark(word)
		}()
	}
}

// =============================================================================
// TestTokenizeStarlarkIndentation
// =============================================================================
//
// Verifies that the lexer produces INDENT and DEDENT tokens for indented blocks.
// This is the core of Starlark's significant-whitespace model.
//
// For the input:
//   def f():
//       return 1
//
// The expected token sequence is:
//   KEYWORD("def"), NAME("f"), LPAREN, RPAREN, COLON, NEWLINE,
//   INDENT, KEYWORD("return"), INT("1"), NEWLINE, DEDENT, NEWLINE, EOF
//
// The INDENT token appears when the indentation level increases from 0 to 4.
// The DEDENT token appears at the end when indentation returns to 0.
func TestTokenizeStarlarkIndentation(t *testing.T) {
	source := "def f():\n    return 1\n"
	tokens, err := TokenizeStarlark(source)
	if err != nil {
		t.Fatalf("Failed to tokenize indented source: %v", err)
	}

	// Look for INDENT and DEDENT tokens by checking TypeName.
	// In indentation mode, INDENT and DEDENT are emitted as tokens with
	// TypeName "INDENT" and "DEDENT" respectively.
	hasIndent := false
	hasDedent := false
	for _, tok := range tokens {
		if tok.TypeName == "INDENT" {
			hasIndent = true
		}
		if tok.TypeName == "DEDENT" {
			hasDedent = true
		}
	}

	if !hasIndent {
		t.Errorf("Expected INDENT token in indented block, got tokens: %v", tokens)
	}
	if !hasDedent {
		t.Errorf("Expected DEDENT token after indented block, got tokens: %v", tokens)
	}

	// Verify that "def" is recognized as a keyword
	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "def" {
		t.Errorf("Expected KEYWORD 'def', got %s %q", tokens[0].Type, tokens[0].Value)
	}
}

// =============================================================================
// TestTokenizeStarlarkBrackets
// =============================================================================
//
// Verifies that NEWLINE tokens are suppressed inside brackets.
// This is critical for Starlark because function calls and list/dict literals
// often span multiple lines:
//
//   cc_library(
//       name = "foo",
//       srcs = ["bar.cc"],
//   )
//
// Without bracket suppression, the lexer would emit NEWLINEs inside the
// parentheses, breaking the parser's ability to parse the call expression.
func TestTokenizeStarlarkBrackets(t *testing.T) {
	// A multi-line function call — NEWLINEs inside parens should be suppressed
	source := "f(\n    1,\n    2\n)\n"
	tokens, err := TokenizeStarlark(source)
	if err != nil {
		t.Fatalf("Failed to tokenize bracketed expression: %v", err)
	}

	// Count NEWLINEs between LPAREN and RPAREN.
	// There should be zero NEWLINEs inside the brackets.
	insideBrackets := false
	newlinesInside := 0
	for _, tok := range tokens {
		if tok.Type == lexer.TokenLParen {
			insideBrackets = true
			continue
		}
		if tok.Type == lexer.TokenRParen {
			insideBrackets = false
			continue
		}
		if insideBrackets && tok.Type == lexer.TokenNewline {
			newlinesInside++
		}
	}

	if newlinesInside > 0 {
		t.Errorf("Expected 0 NEWLINEs inside brackets, got %d. Tokens: %v", newlinesInside, tokens)
	}

	// Verify we still get the values inside
	foundOne := false
	foundTwo := false
	for _, tok := range tokens {
		if tok.Value == "1" {
			foundOne = true
		}
		if tok.Value == "2" {
			foundTwo = true
		}
	}
	if !foundOne || !foundTwo {
		t.Errorf("Expected to find values '1' and '2' inside brackets")
	}
}

// =============================================================================
// TestTokenizeStarlarkOperators
// =============================================================================
//
// Verifies that multi-character operators are tokenized correctly.
// The starlark.tokens grammar defines these in order of length (longest first)
// so that "**" matches before "*", "//" before "/", etc.
//
// This is a critical ordering concern: if "*" were defined before "**", then
// the input "**" would be tokenized as STAR STAR instead of DOUBLE_STAR.
func TestTokenizeStarlarkOperators(t *testing.T) {
	// Test multi-character operators that require longest-match ordering
	testCases := []struct {
		source   string
		expected string
		typeName string
	}{
		{"2 ** 3", "**", "DOUBLE_STAR"},
		{"10 // 3", "//", "FLOOR_DIV"},
		{"x == y", "==", "EQUALS_EQUALS"},
		{"x != y", "!=", "NOT_EQUALS"},
		{"x <= y", "<=", "LESS_EQUALS"},
		{"x >= y", ">=", "GREATER_EQUALS"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeStarlark(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tc.source, err)
		}

		// Find the operator token (should be the second non-whitespace token,
		// at index 1 since index 0 is the left operand)
		found := false
		for _, tok := range tokens {
			if tok.Value == tc.expected {
				if tok.TypeName != tc.typeName {
					t.Errorf("Operator %q: expected TypeName %q, got %q",
						tc.expected, tc.typeName, tok.TypeName)
				}
				found = true
				break
			}
		}
		if !found {
			t.Errorf("Operator %q not found in tokens for %q: %v",
				tc.expected, tc.source, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeStarlarkStrings
// =============================================================================
//
// Verifies that string literals are tokenized correctly, including:
//   - Double-quoted strings: "hello"
//   - Single-quoted strings: 'hello'
//   - Escape sequences: \n, \t, \\, \"
//
// The grammar defines multiple string patterns (STRING_DQ, STRING_SQ,
// STRING_TRIPLE_DQ, etc.) that all emit the same STRING token type via
// the -> STRING alias. This means the parser sees a uniform STRING token
// regardless of quote style.
func TestTokenizeStarlarkStrings(t *testing.T) {
	// Double-quoted string with escape sequences
	source := `x = "hello\nworld"`
	tokens, err := TokenizeStarlark(source)
	if err != nil {
		t.Fatalf("Failed to tokenize string literal: %v", err)
	}

	// Find the STRING token
	found := false
	for _, tok := range tokens {
		if tok.Type == lexer.TokenString {
			found = true
			// The grammar lexer processes escape sequences, so \n becomes
			// an actual newline character in the token value
			if !strings.Contains(tok.Value, "hello") {
				t.Errorf("Expected string to contain 'hello', got %q", tok.Value)
			}
			break
		}
	}
	if !found {
		t.Errorf("No STRING token found in tokens: %v", tokens)
	}

	// Test single-quoted string
	source2 := `y = 'world'`
	tokens2, err := TokenizeStarlark(source2)
	if err != nil {
		t.Fatalf("Failed to tokenize single-quoted string: %v", err)
	}

	found2 := false
	for _, tok := range tokens2 {
		if tok.Type == lexer.TokenString {
			found2 = true
			if tok.Value != "world" {
				t.Errorf("Expected string value 'world', got %q", tok.Value)
			}
			break
		}
	}
	if !found2 {
		t.Errorf("No STRING token found for single-quoted string: %v", tokens2)
	}
}

// =============================================================================
// TestTokenizeStarlarkComments
// =============================================================================
//
// Verifies that comments (# to end of line) are skipped by the lexer.
// Comments produce no tokens — they are matched by the skip: COMMENT pattern
// and consumed silently. This is essential because Starlark BUILD files often
// contain extensive comments explaining build targets and dependencies.
func TestTokenizeStarlarkComments(t *testing.T) {
	source := "x = 1 # this is a comment\n"
	tokens, err := TokenizeStarlark(source)
	if err != nil {
		t.Fatalf("Failed to tokenize source with comment: %v", err)
	}

	// The comment should be completely absent from the token stream.
	// We should see: NAME("x"), EQUALS("="), INT("1"), NEWLINE, NEWLINE, EOF
	for _, tok := range tokens {
		if strings.Contains(tok.Value, "comment") || strings.Contains(tok.Value, "#") {
			t.Errorf("Comment content leaked into token stream: %v", tok)
		}
	}

	// Verify the actual code tokens are present
	if tokens[0].Value != "x" {
		t.Errorf("Expected first token 'x', got %q", tokens[0].Value)
	}
	if tokens[2].Value != "1" {
		t.Errorf("Expected third token '1', got %q", tokens[2].Value)
	}
}

// =============================================================================
// TestTokenizeStarlarkFloat
// =============================================================================
//
// Verifies that floating-point literals are tokenized as FLOAT tokens.
// The starlark.tokens grammar defines FLOAT before INT so that "3.14" is
// matched as a single FLOAT token rather than INT("3") DOT INT("14").
//
// Starlark supports several float formats:
//   - 3.14     (decimal with fractional part)
//   - .5       (leading dot)
//   - 5.       (trailing dot)
//   - 1e10     (scientific notation)
//   - 1.5e-3   (scientific with fractional part)
func TestTokenizeStarlarkFloat(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"x = 3.14", "3.14"},
		{"x = .5", ".5"},
		{"x = 5.", "5."},
		{"x = 1e10", "1e10"},
		{"x = 1.5e-3", "1.5e-3"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeStarlark(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize float %q: %v", tc.source, err)
		}

		// Find the FLOAT token — it should have TypeName "FLOAT"
		found := false
		for _, tok := range tokens {
			if tok.TypeName == "FLOAT" && tok.Value == tc.value {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("Expected FLOAT token with value %q in %q, got tokens: %v",
				tc.value, tc.source, tokens)
		}
	}
}

// =============================================================================
// TestCreateStarlarkLexer
// =============================================================================
//
// Verifies that the factory function CreateStarlarkLexer returns a valid
// GrammarLexer instance that can be used for tokenization. This tests the
// two-step API (create lexer, then call Tokenize) as opposed to the
// one-shot TokenizeStarlark convenience function.
func TestCreateStarlarkLexer(t *testing.T) {
	source := `x = 42`
	starlarkLexer, err := CreateStarlarkLexer(source)
	if err != nil {
		t.Fatalf("Failed to create Starlark lexer: %v", err)
	}

	// The lexer should not be nil
	if starlarkLexer == nil {
		t.Fatal("CreateStarlarkLexer returned nil lexer")
	}

	// Tokenize using the created lexer instance
	tokens := starlarkLexer.Tokenize()

	// Should produce at least: NAME("x"), EQUALS("="), INT("42"), NEWLINE, EOF
	if len(tokens) < 4 {
		t.Fatalf("Expected at least 4 tokens, got %d: %v", len(tokens), tokens)
	}

	// Verify the last token is EOF
	lastToken := tokens[len(tokens)-1]
	if lastToken.Type != lexer.TokenEOF {
		t.Errorf("Expected last token to be EOF, got %s", lastToken.Type)
	}
}

// =============================================================================
// TestTokenizeStarlarkHexOctalIntegers
// =============================================================================
//
// Verifies that hexadecimal and octal integer literals are tokenized correctly.
// The starlark.tokens grammar defines INT_HEX and INT_OCT patterns that emit
// as INT via type aliases. These patterns must come before the decimal INT
// pattern so that "0xFF" is one token, not INT("0") NAME("xFF").
func TestTokenizeStarlarkHexOctalIntegers(t *testing.T) {
	testCases := []struct {
		source string
		value  string
	}{
		{"x = 0xFF", "0xFF"},
		{"x = 0o77", "0o77"},
		{"x = 0xDEAD", "0xDEAD"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeStarlark(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tc.source, err)
		}

		// Find the INT token with the expected value
		found := false
		for _, tok := range tokens {
			if tok.Value == tc.value && tok.TypeName == "INT" {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("Expected INT token with value %q, got tokens: %v", tc.value, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeStarlarkAugmentedAssignment
// =============================================================================
//
// Verifies that augmented assignment operators (+=, -=, *=, etc.) are tokenized
// as single tokens. These are three-character operators defined before the
// two-character operators in starlark.tokens to ensure correct longest-match.
func TestTokenizeStarlarkAugmentedAssignment(t *testing.T) {
	testCases := []struct {
		source   string
		opValue  string
		typeName string
	}{
		{"x += 1", "+=", "PLUS_EQUALS"},
		{"x -= 1", "-=", "MINUS_EQUALS"},
		{"x *= 2", "*=", "STAR_EQUALS"},
		{"x //= 3", "//=", "FLOOR_DIV_EQUALS"},
	}

	for _, tc := range testCases {
		tokens, err := TokenizeStarlark(tc.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tc.source, err)
		}

		found := false
		for _, tok := range tokens {
			if tok.Value == tc.opValue {
				if tok.TypeName != tc.typeName {
					t.Errorf("Operator %q: expected TypeName %q, got %q",
						tc.opValue, tc.typeName, tok.TypeName)
				}
				found = true
				break
			}
		}
		if !found {
			t.Errorf("Augmented assignment %q not found in tokens: %v",
				tc.opValue, tokens)
		}
	}
}

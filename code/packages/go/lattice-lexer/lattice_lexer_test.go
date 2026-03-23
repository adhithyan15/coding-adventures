package latticelexer

// Lattice Lexer Tests
// ===================
//
// These tests verify that TokenizeLatticeLexer produces the correct token
// stream for a wide range of Lattice and CSS input. We use table-driven
// tests (the idiomatic Go testing pattern) organized by feature area.
//
// Test philosophy: test observable behaviour, not internal implementation.
// We check token types (as strings) and values, not the regex internals.
//
// Token type names come from the lattice.tokens grammar:
//   VARIABLE, AT_KEYWORD, IDENT, FUNCTION, HASH, NUMBER, DIMENSION,
//   PERCENTAGE, STRING, COLON, SEMICOLON, LBRACE, RBRACE, COMMA,
//   EQUALS, EQUALS_EQUALS, NOT_EQUALS, GREATER, GREATER_EQUALS,
//   LESS_EQUALS, PLUS, MINUS, STAR, DOT, BANG, etc.

import (
	"testing"
)

// tokenSpec describes a single expected token in a test assertion.
type tokenSpec struct {
	typ   string // expected token type name
	value string // expected token value
}

// runTokenTest tokenizes `source`, discards the EOF token, and checks that
// the remaining tokens match `expected` in order.
func runTokenTest(t *testing.T, source string, expected []tokenSpec) {
	t.Helper()

	tokens, err := TokenizeLatticeLexer(source)
	if err != nil {
		t.Fatalf("TokenizeLatticeLexer(%q) returned error: %v", source, err)
	}

	// Remove the EOF token — all streams end with it; we don't need to
	// re-verify it in every test case.
	//
	// Note: Token.Type is a lexer.TokenType (int). For grammar-driven tokens,
	// the human-readable name is in Token.TypeName (e.g. "VARIABLE", "IDENT").
	nonEOF := tokens[:0:0]
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			nonEOF = append(nonEOF, tok)
		}
	}

	if len(nonEOF) != len(expected) {
		t.Fatalf("TokenizeLatticeLexer(%q):\n  got  %d tokens: %v\n  want %d tokens: %v",
			source, len(nonEOF), nonEOF, len(expected), expected)
	}

	for i, want := range expected {
		got := nonEOF[i]
		if got.TypeName != want.typ {
			t.Errorf("token[%d] type: got %q, want %q (source: %q)",
				i, got.TypeName, want.typ, source)
		}
		if got.Value != want.value {
			t.Errorf("token[%d] value: got %q, want %q (source: %q)",
				i, got.Value, want.value, source)
		}
	}
}

// ============================================================================
// Variable Tokens
// ============================================================================
//
// Variables are the most distinctive Lattice token: $ followed by an
// identifier. The $ character never appears in valid CSS value positions,
// making this completely unambiguous.
//
// Grammar: VARIABLE = /\$[a-zA-Z_][a-zA-Z0-9_-]*/

func TestVariableSimple(t *testing.T) {
	// $color is the canonical variable name format: $ + camelCase or lowercase
	runTokenTest(t, "$color", []tokenSpec{
		{"VARIABLE", "$color"},
	})
}

func TestVariableWithDash(t *testing.T) {
	// CSS convention: use kebab-case for variable names ($font-size, not $fontSize)
	runTokenTest(t, "$font-size-lg", []tokenSpec{
		{"VARIABLE", "$font-size-lg"},
	})
}

func TestVariableUnderscoreStart(t *testing.T) {
	// Underscores are valid at the start of a variable name
	runTokenTest(t, "$_private", []tokenSpec{
		{"VARIABLE", "$_private"},
	})
}

func TestVariableWithNumbers(t *testing.T) {
	// Numbers are valid after the first character
	runTokenTest(t, "$h1Size", []tokenSpec{
		{"VARIABLE", "$h1Size"},
	})
}

func TestVariableDeclaration(t *testing.T) {
	// Full variable declaration: $name: value;
	// This is the atomic unit of Lattice: binds a name to a value.
	runTokenTest(t, "$color: red;", []tokenSpec{
		{"VARIABLE", "$color"},
		{"COLON", ":"},
		{"IDENT", "red"},
		{"SEMICOLON", ";"},
	})
}

func TestVariableWithHashValue(t *testing.T) {
	// Variables commonly hold hex colors. $primary: #4a90d9 is a pattern
	// found in virtually every real-world Lattice stylesheet.
	runTokenTest(t, "$primary: #4a90d9;", []tokenSpec{
		{"VARIABLE", "$primary"},
		{"COLON", ":"},
		{"HASH", "#4a90d9"},
		{"SEMICOLON", ";"},
	})
}

func TestVariableWithDimension(t *testing.T) {
	// Dimension variables: $base-size: 16px
	// DIMENSION token = number + unit letters (16px, 2em, 1.5rem, 100vh)
	runTokenTest(t, "$base-size: 16px;", []tokenSpec{
		{"VARIABLE", "$base-size"},
		{"COLON", ":"},
		{"DIMENSION", "16px"},
		{"SEMICOLON", ";"},
	})
}

// ============================================================================
// CSS Token Preservation
// ============================================================================
//
// Lattice must preserve all CSS tokens unchanged. These tests verify that
// the five new Lattice tokens don't interfere with standard CSS tokenization.

func TestCSSSelector(t *testing.T) {
	// Type selector: the simplest selector — just an element name.
	runTokenTest(t, "h1", []tokenSpec{
		{"IDENT", "h1"},
	})
}

func TestCSSClassSelector(t *testing.T) {
	// Class selector: .classname → DOT + IDENT
	runTokenTest(t, ".primary-btn", []tokenSpec{
		{"DOT", "."},
		{"IDENT", "primary-btn"},
	})
}

func TestCSSIdSelector(t *testing.T) {
	// ID selector: #id → HASH token. Note that HASH is also used for
	// hex colors (#4a90d9). The parser distinguishes them by context.
	runTokenTest(t, "#header", []tokenSpec{
		{"HASH", "#header"},
	})
}

func TestCSSProperty(t *testing.T) {
	// A full CSS declaration: property: value;
	// IDENT for property name, COLON, IDENT for value, SEMICOLON.
	runTokenTest(t, "color: red;", []tokenSpec{
		{"IDENT", "color"},
		{"COLON", ":"},
		{"IDENT", "red"},
		{"SEMICOLON", ";"},
	})
}

func TestCSSRule(t *testing.T) {
	// Complete CSS rule with braces
	runTokenTest(t, "h1 { color: red; }", []tokenSpec{
		{"IDENT", "h1"},
		{"LBRACE", "{"},
		{"IDENT", "color"},
		{"COLON", ":"},
		{"IDENT", "red"},
		{"SEMICOLON", ";"},
		{"RBRACE", "}"},
	})
}

func TestCSSNumberValue(t *testing.T) {
	// z-index: 100 — a plain number (no unit, no percent)
	runTokenTest(t, "z-index: 100;", []tokenSpec{
		{"IDENT", "z-index"},
		{"COLON", ":"},
		{"NUMBER", "100"},
		{"SEMICOLON", ";"},
	})
}

func TestCSSDimension(t *testing.T) {
	// Dimension: 16px — number + unit. DIMENSION must come before IDENT in
	// the grammar to prevent "16" being tokenized as NUMBER and "px" as IDENT.
	runTokenTest(t, "font-size: 16px;", []tokenSpec{
		{"IDENT", "font-size"},
		{"COLON", ":"},
		{"DIMENSION", "16px"},
		{"SEMICOLON", ";"},
	})
}

func TestCSSPercentage(t *testing.T) {
	// Percentage: 50% — number + percent sign.
	// PERCENTAGE must come before NUMBER in the grammar.
	runTokenTest(t, "width: 50%;", []tokenSpec{
		{"IDENT", "width"},
		{"COLON", ":"},
		{"PERCENTAGE", "50%"},
		{"SEMICOLON", ";"},
	})
}

func TestCSSDoubleQuoteString(t *testing.T) {
	// Double-quoted string → STRING token
	runTokenTest(t, `font-family: "Helvetica Neue";`, []tokenSpec{
		{"IDENT", "font-family"},
		{"COLON", ":"},
		{"STRING", "Helvetica Neue"},
		{"SEMICOLON", ";"},
	})
}

func TestCSSSingleQuoteString(t *testing.T) {
	// Single-quoted string → also STRING (via STRING_SQ -> STRING alias)
	runTokenTest(t, `content: 'hello';`, []tokenSpec{
		{"IDENT", "content"},
		{"COLON", ":"},
		{"STRING", "hello"},
		{"SEMICOLON", ";"},
	})
}

func TestCSSFunctionCall(t *testing.T) {
	// CSS function: rgb(255, 0, 128)
	// FUNCTION token = identifier immediately followed by (, e.g. "rgb("
	runTokenTest(t, "color: rgb(255, 0, 128);", []tokenSpec{
		{"IDENT", "color"},
		{"COLON", ":"},
		{"FUNCTION", "rgb("},
		{"NUMBER", "255"},
		{"COMMA", ","},
		{"NUMBER", "0"},
		{"COMMA", ","},
		{"NUMBER", "128"},
		{"RPAREN", ")"},
		{"SEMICOLON", ";"},
	})
}

func TestCSSAtKeyword(t *testing.T) {
	// AT_KEYWORD covers all @-rules: CSS ones and Lattice ones.
	// @media is a CSS at-rule.
	runTokenTest(t, "@media screen", []tokenSpec{
		{"AT_KEYWORD", "@media"},
		{"IDENT", "screen"},
	})
}

func TestCSSCustomProperty(t *testing.T) {
	// CSS custom properties (CSS variables): --var-name
	// These are different from Lattice variables ($name).
	// CSS custom properties survive to the browser; Lattice variables don't.
	runTokenTest(t, "--primary-color: #4a90d9;", []tokenSpec{
		{"CUSTOM_PROPERTY", "--primary-color"},
		{"COLON", ":"},
		{"HASH", "#4a90d9"},
		{"SEMICOLON", ";"},
	})
}

// ============================================================================
// Comparison Operators (New in Lattice)
// ============================================================================
//
// These four operators are used in @if conditions. They MUST come before
// their single-character components in the grammar to avoid ambiguity:
//   == before =, != before !, >= before >, <= before <
//
// Note: there is no < single-character token in the Lattice grammar,
// but <= must still be declared before LESS_EQUALS would need ordering.

func TestEqualsEquals(t *testing.T) {
	// == is the equality comparison operator: @if $theme == dark { ... }
	runTokenTest(t, "==", []tokenSpec{
		{"EQUALS_EQUALS", "=="},
	})
}

func TestNotEquals(t *testing.T) {
	// != is the inequality comparison operator: @if $mode != light { ... }
	runTokenTest(t, "!=", []tokenSpec{
		{"NOT_EQUALS", "!="},
	})
}

func TestGreaterEquals(t *testing.T) {
	// >= is greater-or-equal: @if $count >= 3 { ... }
	runTokenTest(t, ">=", []tokenSpec{
		{"GREATER_EQUALS", ">="},
	})
}

func TestLessEquals(t *testing.T) {
	// <= is less-or-equal: @if $i <= 12 { ... }
	runTokenTest(t, "<=", []tokenSpec{
		{"LESS_EQUALS", "<="},
	})
}

func TestGreater(t *testing.T) {
	// > alone is still the GREATER token (CSS child combinator, also @for)
	runTokenTest(t, ">", []tokenSpec{
		{"GREATER", ">"},
	})
}

func TestEqualsAlone(t *testing.T) {
	// = alone is the EQUALS token (CSS attribute selector =, also @use as)
	runTokenTest(t, "=", []tokenSpec{
		{"EQUALS", "="},
	})
}

func TestNotEqualsVsEqualsEquals(t *testing.T) {
	// Verify disambiguation: != and == are distinct from ! and ==
	runTokenTest(t, "!= ==", []tokenSpec{
		{"NOT_EQUALS", "!="},
		{"EQUALS_EQUALS", "=="},
	})
}

// ============================================================================
// At-Keywords for Lattice Constructs
// ============================================================================
//
// All Lattice at-keywords use the AT_KEYWORD token type — they are
// distinguished from CSS at-rules by the grammar's literal matching.

func TestAtMixin(t *testing.T) {
	runTokenTest(t, "@mixin", []tokenSpec{
		{"AT_KEYWORD", "@mixin"},
	})
}

func TestAtInclude(t *testing.T) {
	runTokenTest(t, "@include", []tokenSpec{
		{"AT_KEYWORD", "@include"},
	})
}

func TestAtIf(t *testing.T) {
	runTokenTest(t, "@if", []tokenSpec{
		{"AT_KEYWORD", "@if"},
	})
}

func TestAtElse(t *testing.T) {
	runTokenTest(t, "@else", []tokenSpec{
		{"AT_KEYWORD", "@else"},
	})
}

func TestAtFor(t *testing.T) {
	runTokenTest(t, "@for", []tokenSpec{
		{"AT_KEYWORD", "@for"},
	})
}

func TestAtEach(t *testing.T) {
	runTokenTest(t, "@each", []tokenSpec{
		{"AT_KEYWORD", "@each"},
	})
}

func TestAtFunction(t *testing.T) {
	runTokenTest(t, "@function", []tokenSpec{
		{"AT_KEYWORD", "@function"},
	})
}

func TestAtReturn(t *testing.T) {
	runTokenTest(t, "@return", []tokenSpec{
		{"AT_KEYWORD", "@return"},
	})
}

func TestAtUse(t *testing.T) {
	runTokenTest(t, "@use", []tokenSpec{
		{"AT_KEYWORD", "@use"},
	})
}

// ============================================================================
// Comment and Whitespace Skipping
// ============================================================================
//
// All whitespace and comments are skip patterns — they produce no tokens.
// This matches CSS behaviour for whitespace, and adds // comments (Lattice).

func TestSkipLineComment(t *testing.T) {
	// // comment to end of line — Lattice extension, not in CSS
	// The two tokens on either side of the comment are what matter.
	runTokenTest(t, "$x: 1; // this is a comment\n$y: 2;", []tokenSpec{
		{"VARIABLE", "$x"},
		{"COLON", ":"},
		{"NUMBER", "1"},
		{"SEMICOLON", ";"},
		{"VARIABLE", "$y"},
		{"COLON", ":"},
		{"NUMBER", "2"},
		{"SEMICOLON", ";"},
	})
}

func TestSkipBlockComment(t *testing.T) {
	// /* block comment */ — standard CSS, also supported by Lattice
	runTokenTest(t, "color: /* brand */ red;", []tokenSpec{
		{"IDENT", "color"},
		{"COLON", ":"},
		{"IDENT", "red"},
		{"SEMICOLON", ";"},
	})
}

func TestSkipWhitespace(t *testing.T) {
	// Whitespace (spaces, tabs, newlines) between tokens is skipped
	runTokenTest(t, "$x  :  \t red  ;", []tokenSpec{
		{"VARIABLE", "$x"},
		{"COLON", ":"},
		{"IDENT", "red"},
		{"SEMICOLON", ";"},
	})
}

func TestSkipMultilineBlockComment(t *testing.T) {
	// Block comments can span multiple lines
	runTokenTest(t, "a /* line1\nline2 */ b", []tokenSpec{
		{"IDENT", "a"},
		{"IDENT", "b"},
	})
}

// ============================================================================
// Full Lattice Snippet Tests
// ============================================================================
//
// These tests verify realistic Lattice code snippets produce the correct
// token streams from start to finish.

func TestMixinDefinitionHeader(t *testing.T) {
	// @mixin button($bg, $fg: white)
	// FUNCTION token for "button(" — function-call form includes the paren
	runTokenTest(t, "@mixin button($bg) {", []tokenSpec{
		{"AT_KEYWORD", "@mixin"},
		{"FUNCTION", "button("},
		{"VARIABLE", "$bg"},
		{"RPAREN", ")"},
		{"LBRACE", "{"},
	})
}

func TestIncludeDirective(t *testing.T) {
	// @include button(#4a90d9); — expand mixin with an argument
	runTokenTest(t, "@include button(#4a90d9);", []tokenSpec{
		{"AT_KEYWORD", "@include"},
		{"FUNCTION", "button("},
		{"HASH", "#4a90d9"},
		{"RPAREN", ")"},
		{"SEMICOLON", ";"},
	})
}

func TestIfDirective(t *testing.T) {
	// @if $theme == dark { ... }
	// All tokens including the comparison operator
	runTokenTest(t, "@if $theme == dark {", []tokenSpec{
		{"AT_KEYWORD", "@if"},
		{"VARIABLE", "$theme"},
		{"EQUALS_EQUALS", "=="},
		{"IDENT", "dark"},
		{"LBRACE", "{"},
	})
}

func TestForDirective(t *testing.T) {
	// @for $i from 1 through 12 {
	// "from" and "through" are IDENT tokens — the grammar matches them literally
	runTokenTest(t, "@for $i from 1 through 12 {", []tokenSpec{
		{"AT_KEYWORD", "@for"},
		{"VARIABLE", "$i"},
		{"IDENT", "from"},
		{"NUMBER", "1"},
		{"IDENT", "through"},
		{"NUMBER", "12"},
		{"LBRACE", "{"},
	})
}

func TestEachDirective(t *testing.T) {
	// @each $color in red, green, blue {
	// "in" is an IDENT token
	runTokenTest(t, "@each $color in red, green, blue {", []tokenSpec{
		{"AT_KEYWORD", "@each"},
		{"VARIABLE", "$color"},
		{"IDENT", "in"},
		{"IDENT", "red"},
		{"COMMA", ","},
		{"IDENT", "green"},
		{"COMMA", ","},
		{"IDENT", "blue"},
		{"LBRACE", "{"},
	})
}

func TestFunctionDefinitionHeader(t *testing.T) {
	// @function spacing($n) {
	runTokenTest(t, "@function spacing($n) {", []tokenSpec{
		{"AT_KEYWORD", "@function"},
		{"FUNCTION", "spacing("},
		{"VARIABLE", "$n"},
		{"RPAREN", ")"},
		{"LBRACE", "{"},
	})
}

func TestReturnDirective(t *testing.T) {
	// @return $n * 8px;
	// * is the STAR token; 8px is a DIMENSION token
	runTokenTest(t, "@return $n * 8px;", []tokenSpec{
		{"AT_KEYWORD", "@return"},
		{"VARIABLE", "$n"},
		{"STAR", "*"},
		{"DIMENSION", "8px"},
		{"SEMICOLON", ";"},
	})
}

func TestUseDirective(t *testing.T) {
	// @use "colors" as c; — import a module
	runTokenTest(t, `@use "colors" as c;`, []tokenSpec{
		{"AT_KEYWORD", "@use"},
		{"STRING", "colors"},
		{"IDENT", "as"},
		{"IDENT", "c"},
		{"SEMICOLON", ";"},
	})
}

// ============================================================================
// Edge Cases
// ============================================================================

func TestEmptyInput(t *testing.T) {
	// Empty input should produce only the EOF token (which we strip).
	// After stripping EOF, the slice should be empty.
	tokens, err := TokenizeLatticeLexer("")
	if err != nil {
		t.Fatalf("unexpected error on empty input: %v", err)
	}
	nonEOF := 0
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			nonEOF++
		}
	}
	if nonEOF != 0 {
		t.Errorf("empty input: got %d non-EOF tokens, want 0", nonEOF)
	}
}

func TestOnlyComments(t *testing.T) {
	// A file with only comments produces no tokens (they are all skipped)
	runTokenTest(t, "// comment\n/* block */", []tokenSpec{})
}

func TestNegativeNumber(t *testing.T) {
	// Negative numbers: -1 is a NUMBER token (the minus is part of the regex)
	runTokenTest(t, "-1", []tokenSpec{
		{"NUMBER", "-1"},
	})
}

func TestNegativeDimension(t *testing.T) {
	// Negative dimension: -10px (common for negative margins)
	runTokenTest(t, "-10px", []tokenSpec{
		{"DIMENSION", "-10px"},
	})
}

func TestDecimalNumber(t *testing.T) {
	// Decimal number: 3.14
	runTokenTest(t, "3.14", []tokenSpec{
		{"NUMBER", "3.14"},
	})
}

func TestEOFTokenPresent(t *testing.T) {
	// The last token in any stream must always be EOF.
	// This is important for parsers that peek ahead.
	tokens, err := TokenizeLatticeLexer("$x: 1;")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(tokens) == 0 {
		t.Fatal("got empty token slice, want at least EOF")
	}
	last := tokens[len(tokens)-1]
	if last.TypeName != "EOF" {
		t.Errorf("last token TypeName: got %q, want %q", last.TypeName, "EOF")
	}
}

func TestCreateLatticeLexerReturnsLexer(t *testing.T) {
	// CreateLatticeLexer should return a non-nil lexer without error
	lex, err := CreateLatticeLexer("$color: red;")
	if err != nil {
		t.Fatalf("CreateLatticeLexer returned error: %v", err)
	}
	if lex == nil {
		t.Fatal("CreateLatticeLexer returned nil lexer")
	}
	// The lexer should produce tokens when called
	tokens := lex.Tokenize()
	if len(tokens) == 0 {
		t.Fatal("Tokenize returned empty slice")
	}
}

func TestVariableInPropertyValue(t *testing.T) {
	// Variable used as a property value: color: $primary;
	// This is the core use case for Lattice variables.
	runTokenTest(t, "color: $primary;", []tokenSpec{
		{"IDENT", "color"},
		{"COLON", ":"},
		{"VARIABLE", "$primary"},
		{"SEMICOLON", ";"},
	})
}

func TestMixinWithDefaultParam(t *testing.T) {
	// @mixin button($bg, $fg: white) — parameter with default value
	runTokenTest(t, "@mixin button($bg, $fg: white) {", []tokenSpec{
		{"AT_KEYWORD", "@mixin"},
		{"FUNCTION", "button("},
		{"VARIABLE", "$bg"},
		{"COMMA", ","},
		{"VARIABLE", "$fg"},
		{"COLON", ":"},
		{"IDENT", "white"},
		{"RPAREN", ")"},
		{"LBRACE", "{"},
	})
}

func TestComparisonWithNumbers(t *testing.T) {
	// @if $count >= 3 — comparison operator between variable and number
	runTokenTest(t, "$count >= 3", []tokenSpec{
		{"VARIABLE", "$count"},
		{"GREATER_EQUALS", ">="},
		{"NUMBER", "3"},
	})
}

func TestComparisonLessEquals(t *testing.T) {
	// $i <= 10 — less-or-equal comparison
	runTokenTest(t, "$i <= 10", []tokenSpec{
		{"VARIABLE", "$i"},
		{"LESS_EQUALS", "<="},
		{"NUMBER", "10"},
	})
}

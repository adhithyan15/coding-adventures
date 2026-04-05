// Tests for the Mosaic lexer.
//
// The Mosaic token grammar defines these categories:
//   - KEYWORD: reserved words (component, slot, text, number, etc.)
//   - NAME:    identifiers (PascalCase, camelCase, kebab-case)
//   - STRING:  double-quoted string literals
//   - NUMBER:  numeric literals (integer and decimal)
//   - DIMENSION: number + unit (16dp, 1.5sp, 100%)
//   - COLOR_HEX: hex colors (#rgb, #rrggbb, #rrggbbaa)
//   - Structural tokens: { } < > : ; , . = @
//   - Comments and whitespace are silently skipped
package mosaiclexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// helper: tokenize and fatalf on error
func mustTokenize(t *testing.T, source string) []lexer.Token {
	t.Helper()
	tokens, err := Tokenize(source)
	if err != nil {
		t.Fatalf("Tokenize(%q) error: %v", source, err)
	}
	return tokens
}

// helper: find first token with given TypeName
func findByType(tokens []lexer.Token, typeName string) *lexer.Token {
	for i := range tokens {
		if tokens[i].TypeName == typeName {
			return &tokens[i]
		}
	}
	return nil
}

// =============================================================================
// TestTokenizeKeywordComponent
// =============================================================================
//
// "component" is a reserved keyword. It should tokenize as KEYWORD, not NAME.
func TestTokenizeKeywordComponent(t *testing.T) {
	tokens := mustTokenize(t, "component")
	tok := findByType(tokens, "KEYWORD")
	if tok == nil {
		t.Fatalf("Expected KEYWORD token, got: %v", tokens)
	}
	if tok.Value != "component" {
		t.Errorf("Expected value 'component', got %q", tok.Value)
	}
}

// =============================================================================
// TestTokenizeKeywordSlot
// =============================================================================
//
// "slot" is a reserved keyword.
func TestTokenizeKeywordSlot(t *testing.T) {
	tokens := mustTokenize(t, "slot")
	tok := findByType(tokens, "KEYWORD")
	if tok == nil {
		t.Fatalf("Expected KEYWORD token")
	}
	if tok.Value != "slot" {
		t.Errorf("Expected 'slot', got %q", tok.Value)
	}
}

// =============================================================================
// TestTokenizeAllKeywords
// =============================================================================
//
// Every Mosaic keyword must tokenize as KEYWORD rather than NAME.
// Keywords: component slot import from as text number bool image color node list
//           true false when each
func TestTokenizeAllKeywords(t *testing.T) {
	keywords := []string{
		"component", "slot", "import", "from", "as",
		"text", "number", "bool", "image", "color", "node", "list",
		"true", "false", "when", "each",
	}
	for _, kw := range keywords {
		tokens := mustTokenize(t, kw)
		tok := findByType(tokens, "KEYWORD")
		if tok == nil {
			t.Errorf("Keyword %q: expected KEYWORD token, got %v", kw, tokens)
			continue
		}
		if tok.Value != kw {
			t.Errorf("Keyword %q: expected value %q, got %q", kw, kw, tok.Value)
		}
	}
}

// =============================================================================
// TestTokenizeName
// =============================================================================
//
// Identifiers that are not keywords should tokenize as NAME.
// Mosaic names can include hyphens for CSS-like property names.
func TestTokenizeName(t *testing.T) {
	cases := []string{
		"Button", "ProfileCard", "corner-radius", "font-size", "a11y-label",
		"myComponent", "_internal",
	}
	for _, name := range cases {
		tokens := mustTokenize(t, name)
		tok := findByType(tokens, "NAME")
		if tok == nil {
			t.Errorf("Name %q: expected NAME token, got %v", name, tokens)
			continue
		}
		if tok.Value != name {
			t.Errorf("Name %q: expected value %q, got %q", name, name, tok.Value)
		}
	}
}

// =============================================================================
// TestTokenizeString
// =============================================================================
//
// Mosaic strings are double-quoted. The lexer strips the surrounding quotes.
func TestTokenizeString(t *testing.T) {
	tokens := mustTokenize(t, `"hello world"`)
	tok := findByType(tokens, "STRING")
	if tok == nil {
		t.Fatalf("Expected STRING token")
	}
	if tok.Value != "hello world" {
		t.Errorf("Expected 'hello world', got %q", tok.Value)
	}
}

// =============================================================================
// TestTokenizeNumber
// =============================================================================
//
// Numeric literals (integers and decimals) tokenize as NUMBER.
func TestTokenizeNumber(t *testing.T) {
	cases := []struct {
		src string
		val string
	}{
		{"0", "0"},
		{"42", "42"},
		{"-3.14", "-3.14"},
		{"100", "100"},
		{"0.5", "0.5"},
	}
	for _, tc := range cases {
		tokens := mustTokenize(t, tc.src)
		tok := findByType(tokens, "NUMBER")
		if tok == nil {
			t.Errorf("%q: expected NUMBER token, got %v", tc.src, tokens)
			continue
		}
		if tok.Value != tc.val {
			t.Errorf("%q: expected %q, got %q", tc.src, tc.val, tok.Value)
		}
	}
}

// =============================================================================
// TestTokenizeDimension
// =============================================================================
//
// DIMENSION tokens are a number immediately followed by a unit suffix.
// The grammar ensures DIMENSION is matched before NUMBER (same input,
// longer match wins).
func TestTokenizeDimension(t *testing.T) {
	cases := []struct {
		src string
		val string
	}{
		{"16dp", "16dp"},
		{"1.5sp", "1.5sp"},
		{"100%", "100%"},
		{"-8dp", "-8dp"},
		{"24px", "24px"},
	}
	for _, tc := range cases {
		tokens := mustTokenize(t, tc.src)
		tok := findByType(tokens, "DIMENSION")
		if tok == nil {
			t.Errorf("%q: expected DIMENSION token, got %v", tc.src, tokens)
			continue
		}
		if tok.Value != tc.val {
			t.Errorf("%q: expected %q, got %q", tc.src, tc.val, tok.Value)
		}
	}
}

// =============================================================================
// TestTokenizeColorHex
// =============================================================================
//
// COLOR_HEX tokens start with # and contain 3, 6, or 8 hex digits.
func TestTokenizeColorHex(t *testing.T) {
	cases := []struct {
		src string
		val string
	}{
		{"#fff", "#fff"},
		{"#2563eb", "#2563eb"},
		{"#ff000080", "#ff000080"},
		{"#abc", "#abc"},
	}
	for _, tc := range cases {
		tokens := mustTokenize(t, tc.src)
		tok := findByType(tokens, "COLOR_HEX")
		if tok == nil {
			t.Errorf("%q: expected COLOR_HEX token, got %v", tc.src, tokens)
			continue
		}
		if tok.Value != tc.val {
			t.Errorf("%q: expected %q, got %q", tc.src, tc.val, tok.Value)
		}
	}
}

// =============================================================================
// TestTokenizeStructuralTokens
// =============================================================================
//
// Mosaic has 9 structural tokens. Each maps to a well-known TypeName.
func TestTokenizeStructuralTokens(t *testing.T) {
	cases := []struct {
		src      string
		typeName string
	}{
		{"{", "LBRACE"},
		{"}", "RBRACE"},
		{"<", "LANGLE"},
		{">", "RANGLE"},
		{":", "COLON"},
		{";", "SEMICOLON"},
		{",", "COMMA"},
		{".", "DOT"},
		{"=", "EQUALS"},
		{"@", "AT"},
	}
	for _, tc := range cases {
		tokens := mustTokenize(t, tc.src)
		tok := findByType(tokens, tc.typeName)
		if tok == nil {
			t.Errorf("%q: expected %s token, got %v", tc.src, tc.typeName, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeLineCommentSkipped
// =============================================================================
//
// Line comments (// ...) are consumed by the skip pattern and produce no tokens.
func TestTokenizeLineCommentSkipped(t *testing.T) {
	tokens := mustTokenize(t, "// this is a comment\n42")
	// Should only have NUMBER and EOF
	for _, tok := range tokens {
		if tok.TypeName == "LINE_COMMENT" {
			t.Errorf("Line comment should be skipped, got LINE_COMMENT token")
		}
	}
	numTok := findByType(tokens, "NUMBER")
	if numTok == nil || numTok.Value != "42" {
		t.Errorf("Expected NUMBER(42) after comment, got %v", tokens)
	}
}

// =============================================================================
// TestTokenizeBlockCommentSkipped
// =============================================================================
//
// Block comments (/* ... */) are consumed by the skip pattern.
func TestTokenizeBlockCommentSkipped(t *testing.T) {
	tokens := mustTokenize(t, "/* block comment */ 99")
	numTok := findByType(tokens, "NUMBER")
	if numTok == nil || numTok.Value != "99" {
		t.Errorf("Expected NUMBER(99) after block comment, got %v", tokens)
	}
}

// =============================================================================
// TestTokenizeSimpleComponent
// =============================================================================
//
// A minimal Mosaic component should tokenize into the expected sequence.
func TestTokenizeSimpleComponent(t *testing.T) {
	source := `component Button {
  slot label: text;
  Text {}
}`
	tokens := mustTokenize(t, source)

	// Verify key tokens are present
	found := map[string]bool{}
	for _, tok := range tokens {
		found[tok.TypeName+":"+tok.Value] = true
	}

	checks := []string{
		"KEYWORD:component", "NAME:Button", "LBRACE:{",
		"KEYWORD:slot", "NAME:label", "COLON::", "KEYWORD:text", "SEMICOLON:;",
		"NAME:Text", "LBRACE:{", "RBRACE:}", "RBRACE:}",
	}
	for _, check := range checks {
		if !found[check] {
			t.Errorf("Expected token %q in stream, got %v", check, tokens)
		}
	}
}

// =============================================================================
// TestTokenizeSlotDeclaration
// =============================================================================
//
// A slot declaration with type annotation and default value.
func TestTokenizeSlotDeclaration(t *testing.T) {
	source := `slot count: number = 0;`
	tokens := mustTokenize(t, source)

	typeNames := make([]string, 0)
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			typeNames = append(typeNames, tok.TypeName+":"+tok.Value)
		}
	}

	expected := []string{
		"KEYWORD:slot", "NAME:count", "COLON::", "KEYWORD:number", "EQUALS:=", "NUMBER:0", "SEMICOLON:;",
	}
	if len(typeNames) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(typeNames), typeNames)
	}
	for i, exp := range expected {
		if typeNames[i] != exp {
			t.Errorf("Token %d: expected %q, got %q", i, exp, typeNames[i])
		}
	}
}

// =============================================================================
// TestTokenizeListType
// =============================================================================
//
// list<text> uses KEYWORD, LANGLE, KEYWORD, RANGLE tokens.
func TestTokenizeListType(t *testing.T) {
	source := `list<text>`
	tokens := mustTokenize(t, source)

	typeNames := make([]string, 0)
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			typeNames = append(typeNames, tok.TypeName+":"+tok.Value)
		}
	}

	expected := []string{
		"KEYWORD:list", "LANGLE:<", "KEYWORD:text", "RANGLE:>",
	}
	if len(typeNames) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(typeNames), typeNames)
	}
	for i, exp := range expected {
		if typeNames[i] != exp {
			t.Errorf("Token %d: expected %q, got %q", i, exp, typeNames[i])
		}
	}
}

// =============================================================================
// TestTokenizeSlotReference
// =============================================================================
//
// A slot reference (@name) tokenizes as AT + NAME.
func TestTokenizeSlotReference(t *testing.T) {
	source := `@title`
	tokens := mustTokenize(t, source)

	typeNames := make([]string, 0)
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			typeNames = append(typeNames, tok.TypeName+":"+tok.Value)
		}
	}

	expected := []string{"AT:@", "NAME:title"}
	if len(typeNames) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(typeNames), typeNames)
	}
	for i, exp := range expected {
		if typeNames[i] != exp {
			t.Errorf("Token %d: expected %q, got %q", i, exp, typeNames[i])
		}
	}
}

// =============================================================================
// TestTokenizeEnumValue
// =============================================================================
//
// An enum-style value (align.center) tokenizes as NAME DOT NAME.
func TestTokenizeEnumValue(t *testing.T) {
	source := `align.center`
	tokens := mustTokenize(t, source)

	typeNames := make([]string, 0)
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			typeNames = append(typeNames, tok.TypeName+":"+tok.Value)
		}
	}

	expected := []string{"NAME:align", "DOT:.", "NAME:center"}
	if len(typeNames) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(typeNames), typeNames)
	}
	for i, exp := range expected {
		if typeNames[i] != exp {
			t.Errorf("Token %d: expected %q, got %q", i, exp, typeNames[i])
		}
	}
}

// =============================================================================
// TestTokenizeEOF
// =============================================================================
//
// Every token stream ends with an EOF token.
func TestTokenizeEOF(t *testing.T) {
	cases := []string{
		`component Foo {}`,
		`"hello"`,
		`42`,
		`#fff`,
		`16dp`,
	}
	for _, src := range cases {
		tokens := mustTokenize(t, src)
		if len(tokens) == 0 {
			t.Errorf("%q: got empty token slice", src)
			continue
		}
		last := tokens[len(tokens)-1]
		if last.TypeName != "EOF" {
			t.Errorf("%q: expected last token EOF, got %s", src, last.TypeName)
		}
	}
}

// =============================================================================
// TestCreateLexer
// =============================================================================
//
// The factory function CreateLexer returns a non-nil GrammarLexer that can
// be used directly for tokenization.
func TestCreateLexer(t *testing.T) {
	lex, err := CreateLexer(`component Foo {}`)
	if err != nil {
		t.Fatalf("CreateLexer failed: %v", err)
	}
	if lex == nil {
		t.Fatal("CreateLexer returned nil")
	}
	tokens := lex.Tokenize()
	if len(tokens) == 0 {
		t.Fatal("Tokenize returned empty slice")
	}
}

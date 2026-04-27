package veriloglexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ---------------------------------------------------------------------------
// Helper — find a token by TypeName in a token list
// ---------------------------------------------------------------------------

func findToken(tokens []lexer.Token, typeName string) *lexer.Token {
	for i := range tokens {
		if tokens[i].TypeName == typeName {
			return &tokens[i]
		}
	}
	return nil
}

func findTokenByValue(tokens []lexer.Token, value string) *lexer.Token {
	for i := range tokens {
		if tokens[i].Value == value {
			return &tokens[i]
		}
	}
	return nil
}

func countTokensByType(tokens []lexer.Token, typeName string) int {
	count := 0
	for _, t := range tokens {
		if t.TypeName == typeName {
			count++
		}
	}
	return count
}

// ---------------------------------------------------------------------------
// Basic Tokenization
// ---------------------------------------------------------------------------

func TestTokenizeSimpleModule(t *testing.T) {
	source := `module m; endmodule`
	tokens, err := TokenizeVerilog(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Expected: KEYWORD(module) NAME(m) SEMICOLON(;) KEYWORD(endmodule) EOF
	if len(tokens) != 5 {
		t.Fatalf("Expected 5 tokens, got %d: %v", len(tokens), tokens)
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "module" {
		t.Errorf("Expected KEYWORD 'module', got %s %q", tokens[0].TypeName, tokens[0].Value)
	}
	if tokens[1].Type != lexer.TokenName || tokens[1].Value != "m" {
		t.Errorf("Expected NAME 'm', got %s %q", tokens[1].TypeName, tokens[1].Value)
	}
	if tokens[2].TypeName != "SEMICOLON" {
		t.Errorf("Expected SEMICOLON, got %s", tokens[2].TypeName)
	}
	if tokens[3].Type != lexer.TokenKeyword || tokens[3].Value != "endmodule" {
		t.Errorf("Expected KEYWORD 'endmodule', got %s %q", tokens[3].TypeName, tokens[3].Value)
	}
	if tokens[4].Type != lexer.TokenEOF {
		t.Errorf("Expected EOF, got %s", tokens[4].TypeName)
	}
}

// ---------------------------------------------------------------------------
// Keywords
// ---------------------------------------------------------------------------

func TestTokenizeKeywords(t *testing.T) {
	keywords := []string{
		"always", "assign", "begin", "case", "default", "else", "end",
		"endcase", "endmodule", "for", "function", "generate", "if",
		"initial", "input", "integer", "module", "output", "parameter",
		"posedge", "negedge", "reg", "wire",
	}

	for _, kw := range keywords {
		tokens, err := TokenizeVerilog(kw)
		if err != nil {
			t.Fatalf("Failed to tokenize keyword %q: %v", kw, err)
		}
		if tokens[0].Type != lexer.TokenKeyword {
			t.Errorf("Expected %q to be KEYWORD, got %s", kw, tokens[0].TypeName)
		}
		if tokens[0].Value != kw {
			t.Errorf("Expected value %q, got %q", kw, tokens[0].Value)
		}
	}
}

func TestTokenizeVersionedVerilog(t *testing.T) {
	for _, version := range []string{"1995", "2001", "2005"} {
		tokens, err := TokenizeVerilogVersion("module top; endmodule", version)
		if err != nil {
			t.Fatalf("version %s: %v", version, err)
		}
		if tokens[0].Value != "module" {
			t.Fatalf("version %s: expected first token to be module, got %q", version, tokens[0].Value)
		}
	}
}

func TestTokenizeVerilogVersionRejectsUnknownVersion(t *testing.T) {
	if _, err := TokenizeVerilogVersion("module top; endmodule", "2099"); err == nil {
		t.Fatal("expected unknown Verilog version to fail")
	}
}

// ---------------------------------------------------------------------------
// Sized Numbers — The unique Verilog number format
// ---------------------------------------------------------------------------
//
// Verilog numbers carry bit-width information because every signal in
// hardware has a specific width:
//
//   4'b1010       → 4-bit binary
//   8'hFF         → 8-bit hex
//   32'd42        → 32-bit decimal
//   'o77          → unsized octal

func TestTokenizeSizedNumbers(t *testing.T) {
	tests := []struct {
		source   string
		expected string
	}{
		{"4'b1010", "4'b1010"},
		{"8'hFF", "8'hFF"},
		{"32'd42", "32'd42"},
		{"'o77", "'o77"},
		{"16'hDEAD", "16'hDEAD"},
		{"8'b1010_0011", "8'b1010_0011"},
		{"4'bxxzz", "4'bxxzz"},
		{"8'sb10", "8'sb10"}, // signed
		{"8'Sb10", "8'Sb10"}, // signed (capital S)
	}

	for _, tt := range tests {
		tokens, err := TokenizeVerilog(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, "SIZED_NUMBER")
		if tok == nil {
			t.Errorf("Expected SIZED_NUMBER for %q, got %v", tt.source, tokens)
			continue
		}
		if tok.Value != tt.expected {
			t.Errorf("Expected value %q, got %q", tt.expected, tok.Value)
		}
	}
}

// ---------------------------------------------------------------------------
// Real Numbers
// ---------------------------------------------------------------------------

func TestTokenizeRealNumbers(t *testing.T) {
	tests := []struct {
		source   string
		expected string
	}{
		{"3.14", "3.14"},
		{"1.5e3", "1.5e3"},
		{"2.0E10", "2.0E10"},
		{"1.5e-3", "1.5e-3"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVerilog(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, "REAL_NUMBER")
		if tok == nil {
			t.Errorf("Expected REAL_NUMBER for %q, got %v", tt.source, tokens)
			continue
		}
		if tok.Value != tt.expected {
			t.Errorf("Expected value %q, got %q", tt.expected, tok.Value)
		}
	}
}

// ---------------------------------------------------------------------------
// Plain Numbers
// ---------------------------------------------------------------------------

func TestTokenizePlainNumbers(t *testing.T) {
	tokens, err := TokenizeVerilog("42")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if tokens[0].TypeName != "NUMBER" {
		t.Errorf("Expected NUMBER, got %s", tokens[0].TypeName)
	}
	if tokens[0].Value != "42" {
		t.Errorf("Expected '42', got %q", tokens[0].Value)
	}
}

func TestTokenizeNumberWithUnderscores(t *testing.T) {
	tokens, err := TokenizeVerilog("1_000_000")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if tokens[0].TypeName != "NUMBER" {
		t.Errorf("Expected NUMBER, got %s", tokens[0].TypeName)
	}
}

// ---------------------------------------------------------------------------
// System Identifiers — $display, $time, $finish, etc.
// ---------------------------------------------------------------------------
//
// System tasks and functions are prefixed with $ and are used for
// simulation (not synthesis). The lexer tokenizes them as SYSTEM_ID.

func TestTokenizeSystemIdentifiers(t *testing.T) {
	tests := []string{"$display", "$time", "$finish", "$random", "$monitor"}

	for _, sysId := range tests {
		tokens, err := TokenizeVerilog(sysId)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", sysId, err)
		}
		tok := findToken(tokens, "SYSTEM_ID")
		if tok == nil {
			t.Errorf("Expected SYSTEM_ID for %q, got %v", sysId, tokens)
			continue
		}
		if tok.Value != sysId {
			t.Errorf("Expected value %q, got %q", sysId, tok.Value)
		}
	}
}

// ---------------------------------------------------------------------------
// Directives — `timescale, `define, etc. (raw mode)
// ---------------------------------------------------------------------------
//
// When preprocessing is disabled, directives appear as DIRECTIVE tokens.

func TestTokenizeDirectivesRaw(t *testing.T) {
	tokens, err := TokenizeVerilogRaw("`timescale")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	tok := findToken(tokens, "DIRECTIVE")
	if tok == nil {
		t.Errorf("Expected DIRECTIVE token, got %v", tokens)
	}
	if tok != nil && tok.Value != "`timescale" {
		t.Errorf("Expected '`timescale', got %q", tok.Value)
	}
}

// ---------------------------------------------------------------------------
// Escaped Identifiers — \my.odd.name
// ---------------------------------------------------------------------------
//
// Verilog allows any characters in identifiers when prefixed with \.
// The identifier is terminated by whitespace.

func TestTokenizeEscapedIdentifier(t *testing.T) {
	tokens, err := TokenizeVerilog(`\my.odd.name `)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	tok := findToken(tokens, "ESCAPED_IDENT")
	if tok == nil {
		t.Errorf("Expected ESCAPED_IDENT token, got %v", tokens)
	}
	if tok != nil && tok.Value != `\my.odd.name` {
		t.Errorf("Expected '\\my.odd.name', got %q", tok.Value)
	}
}

// ---------------------------------------------------------------------------
// String Literals
// ---------------------------------------------------------------------------

func TestTokenizeString(t *testing.T) {
	tokens, err := TokenizeVerilog(`"Hello, world!\n"`)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	tok := findToken(tokens, "STRING")
	if tok == nil {
		t.Errorf("Expected STRING token, got %v", tokens)
	}
}

// ---------------------------------------------------------------------------
// Three-Character Operators
// ---------------------------------------------------------------------------

func TestTokenizeThreeCharOperators(t *testing.T) {
	tests := []struct {
		source   string
		typeName string
	}{
		{"<<<", "ARITH_LEFT_SHIFT"},
		{">>>", "ARITH_RIGHT_SHIFT"},
		{"===", "CASE_EQ"},
		{"!==", "CASE_NEQ"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVerilog(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, tt.typeName)
		if tok == nil {
			t.Errorf("Expected %s for %q, got %v", tt.typeName, tt.source, tokens)
		}
	}
}

// ---------------------------------------------------------------------------
// Two-Character Operators
// ---------------------------------------------------------------------------

func TestTokenizeTwoCharOperators(t *testing.T) {
	tests := []struct {
		source   string
		typeName string
	}{
		{"&&", "LOGIC_AND"},
		{"||", "LOGIC_OR"},
		{"<<", "LEFT_SHIFT"},
		{">>", "RIGHT_SHIFT"},
		{"==", "EQUALS_EQUALS"},
		{"!=", "NOT_EQUALS"},
		{"<=", "LESS_EQUALS"},
		{">=", "GREATER_EQUALS"},
		{"**", "POWER"},
		{"->", "TRIGGER"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVerilog(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, tt.typeName)
		if tok == nil {
			t.Errorf("Expected %s for %q, got %v", tt.typeName, tt.source, tokens)
		}
	}
}

// ---------------------------------------------------------------------------
// Single-Character Operators
// ---------------------------------------------------------------------------

func TestTokenizeSingleCharOperators(t *testing.T) {
	tests := []struct {
		source   string
		typeName string
	}{
		{"+", "PLUS"},
		{"-", "MINUS"},
		{"*", "STAR"},
		{"/", "SLASH"},
		{"%", "PERCENT"},
		{"&", "AMP"},
		{"|", "PIPE"},
		{"^", "CARET"},
		{"~", "TILDE"},
		{"!", "BANG"},
		{"<", "LESS_THAN"},
		{">", "GREATER_THAN"},
		{"=", "EQUALS"},
		{"?", "QUESTION"},
		{":", "COLON"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVerilog(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, tt.typeName)
		if tok == nil {
			t.Errorf("Expected %s for %q, got %v", tt.typeName, tt.source, tokens)
		}
	}
}

// ---------------------------------------------------------------------------
// Delimiters
// ---------------------------------------------------------------------------

func TestTokenizeDelimiters(t *testing.T) {
	tests := []struct {
		source   string
		typeName string
	}{
		{"(", "LPAREN"},
		{")", "RPAREN"},
		{"[", "LBRACKET"},
		{"]", "RBRACKET"},
		{"{", "LBRACE"},
		{"}", "RBRACE"},
		{";", "SEMICOLON"},
		{",", "COMMA"},
		{".", "DOT"},
		{"#", "HASH"},
		{"@", "AT"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVerilog(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, tt.typeName)
		if tok == nil {
			t.Errorf("Expected %s for %q, got %v", tt.typeName, tt.source, tokens)
		}
	}
}

// ---------------------------------------------------------------------------
// Comments (should be skipped)
// ---------------------------------------------------------------------------

func TestSkipLineComment(t *testing.T) {
	source := "wire a; // this is a comment"
	tokens, err := TokenizeVerilog(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Should not contain comment text as a token
	for _, tok := range tokens {
		if tok.Value == "// this is a comment" {
			t.Error("Line comment should be skipped")
		}
	}
}

func TestSkipBlockComment(t *testing.T) {
	source := "wire /* block */ a;"
	tokens, err := TokenizeVerilog(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Should have: KEYWORD(wire) NAME(a) SEMICOLON(;) EOF
	kwCount := countTokensByType(tokens, "KEYWORD")
	nameCount := countTokensByType(tokens, "NAME")
	if kwCount != 1 || nameCount != 1 {
		t.Errorf("Expected 1 keyword and 1 name after comment skip, got kw=%d name=%d, tokens=%v", kwCount, nameCount, tokens)
	}
}

// ---------------------------------------------------------------------------
// Full Module Tokenization
// ---------------------------------------------------------------------------

func TestTokenizeFullModule(t *testing.T) {
	source := `module and_gate(input a, input b, output y);
    assign y = a & b;
endmodule`

	tokens, err := TokenizeVerilog(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Verify key tokens exist
	if findTokenByValue(tokens, "module") == nil {
		t.Error("Expected 'module' keyword")
	}
	if findTokenByValue(tokens, "and_gate") == nil {
		t.Error("Expected 'and_gate' name")
	}
	if findTokenByValue(tokens, "assign") == nil {
		t.Error("Expected 'assign' keyword")
	}
	if findTokenByValue(tokens, "endmodule") == nil {
		t.Error("Expected 'endmodule' keyword")
	}
}

// ---------------------------------------------------------------------------
// Sensitivity List — @(posedge clk)
// ---------------------------------------------------------------------------

func TestTokenizeSensitivityList(t *testing.T) {
	source := `@(posedge clk)`
	tokens, err := TokenizeVerilog(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findToken(tokens, "AT") == nil {
		t.Error("Expected AT token")
	}
	if findTokenByValue(tokens, "posedge") == nil {
		t.Error("Expected 'posedge' keyword")
	}
	if findTokenByValue(tokens, "clk") == nil {
		t.Error("Expected 'clk' name")
	}
}

// ---------------------------------------------------------------------------
// Preprocessor + Lexer Integration
// ---------------------------------------------------------------------------

func TestPreprocessAndTokenize(t *testing.T) {
	source := "`define WIDTH 8\nwire [`WIDTH-1:0] data;"

	tokens, err := TokenizeVerilog(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// The preprocessor should expand `WIDTH to 8
	tok := findTokenByValue(tokens, "8")
	if tok == nil {
		t.Error("Expected preprocessor to expand `WIDTH to 8")
	}
}

// ---------------------------------------------------------------------------
// CreateVerilogLexer and CreateVerilogLexerRaw
// ---------------------------------------------------------------------------

func TestCreateVerilogLexer(t *testing.T) {
	lex, err := CreateVerilogLexer("wire a;")
	if err != nil {
		t.Fatalf("Failed to create lexer: %v", err)
	}
	tokens := lex.Tokenize()
	if len(tokens) < 3 {
		t.Fatalf("Expected at least 3 tokens, got %d", len(tokens))
	}
}

func TestCreateVerilogLexerRaw(t *testing.T) {
	lex, err := CreateVerilogLexerRaw("`define FOO 1")
	if err != nil {
		t.Fatalf("Failed to create raw lexer: %v", err)
	}
	tokens := lex.Tokenize()
	// In raw mode, `define should be a DIRECTIVE token
	tok := findToken(tokens, "DIRECTIVE")
	if tok == nil {
		t.Error("Expected DIRECTIVE token in raw mode")
	}
}

// ---------------------------------------------------------------------------
// Dollar-sign in identifiers
// ---------------------------------------------------------------------------

func TestTokenizeIdentifierWithDollar(t *testing.T) {
	tokens, err := TokenizeVerilog("my_var$0")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if tokens[0].TypeName != "NAME" {
		t.Errorf("Expected NAME, got %s", tokens[0].TypeName)
	}
	if tokens[0].Value != "my_var$0" {
		t.Errorf("Expected 'my_var$0', got %q", tokens[0].Value)
	}
}

// ---------------------------------------------------------------------------
// Hash for parameter override
// ---------------------------------------------------------------------------

func TestTokenizeHashParameterOverride(t *testing.T) {
	source := `mod #(8, 16) inst()`
	tokens, err := TokenizeVerilog(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findToken(tokens, "HASH") == nil {
		t.Error("Expected HASH token for parameter override")
	}
}

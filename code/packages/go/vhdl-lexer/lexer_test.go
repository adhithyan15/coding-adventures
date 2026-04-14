package vhdllexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ============================================================================
// Test Helpers
// ============================================================================
//
// These helper functions make test assertions cleaner by searching token
// lists for specific token types or values.

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

// ============================================================================
// Entity Declarations
// ============================================================================
//
// The entity is VHDL's primary design unit — it defines the external
// interface of a component (like a module in Verilog). An entity
// declaration lists ports with their directions and types.
//
// Example:
//   entity adder is
//       port (a, b : in  std_logic_vector(7 downto 0);
//             sum  : out std_logic_vector(7 downto 0));
//   end entity adder;

func TestTokenizeSimpleEntity(t *testing.T) {
	source := `entity e is end entity e;`
	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Expected: KEYWORD(entity) NAME(e) KEYWORD(is) KEYWORD(end) KEYWORD(entity) NAME(e) SEMICOLON(;) EOF
	if len(tokens) != 8 {
		t.Fatalf("Expected 8 tokens, got %d: %v", len(tokens), tokens)
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "entity" {
		t.Errorf("Expected KEYWORD 'entity', got %s %q", tokens[0].TypeName, tokens[0].Value)
	}
	if tokens[1].Type != lexer.TokenName || tokens[1].Value != "e" {
		t.Errorf("Expected NAME 'e', got %s %q", tokens[1].TypeName, tokens[1].Value)
	}
	if tokens[2].Type != lexer.TokenKeyword || tokens[2].Value != "is" {
		t.Errorf("Expected KEYWORD 'is', got %s %q", tokens[2].TypeName, tokens[2].Value)
	}
	if tokens[len(tokens)-1].Type != lexer.TokenEOF {
		t.Errorf("Expected EOF, got %s", tokens[len(tokens)-1].TypeName)
	}
}

func TestTokenizeEntityWithPorts(t *testing.T) {
	source := `entity and_gate is
    port (
        a, b : in  std_logic;
        y    : out std_logic
    );
end entity and_gate;`

	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Verify key tokens exist
	if findTokenByValue(tokens, "entity") == nil {
		t.Error("Expected 'entity' keyword")
	}
	if findTokenByValue(tokens, "and_gate") == nil {
		t.Error("Expected 'and_gate' name")
	}
	if findTokenByValue(tokens, "port") == nil {
		t.Error("Expected 'port' keyword")
	}
	if findTokenByValue(tokens, "in") == nil {
		t.Error("Expected 'in' keyword")
	}
	if findTokenByValue(tokens, "out") == nil {
		t.Error("Expected 'out' keyword")
	}
	if findTokenByValue(tokens, "std_logic") == nil {
		t.Error("Expected 'std_logic' name")
	}
}

func TestTokenizeVersionedVhdl(t *testing.T) {
	for _, version := range []string{"1987", "1993", "2002", "2008", "2019"} {
		tokens, err := TokenizeVhdlVersion("ENTITY top IS END ENTITY top;", version)
		if err != nil {
			t.Fatalf("version %s: %v", version, err)
		}
		if tokens[0].Value != "entity" {
			t.Fatalf("version %s: expected normalized entity keyword, got %q", version, tokens[0].Value)
		}
	}
}

func TestTokenizeVhdlVersionRejectsUnknownVersion(t *testing.T) {
	if _, err := TokenizeVhdlVersion("entity top is end entity top;", "2099"); err == nil {
		t.Fatal("expected unknown VHDL version to fail")
	}
}

// ============================================================================
// Architecture Bodies
// ============================================================================
//
// An architecture defines the internal implementation of an entity.
// VHDL separates interface (entity) from implementation (architecture),
// allowing multiple architectures for the same entity — useful for
// behavioral vs. structural descriptions.
//
// Example:
//   architecture rtl of and_gate is
//   begin
//       y <= a and b;
//   end architecture rtl;

func TestTokenizeArchitecture(t *testing.T) {
	source := `architecture rtl of and_gate is
begin
    y <= a and b;
end architecture rtl;`

	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findTokenByValue(tokens, "architecture") == nil {
		t.Error("Expected 'architecture' keyword")
	}
	if findTokenByValue(tokens, "rtl") == nil {
		t.Error("Expected 'rtl' name")
	}
	if findTokenByValue(tokens, "of") == nil {
		t.Error("Expected 'of' keyword")
	}
	if findTokenByValue(tokens, "begin") == nil {
		t.Error("Expected 'begin' keyword")
	}
	if findTokenByValue(tokens, "and") == nil {
		t.Error("Expected 'and' keyword operator")
	}
}

// ============================================================================
// Case Insensitivity
// ============================================================================
//
// VHDL is case-insensitive. ENTITY, Entity, and entity are all the same
// keyword. Our lexer normalizes NAME and KEYWORD tokens to lowercase.
//
// This is one of the key differences from Verilog, which is case-sensitive
// (module != Module != MODULE).

func TestCaseInsensitiveKeywords(t *testing.T) {
	// All three should produce the same keyword token value: "entity"
	variants := []string{"ENTITY", "Entity", "entity", "eNtItY"}

	for _, variant := range variants {
		tokens, err := TokenizeVhdl(variant)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", variant, err)
		}
		if tokens[0].Type != lexer.TokenKeyword {
			t.Errorf("Expected %q to be KEYWORD, got %s", variant, tokens[0].TypeName)
		}
		if tokens[0].Value != "entity" {
			t.Errorf("Expected value 'entity' for input %q, got %q", variant, tokens[0].Value)
		}
	}
}

func TestCaseInsensitiveNames(t *testing.T) {
	// Names should also be lowercased
	variants := []string{"MySignal", "MYSIGNAL", "mysignal", "mYsIgNaL"}

	for _, variant := range variants {
		tokens, err := TokenizeVhdl(variant)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", variant, err)
		}
		if tokens[0].Type != lexer.TokenName {
			t.Errorf("Expected %q to be NAME, got %s", variant, tokens[0].TypeName)
		}
		if tokens[0].Value != "mysignal" {
			t.Errorf("Expected value 'mysignal' for input %q, got %q", variant, tokens[0].Value)
		}
	}
}

func TestCaseInsensitiveKeywordList(t *testing.T) {
	// Verify that various VHDL keywords in different cases all normalize
	tests := []struct {
		input    string
		expected string
	}{
		{"SIGNAL", "signal"},
		{"Signal", "signal"},
		{"PROCESS", "process"},
		{"Process", "process"},
		{"ARCHITECTURE", "architecture"},
		{"BEGIN", "begin"},
		{"END", "end"},
		{"VARIABLE", "variable"},
		{"CONSTANT", "constant"},
		{"LIBRARY", "library"},
		{"USE", "use"},
		{"PORT", "port"},
		{"GENERIC", "generic"},
		{"IF", "if"},
		{"THEN", "then"},
		{"ELSE", "else"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVhdl(tt.input)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.input, err)
		}
		if tokens[0].Type != lexer.TokenKeyword {
			t.Errorf("Expected %q to be KEYWORD, got %s", tt.input, tokens[0].TypeName)
			continue
		}
		if tokens[0].Value != tt.expected {
			t.Errorf("Expected value %q for input %q, got %q", tt.expected, tt.input, tokens[0].Value)
		}
	}
}

// ============================================================================
// Signal, Variable, and Constant Declarations
// ============================================================================
//
// VHDL distinguishes between signals (hardware wires), variables
// (sequential computation), and constants (fixed values):
//
//   signal clk : std_logic;                      -- hardware wire
//   variable count : integer := 0;               -- sequential variable
//   constant WIDTH : integer := 8;               -- fixed value
//
// The := operator is for variable/constant initialization.
// The <= operator is for signal assignment.

func TestTokenizeSignalDeclaration(t *testing.T) {
	source := `signal clk : std_logic;`
	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findTokenByValue(tokens, "signal") == nil {
		t.Error("Expected 'signal' keyword")
	}
	if findTokenByValue(tokens, "clk") == nil {
		t.Error("Expected 'clk' name")
	}
	if findToken(tokens, "COLON") == nil {
		t.Error("Expected COLON token")
	}
}

func TestTokenizeVariableDeclaration(t *testing.T) {
	source := `variable count : integer := 0;`
	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findTokenByValue(tokens, "variable") == nil {
		t.Error("Expected 'variable' keyword")
	}
	if findToken(tokens, "VAR_ASSIGN") == nil {
		t.Error("Expected VAR_ASSIGN (:=) token")
	}
}

func TestTokenizeConstantDeclaration(t *testing.T) {
	source := `constant WIDTH : integer := 8;`
	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findTokenByValue(tokens, "constant") == nil {
		t.Error("Expected 'constant' keyword")
	}
	if findTokenByValue(tokens, "width") == nil {
		t.Error("Expected 'width' name (lowercased from WIDTH)")
	}
}

// ============================================================================
// Character Literals
// ============================================================================
//
// VHDL character literals are single characters between tick marks:
//   '0'  '1'  'X'  'Z'  'U'  'H'  'L'  '-'
//
// These are the values of std_logic, VHDL's most important type for
// modeling digital signals:
//
//   '0' — logic low (driven)
//   '1' — logic high (driven)
//   'X' — unknown (conflict)
//   'Z' — high impedance (tri-state)
//   'U' — uninitialized
//   'H' — weak high
//   'L' — weak low
//   '-' — don't care

func TestTokenizeCharacterLiterals(t *testing.T) {
	type charTest struct {
		input    string
		expected string
	}
	tests := []charTest{
		{"'0'", "'0'"}, {"'1'", "'1'"}, {"'X'", "'x'"}, {"'Z'", "'z'"},
		{"'U'", "'u'"}, {"'H'", "'h'"}, {"'L'", "'l'"}, {"'-'", "'-'"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVhdl(tt.input)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.input, err)
		}
		tok := findToken(tokens, "CHAR_LITERAL")
		if tok == nil {
			t.Errorf("Expected CHAR_LITERAL for %q, got %v", tt.input, tokens)
			continue
		}
		if tok.Value != tt.expected {
			t.Errorf("Expected value %q for input %q, got %q", tt.expected, tt.input, tok.Value)
		}
	}
}

// ============================================================================
// Bit String Literals
// ============================================================================
//
// Bit strings specify binary values using a base prefix:
//
//   B"1010"  — binary (each character is one bit)
//   O"77"    — octal (each character is three bits)
//   X"FF"    — hexadecimal (each character is four bits)
//   D"42"    — decimal (VHDL-2008)
//
// These are the VHDL equivalent of Verilog's sized literals:
//   Verilog: 8'hFF    →  VHDL: X"FF"
//   Verilog: 4'b1010  →  VHDL: B"1010"

func TestTokenizeBitStrings(t *testing.T) {
	tests := []struct {
		source   string
		expected string
	}{
		{`B"1010"`, `b"1010"`},
		{`b"1010"`, `b"1010"`},
		{`O"77"`, `o"77"`},
		{`o"77"`, `o"77"`},
		{`X"FF"`, `x"ff"`},
		{`x"ff"`, `x"ff"`},
		{`D"42"`, `d"42"`},
		{`X"DEAD_BEEF"`, `x"dead_beef"`},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVhdl(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, "BIT_STRING")
		if tok == nil {
			t.Errorf("Expected BIT_STRING for %q, got %v", tt.source, tokens)
			continue
		}
		if tok.Value != tt.expected {
			t.Errorf("Expected value %q, got %q", tt.expected, tok.Value)
		}
	}
}

// ============================================================================
// Based Literals
// ============================================================================
//
// Based literals use the format: base#digits#
//
//   16#FF#     — hexadecimal 255
//   2#1010#    — binary 10
//   8#77#      — octal 63
//   16#FF#E2   — hex 255 * 10^2
//
// The base can be any integer from 2 to 16. This is more explicit than
// Verilog's format and makes the base clearly visible.

func TestTokenizeBasedLiterals(t *testing.T) {
	tests := []struct {
		source   string
		expected string
	}{
		{"16#FF#", "16#ff#"},
		{"2#1010#", "2#1010#"},
		{"8#77#", "8#77#"},
		{"16#FF#E2", "16#ff#e2"},
		{"2#1010_0011#", "2#1010_0011#"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVhdl(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, "BASED_LITERAL")
		if tok == nil {
			t.Errorf("Expected BASED_LITERAL for %q, got %v", tt.source, tokens)
			continue
		}
		if tok.Value != tt.expected {
			t.Errorf("Expected value %q, got %q", tt.expected, tok.Value)
		}
	}
}

// ============================================================================
// Real Numbers
// ============================================================================

func TestTokenizeRealNumbers(t *testing.T) {
	tests := []struct {
		source   string
		expected string
	}{
		{"3.14", "3.14"},
		{"1.0E-3", "1.0e-3"},
		{"2.5e+10", "2.5e+10"},
		{"1_000.5", "1_000.5"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVhdl(tt.source)
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

// ============================================================================
// Plain Numbers
// ============================================================================

func TestTokenizePlainNumbers(t *testing.T) {
	tokens, err := TokenizeVhdl("42")
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
	tokens, err := TokenizeVhdl("1_000_000")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if tokens[0].TypeName != "NUMBER" {
		t.Errorf("Expected NUMBER, got %s", tokens[0].TypeName)
	}
}

// ============================================================================
// Two-Character Operators
// ============================================================================
//
// VHDL's two-character operators differ significantly from Verilog's:
//
//   := — Variable assignment (Verilog uses = for blocking assignment)
//   <= — Signal assignment AND less-than-or-equal (context-dependent)
//   => — Port map arrow (no Verilog equivalent)
//   /= — Not equal (Verilog uses !=)
//   ** — Exponentiation (same as Verilog)
//   <> — Unconstrained range / box (no Verilog equivalent)

func TestTokenizeTwoCharOperators(t *testing.T) {
	tests := []struct {
		source   string
		typeName string
	}{
		{":=", "VAR_ASSIGN"},
		{"<=", "LESS_EQUALS"},
		{">=", "GREATER_EQUALS"},
		{"=>", "ARROW"},
		{"/=", "NOT_EQUALS"},
		{"**", "POWER"},
		{"<>", "BOX"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVhdl(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, tt.typeName)
		if tok == nil {
			t.Errorf("Expected %s for %q, got %v", tt.typeName, tt.source, tokens)
		}
	}
}

// ============================================================================
// Single-Character Operators
// ============================================================================
//
// Note: VHDL uses & for concatenation, not bitwise AND.
// Logical operations (and, or, xor, not) are keywords, not symbols.

func TestTokenizeSingleCharOperators(t *testing.T) {
	tests := []struct {
		source   string
		typeName string
	}{
		{"+", "PLUS"},
		{"-", "MINUS"},
		{"*", "STAR"},
		{"/", "SLASH"},
		{"&", "AMPERSAND"},
		{"<", "LESS_THAN"},
		{">", "GREATER_THAN"},
		{"=", "EQUALS"},
		{"|", "PIPE"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVhdl(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, tt.typeName)
		if tok == nil {
			t.Errorf("Expected %s for %q, got %v", tt.typeName, tt.source, tokens)
		}
	}
}

// ============================================================================
// Delimiters
// ============================================================================

func TestTokenizeDelimiters(t *testing.T) {
	tests := []struct {
		source   string
		typeName string
	}{
		{"(", "LPAREN"},
		{")", "RPAREN"},
		{"[", "LBRACKET"},
		{"]", "RBRACKET"},
		{";", "SEMICOLON"},
		{",", "COMMA"},
		{".", "DOT"},
		{":", "COLON"},
	}

	for _, tt := range tests {
		tokens, err := TokenizeVhdl(tt.source)
		if err != nil {
			t.Fatalf("Failed to tokenize %q: %v", tt.source, err)
		}
		tok := findToken(tokens, tt.typeName)
		if tok == nil {
			t.Errorf("Expected %s for %q, got %v", tt.typeName, tt.source, tokens)
		}
	}
}

// ============================================================================
// Tick (Attribute Access)
// ============================================================================
//
// The tick character (') has dual purpose in VHDL:
//   1. Character literals: '0', '1', 'X'
//   2. Attribute access: signal'event, type'range, clk'rising_edge
//
// The lexer handles this by matching CHAR_LITERAL first (which requires
// exactly one character between ticks), and treating bare ' as TICK.

func TestTokenizeTick(t *testing.T) {
	// signal'event — TICK separates signal name from attribute
	source := `clk'event`
	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findTokenByValue(tokens, "clk") == nil {
		t.Error("Expected 'clk' name")
	}
	if findToken(tokens, "TICK") == nil {
		t.Error("Expected TICK token")
	}
	if findTokenByValue(tokens, "event") == nil {
		t.Error("Expected 'event' name")
	}
}

// ============================================================================
// Keyword Operators
// ============================================================================
//
// In VHDL, logical and arithmetic operations use keyword operators
// instead of symbols. This makes VHDL more readable but more verbose:
//
//   Verilog: y = (a & b) | (c ^ d);
//   VHDL:    y <= (a and b) or (c xor d);
//
// The full set of keyword operators:
//   Logical: and, or, nand, nor, xor, xnor, not
//   Shift:   sll, srl, sla, sra, rol, ror
//   Arith:   mod, rem, abs

func TestTokenizeKeywordOperators(t *testing.T) {
	keywords := []string{
		"and", "or", "nand", "nor", "xor", "xnor", "not",
		"sll", "srl", "sla", "sra", "rol", "ror",
		"mod", "rem", "abs",
	}

	for _, kw := range keywords {
		tokens, err := TokenizeVhdl(kw)
		if err != nil {
			t.Fatalf("Failed to tokenize keyword operator %q: %v", kw, err)
		}
		if tokens[0].Type != lexer.TokenKeyword {
			t.Errorf("Expected %q to be KEYWORD, got %s", kw, tokens[0].TypeName)
		}
		if tokens[0].Value != kw {
			t.Errorf("Expected value %q, got %q", kw, tokens[0].Value)
		}
	}
}

// ============================================================================
// Comments (VHDL uses -- for single-line comments)
// ============================================================================
//
// VHDL comments start with two dashes and extend to end of line:
//   signal clk : std_logic; -- clock input
//
// Unlike Verilog, VHDL (pre-2008) has no block comments. Comments are
// skipped during tokenization — they don't appear in the token stream.

func TestSkipLineComment(t *testing.T) {
	source := "signal a : std_logic; -- this is a comment"
	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Comment text should not appear in token stream
	for _, tok := range tokens {
		if tok.Value == "-- this is a comment" {
			t.Error("Comment should be skipped")
		}
	}

	// But the signal declaration should be present
	if findTokenByValue(tokens, "signal") == nil {
		t.Error("Expected 'signal' keyword before comment")
	}
}

func TestCommentOnlySource(t *testing.T) {
	source := "-- just a comment"
	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Should only have EOF
	if len(tokens) != 1 || tokens[0].Type != lexer.TokenEOF {
		t.Errorf("Expected only EOF for comment-only source, got %v", tokens)
	}
}

// ============================================================================
// String Literals
// ============================================================================
//
// VHDL strings use double quotes. To embed a quote, double it:
//   "Hello, World!"
//   "He said ""hello"""  →  He said "hello"

func TestTokenizeString(t *testing.T) {
	tokens, err := TokenizeVhdl(`"Hello, World!"`)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	tok := findToken(tokens, "STRING")
	if tok == nil {
		t.Error("Expected STRING token")
	}
}

func TestTokenizeStringWithEscapedQuotes(t *testing.T) {
	tokens, err := TokenizeVhdl(`"He said ""hello"""`)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	tok := findToken(tokens, "STRING")
	if tok == nil {
		t.Error("Expected STRING token for escaped quotes")
	}
}

// ============================================================================
// Extended Identifiers
// ============================================================================
//
// Extended identifiers are enclosed in backslashes and preserve case:
//   \my odd name\    — allows spaces and special characters
//   \VHDL-2008\      — preserves case (NOT normalized to lowercase)
//
// These are rarely used in practice but necessary for tool interop.

func TestTokenizeExtendedIdentifier(t *testing.T) {
	tokens, err := TokenizeVhdl(`\my odd name\ `)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	tok := findToken(tokens, "EXTENDED_IDENT")
	if tok == nil {
		t.Errorf("Expected EXTENDED_IDENT token, got %v", tokens)
	}
	if tok != nil && tok.Value != `\my odd name\` {
		t.Errorf("Expected '\\my odd name\\', got %q", tok.Value)
	}
}

// ============================================================================
// Keywords
// ============================================================================

func TestTokenizeKeywords(t *testing.T) {
	keywords := []string{
		"entity", "architecture", "begin", "end", "is", "of",
		"signal", "variable", "constant", "port", "generic",
		"process", "if", "then", "else", "elsif", "case", "when",
		"for", "while", "loop", "function", "procedure", "return",
		"library", "use", "package", "body", "type", "subtype",
		"array", "record", "range", "downto", "to", "in", "out",
		"inout", "buffer", "component", "generate", "map",
		"wait", "until", "after", "report", "assert", "severity",
		"null", "open", "others", "select", "with",
	}

	for _, kw := range keywords {
		tokens, err := TokenizeVhdl(kw)
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

// ============================================================================
// Complete VHDL Snippets
// ============================================================================
//
// These tests verify that the lexer handles realistic VHDL code correctly,
// not just isolated tokens.

func TestTokenizeFullEntityAndArchitecture(t *testing.T) {
	source := `-- Simple AND gate
library ieee;
use ieee.std_logic_1164.all;

entity and_gate is
    port (
        a, b : in  std_logic;
        y    : out std_logic
    );
end entity and_gate;

architecture rtl of and_gate is
begin
    y <= a and b;
end architecture rtl;`

	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Verify key structural tokens
	if findTokenByValue(tokens, "library") == nil {
		t.Error("Expected 'library' keyword")
	}
	if findTokenByValue(tokens, "ieee") == nil {
		t.Error("Expected 'ieee' name")
	}
	if findTokenByValue(tokens, "use") == nil {
		t.Error("Expected 'use' keyword")
	}
	if findTokenByValue(tokens, "entity") == nil {
		t.Error("Expected 'entity' keyword")
	}
	if findTokenByValue(tokens, "architecture") == nil {
		t.Error("Expected 'architecture' keyword")
	}
	if findTokenByValue(tokens, "rtl") == nil {
		t.Error("Expected 'rtl' name")
	}

	// Check that comment was skipped
	for _, tok := range tokens {
		if tok.Value == "-- Simple AND gate" {
			t.Error("Comment should be skipped")
		}
	}
}

func TestTokenizeProcess(t *testing.T) {
	source := `process(clk)
begin
    if clk'event and clk = '1' then
        q <= d;
    end if;
end process;`

	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findTokenByValue(tokens, "process") == nil {
		t.Error("Expected 'process' keyword")
	}
	if findToken(tokens, "TICK") == nil {
		t.Error("Expected TICK token for attribute access")
	}
	// '1' should be a character literal
	tok := findToken(tokens, "CHAR_LITERAL")
	if tok == nil {
		t.Error("Expected CHAR_LITERAL for '1'")
	}
	// <= should be LESS_EQUALS (signal assignment)
	if findToken(tokens, "LESS_EQUALS") == nil {
		t.Error("Expected LESS_EQUALS for signal assignment")
	}
}

func TestTokenizeGenericMux(t *testing.T) {
	source := `entity mux2 is
    generic (
        WIDTH : integer := 8
    );
    port (
        sel    : in  std_logic;
        a, b   : in  std_logic_vector(WIDTH-1 downto 0);
        y      : out std_logic_vector(WIDTH-1 downto 0)
    );
end entity mux2;

architecture rtl of mux2 is
begin
    y <= a when sel = '0' else b;
end architecture rtl;`

	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findTokenByValue(tokens, "generic") == nil {
		t.Error("Expected 'generic' keyword")
	}
	if findTokenByValue(tokens, "when") == nil {
		t.Error("Expected 'when' keyword")
	}
	if findTokenByValue(tokens, "downto") == nil {
		t.Error("Expected 'downto' keyword")
	}
	// WIDTH should be normalized to lowercase
	if findTokenByValue(tokens, "width") == nil {
		t.Error("Expected 'width' (normalized from WIDTH)")
	}
}

func TestTokenizeWithPortMap(t *testing.T) {
	source := `u1 : and_gate port map (
    a => sig_a,
    b => sig_b,
    y => sig_y
);`

	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// => should be ARROW
	arrowCount := countTokensByType(tokens, "ARROW")
	if arrowCount != 3 {
		t.Errorf("Expected 3 ARROW tokens for port map, got %d", arrowCount)
	}
	if findTokenByValue(tokens, "map") == nil {
		t.Error("Expected 'map' keyword")
	}
}

func TestTokenizeSignalAssignmentVsComparison(t *testing.T) {
	// Both uses of <= in one snippet
	source := `y <= '1' when a <= b else '0';`

	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Should have exactly 2 LESS_EQUALS tokens
	leCount := countTokensByType(tokens, "LESS_EQUALS")
	if leCount != 2 {
		t.Errorf("Expected 2 LESS_EQUALS tokens, got %d", leCount)
	}
}

func TestTokenizeConcatenation(t *testing.T) {
	source := `result <= a & b & "0000";`

	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	ampCount := countTokensByType(tokens, "AMPERSAND")
	if ampCount != 2 {
		t.Errorf("Expected 2 AMPERSAND tokens for concatenation, got %d", ampCount)
	}
}

func TestTokenizeNotEqualsOperator(t *testing.T) {
	source := `a /= b`
	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findToken(tokens, "NOT_EQUALS") == nil {
		t.Error("Expected NOT_EQUALS for /=")
	}
}

func TestTokenizeBoxOperator(t *testing.T) {
	source := `type word is array (natural range <>) of std_logic;`
	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findToken(tokens, "BOX") == nil {
		t.Error("Expected BOX for <>")
	}
}

func TestTokenizePowerOperator(t *testing.T) {
	source := `result := 2 ** 8;`
	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	if findToken(tokens, "POWER") == nil {
		t.Error("Expected POWER for **")
	}
}

// ============================================================================
// CreateVhdlLexer — Direct Lexer Access
// ============================================================================

func TestCreateVhdlLexer(t *testing.T) {
	lex, err := CreateVhdlLexer("signal a : std_logic;")
	if err != nil {
		t.Fatalf("Failed to create lexer: %v", err)
	}
	tokens := lex.Tokenize()
	if len(tokens) < 4 {
		t.Fatalf("Expected at least 4 tokens, got %d", len(tokens))
	}
}

// ============================================================================
// Edge Cases
// ============================================================================

func TestTokenizeEmptySource(t *testing.T) {
	tokens, err := TokenizeVhdl("")
	if err != nil {
		t.Fatalf("Failed to tokenize empty source: %v", err)
	}
	if len(tokens) != 1 || tokens[0].Type != lexer.TokenEOF {
		t.Errorf("Expected only EOF for empty source, got %v", tokens)
	}
}

func TestTokenizeWhitespaceOnly(t *testing.T) {
	tokens, err := TokenizeVhdl("   \t\n\n   ")
	if err != nil {
		t.Fatalf("Failed to tokenize whitespace: %v", err)
	}
	if len(tokens) != 1 || tokens[0].Type != lexer.TokenEOF {
		t.Errorf("Expected only EOF for whitespace source, got %v", tokens)
	}
}

func TestTokenizeMultipleStatements(t *testing.T) {
	source := `signal a : std_logic;
signal b : std_logic;
signal c : std_logic;`

	tokens, err := TokenizeVhdl(source)
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Should have 3 signal keywords
	sigCount := 0
	for _, tok := range tokens {
		if tok.Value == "signal" && tok.Type == lexer.TokenKeyword {
			sigCount++
		}
	}
	if sigCount != 3 {
		t.Errorf("Expected 3 'signal' keywords, got %d", sigCount)
	}
}

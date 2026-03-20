package grammartools

import (
	"reflect"
	"testing"
)

func TestParseTokenGrammar(t *testing.T) {
	source := `
NAME = /[a-zA-Z_]+/
EQUALS = "="
keywords:
  if
  else
`
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse token grammar: %v", err)
	}

	if len(grammar.Definitions) != 2 {
		t.Fatalf("Expected 2 definitions, got %d", len(grammar.Definitions))
	}
	if grammar.Definitions[0].Name != "NAME" || grammar.Definitions[0].Pattern != "[a-zA-Z_]+" {
		t.Errorf("Mismatch mapping regex definition natively translating boundaries properly")
	}

	if len(grammar.Keywords) != 2 || grammar.Keywords[0] != "if" {
		t.Errorf("Mismatch parsing keywords extracting configuration spaces natively %v", grammar.Keywords)
	}
}

func TestParseModeDirective(t *testing.T) {
	source := "mode: indentation\nNAME = /[a-z]+/"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if grammar.Mode != "indentation" {
		t.Errorf("Expected mode 'indentation', got %q", grammar.Mode)
	}
}

func TestParseSkipSection(t *testing.T) {
	source := "NAME = /[a-z]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n  COMMENT = /#[^\\n]*/"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.SkipDefinitions) != 2 {
		t.Fatalf("Expected 2 skip defs, got %d", len(grammar.SkipDefinitions))
	}
	if grammar.SkipDefinitions[0].Name != "WHITESPACE" {
		t.Errorf("Expected 'WHITESPACE', got %q", grammar.SkipDefinitions[0].Name)
	}
}

func TestParseReservedSection(t *testing.T) {
	source := "NAME = /[a-z]+/\nreserved:\n  class\n  import"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.ReservedKeywords) != 2 {
		t.Fatalf("Expected 2 reserved, got %d", len(grammar.ReservedKeywords))
	}
	if grammar.ReservedKeywords[0] != "class" {
		t.Errorf("Expected 'class', got %q", grammar.ReservedKeywords[0])
	}
}

func TestParseAlias(t *testing.T) {
	source := `STRING_DQ = /"[^"]*"/ -> STRING`
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if grammar.Definitions[0].Alias != "STRING" {
		t.Errorf("Expected alias 'STRING', got %q", grammar.Definitions[0].Alias)
	}
}

func TestParseLiteralAlias(t *testing.T) {
	source := `PLUS_SIGN = "+" -> PLUS`
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if grammar.Definitions[0].Alias != "PLUS" {
		t.Errorf("Expected alias 'PLUS', got %q", grammar.Definitions[0].Alias)
	}
}

func TestTokenNamesIncludesAliases(t *testing.T) {
	source := `STRING_DQ = /"[^"]*"/ -> STRING`
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	names := grammar.TokenNames()
	if !names["STRING_DQ"] || !names["STRING"] {
		t.Errorf("TokenNames should include both original and alias")
	}
}

func TestParseModeMissingValue(t *testing.T) {
	_, err := ParseTokenGrammar("mode:")
	if err == nil {
		t.Fatal("Expected error for missing mode value")
	}
}

func TestParseAliasMissing(t *testing.T) {
	_, err := ParseTokenGrammar("FOO = /x/ ->")
	if err == nil {
		t.Fatal("Expected error for missing alias")
	}
}

func TestParseUnclosedRegex(t *testing.T) {
	_, err := ParseTokenGrammar("FOO = /unclosed")
	if err == nil {
		t.Fatal("Expected error for unclosed regex")
	}
}

func TestParseUnclosedLiteral(t *testing.T) {
	_, err := ParseTokenGrammar(`FOO = "unclosed`)
	if err == nil {
		t.Fatal("Expected error for unclosed literal")
	}
}

func TestParseSkipMissingEquals(t *testing.T) {
	_, err := ParseTokenGrammar("skip:\n  BAD_PATTERN")
	if err == nil {
		t.Fatal("Expected error for skip without equals")
	}
}

func TestParseSkipIncomplete(t *testing.T) {
	_, err := ParseTokenGrammar("skip:\n  BAD =")
	if err == nil {
		t.Fatal("Expected error for incomplete skip")
	}
}

func TestParseStarlarkTokens(t *testing.T) {
	// Test with the real starlark.tokens file if available
	source := `
mode: indentation

NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
INT = /[0-9]+/
EQUALS = "="
PLUS = "+"
COLON = ":"
LPAREN = "("
RPAREN = ")"
COMMA = ","

keywords:
  def
  return
  if
  else
  for
  in
  pass

reserved:
  class
  import

skip:
  WHITESPACE = /[ \t]+/
  COMMENT = /#[^\n]*/
`
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse starlark-like tokens: %v", err)
	}
	if grammar.Mode != "indentation" {
		t.Errorf("Expected mode 'indentation'")
	}
	if len(grammar.ReservedKeywords) != 2 {
		t.Errorf("Expected 2 reserved keywords, got %d", len(grammar.ReservedKeywords))
	}
	if len(grammar.SkipDefinitions) != 2 {
		t.Errorf("Expected 2 skip definitions, got %d", len(grammar.SkipDefinitions))
	}
}

func TestParseParserGrammar(t *testing.T) {
	source := `
expression = term { ( PLUS | MINUS ) term } ;
term = NUMBER ;
`
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse grammar structurally safely %v", err)
	}
	
	if len(grammar.Rules) != 2 {
		t.Fatalf("Expected 2 rules, got %v", len(grammar.Rules))
	}

	expectedType := reflect.TypeOf(Sequence{})
	actualType := reflect.TypeOf(grammar.Rules[0].Body)
	if expectedType != actualType {
		t.Errorf("Mismatch evaluating nested blocks dynamically translating sequences explicitly.")
	}
}

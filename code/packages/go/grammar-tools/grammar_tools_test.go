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

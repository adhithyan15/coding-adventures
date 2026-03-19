package lexer

import (
	"testing"
)

func TestLexerMath(t *testing.T) {
	source := "x = 1 + 2 * 3"
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()

	expected := []struct{
		TType TokenType
		Value string
	}{
		{TokenName, "x"},
		{TokenEquals, "="},
		{TokenNumber, "1"},
		{TokenPlus, "+"},
		{TokenNumber, "2"},
		{TokenStar, "*"},
		{TokenNumber, "3"},
		{TokenEOF, ""},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d", len(expected), len(tokens))
	}

	for i, tok := range tokens {
		exp := expected[i]
		if tok.Type != exp.TType || tok.Value != exp.Value {
			t.Errorf("Mismatch on token %d: expected (%v, %s), got (%v, %s)",
			i, exp.TType, exp.Value, tok.Type, tok.Value)
		}
	}
}

func TestKeywords(t *testing.T) {
	source := "if x == 5"
	cfg := LexerConfig{Keywords: []string{"if"}}
	lexer := NewLexer(source, &cfg)
	tokens := lexer.Tokenize()

	if tokens[0].Type != TokenKeyword || tokens[0].Value != "if" {
		t.Errorf("Expected first token to map KeyWord successfully.")
	}

	if tokens[2].Type != TokenEqualsEquals || tokens[2].Value != "==" {
		t.Errorf("Expected equals equals double delimiter mapped to tokenizer natively.")
	}
}

func TestStrings(t *testing.T) {
	source := `print("Hello\n")`
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()

	if tokens[2].Type != TokenString || tokens[2].Value != "Hello\n" {
		t.Errorf("Expected newline parsing accurately resolving escape values %q.", tokens[2].Value)
	}
}

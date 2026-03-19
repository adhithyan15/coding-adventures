package rubylexer

import (
	"testing"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func TestTokenizeRuby(t *testing.T) {
	source := `puts "hello" if true`
	tokens, err := TokenizeRuby(source)
	if err != nil {
		t.Fatalf("Failed to tokenize ruby source: %v", err)
	}

	if len(tokens) != 5 {
		t.Fatalf("Expected 5 tokens explicitly extracting keywords natively, got %v", len(tokens))
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "puts" {
		t.Errorf("Mismatch capturing Ruby configuration overrides, puts should be keyword.")
	}
	if tokens[3].Type != lexer.TokenKeyword || tokens[3].Value != "true" {
		t.Errorf("Mismatch capturing Ruby explicit value overrides dynamically.")
	}
}

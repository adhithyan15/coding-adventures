package pythonlexer

import (
	"testing"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func TestTokenizePython(t *testing.T) {
	source := `print("hello") if True else False`
	tokens, err := TokenizePython(source)
	if err != nil {
		t.Fatalf("Failed to tokenize python source: %v", err)
	}

	if len(tokens) != 9 {
		t.Fatalf("Expected 9 tokens, got %v", len(tokens))
	}

	if tokens[4].Type != lexer.TokenKeyword || tokens[4].Value != "if" {
		t.Errorf("Expected 'if' to be mapped natively into keywords from python limits.")
	}

	if tokens[5].Type != lexer.TokenKeyword || tokens[5].Value != "True" {
		t.Errorf("Expected 'True' properly resolving Python specifics configurations explicitly.")
	}
}

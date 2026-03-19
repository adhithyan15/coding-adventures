package javascriptlexer

import (
	"testing"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func TestTokenizeJavascript(t *testing.T) {
	source := `let x = 1 + 2;`
	tokens, err := TokenizeJavascript(source)
	if err != nil {
		t.Fatalf("Failed to tokenize JavaScript source: %v", err)
	}

	// Expected: KEYWORD(let) NAME(x) EQUALS(=) NUMBER(1) PLUS(+) NUMBER(2) SEMICOLON(;) EOF
	if len(tokens) != 8 {
		t.Fatalf("Expected 8 tokens, got %v", len(tokens))
	}

	if tokens[0].Type != lexer.TokenKeyword || tokens[0].Value != "let" {
		t.Errorf("Expected first token to be KEYWORD 'let', got %v %v", tokens[0].Type, tokens[0].Value)
	}

	if tokens[1].Type != lexer.TokenName || tokens[1].Value != "x" {
		t.Errorf("Expected second token to be NAME 'x', got %v %v", tokens[1].Type, tokens[1].Value)
	}

	if tokens[6].Type != lexer.TokenSemicolon || tokens[6].Value != ";" {
		t.Errorf("Expected semicolon token, got %v %v", tokens[6].Type, tokens[6].Value)
	}
}

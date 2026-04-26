package lisplexer

import (
	"path/filepath"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func TestTokenizeLispDefinition(t *testing.T) {
	tokens, err := TokenizeLisp("(define x 42)")
	if err != nil {
		t.Fatalf("tokenize failed: %v", err)
	}
	wantTypes := []string{"LPAREN", "SYMBOL", "SYMBOL", "NUMBER", "RPAREN", "EOF"}
	if len(tokens) != len(wantTypes) {
		t.Fatalf("expected %d tokens, got %d: %#v", len(wantTypes), len(tokens), tokens)
	}
	for i, want := range wantTypes {
		if tokens[i].TypeName != want {
			t.Fatalf("token %d: expected %s, got %#v", i, want, tokens[i])
		}
	}
	if tokens[1].Value != "define" || tokens[3].Value != "42" {
		t.Fatalf("unexpected token values: %#v", tokens)
	}
}

func TestTokenizeLispSymbolsAndNumbers(t *testing.T) {
	tokens, err := TokenizeLisp("(+ -42 (* x 2))")
	if err != nil {
		t.Fatalf("tokenize failed: %v", err)
	}
	if tokens[1].TypeName != "SYMBOL" || tokens[1].Value != "+" {
		t.Fatalf("expected + symbol, got %#v", tokens[1])
	}
	if tokens[2].TypeName != "NUMBER" || tokens[2].Value != "-42" {
		t.Fatalf("expected -42 number, got %#v", tokens[2])
	}
	if tokens[4].TypeName != "SYMBOL" || tokens[4].Value != "*" {
		t.Fatalf("expected * symbol, got %#v", tokens[4])
	}
}

func TestTokenizeLispCommentsQuotesAndDottedPairs(t *testing.T) {
	tokens, err := TokenizeLisp("; ignore\n'(a . b)")
	if err != nil {
		t.Fatalf("tokenize failed: %v", err)
	}
	wantTypes := []string{"QUOTE", "LPAREN", "SYMBOL", "DOT", "SYMBOL", "RPAREN", "EOF"}
	for i, want := range wantTypes {
		if tokens[i].TypeName != want {
			t.Fatalf("token %d: expected %s, got %#v", i, want, tokens[i])
		}
	}
}

func TestCreateLispLexer(t *testing.T) {
	lispLexer, err := CreateLispLexer("\"hello\\nworld\"")
	if err != nil {
		t.Fatalf("create lexer failed: %v", err)
	}
	tokens := lispLexer.Tokenize()
	if tokens[0].TypeName != "STRING" || tokens[0].Value != "hello\\nworld" {
		t.Fatalf("expected string token, got %#v", tokens[0])
	}
	if tokens[len(tokens)-1].Type != lexer.TokenEOF {
		t.Fatalf("expected EOF token, got %#v", tokens[len(tokens)-1])
	}
}

func TestGrammarPath(t *testing.T) {
	path := getTokensPath()
	if filepath.Base(path) != "lisp.tokens" {
		t.Fatalf("expected lisp.tokens path, got %s", path)
	}
}

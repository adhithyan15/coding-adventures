package haskelllexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func TestTokenizeHaskellDefaultVersion(t *testing.T) {
	tokens, err := TokenizeHaskell("x", "")
	if err != nil {
		t.Fatalf("tokenize failed: %v", err)
	}
	if len(tokens) < 2 {
		t.Fatalf("expected name and EOF tokens, got %#v", tokens)
	}
	if tokens[0].TypeName != "NAME" || tokens[0].Value != "x" {
		t.Fatalf("expected NAME x, got %#v", tokens[0])
	}
	if tokens[len(tokens)-1].Type != lexer.TokenEOF {
		t.Fatalf("expected EOF token, got %#v", tokens[len(tokens)-1])
	}
}

func TestTokenizeHaskellEmitsLayoutTokens(t *testing.T) {
	tokens, err := TokenizeHaskell("let\n  x = y\nin x", "")
	if err != nil {
		t.Fatalf("tokenize failed: %v", err)
	}

	seen := map[string]bool{}
	for _, token := range tokens {
		seen[token.TypeName] = true
	}
	if !seen["VIRTUAL_LBRACE"] || !seen["VIRTUAL_RBRACE"] {
		t.Fatalf("expected virtual layout braces, saw %#v", seen)
	}
}

func TestTokenizeHaskellVersions(t *testing.T) {
	for _, version := range ValidVersions() {
		t.Run(version, func(t *testing.T) {
			tokens, err := TokenizeHaskell("x", version)
			if err != nil {
				t.Fatalf("tokenize %s failed: %v", version, err)
			}
			if tokens[0].TypeName != "NAME" {
				t.Fatalf("expected NAME for %s, got %#v", version, tokens[0])
			}
		})
	}
}

func TestCreateHaskellLexer(t *testing.T) {
	haskellLexer, err := CreateHaskellLexer("x", "2010")
	if err != nil {
		t.Fatalf("create lexer failed: %v", err)
	}
	if haskellLexer == nil {
		t.Fatal("expected non-nil lexer")
	}
}

func TestTokenizeHaskellUnknownVersion(t *testing.T) {
	_, err := TokenizeHaskell("x", "2020")
	if err == nil {
		t.Fatal("expected error for unknown version")
	}
}

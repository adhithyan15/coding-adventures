package niblexer

import "testing"

func TestTokenizeNibReclassifiesKeywords(t *testing.T) {
	tokens := TokenizeNib("fn main() { let x: u4 = 5; }")
	if len(tokens) == 0 {
		t.Fatal("expected tokens")
	}
	if tokens[0].TypeName != "fn" {
		t.Fatalf("expected first token to be reclassified keyword, got %q", tokens[0].TypeName)
	}
}

package grammartools

// LANG84 E1 (definition guards) and E8 (refusal transitions, `if`
// conditions) are implemented by the Rust grammar-tools only. LANG84 requires
// every other port to refuse a grammar that uses them, with a clear error,
// rather than ignore the syntax and lex differently from Rust.

import (
	"strings"
	"testing"
)

func TestLang84E1E8SyntaxIsRejected(t *testing.T) {
	cases := []struct{ source, needle string }{
		{"X = /x/ @line_start\n", "@line_start"},
		{"X = /x/ @input_start\n", "@input_start"},
		{"X = /x/ -> Y @line_start\n", "not implemented"},
		{"X = \"x\" -> Y @input_start\n", "not implemented"},
		{"X = /x/\ntransitions:\n  on X -> refuse\n", "refuse"},
		{"X = /x/\ntransitions:\n  on X if space_before && !space_after -> pop\n", "not implemented"},
		{"X = /x/\ntransitions:\n  on (X | Y) in default if line_start -> pop\n", "not implemented"},
		// Security review L3: a guard glued to the alias is still a guard.
		{"X = /x/ ->Y@line_start\n", "not implemented"},
		// Security review L1: a tab before `if`, and a bare trailing `if`.
		{"X = /x/\ntransitions:\n  on X\tif space_before -> pop\n", "not implemented"},
		{"X = /x/\ntransitions:\n  on X if -> pop\n", "not implemented"},
	}
	for _, c := range cases {
		_, err := ParseTokenGrammar(c.source)
		if err == nil {
			t.Errorf("%q: expected an error", c.source)
			continue
		}
		if !strings.Contains(err.Error(), c.needle) {
			t.Errorf("%q: error %q does not mention %q", c.source, err.Error(), c.needle)
		}
	}
}

func TestLang84E8KeywordValueIfIsNotACondition(t *testing.T) {
	g, err := ParseTokenGrammar("X = /x/\ntransitions:\n  on KEYWORD=\"if\" -> set-mode default\n")
	if err != nil {
		t.Fatal(err)
	}
	if g.Transitions[0].OnValue != "if" {
		t.Errorf("OnValue = %q, want \"if\"", g.Transitions[0].OnValue)
	}
}

// `IF_X` is a token name, not the condition keyword.
func TestLang84E8IfMustBeAWholeWord(t *testing.T) {
	g, err := ParseTokenGrammar("IF_X = /i/\nX = /x/\ntransitions:\n  on (X | IF_X) -> set-mode default\n")
	if err != nil {
		t.Fatal(err)
	}
	if got := strings.Join(g.Transitions[0].OnTokens, ","); got != "X,IF_X" {
		t.Errorf("OnTokens = %q, want X,IF_X", got)
	}
}

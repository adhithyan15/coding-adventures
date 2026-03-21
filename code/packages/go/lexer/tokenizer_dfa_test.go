package lexer

import "testing"

// =========================================================================
// ClassifyChar tests
// =========================================================================

func TestClassifyCharEOF(t *testing.T) {
	if got := ClassifyChar(0, false); got != "eof" {
		t.Errorf("ClassifyChar(0, false) = %q, want %q", got, "eof")
	}
}

func TestClassifyCharDigit(t *testing.T) {
	for _, ch := range "0123456789" {
		if got := ClassifyChar(ch, true); got != "digit" {
			t.Errorf("ClassifyChar(%q, true) = %q, want %q", ch, got, "digit")
		}
	}
}

func TestClassifyCharAlpha(t *testing.T) {
	for _, ch := range "azAZ" {
		if got := ClassifyChar(ch, true); got != "alpha" {
			t.Errorf("ClassifyChar(%q, true) = %q, want %q", ch, got, "alpha")
		}
	}
}

func TestClassifyCharUnderscore(t *testing.T) {
	if got := ClassifyChar('_', true); got != "underscore" {
		t.Errorf("ClassifyChar('_', true) = %q, want %q", got, "underscore")
	}
}

func TestClassifyCharWhitespace(t *testing.T) {
	for _, ch := range " \t\r" {
		if got := ClassifyChar(ch, true); got != "whitespace" {
			t.Errorf("ClassifyChar(%q, true) = %q, want %q", ch, got, "whitespace")
		}
	}
}

func TestClassifyCharNewline(t *testing.T) {
	if got := ClassifyChar('\n', true); got != "newline" {
		t.Errorf("ClassifyChar('\\n', true) = %q, want %q", got, "newline")
	}
}

func TestClassifyCharQuote(t *testing.T) {
	if got := ClassifyChar('"', true); got != "quote" {
		t.Errorf("ClassifyChar('\"', true) = %q, want %q", got, "quote")
	}
}

func TestClassifyCharEquals(t *testing.T) {
	if got := ClassifyChar('=', true); got != "equals" {
		t.Errorf("ClassifyChar('=', true) = %q, want %q", got, "equals")
	}
}

func TestClassifyCharOperators(t *testing.T) {
	for _, ch := range "+-*/" {
		if got := ClassifyChar(ch, true); got != "operator" {
			t.Errorf("ClassifyChar(%q, true) = %q, want %q", ch, got, "operator")
		}
	}
}

func TestClassifyCharDelimiters(t *testing.T) {
	tests := map[rune]string{
		'(': "open_paren",
		')': "close_paren",
		',': "comma",
		':': "colon",
		';': "semicolon",
		'{': "open_brace",
		'}': "close_brace",
		'[': "open_bracket",
		']': "close_bracket",
		'.': "dot",
		'!': "bang",
	}
	for ch, want := range tests {
		if got := ClassifyChar(ch, true); got != want {
			t.Errorf("ClassifyChar(%q, true) = %q, want %q", ch, got, want)
		}
	}
}

func TestClassifyCharOther(t *testing.T) {
	for _, ch := range "@#$" {
		if got := ClassifyChar(ch, true); got != "other" {
			t.Errorf("ClassifyChar(%q, true) = %q, want %q", ch, got, "other")
		}
	}
}

// =========================================================================
// DFA construction and transition tests
// =========================================================================

func TestNewTokenizerDFACreation(t *testing.T) {
	dfa := NewTokenizerDFA()
	if dfa.CurrentState() != "start" {
		t.Errorf("Initial state = %q, want %q", dfa.CurrentState(), "start")
	}
}

func TestDFAIsComplete(t *testing.T) {
	dfa := NewTokenizerDFA()
	if !dfa.IsComplete() {
		t.Error("TOKENIZER_DFA should be complete (transition for every state/input pair)")
	}
}

func TestDFAStartToInNumberOnDigit(t *testing.T) {
	dfa := NewTokenizerDFA()
	next := dfa.Process("digit")
	if next != "in_number" {
		t.Errorf("start + digit = %q, want %q", next, "in_number")
	}
}

func TestDFAStartToInNameOnAlpha(t *testing.T) {
	dfa := NewTokenizerDFA()
	next := dfa.Process("alpha")
	if next != "in_name" {
		t.Errorf("start + alpha = %q, want %q", next, "in_name")
	}
}

func TestDFAStartToInNameOnUnderscore(t *testing.T) {
	dfa := NewTokenizerDFA()
	next := dfa.Process("underscore")
	if next != "in_name" {
		t.Errorf("start + underscore = %q, want %q", next, "in_name")
	}
}

func TestDFAStartToDoneOnEOF(t *testing.T) {
	dfa := NewTokenizerDFA()
	next := dfa.Process("eof")
	if next != "done" {
		t.Errorf("start + eof = %q, want %q", next, "done")
	}
}

func TestDFAStartToErrorOnOther(t *testing.T) {
	dfa := NewTokenizerDFA()
	next := dfa.Process("other")
	if next != "error" {
		t.Errorf("start + other = %q, want %q", next, "error")
	}
}

func TestDFADispatchMatchesExistingBehavior(t *testing.T) {
	// Tokenize a realistic expression and verify it still works correctly.
	l := NewLexer("x = 42 + y", nil)
	tokens := l.Tokenize()

	expected := []TokenType{
		TokenName,        // x
		TokenEquals,      // =
		TokenNumber,      // 42
		TokenPlus,        // +
		TokenName,        // y
		TokenEOF,
	}

	if len(tokens) != len(expected) {
		t.Fatalf("got %d tokens, want %d", len(tokens), len(expected))
	}
	for i, want := range expected {
		if tokens[i].Type != want {
			t.Errorf("token %d: got %v, want %v", i, tokens[i].Type, want)
		}
	}
}

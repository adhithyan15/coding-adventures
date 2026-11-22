package stracelexer

import (
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// The "???" lines are whole-line tokens in exactly the spellings the old parser
// accepted; every other "???(" or "<... ??? resumed>" is NOT a dead-line token.
func TestDeadLineExactness(t *testing.T) {
	good := map[string]string{
		"1  ???( <unfinished ...>":   "DEAD_OPEN",
		"1  ???() = ?":               "DEAD_WHOLE",
		"1  ???()      = ?":          "DEAD_WHOLE",
		"1  <... ??? resumed>) = ?":  "DEAD_RESUMED",
		"1  <... ??? resumed>)  = ?": "DEAD_RESUMED",
	}
	for line, want := range good {
		if got := lex(t, line)[1].typ; got != want {
			t.Errorf("%q: %s, want %s", line, got, want)
		}
	}
	notDead := []string{
		"1  ???() = 0", "1  ???()", "1  ???()= ?", "1  ???(<unfinished ...>", "1  ???(  <unfinished ...>",
		"1  <... ??? resumed>) = 0", "1  <... ??? resumed>)= ?", "1  <... ??? resumed>0x1) = ?", "1  <... ??? resumed>)",
	}
	for _, line := range notDead {
		for _, tk := range lex(t, line) {
			if strings.HasPrefix(tk.typ, "DEAD_") {
				t.Errorf("%q lexed %s %q", line, tk.typ, tk.val)
			}
		}
	}
	// "???" is not a resume name any more: it is a bad marker unless it is
	// exactly the dead resume.
	if tk := lex(t, "1  <... ??? resumed>) = 0")[1]; tk.typ != "ERR_RESUME" {
		t.Errorf("got %v", tk)
	}
	if tk := lex(t, "1  <... a?b resumed>) = 0")[1]; tk.typ != "ERR_RESUME" {
		t.Errorf("got %v", tk)
	}
	// A line with a trailing token after a dead token is not a line (parser).
	if got := lex(t, "1  ???() = ? x"); got[len(got)-1].typ != "WORD" {
		t.Errorf("%v", got)
	}
}

func TestLineByteLimit(t *testing.T) {
	line := "1  f(" + strings.Repeat("a", 100) + ")"
	if _, err := TokenizeLineMax(line, len(line)); err != nil {
		t.Errorf("at the limit: %v", err)
	}
	if _, err := TokenizeLineMax(line, len(line)-1); err != ErrLineTooLong {
		t.Errorf("over the limit: %v", err)
	}
	// Zero means the default, 1 MiB.
	big := "1  f(" + strings.Repeat("a,", DefaultMaxLineBytes/2) + ")"
	if _, err := TokenizeLine(big); err != ErrLineTooLong {
		t.Errorf("a line over %d bytes: %v", DefaultMaxLineBytes, err)
	}
	if _, err := TokenizeLineMax("1  f()", 0); err != nil {
		t.Errorf("zero limit: %v", err)
	}
	// The length check comes first: an over-long line with a newline is a length error.
	if _, err := TokenizeLineMax("1  f()\n", 3); err != ErrLineTooLong {
		t.Errorf("%v", err)
	}
}

func TestTokenizeLineMaxIntoPreservesChecksAndResults(t *testing.T) {
	line := `1  close(3</e>) = 0`
	want, err := TokenizeLineMax(line, len(line))
	if err != nil {
		t.Fatal(err)
	}

	storage := []lexer.Token{{TypeName: "SENTINEL", Value: "untouched"}}
	before := append([]lexer.Token(nil), storage...)
	if _, err := TokenizeLineMaxInto(line+"\n", len(line), storage); err != ErrLineTooLong {
		t.Fatalf("over-limit newline error = %v, want ErrLineTooLong", err)
	}
	if !reflect.DeepEqual(storage, before) {
		t.Fatal("pre-lex length refusal changed caller storage")
	}
	if _, err := TokenizeLineMaxInto(line+"\n", len(line)+1, storage); err != ErrNewline {
		t.Fatalf("in-limit newline error = %v, want ErrNewline", err)
	}
	if !reflect.DeepEqual(storage, before) {
		t.Fatal("pre-lex newline refusal changed caller storage")
	}

	got, err := TokenizeLineMaxInto(line, len(line), storage)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("Into tokens differ:\n got: %#v\nwant: %#v", got, want)
	}
}

// A decoded string is arbitrary bytes, NUL included: consumers must reject it.
func TestDecodeStringCanReturnNUL(t *testing.T) {
	got, err := DecodeString(`a\0b`)
	if err != nil || string(got) != "a\x00b" {
		t.Fatalf("%q %v", got, err)
	}
}

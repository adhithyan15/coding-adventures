package straceparser

import (
	"errors"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	stracelexer "github.com/adhithyan15/coding-adventures/code/packages/go/strace-lexer"
)

func kindOf(t *testing.T, line string, o Options) Kind {
	t.Helper()
	_, _, err := ParseLineTokens(line, o)
	var le *LineError
	if !errors.As(err, &le) {
		t.Fatalf("%q: want a LineError, got %v", line, err)
	}
	return le.Kind
}

// Killed-at-entry lines are exact: a "???" call that returned, or any other
// spelling, is refused, never silently a dead line.
func TestDeadLinesAreExact(t *testing.T) {
	for _, l := range []string{
		`1  ???() = 0`, `1  ???()`, `1  ???()= ?`, `1  ???() = ? x`, `1  ???(<unfinished ...>`, `1  ???(  <unfinished ...>`,
		`1  ???(0x1) = ?`, `1  <... ??? resumed>) = 0`, `1  <... ??? resumed>)= ?`, `1  <... ??? resumed>0x1) = ?`,
		`1  <... ??? resumed>) = ? x`,
	} {
		if _, err := ParseLine(l, Options{}); err == nil {
			t.Errorf("%q accepted", l)
		}
	}
	for l, kind := range map[string]string{
		`1  ???( <unfinished ...>`:  "dead_line",
		`1  ???() = ?`:              "dead_line",
		`1  ???()      = ?`:         "dead_line",
		`1  <... ??? resumed>) = ?`: "dead_resumed_line",
	} {
		ast, err := ParseLine(l, Options{})
		if err != nil || BodyKind(ast) != kind {
			t.Errorf("%q: %v %q, want %q", l, err, BodyKind(ast), kind)
		}
	}
	// "???" is no longer an ordinary resume name.
	ast, err := ParseLine(`1  <... read resumed>) = 0`, Options{})
	if err != nil || BodyKind(ast) != "resumed_line" {
		t.Errorf("ordinary resume: %v", err)
	}
}

// Stricter-only divergences found in review (KILN04 section 4.9 table).
func TestReviewDivergences(t *testing.T) {
	// A signed pid: the old parser's Atoi accepted "[pid +5]".
	if k := kindOf(t, `[pid +5] f() = 0`, Options{}); k != KindErrToken {
		t.Errorf("signed pid: %d", k)
	}
	// The resumed half is validated by itself.  The old parser glued it to the
	// prefix first ("3" + "</x>) = 0" read as the annotation 3</x>); here the
	// half has a bare "<", a lone quote, or a lone "<" at its seam.
	for _, l := range []string{
		`1  <... f resumed></x>) = 0`,
		`1  <... f resumed>") = 0`,
		`1  <... f resumed><) = 0`,
	} {
		if k := kindOf(t, l, Options{}); k != KindErrToken {
			t.Errorf("%q: kind %d", l, k)
		}
	}
	// A signal line with more than 65,536 tokens (space runs) hits the token cap.
	sig := `1  --- ` + strings.Repeat("a ", 40000) + `---`
	_, _, err := ParseLineTokens(sig, Options{})
	var le *LineError
	if !errors.As(err, &le) || le.Kind != KindLimit || !errors.Is(err, parser.ErrMaxTokens) {
		t.Errorf("long signal line: %v", err)
	}
}

func TestMaxLineBytes(t *testing.T) {
	l := `1  stat("a", 0x1) = 0`
	o := Options{MaxLineBytes: len(l)}
	if _, err := ParseLine(l, o); err != nil {
		t.Fatal(err)
	}
	o.MaxLineBytes = len(l) - 1
	_, _, err := ParseLineTokens(l, o)
	var le *LineError
	if !errors.As(err, &le) || le.Kind != KindLimit || !errors.Is(err, stracelexer.ErrLineTooLong) {
		t.Errorf("got %v", err)
	}
	// The default is 1 MiB, applied before lexing (a huge line is refused cheaply).
	huge := `1  f(` + strings.Repeat("a", stracelexer.DefaultMaxLineBytes) + `)`
	if k := kindOf(t, huge, Options{}); k != KindLimit {
		t.Errorf("huge line: %d", k)
	}
	if DefaultOptions().MaxLineBytes != 1<<20 {
		t.Error("default MaxLineBytes")
	}
}

// Each zero limit is filled on its own: setting one field does not leave the
// others unlimited.
func TestPerFieldDefaults(t *testing.T) {
	got := Options{Options: parser.Options{StartRule: StartRule}}.withDefaults()
	want := DefaultOptions()
	if got.MaxDepth != want.MaxDepth || got.MaxTokens != want.MaxTokens || got.MaxMemoSlots != want.MaxMemoSlots || got.MaxLineBytes != want.MaxLineBytes {
		t.Errorf("got %+v, want %+v", got, want)
	}
	// A caller-set limit is kept; a zero memo follows the caller's token cap.
	got = Options{Options: parser.Options{MaxTokens: 100}}.withDefaults()
	if got.MaxTokens != 100 || got.MaxMemoSlots != memoFor(100) || got.MaxDepth != DefaultMaxDepth {
		t.Errorf("got %+v", got)
	}
	got = Options{Options: parser.Options{MaxDepth: 5, MaxMemoSlots: 7}, MaxLineBytes: 9}.withDefaults()
	if got.MaxDepth != 5 || got.MaxMemoSlots != 7 || got.MaxLineBytes != 9 || got.MaxTokens != DefaultMaxTokens {
		t.Errorf("got %+v", got)
	}
	// End to end: only StartRule set, yet the token cap applies.
	line := `1  f(` + strings.Repeat("a,", 40000) + `a)`
	o := Options{Options: parser.Options{StartRule: StartRule}}
	if k := kindOf(t, line, o); k != KindLimit {
		t.Errorf("token cap not applied: %d", k)
	}
	// And deep nesting is limited with only MaxTokens set.
	deep := `1  f(` + strings.Repeat("[", 300) + strings.Repeat("]", 300) + `)`
	if k := kindOf(t, deep, Options{Options: parser.Options{MaxTokens: 5000}}); k != KindLimit {
		t.Errorf("depth cap not applied: %d", k)
	}
}

func TestStrictQuoted(t *testing.T) {
	for _, l := range []string{
		`1  f("a\qb")`, `1  f("\400")`, `1  f("\x4")`, `1  f("\x4g")`, `1  f("a\\\")`, `1  f("\<")`,
	} {
		_, _, err := ParseLineTokens(l, Options{})
		var le *LineError
		// "a\\\" ends with an escaped quote and is unterminated: ERR_QUOTE, not a bad string.
		if !errors.As(err, &le) || (le.Kind != KindBadQuoted && le.Kind != KindErrToken) {
			t.Errorf("%q: %v", l, err)
			continue
		}
		if le.Kind == KindBadQuoted && !le.Ambiguous() {
			t.Errorf("%q: a bad string is an ambiguous path", l)
		}
	}
	// A trailing backslash inside the quotes: "a\" is an escaped quote (unterminated).
	if k := kindOf(t, `1  f("a\")`, Options{}); k != KindErrToken {
		t.Errorf("trailing backslash: %d", k)
	}
	// Valid escapes pass, and the check can be turned off.
	if _, err := ParseLine(`1  f("a\tb\"c\303\251\x41", "\0")`, Options{}); err != nil {
		t.Errorf("valid escapes: %v", err)
	}
	bad := `1  f("a\qb")`
	if _, err := ParseLine(bad, Options{LenientQuoted: true}); err != nil {
		t.Errorf("lenient: %v", err)
	}
	if k := kindOf(t, bad, Options{}); k != KindBadQuoted {
		t.Errorf("strict: %d", k)
	}
}

func TestPrefixBeforeUnfinished(t *testing.T) {
	cases := map[string]string{
		`1  wait4(-1,  <unfinished ...>`:                                            `wait4(-1, `,
		`1  wait4(-1, <unfinished ...>`:                                             `wait4(-1,`,
		`1  openat(AT_FDCWD</w>, "main.o", O_WRONLY|O_CREAT, 0666 <unfinished ...>`: `openat(AT_FDCWD</w>, "main.o", O_WRONLY|O_CREAT, 0666`,
		`1  vfork( <unfinished ...>`:                                                `vfork(`,
		`[pid 7]   stat("/tmp/x"<unfinished ...>`:                                   `stat("/tmp/x"`,
		`1  f(a,   <unfinished ...>`:                                                `f(a,  `,
	}
	for line, want := range cases {
		ast, err := ParseLine(line, Options{})
		if err != nil {
			t.Fatalf("%q: %v", line, err)
		}
		got, ok := PrefixBeforeUnfinished(line, ast)
		if !ok || got != want {
			t.Errorf("%q: %q %v, want %q", line, got, ok, want)
		}
	}
	for _, line := range []string{`1  f(a) = 0`, `1  <... f resumed>) = 0`, `1  +++ exited with 0 +++`, `1  ???( <unfinished ...>`} {
		ast, _ := ParseLine(line, Options{})
		if _, ok := PrefixBeforeUnfinished(line, ast); ok {
			t.Errorf("%q: reported a prefix", line)
		}
	}
	if _, ok := PrefixBeforeUnfinished("", nil); ok {
		t.Error("nil tree")
	}
}

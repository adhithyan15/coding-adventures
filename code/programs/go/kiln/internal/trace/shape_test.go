package trace

import (
	"io"
	"strings"
	"testing"
)

func openString(s string) func() (io.ReadCloser, error) {
	return func() (io.ReadCloser, error) { return io.NopCloser(strings.NewReader(s)), nil }
}

// The outline of an ordinary line shows token types, a class for each word,
// and a call name only because the name is in the closed table.
func TestShapeOutline(t *testing.T) {
	const text = "1234  openat(AT_FDCWD</w>, \"/etc/passwd\", O_RDONLY|0x7f) = 3</etc/passwd>\n"
	got := Shape(openString(text), 1)
	if len(got) != 1 || got[0].Role != "stop" || got[0].Line != 1 {
		t.Fatalf("shapes = %+v", got)
	}
	want := "PID SYSCALL:openat ANNOTATED COMMA QUOTED COMMA WORD:upper PUNCT[|] WORD:hex RPAREN PUNCT[=] WORD:digits"
	// The result "3</etc/passwd>" lexes as one ANNOTATED.
	want = strings.Replace(want, "WORD:digits", "ANNOTATED", 1)
	if got[0].Tokens != want {
		t.Errorf("tokens = %q\nwant     %q", got[0].Tokens, want)
	}
	if strings.Contains(got[0].Facts, "resumed") || !strings.Contains(got[0].Facts, "tokens=") {
		t.Errorf("facts = %q", got[0].Facts)
	}
}

// A resumed line is shown with the pid's pending half first, and nothing else.
func TestShapePendingHalf(t *testing.T) {
	const text = "7  wait4(-1,  <unfinished ...>\n8  getpid()                     = 8\n7  <... wait4 resumed>[{WIFEXITED(s)}], 0, NULL) = 9\n"
	got := Shape(openString(text), 3)
	if len(got) != 2 || got[0].Role != "pending" || got[0].Line != 1 || got[1].Role != "stop" || got[1].Line != 3 {
		t.Fatalf("shapes = %+v", got)
	}
	if !strings.Contains(got[0].Facts, "unfinished-marker") || !strings.Contains(got[1].Facts, "resumed") {
		t.Errorf("facts: %q / %q", got[0].Facts, got[1].Facts)
	}
	if !strings.Contains(got[1].Tokens, "RESUMED:wait4") {
		t.Errorf("tokens = %q", got[1].Tokens)
	}
	// A pid whose last line is not unfinished has no pending half.
	if got := Shape(openString("7  getpid() = 1\n7  getpid() = 2\n"), 2); len(got) != 1 {
		t.Errorf("shapes = %+v, want the stop line alone", got)
	}
	// Absent lines.
	if Shape(openString(text), 99) != nil || Shape(openString(text), 0) != nil {
		t.Error("a line that is not there has a shape")
	}
}

// The line cap and the token cap.
func TestShapeBounds(t *testing.T) {
	line := "5  f(" + strings.Repeat("1,", 500) + ") = 0"
	got := Shape(openString(line+"\n"), 1)
	if len(got) != 1 || !got[0].Truncated || !strings.Contains(got[0].Tokens, " more") {
		t.Fatalf("shapes = %+v", got)
	}
	if n := len(strings.Fields(got[0].Tokens)); n > MaxShapeTokens+2 {
		t.Errorf("%d tokens printed", n)
	}
	huge := "5  f(\"" + strings.Repeat("x", maxShapeLine+10) + "\") = 0\n"
	got = Shape(openString(huge), 1)
	if len(got) != 1 || !got[0].Truncated {
		t.Fatalf("an over-long line was not marked truncated: %+v", got)
	}
}

func TestWordClass(t *testing.T) {
	for in, want := range map[string]string{
		"123": "digits", "-1": "digits", "0x7f00": "hex", "-0xFF": "hex", "0x": "lower", "0xZZ": "mixed",
		"O_RDONLY": "upper", "AT_FDCWD": "upper", "read": "lower", "a_b-c1": "lower", "Foo": "mixed",
		"-": "mixed", "1_2": "mixed",
	} {
		if got := wordClass(in); got != want {
			t.Errorf("wordClass(%q) = %q, want %q", in, got, want)
		}
	}
}

func TestPunctShape(t *testing.T) {
	for in, want := range map[string]string{
		"|": "|", "&&": "&&", "/*": "/*", "é": `\x80`, "éé": `\x80`, "=>": "=>",
		"!!!!!!!!!!!!!": "!!!!!!!!+",
	} {
		if got := punctShape(in); got != want {
			t.Errorf("punctShape(%q) = %q, want %q", in, got, want)
		}
	}
}

// HOSTILE INPUT.  A secret-looking string in every token kind (and in the
// pending half): no byte of any of them may come out of Shape.  The probes are
// chosen so that none can appear by accident in a fixed word of the outline.
func TestShapeLeaksNoTokenContent(t *testing.T) {
	secrets := []string{
		"zqSecretQuoted", "zqSecretAnnot", "zqSecretWord", "zqsecretword", "ZQSECRETUPPER", "zqSecretAssign",
		"zqSecretFunc", "zqsecretcall", "ZqSecretResumed", "zqSecretSig", "zqSecretErr", "hunter2", "zq9",
	}
	lines := []string{
		// QUOTED, ANNOTATED, WORD (all classes), ASSIGN, FUNC, SYSCALL, PUNCT (non-ASCII), nested.
		"11  zqsecretcall(AT_FDCWD<zqSecretAnnot>, \"zqSecretQuoted\"..., zqSecretWord|ZQSECRETUPPER, zqSecretAssign=hunter2, zqSecretFunc(zq9), éé /* zq9 */) = 0",
		// A bad annotation, an unterminated quote.
		"11  openat(AT_FDCWD<zqSecretAnnot, \"zqSecretQuoted",
		"11  openat(zqSecretWord<zq\x01Secret>)",
		// A resumed marker naming something outside the table.
		"11  <... ZqSecretResumed resumed>zqSecretWord) = 0",
		// A signal line: SIG_TEXT swallows its text.
		"11  --- zqSecretSig {si_signo=zqSecretSig, si_pid=hunter2} ---",
		// A bad pid prefix swallows the line.
		"zqSecretErr hunter2 \"zqSecretQuoted\"",
		// An exited line.
		"11  +++ exited with 3 +++",
		// ERR_ANY / control characters.
		"11  openat(\tzqSecretWord)",
	}
	pending := "11  zqsecretcall(\"zqSecretQuoted\", zqSecretWord,  <unfinished ...>"
	for i, l := range lines {
		text := pending + "\n" + l + "\n"
		for _, n := range []int{1, 2} {
			for _, sh := range Shape(openString(text), n) {
				out := sh.Tokens + "\n" + sh.Facts + "\n" + sh.Role
				for _, s := range secrets {
					if strings.Contains(out, s) {
						t.Errorf("line %d (shape of line %d): output %q leaks %q", i, n, out, s)
					}
				}
				for _, r := range out {
					if r > 0x7e || (r < 0x20 && r != '\n') {
						t.Errorf("line %d: output %q holds a non-printable rune %q", i, out, r)
					}
				}
			}
		}
	}
}

// A token type name that is not on the closed list is never printed.
func TestShapeTokenIsClosed(t *testing.T) {
	if got := shapeToken("NOT_A_TOKEN zqSecret", "zqSecret"); got != "TOKEN?" {
		t.Errorf("got %q", got)
	}
	if got := shapeToken("QUOTED", "\"zqSecret\""); got != "QUOTED" {
		t.Errorf("got %q", got)
	}
	if got := shapeToken("ANNOTATED", "fd<zqSecret>"); got != "ANNOTATED" {
		t.Errorf("got %q", got)
	}
	if got := shapeToken("WORD", "ptrace"); got != "WORD:ptrace" {
		t.Errorf("a table name is the one literal a word may print, got %q", got)
	}
	if got := shapeToken("WORD", "zqsecret"); got != "WORD:lower" {
		t.Errorf("got %q", got)
	}
}

// ParseCrossCheckedDetail names the row of each allowed divergence, in order,
// with the line it happened on, and ParseCrossChecked still counts them.
func TestParseCrossCheckedDetailNamesTheRow(t *testing.T) {
	const text = "2  chdir(a,  <unfinished ...>\n2  --- SIGCHLD {si_signo=SIGCHLD} ---\n2  <... chdir resumed> -1,  , b) = 0\n"
	_, err, m, d := ParseCrossCheckedDetail(strings.NewReader(text), "/work/repo", Options{})
	if m != nil || err == nil || d.Allowed != 1 || len(d.Hits) != 1 {
		t.Fatalf("err=%v mismatch=%v detail=%+v", err, m, d)
	}
	if d.Hits[0].Row == "" || d.Rows[d.Hits[0].Row] != 1 || d.Hits[0].Line <= 0 {
		t.Errorf("hit = %+v rows = %v", d.Hits[0], d.Rows)
	}
	// A clean refusal allows nothing, and the next call starts from nothing.
	_, _, _, d = ParseCrossCheckedDetail(strings.NewReader("garbage\n"), "/work/repo", Options{})
	if d.Allowed != 0 || len(d.Hits) != 0 || len(d.Rows) != 0 {
		t.Errorf("detail = %+v", d)
	}
}

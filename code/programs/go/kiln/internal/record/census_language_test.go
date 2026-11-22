package record

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// refusalSyscall: a refused:<name> code is its own name when the table has it,
// else the error's Call when the table has THAT, else "other".
func TestRefusalSyscall(t *testing.T) {
	for _, c := range []struct {
		code string
		err  *trace.Error
		want string
	}{
		{"refused:ptrace", nil, "ptrace"},
		{"refused:other", &trace.Error{Call: "chdir"}, "chdir"}, // the code names none; the error does
		{"cwd-race", &trace.Error{Call: "chdir"}, "chdir"},
		{"cwd-race", &trace.Error{Call: "fchdir"}, "fchdir"},
		{"fd-path", &trace.Error{Call: "fchmod"}, "fchmod"},
		{"fd-path", &trace.Error{Call: "no_such_call"}, "other"},
		{"fd-path", &trace.Error{Call: "/home/secret"}, "other"},
		{"unparsable-line", &trace.Error{}, "other"},
		{"unparsable-line", nil, "other"},
		{"refused:/etc/passwd", nil, "other"},
	} {
		if got := refusalSyscall(c.code, c.err); got != c.want {
			t.Errorf("refusalSyscall(%q, %v) = %q, want %q", c.code, c.err, got, c.want)
		}
	}
}

// D44: a dying call outside the table reports one of two fixed codes, and a
// hostile or stale value is never echoed.
func TestRefusalSyscallUnknownNumber(t *testing.T) {
	for _, c := range []struct {
		code string
		err  *trace.Error
		want string
	}{
		{"unresumed-call", &trace.Error{UnknownNumber: "unknown-number:yes"}, "unknown-number:yes"},
		{"unresumed-call", &trace.Error{UnknownNumber: "unknown-number:no"}, "unknown-number:no"},
		{"unresumed-call", &trace.Error{UnknownNumber: "syscall_0x1c8"}, "other"},
		{"unresumed-call", &trace.Error{UnknownNumber: "evil\n::error::x"}, "other"},
		{"unresumed-call", &trace.Error{UnknownNumber: "unknown-number:yes\n"}, "other"},
		{"cwd-race", &trace.Error{UnknownNumber: "unknown-number:yes"}, "other"}, // only for unresumed-call
		{"unresumed-call", &trace.Error{}, "other"},
	} {
		if got := refusalSyscall(c.code, c.err); got != c.want {
			t.Errorf("refusalSyscall(%q, %+v) = %q, want %q", c.code, c.err, got, c.want)
		}
	}
}

func TestCountTables(t *testing.T) {
	t2 := CountTable{}
	t2.add("go", "cwd-race")
	t2.add("go", "cwd-race")
	t2.add("c", "fd-path")
	if t2["go"]["cwd-race"] != 2 || t2["c"]["fd-path"] != 1 || len(t2) != 2 {
		t.Errorf("%v", t2)
	}
	t3 := CountTable3{}
	t3.add("go", "cwd-race", "chdir")
	t3.add("go", "cwd-race", "chdir")
	t3.add("go", "cwd-race", "fchdir")
	if t3["go"]["cwd-race"]["chdir"] != 2 || t3["go"]["cwd-race"]["fchdir"] != 1 {
		t.Errorf("%v", t3)
	}
}

// The new Markdown sections put every language, code and syscall name in a code
// span, in a list item: a name that could autolink or render as an emoji never
// appears bare.
func TestMarkdownParseRefusalTablesAreCodeSpans(t *testing.T) {
	s := &CensusSummary{Explain: newExplain()}
	s.Explain.RecordParseReasonsByLanguage.add("www.evil.example", ":smile:")
	s.Explain.ParseRefusalsByLanguage.add("a:smile:b", "www.evil.example")
	s.Explain.ParseRefusalSyscalls.add(":smile:", "www.evil.example")
	s.Explain.ParseRefusalSyscallsByLanguage.add("www.evil.example", "a:smile:b", ":smile:")
	md, _, _ := RenderCensusMarkdown(s)
	outside := stripSpans(md)
	for _, bad := range []string{"www.evil", ":smile:", "a:smile"} {
		if strings.Contains(outside, bad) {
			t.Errorf("%q occurs outside a code span:\n%s", bad, md)
		}
	}
	for _, l := range strings.Split(md, "\n") {
		if strings.HasPrefix(l, "|") && (strings.Contains(l, "evil") || strings.Contains(l, "smile")) {
			t.Errorf("a table row holds a name: %q", l)
		}
	}
	for _, want := range []string{"### Parse reasons by language", "### Parse refusals by system call"} {
		if !strings.Contains(md, want) {
			t.Errorf("missing section %q", want)
		}
	}
}

// An empty summary renders the sections as "none".
func TestMarkdownParseRefusalTablesEmpty(t *testing.T) {
	md, _, _ := RenderCensusMarkdown(&CensusSummary{})
	if !strings.Contains(md, "### Parse refusals by system call") {
		t.Errorf("section missing:\n%s", md)
	}
}

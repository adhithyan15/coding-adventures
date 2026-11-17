package trace

import (
	"errors"
	"strings"
	"testing"
)

// D31: strace may print a signal line (--- SIG ... ---) for a pid between the
// two halves of one of its split calls.  The old scanner ignores such a line
// (it leaves the pending call alone); the cross-check's "too many arguments and
// a later error" row rebuilds the glued call from the earlier lines, and used
// to forget the prefix at the signal line, so the row did not match and a
// legitimate stricter-only divergence was reported as outside the list.

func crossCheckOf(text string) (err error, mismatch *CrossCheckMismatch) {
	defer func() {
		if r := recover(); r != nil {
			m, ok := r.(*CrossCheckMismatch)
			if !ok {
				panic(r)
			}
			mismatch = m
		}
	}()
	_, err = parseCrossChecked(strings.NewReader(text), "/work/repo", Options{})
	return
}

func TestSignalBetweenSplitCallHalves(t *testing.T) {
	const open = "2  chdir(a,  <unfinished ...>\n"
	const resumed = "2  <... chdir resumed> -1,  , b) = 0\n"
	const sig = "2  --- SIGCHLD {si_signo=SIGCHLD} ---\n"
	const otherSig = "3  --- SIGCHLD {si_signo=SIGCHLD} ---\n"
	const otherExit = "3  +++ exited with 0 +++\n"
	cases := map[string]string{
		"no interleaving":        open + resumed,
		"same-pid signal":        open + sig + resumed,
		"two same-pid signals":   open + sig + sig + resumed,
		"other-pid signal":       open + otherSig + resumed,
		"other-pid exit":         open + otherExit + resumed,
		"other-pid signal+exit":  open + otherSig + otherExit + sig + resumed,
		"signal after the glue":  open + resumed + sig,
		"other-pid unfinished":   open + "3  chdir(b,  <unfinished ...>\n" + sig + resumed,
		"signal with no resumed": open + sig + resumed + otherSig,
	}
	for name, text := range cases {
		err, m := crossCheckOf(text)
		if m != nil {
			t.Errorf("%s: cross-check failed: %v", name, m)
			continue
		}
		var e *Error
		if !errors.As(err, &e) {
			t.Errorf("%s: want a refusal, got %v", name, err)
		}
	}
	// The row must actually have been taken for the signal variant.
	before := AllowedDivergences()
	if _, m := crossCheckOf(open + sig + resumed); m != nil {
		t.Fatal(m)
	}
	if AllowedDivergences() != before+1 {
		t.Errorf("the stricter-only row was not recorded for the signal variant")
	}
}

// An exit line of the SAME pid ends its pending call, so the resumed half has
// nothing to glue to.  Whatever happens, the cross-check must not fail open.
func TestExitBetweenSplitCallHalves(t *testing.T) {
	text := "2  chdir(a,  <unfinished ...>\n2  +++ exited with 0 +++\n2  <... chdir resumed> -1,  , b) = 0\n"
	err, m := crossCheckOf(text)
	if err == nil {
		t.Fatal("must be refused")
	}
	if m != nil && strings.Contains(m.Detail, "FAIL-OPEN") {
		t.Fatalf("fail-open: %v", m)
	}
}

// A signal line does not turn the arity row into a catch-all: the row still
// needs a pending call of the same pid, an old reason of arity, and a glued
// call over its arity cap.  And a genuinely unlisted pair still fails.
func TestSignalDoesNotWidenTheTable(t *testing.T) {
	const row = "too many arguments and a later error"
	within := []byte("2  chdir(a <unfinished ...>\n2  --- SIGCHLD {si_signo=SIGCHLD} ---\n2  <... chdir resumed> ) = 0\n")
	if got := stricterRow(lineAt(within, 3), reasonArity, reasonUnparsable, within, 3, 1<<20); got == row {
		t.Errorf("within-arity glued call matched the row")
	}
	over := []byte("2  chdir(a,  <unfinished ...>\n2  --- SIGCHLD {si_signo=SIGCHLD} ---\n2  <... chdir resumed> -1,  , b) = 0\n")
	if got := stricterRow(lineAt(over, 3), reasonArity, reasonUnparsable, over, 3, 1<<20); got != row {
		t.Errorf("over-arity glued call across a signal: row = %q", got)
	}
	if got := stricterRow(lineAt(over, 3), reasonUnmatched, reasonUnparsable, over, 3, 1<<20); got == row {
		t.Errorf("row matched for an old reason other than arity")
	}
	foreign := []byte("3  chdir(a,  <unfinished ...>\n2  --- SIGCHLD {si_signo=SIGCHLD} ---\n2  <... chdir resumed> -1,  , b) = 0\n")
	if got := stricterRow(lineAt(foreign, 3), reasonArity, reasonUnparsable, foreign, 3, 1<<20); got == row {
		t.Errorf("row matched with no pending call for the pid")
	}
	exited := []byte("2  chdir(a,  <unfinished ...>\n2  +++ exited with 0 +++\n2  <... chdir resumed> -1,  , b) = 0\n")
	if got := stricterRow(lineAt(exited, 3), reasonArity, reasonUnparsable, exited, 3, 1<<20); got == row {
		t.Errorf("row matched after the pid's exit line ended its pending call")
	}
	// A real unlisted pair is still reported.
	d, _ := compareParses([]byte("1  chdir(a) = 0\n"), Options{}, false, nil, &Error{Reason: reasonArity, Line: 1}, nil, &Error{Reason: reasonSize, Line: 1})
	if d == "" {
		t.Errorf("an unlisted divergence passed compareParses")
	}
}

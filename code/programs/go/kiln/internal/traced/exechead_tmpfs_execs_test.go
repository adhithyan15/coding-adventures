package traced

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
)

// KILN04 D48 step 6, the vacuity counter.  Census.TmpfsExecs counts the
// successful execs under the step tmpfs that reached the exec-head pairing, so a
// run whose every sub-code is 0 can still be told apart: "nothing was refused"
// and "nothing was judged" are different facts.  The counter sits in loadChain's
// tmpfs branch, the one place the exechead-* sub-codes are raised, so it can
// never drift below their sum.

// execHeadSubTotal is the occurrences of every exechead-* exec-unreadable
// sub-code the census holds.
func execHeadSubTotal(c *Census) int64 {
	var n int64
	for _, e := range c.Entries() {
		if s, ok := strings.CutPrefix(e.Kind, "trace-incomplete:exec-unreadable:"); ok && strings.HasPrefix(s, "exechead-") {
			n += e.Count
		}
	}
	return n
}

// Nothing exec'd under the tmpfs: only the init's host shell.  The mechanism had
// nothing to judge, and the counter says so even though a supervisor was present.
func TestTmpfsExecsIsZeroWhenNothingExecdUnderTheTmpfs(t *testing.T) {
	h := newHarness(t)
	_, c := h.ehRun(ehFile(ehKernel, shellLine()), ehTrace(1, shellEvent()))
	if c.TmpfsExecs != 0 {
		t.Fatalf("TmpfsExecs %d, want 0", c.TmpfsExecs)
	}
	if n := execHeadSubTotal(c); n != 0 {
		t.Fatalf("sub-codes %d, want 0", n)
	}
}

// Without a supervisor an exec under the tmpfs is refused as `under-tmp` (today's
// behaviour) and never reaches the pairing: it is not counted.
func TestTmpfsExecsIgnoresAnUnsupervisedStep(t *testing.T) {
	h := newHarness(t)
	_, c := h.ehRun(nil, ehTrace(1, shellEvent(), execAs(2, ehBin)))
	if c.TmpfsExecs != 0 {
		t.Fatalf("TmpfsExecs %d, want 0 for a step with no supervisor", c.TmpfsExecs)
	}
}

// A believed exec raises no reason at all, so a census built only from reasons
// would call this run empty.  The counter still sees it.
func TestTmpfsExecsCountsABelievedExec(t *testing.T) {
	h := newHarness(t)
	rec, c := h.ehRun(ehFile(ehKernel, shellLine(), ehConfirmed(2, 2, ehBin, ehELF(ehUnknownLoader))), ehTrace(1, shellEvent(), execAs(2, ehBin)))
	if hasReason(rec, rExecUnreadable) {
		t.Fatalf("the exec must be believed: %q", rec.Reasons)
	}
	if c.TmpfsExecs != 1 || execHeadSubTotal(c) != 0 {
		t.Fatalf("TmpfsExecs %d sub-codes %d, want 1 and 0", c.TmpfsExecs, execHeadSubTotal(c))
	}
}

// One believed exec and two refused (a mismatch status, and an exec with no line
// at all: exechead-pair).  All three were judged; only the refused two are
// sub-codes, so the counter is at least their sum, and here exactly one more.
func TestTmpfsExecsCountsBelievedAndRefusedExecs(t *testing.T) {
	h := newHarness(t)
	file := ehFile(ehKernel, shellLine(),
		ehConfirmed(2, 2, ehBin, ehELF(ehUnknownLoader)),
		ehPlain(3, ehBin2, exechead.StatusMismatch),
	)
	tr := ehTrace(1, shellEvent(), execAs(2, ehBin), execAs(3, ehBin2), execAs(4, "/tmp/kiln-1/go-build/b003/z.test"))
	rec, c := h.ehRun(file, tr)
	if !hasReason(rec, rExecUnreadable) {
		t.Fatalf("the refused execs must void the record: %q", rec.Reasons)
	}
	subsTotal := execHeadSubTotal(c)
	if subsTotal != 2 {
		t.Fatalf("sub-codes %d (%v), want 2", subsTotal, subs(c))
	}
	if c.TmpfsExecs != 3 {
		t.Fatalf("TmpfsExecs %d, want 3", c.TmpfsExecs)
	}
	if c.TmpfsExecs < subsTotal {
		t.Fatalf("TmpfsExecs %d is below the sub-code sum %d", c.TmpfsExecs, subsTotal)
	}
}

// The counter is bounded like the other counts: it saturates, it does not wrap.
func TestTmpfsExecsSaturates(t *testing.T) {
	c := NewCensus()
	c.TmpfsExecs = MaxTmpfsExecs
	c.noteTmpfsExec()
	if c.TmpfsExecs != MaxTmpfsExecs {
		t.Fatalf("TmpfsExecs %d, want it held at %d", c.TmpfsExecs, int64(MaxTmpfsExecs))
	}
	var nilCensus *Census
	nilCensus.noteTmpfsExec() // a classifier with no observer must not panic
}

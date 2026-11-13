package traced

import "testing"

func TestOverlappingRemovalOfAnOwnPath(t *testing.T) {
	// P1 makes d1 and stats it while P2's rename of d1 is in flight: the stat may
	// have run after the rename took effect, so "d1 is the action's own" does not
	// stand.
	mk := timed(mkdir("code/p/a/d1"), 1, 2, 2)
	rn := timed(rename("code/p/a/d1", "code/p/a/d2"), 2, 3, 6)
	st := timed(stat("code/p/a/d1"), 1, 4, 5)
	h := newHarness(t)
	h.noSettle = true
	h.fs.dir(abs("code/p/a/d2"))
	rec := h.runSteps(mtrace(mk, rn, st))
	wantReason(t, rec, "trace-incomplete:unstable-path")
	// A rename that began after the stat completed is not an overlap.
	rn2 := timed(rename("code/p/a/d1", "code/p/a/d2"), 2, 7, 8)
	h = newHarness(t)
	h.noSettle = true
	h.fs.dir(abs("code/p/a/d2"))
	h.fs.dir(abs("code/p/a/d1"))
	rec = h.runSteps(mtrace(mk, timed(lstat("code/p/a/d1"), 1, 3, 4), rn2))
	if hasReason(rec, "trace-incomplete:unstable-path") {
		t.Fatalf("%q", rec.Reasons)
	}
}

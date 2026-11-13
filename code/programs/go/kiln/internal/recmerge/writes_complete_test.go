package recmerge

import "testing"

// M3: recorder version 2 requires writes_complete.  A record without it never
// proved its write set complete, so it is unknown even with NO trace-incomplete
// reason, with the switch off or on.
func TestMissingWritesCompleteIsUnknown(t *testing.T) {
	for _, honour := range []bool{false, true} {
		w := std(t)
		w.honour = honour
		w.rec(2, "pkg/d").Complete = nil
		r := w.merge()
		wantVoid(t, "missing writes_complete", r, "M1")
		if r.Emitted || r.relaxedOK() {
			t.Errorf("honour=%v: a record without writes_complete must void the run", honour)
		}
	}
	// A classifier that could not vouch for its writes also left the reason,
	// which a merge that ignores the key still treats as unknown (switch off).
	w := std(t)
	w.rec(2, "pkg/d").Record.Reasons = sl("trace-incomplete:writes-unknown")
	no := false
	w.rec(2, "pkg/d").Complete = &no
	if r := w.merge(); r.Emitted {
		t.Error("writes-unknown with writes_complete=false must void the run")
	}
	w.honour = true
	if r := w.merge(); r.Emitted {
		t.Error("writes-unknown with writes_complete=false must void the run, switch on")
	}
}

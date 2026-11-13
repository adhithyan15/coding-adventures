package recmerge

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// L6: an action that did not run to completion (failed, refused, gated) was
// never classified, so its writes_complete is an explicit false (or absent in
// memory).  That is a truncated execution (M-1): the strict verdict is void,
// and the relaxed verdict charges only that action.  A BUILT record with a
// false or missing key stays an unknown write set.
func TestNonBuiltRecordWithFalseIsATruncationNotAnUnknownWrite(t *testing.T) {
	no := false
	for _, c := range []struct {
		name string
		set  *bool
	}{{"false", &no}, {"nil", nil}} {
		w := std(t)
		w.honour = true
		r := w.rec(2, "pkg/d")
		r.Status = "failed"
		r.Complete = c.set
		res := w.merge()
		if res.Emitted {
			t.Errorf("%s: a failed action must void the strict run", c.name)
		}
		if !res.relaxedOK() {
			t.Errorf("%s: the relaxed verdict must charge only the action, got %v", c.name, res.RunVoid)
		}
		wantRelaxed(t, res, "pkg/a", "pkg/b", "pkg/c")
	}
	// Built: false or missing is unknown, strict and relaxed.
	for _, c := range []struct {
		name string
		set  *bool
	}{{"false", &no}, {"nil", nil}} {
		w := std(t)
		w.honour = true
		w.rec(2, "pkg/d").Complete = c.set
		res := w.merge()
		if res.Emitted || res.relaxedOK() {
			t.Errorf("built with %s: must void the run in both verdicts", c.name)
		}
	}
}

func TestCheckCompleteForNonBuiltRecords(t *testing.T) {
	no, yes := false, true
	r := recfmt.ShardRecord{Action: "a", Status: "failed", Complete: &no}
	if err := recfmt.CheckComplete(r); err != nil {
		t.Errorf("a non-built record with an explicit false needs no raw record: %v", err)
	}
	r.Complete = nil
	if recfmt.CheckComplete(r) == nil {
		t.Error("the key stays required for a non-built record")
	}
	r.Complete = &yes
	if recfmt.CheckComplete(r) == nil {
		t.Error("a true must still match the classifier's own answer")
	}
	r.Status = recfmt.StatusBuilt
	r.Complete = &no
	r.Record.WritesComplete = true
	if recfmt.CheckComplete(r) == nil {
		t.Error("a built record must match the classifier")
	}
}

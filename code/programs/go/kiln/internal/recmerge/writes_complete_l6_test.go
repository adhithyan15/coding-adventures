package recmerge

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// L6, L8: a record that RAN NOTHING (refused or gated: steps_run 0) was never
// classified, so its writes_complete is an explicit false (or absent in
// memory); that is a truncated execution (M-1): the strict verdict is void and
// the relaxed verdict charges only that action.  Any record that ran a step,
// failed or built, was classified: a false or missing key is an unknown write
// set in both verdicts (a hostile BUILD could otherwise pick a nonzero exit).
func TestRecordsThatRanNothingAreTruncationsNotUnknownWrites(t *testing.T) {
	no := false
	for _, c := range []struct {
		name string
		set  *bool
	}{{"false", &no}, {"nil", nil}} {
		// (a refused record needs a binding that refuses the package, B5; the gated
		// one is the same case with a weak key)
		for _, kind := range []string{"gated"} {
			w := std(t)
			w.honour = true
			r := w.rec(2, "pkg/d")
			r.Status = kind
			r.StepsRun, r.Steps = 0, []recfmt.Step{}
			r.StepsSHA = ""
			r.Record = recfmt.Normalize(recfmt.RawRecord{Package: r.Action}) // ran nothing: nothing classified
			r.Gate = "precondition"
			r.Complete = c.set
			res := w.merge()
			if res.Emitted {
				t.Errorf("%s %s: must void the strict run", kind, c.name)
			}
			if !res.relaxedOK() {
				t.Errorf("%s %s: the relaxed verdict must charge only the action, got %v", kind, c.name, res.RunVoid)
			}
			wantRelaxed(t, res, "pkg/a", "pkg/b", "pkg/c")
		}
	}
	// Anything that ran a step: false or missing is unknown, strict and relaxed.
	for _, status := range []string{"built", "failed"} {
		for _, c := range []struct {
			name string
			set  *bool
		}{{"false", &no}, {"nil", nil}} {
			w := std(t)
			w.honour = true
			r := w.rec(2, "pkg/d")
			r.Status = status
			r.Complete = c.set
			res := w.merge()
			if res.Emitted || res.relaxedOK() {
				t.Errorf("%s with %s: must void the run in both verdicts", status, c.name)
			}
		}
	}
}

func TestCheckCompleteForRecordsThatRanNothing(t *testing.T) {
	no, yes := false, true
	r := recfmt.ShardRecord{Action: "a", Status: "refused", StepsRun: 0, Complete: &no}
	if err := recfmt.CheckComplete(r); err != nil {
		t.Errorf("a record that ran nothing, with an explicit false, needs no raw record: %v", err)
	}
	r.Complete = nil
	if recfmt.CheckComplete(r) == nil {
		t.Error("the key stays required")
	}
	r.Complete = &yes
	if recfmt.CheckComplete(r) == nil {
		t.Error("a true must match the classifier's own answer")
	}
	// A failed record that ran a step was classified: it must agree.
	r = recfmt.ShardRecord{Action: "a", Status: "failed", StepsRun: 1, Complete: &no}
	r.Record.WritesComplete = true
	if recfmt.CheckComplete(r) == nil {
		t.Error("a failed record that ran a step must match the classifier")
	}
	r.Record.WritesComplete = false
	if err := recfmt.CheckComplete(r); err != nil {
		t.Error(err)
	}
	r.Status = recfmt.StatusBuilt
	r.Complete, r.Record.WritesComplete = &no, true
	if recfmt.CheckComplete(r) == nil {
		t.Error("a built record must match the classifier")
	}
}

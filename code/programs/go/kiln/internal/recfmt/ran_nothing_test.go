package recfmt

import "testing"

// A record that ran nothing was never classified: any raw-record content is a
// recorder bug and is refused.
func TestRecordThatRanNothingCarriesNoClassification(t *testing.T) {
	mk := func() ShardRecord {
		r := goodRecord("a")
		r.Status, r.StepsRun, r.Steps, r.StepsSHA = "gated", 0, []Step{}, ""
		r.Gate = "precondition"
		r.Record = Normalize(RawRecord{Package: "a"})
		f := false
		r.Complete = &f
		return r
	}
	r := mk()
	if err := r.Validate(); err != nil {
		t.Fatalf("an empty raw record is fine: %v", err)
	}
	for name, fn := range map[string]func(*ShardRecord){
		"entries":  func(r *ShardRecord) { r.Record.Entries = []string{"pkg:a"} },
		"writes":   func(r *ShardRecord) { r.Record.Writes = []string{"x"} },
		"tracked":  func(r *ShardRecord) { r.Record.TrackedWrites = []string{"x"} },
		"reasons":  func(r *ShardRecord) { r.Record.Reasons = []string{"failed"} },
		"binds":    func(r *ShardRecord) { r.Record.Binds = []string{"tcp/127.0.0.1:1"} },
		"absent":   func(r *ShardRecord) { r.Record.AbsentObservations = []string{"x"} },
		"untrack":  func(r *ShardRecord) { r.Record.UntrackedObservations = []string{"x"} },
		"traverse": func(r *ShardRecord) { r.Record.Traversed = []string{"x"} },
	} {
		r := mk()
		fn(&r)
		if r.Validate() == nil {
			t.Errorf("%s: a record that ran nothing was accepted with content", name)
		}
	}
}

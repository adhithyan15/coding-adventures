package recmerge

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// Every binding failure is run-void: nothing is emitted and the report names it.
func wantVoid(t *testing.T, name string, r Result, cause string) {
	t.Helper()
	if r.Emitted || r.RunVoidCount == 0 {
		t.Errorf("%s: the run was not voided", name)
	}
	if len(r.Manifests) != 0 || len(r.Passes) != 0 || len(r.emittedActions()) != 0 {
		t.Errorf("%s: something was emitted", name)
	}
	if !r.cause(cause) {
		t.Errorf("%s: no cause mentions %q: %v", name, cause, r.RunVoid)
	}
}

func TestBindingRefusals(t *testing.T) {
	cases := []struct {
		name  string
		cause string
		mut   func(w *world)
	}{
		{"missing shard", "shard 2 is missing", func(w *world) { w.shards = w.shards[:2] }},
		{"duplicate index", "more than once", func(w *world) { w.shards = append(w.shards, w.shards[1]) }},
		{"extra shard", "outside the plan", func(w *world) {
			c := *w.shards[0]
			c.ArtifactIndex = 7
			w.shards = append(w.shards, &c)
		}},
		{"nil shard", "nil shard", func(w *world) { w.shards = append(w.shards, nil) }},
		{"swapped directories", "B2", func(w *world) { w.shards[0].ArtifactIndex, w.shards[1].ArtifactIndex = 1, 0 }},
		{"header index", "header says", func(w *world) { w.shards[1].Header.ShardIndex = 0 }},
		{"binding index", "binding says", func(w *world) { w.shards[1].Binding.ShardIndex = 2 }},
		{"binding shard_count", "shard_count", func(w *world) { w.shards[1].Binding.ShardCount = 4 }},
		{"header shard_count", "shard_count", func(w *world) { w.shards[1].Header.ShardCount = 4 }},
		{"header binding id", "another binding", func(w *world) { w.shards[1].Header.BindingID = dSteps }},
		{"another repository", "another repository", func(w *world) { w.shards[1].Binding.Repository = "evil/fork" }},
		{"replayed from another run", "another run", func(w *world) { w.shards[1].Binding.Run = "41" }},
		{"another revision", "another revision", func(w *world) { w.shards[1].Binding.Revision = strings.Repeat("c", 40) }},
		{"later attempt", "later attempt", func(w *world) { w.shards[1].Binding.Attempt = 3 }},
		{"other recorder", "recorder", func(w *world) { w.shards[1].Binding.Recorder = legacy.RecorderVersion + 1 }},
		{"stale graph", "graph", func(w *world) { w.shards[1].Binding.GraphSHA256 = dSteps }},
		{"stale steps", "steps differ", func(w *world) { w.shards[1].Binding.StepsSHA256 = dGraph }},
		{"wrong partition", "partition", func(w *world) { w.shards[1].Binding.PartitionSHA256 = dGraph }},
		{"host facts not the bound file", "host-facts", func(w *world) { w.shards[1].HostFactsSHA256 = dGraph }},
		{"inventory not the bound file", "inventory.json", func(w *world) { w.shards[1].InventorySHA256 = dGraph }},
		{"malformed binding", "binding shard 1", func(w *world) { w.shards[1].Binding.Repository = "" }},
		{"malformed facts", "host facts", func(w *world) { w.shards[1].Facts.Root = "" }},
		{"malformed inventory", "inventory", func(w *world) { w.shards[1].Inventory.Paths = []string{"b", "a"} }},
		{"malformed header", "shard header", func(w *world) { w.shards[1].Header.Schema = "x" }},
		{"a shard claiming an action outside its plan", "does not run", func(w *world) {
			r := *w.rec(2, "pkg/d")
			r.Action, r.Record.Package = "pkg/zzz", "pkg/zzz"
			w.shards[2].Records = append(w.shards[2].Records, r)
		}},
		{"a missing action", "no record for pkg/c", func(w *world) { w.shards[1].Records = w.shards[1].Records[:1] }},
		{"a duplicate record", "two records", func(w *world) { w.shards[1].Records = append(w.shards[1].Records, w.shards[1].Records[0]) }},
		{"a record line of another binding", "another binding", func(w *world) { w.rec(1, "pkg/c").BindingID = dGraph }},
		{"a malformed record", "B4", func(w *world) { w.rec(1, "pkg/c").Record.Writes = sl("/abs") }},
		{"weak key differs", "weak key", func(w *world) { w.rec(1, "pkg/c").Weak = dGraph }},
		{"shard keyed from other facts", "weak key", func(w *world) { w.shards[1].Facts.Image = "another-image" }},
		{"environment value differs", "weak key", func(w *world) { w.shards[1].Facts.Env = []string{"LANG=C.UTF-8", "SECRET_ISH=1"} }},
		{"merge refuses what the shard keyed", "refuses", func(w *world) { w.keys.refuse = map[string]string{"pkg/c": "host-unknown"} }},
		{"shard refused what the merge keys", "refused", func(w *world) {
			w.rec(1, "pkg/c").Weak, w.rec(1, "pkg/c").Refusal = "", "host-unknown"
		}},
		{"refusal reasons differ", "refused", func(w *world) {
			w.keys.refuse = map[string]string{"pkg/c": "unknown-toolchain:x"}
			w.rec(1, "pkg/c").Weak, w.rec(1, "pkg/c").Refusal = "", "host-unknown"
		}},
		{"the key source fails", "keys:", func(w *world) { w.keys.weakErr = errString("boom") }},
		{"a recipe the merge does not run", "M1", func(w *world) { w.rec(1, "pkg/c").StepsSHA = dGraph }},
		{"a plan with no shards", "no shards", func(w *world) { w.plan = nil }},
		{"a plan with a gap", "shard indexes", func(w *world) { w.plan[1].Index = 5 }},
		{"a plan assigning a package it does not run", "does not run", func(w *world) { w.plan[0].Assigned = append(w.plan[0].Assigned, "pkg/zz") }},
		{"a plan assigning a package twice", "assigned to shards", func(w *world) { w.plan[1].Assigned = append(w.plan[1].Assigned, "pkg/a") }},
		{"a plan with an unassigned package", "assigned to none", func(w *world) { w.plan[2].Packages = append(w.plan[2].Packages, "pkg/x") }},
		{"a plan with a bad package", "invalidly or twice", func(w *world) { w.plan[2].Packages = append(w.plan[2].Packages, "/x") }},
		{"a plan with a repeated package", "invalidly or twice", func(w *world) { w.plan[2].Packages = append(w.plan[2].Packages, "pkg/d") }},
	}
	for _, c := range cases {
		w := std(t)
		c.mut(w)
		wantVoid(t, c.name, w.merge(), c.cause)
	}
	// And with no key source at all.
	in := std(t).input()
	in.Keys = nil
	wantVoid(t, "nil keys", Merge(in), "no key source")
	// The binding's attempt may be earlier than or equal to the merge's.
	w := std(t)
	w.shards[0].Binding.Attempt = 2
	if r := w.merge(); !r.Emitted {
		t.Errorf("an equal attempt was refused: %v", r.RunVoid)
	}
	// Image ids may differ between shards (an image rollout).
	if r := std(t).merge(); !r.Emitted {
		t.Error("per-shard image ids must be allowed")
	}
}

// A new nested package absent from a stale graph: the regenerated graph digest
// differs from the one the shard bound (D10).
func TestStaleGraphWithANewNestedPackage(t *testing.T) {
	w := std(t)
	in := w.input()
	in.Expected.GraphSHA256 = dSteps // the merge regenerated a graph that has the new package
	wantVoid(t, "stale graph", Merge(in), "graph")
}

func TestRecipeMismatchOnABuiltAction(t *testing.T) {
	// Until PR 4 the mismatch is an unknown write set: run-void.
	w := std(t)
	w.rec(1, "pkg/c").StepsSHA = recfmt.StepsDigest([]string{"echo rewritten"})
	r := w.merge()
	wantVoid(t, "mismatch", r, "M1")
	if !r.out(t, "pkg/c").has("trace-incomplete:recipe-mismatch") {
		t.Error("the reason is missing")
	}
	// L-1: not even writes_complete=true and the PR 4 switch repair a recipe
	// mismatch.  The steps that ran are not the steps the weak key covers, so
	// the trace says nothing about what the recipe writes.
	w = std(t)
	w.honour = true
	yes := true
	w.rec(1, "pkg/c").StepsSHA = recfmt.StepsDigest([]string{"echo rewritten"})
	w.rec(1, "pkg/c").Complete = &yes
	wantVoid(t, "PoC5 mismatch with honour", w.merge(), "recipe-mismatch")
	if w.merge().relaxedOK() {
		t.Error("a recipe mismatch must void the run in the relaxed verdict too")
	}
	// The classifier's own recipe-mismatch reason is the same.
	w = std(t)
	w.honour = true
	w.rec(2, "pkg/d").Record.Reasons = sl("trace-incomplete:recipe-mismatch")
	w.rec(2, "pkg/d").Complete = &yes
	wantVoid(t, "classifier recipe-mismatch", w.merge(), "recipe-mismatch")
	// A non-built action's steps are not compared.
	w = std(t)
	w.rec(1, "pkg/c").Status = "failed"
	w.rec(1, "pkg/c").StepsSHA = dGraph
	if r := w.merge(); !r.relaxedOK() {
		t.Errorf("%v", r.RelaxedRunVoid)
	}
	// A package the regenerated steps do not know is a mismatch too.
	w = std(t)
	delete(w.steps, "pkg/d")
	wantVoid(t, "unknown package", w.merge(), "M1")
}

func TestRunVoidCausesAreBoundedAndSorted(t *testing.T) {
	w := std(t)
	for i := range w.shards {
		w.shards[i].Binding.Run = "1"
	}
	w.shards = w.shards[:1]
	r := w.merge()
	if r.RunVoidCount < 3 {
		t.Fatal(r.RunVoidCount)
	}
	for i := 1; i < len(r.RunVoid); i++ {
		if r.RunVoid[i-1] >= r.RunVoid[i] {
			t.Error("causes are not sorted")
		}
	}
	// Many causes are cut to MaxCauses, with the true count kept.
	w = std(t)
	var ghosts []string
	for i := 0; i < 200; i++ {
		ghosts = append(ghosts, "ghost/"+strings.Repeat("g", i+1))
	}
	w.rec(0, "pkg/b").Record.UntrackedObservations = sl(ghosts...)
	r = w.merge()
	if r.RunVoidCount < 100 || len(r.RunVoid) != MaxCauses {
		t.Errorf("%d causes, %d listed", r.RunVoidCount, len(r.RunVoid))
	}
}

// ---------------------------------------------------------------------------
// Determinism.

func TestOutputIsIndependentOfInputOrder(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/b").Record.Writes = sl("out/x")
	w.rec(0, "pkg/a").Record.UntrackedObservations = sl("out/x")
	w.inv(0, "out")
	w.rec(1, "pkg/c").Record.Binds = sl("stream/1.1.1.1:1")
	w.rec(2, "pkg/d").Record.Binds = sl("stream/1.1.1.1:1")
	w.rec(2, "pkg/d").Record.TrackedWrites = sl("cfg/x")
	want := w.merge()
	if want.out(t, "pkg/a").RelaxedEmittable || len(want.relaxedActions()) != 1 {
		t.Fatalf("a poor fixture: %+v", want.Outcomes)
	}
	for _, p := range [][]int{{0, 1, 2}, {2, 1, 0}, {1, 0, 2}, {2, 0, 1}} {
		w2 := *w
		w2.shards = nil
		for _, i := range p {
			sc := *w.shards[i]
			sc.Records = append([]recfmt.ShardRecord(nil), w.shards[i].Records...)
			// Reverse the record order inside the shard as well.
			for a, b := 0, len(sc.Records)-1; a < b; a, b = a+1, b-1 {
				sc.Records[a], sc.Records[b] = sc.Records[b], sc.Records[a]
			}
			w2.shards = append(w2.shards, &sc)
		}
		// And a reversed plan.
		w2.plan = append([]recfmt.ShardPlan(nil), w.plan...)
		for a, b := 0, len(w2.plan)-1; a < b; a, b = a+1, b-1 {
			w2.plan[a], w2.plan[b] = w2.plan[b], w2.plan[a]
		}
		got := w2.merge()
		if !equalResult(want, got) {
			t.Fatalf("order %v changed the result:\n%+v\n%+v", p, want, got)
		}
	}
}

func equalResult(a, b Result) bool {
	if a.Emitted != b.Emitted || a.RunVoidCount != b.RunVoidCount || strings.Join(a.RunVoid, "\n") != strings.Join(b.RunVoid, "\n") ||
		len(a.Outcomes) != len(b.Outcomes) || len(a.Manifests) != len(b.Manifests) || len(a.ShardVoid) != len(b.ShardVoid) {
		return false
	}
	if a.RelaxedRunVoidCount != b.RelaxedRunVoidCount || strings.Join(a.RelaxedRunVoid, "\n") != strings.Join(b.RelaxedRunVoid, "\n") {
		return false
	}
	for i := range a.Outcomes {
		x, y := a.Outcomes[i], b.Outcomes[i]
		if x.Action != y.Action || x.Assigned != y.Assigned || x.Recordable != y.Recordable || x.Emitted != y.Emitted ||
			x.ShardVoid != y.ShardVoid || x.Truncated != y.Truncated || x.OrderDependent != y.OrderDependent ||
			x.RelaxedEmittable != y.RelaxedEmittable || strings.Join(x.Reasons, "\n") != strings.Join(y.Reasons, "\n") {
			return false
		}
	}
	for i := range a.Manifests {
		if a.Manifests[i].ID() != b.Manifests[i].ID() || a.Manifests[i].Weak != b.Manifests[i].Weak ||
			a.Passes[i].Key != b.Passes[i].Key || !a.Manifests[i].At.Equal(b.Manifests[i].At) {
			return false
		}
	}
	return true
}

// ---------------------------------------------------------------------------
// "Reviewer will try".

func TestReviewerPathAndCaseVariants(t *testing.T) {
	// Spelling variants never reach the merge: the decoder refuses them (see
	// recfmt), and Merge treats a record that fails validation as a B4 void.
	for _, bad := range []string{"Out/x/", "out//x", "out/./x", "/out/x", "", "."} {
		w := std(t)
		w.rec(0, "pkg/b").Record.Writes = []string{bad}
		wantVoid(t, "write "+bad, w.merge(), "B4")
	}
	// Case is significant: Out/x and out/x are different paths.
	w := std(t)
	w.rec(0, "pkg/b").Record.Writes = sl("Out/x")
	w.rec(0, "pkg/a").Record.AbsentObservations = sl("out/x")
	if r := w.merge(); !r.out(t, "pkg/a").Emitted {
		t.Error("case-distinct names collided")
	}
}

func TestReviewerHeaderVoidIsNotIgnored(t *testing.T) {
	w := std(t)
	w.shards[2].Header.Void = []string{"tracked-changed"}
	if r := w.merge(); r.out(t, "pkg/d").Emitted {
		t.Error("a header void was ignored")
	}
}

func TestReviewerEmptyListsWithUnknownWrites(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/b").Unknown = "parse:unknown-syscall"
	wantVoid(t, "empty lists", w.merge(), "M1")
}

func TestPartlyRunStepsAreUnknown(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/b").StepsTotal = 3
	w.rec(0, "pkg/b").StepsRun, w.rec(0, "pkg/b").Steps = 2, []recfmt.Step{{Clean: true}, {Clean: true}}
	wantVoid(t, "steps", w.merge(), "steps-missing")
}

func TestTheReportListsEveryActionEvenWhenVoid(t *testing.T) {
	w := std(t)
	w.shards = w.shards[:1]
	r := w.merge()
	if r.Emitted || len(r.Outcomes) != 2 {
		t.Fatalf("%+v", r.Outcomes)
	}
	for _, o := range r.Outcomes {
		if o.Emitted || o.Reasons == nil {
			t.Errorf("%+v", o)
		}
	}
}

package recmerge

// Regression tests for the D13 adversarial review of stage 4 PR 1.  Each test
// is the review's proof-of-concept turned into an assertion of the FIXED
// behaviour; the names say which finding they close (M-1..M-3, L-1..L-7).

import (
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// M-1 (PoC1): a writer that FAILED after running every step is a truncated
// execution: the build on main, which does not fail here, would have run
// further.  Another action's absent probe could have been wrong.
func TestM1FailedWriterWithAllStepsRunVoidsTheRun(t *testing.T) {
	w := std(t)
	w.rec(2, "pkg/d").Status = "failed"
	w.rec(2, "pkg/d").Record.Reasons = sl("failed")
	w.rec(0, "pkg/b").Record.AbsentObservations = sl("pkg/d/out")
	r := w.merge()
	wantVoid(t, "failed writer", r, "status failed")
	if !r.out(t, "pkg/d").Truncated {
		t.Error("the action is not marked truncated")
	}
	// The relaxed verdict charges it to the action alone.
	if !r.relaxedOK() || r.out(t, "pkg/d").RelaxedEmittable || !r.out(t, "pkg/b").RelaxedEmittable {
		t.Errorf("relaxed: %v %+v", r.RelaxedRunVoid, r.Outcomes)
	}
}

func TestM1PartlyRunWriterVoidsTheRun(t *testing.T) {
	w := std(t)
	w.rec(2, "pkg/d").Status = "failed"
	w.rec(2, "pkg/d").StepsTotal = 2
	w.rec(0, "pkg/b").Record.AbsentObservations = sl("pkg/d/out")
	wantVoid(t, "partly run", w.merge(), "status failed")
	// Even a "built" action with steps missing.
	w = std(t)
	w.rec(2, "pkg/d").StepsTotal = 2
	wantVoid(t, "steps missing", w.merge(), "steps-missing")
}

// M-1 (PoC2): a process left running after a step may still write.
func TestM1DaemonLeftIsATruncatedExecution(t *testing.T) {
	w := std(t)
	w.rec(2, "pkg/d").Record.Reasons = sl("daemon-left")
	w.rec(0, "pkg/b").Record.AbsentObservations = sl("pkg/d/out")
	w.rec(0, "pkg/b").Record.Binds = sl("stream/127.0.0.1:8080")
	r := w.merge()
	wantVoid(t, "daemon-left", r, "daemon-left")
	if !r.out(t, "pkg/d").Truncated {
		t.Error("not marked truncated")
	}
	// The kinds are decided explicitly: daemon-left is a daemon for M13 too
	// (hasKind(..., "daemon") alone silently missed it), so a peer that bound a
	// port is order-dependent in both verdicts.
	if !r.out(t, "pkg/b").has("reads-product:daemon-peer") {
		t.Errorf("a peer of a daemon-left action was not flagged: %+v", r.out(t, "pkg/b"))
	}
	if !isDaemonKind("daemon") || !isDaemonKind("daemon-left") || isDaemonKind("daemon-x") || isDaemonKind("network") {
		t.Error("isDaemonKind")
	}
}

// M-2 (PoC3): a tracked file changed that no record's tracked_writes covers is
// a write no trace saw.
func TestM2UnexplainedTrackedChangeVoidsTheRun(t *testing.T) {
	w := std(t)
	w.shards[2].Header.Void = sl("tracked-changed")
	w.shards[2].Header.TrackedChangedUnexplained = 1
	w.entries(0, "pkg/b", "path:shared/config.json")
	r := w.merge()
	wantVoid(t, "tracked-changed unexplained", r, "M2b")
	if r.relaxedOK() {
		t.Error("M-2 is not relaxed")
	}
	// The count without the token is just as void.
	w = std(t)
	w.shards[1].Header.TrackedChangedUnexplained = 2
	wantVoid(t, "count only", w.merge(), "M2b")
	// A token whose changes ARE all explained is the shard-level void (M4).
	w = std(t)
	w.shards[0].Header.Void = sl("tracked-changed")
	r = w.merge()
	if !r.Emitted || len(r.ShardVoid[0]) != 1 {
		t.Errorf("%v %v", r.RunVoid, r.ShardVoid)
	}
}

// M-3 (PoC4): second-order order dependence.  c probed pkg/d/out absent while
// d writes it, so c's own writes are not the ones it would make in the other
// order; every cross-action rule quantifies over write sets.
func TestM3OrderDependentActionVoidsTheRun(t *testing.T) {
	w := std(t)
	w.rec(2, "pkg/d").Record.Writes = sl("pkg/d/out")
	w.rec(1, "pkg/c").Record.AbsentObservations = sl("pkg/d/out")
	w.entries(0, "pkg/b", "list:pkg/b")
	r := w.merge()
	wantVoid(t, "second order", r, "M3 order-dependent: pkg/c")
	if !r.out(t, "pkg/c").OrderDependent || r.out(t, "pkg/b").OrderDependent {
		t.Errorf("%+v", r.Outcomes)
	}
	// The relaxed verdict loses c only.
	if !r.relaxedOK() {
		t.Fatalf("%v", r.RelaxedRunVoid)
	}
	wantRelaxed(t, r, "pkg/a", "pkg/b", "pkg/d")
}

// Every reason the merge derives from ANOTHER action's records is an
// order-dependence reason; the classifier's own reasons are not.
func TestM3ListedReasonClasses(t *testing.T) {
	setups := map[string]func(w *world){
		"M5 untracked": func(w *world) { w.inv(0, "x"); w.rec(0, "pkg/b").Record.UntrackedObservations = sl("x") },
		"M6 absent": func(w *world) {
			w.rec(2, "pkg/d").Record.Writes = sl("x")
			w.rec(0, "pkg/b").Record.AbsentObservations = sl("x")
		},
		"M7 listing": func(w *world) { w.rec(2, "pkg/d").Record.Writes = sl("x/y"); w.entries(0, "pkg/b", "list:x") },
		"M8 traversed": func(w *world) {
			w.rec(2, "pkg/d").Record.Writes = sl("x")
			w.rec(0, "pkg/b").Record.Traversed = sl("x")
		},
		"M9 inventory": func(w *world) { w.inv(0, "x"); w.rec(0, "pkg/b").Record.Traversed = sl("x") },
		"M10 overlap":  func(w *world) { w.rec(2, "pkg/d").Record.Writes = sl("x"); w.rec(0, "pkg/b").Record.Writes = sl("x") },
		"M11 tainted":  func(w *world) { w.rec(2, "pkg/d").Record.TrackedWrites = sl("x"); w.entries(0, "pkg/b", "path:x") },
		"M12 metadata": func(w *world) { w.rec(2, "pkg/d").Record.TrackedWrites = sl("pkg/b/BUILD") },
		"M14 port": func(w *world) {
			w.rec(2, "pkg/d").Record.Binds = sl("stream/1.1.1.1:1")
			w.rec(0, "pkg/b").Record.Binds = sl("stream/1.1.1.1:1")
		},
		"M13 daemon": func(w *world) {
			w.rec(2, "pkg/d").Record.Reasons = sl("daemon:x")
			w.rec(0, "pkg/b").Record.Binds = sl("stream/1.1.1.1:1")
		},
		"own reads-prd": func(w *world) { w.rec(0, "pkg/b").Record.Reasons = sl("reads-product:q") },
	}
	for name, setup := range setups {
		w := std(t)
		setup(w)
		r := w.merge()
		if !r.out(t, "pkg/b").OrderDependent || r.Emitted {
			t.Errorf("%s: %+v void=%v", name, r.out(t, "pkg/b"), r.RunVoid)
		}
	}
	// Classifier reasons that do not mean "depends on another action" do not
	// make an action order-dependent (they still cost the action, M15).
	w := std(t)
	w.rec(0, "pkg/b").Record.Reasons = sl("network:connect", "host-read:/etc/x")
	r := w.merge()
	if r.out(t, "pkg/b").OrderDependent || !r.Emitted || r.out(t, "pkg/b").Emitted {
		t.Errorf("%+v %v", r.out(t, "pkg/b"), r.RunVoid)
	}
}

// The relaxed verdict is report-only: it can say an action WOULD be emitted,
// but no manifest, pass or Emitted flag ever comes from it.
func TestRelaxedVerdictNeverEmits(t *testing.T) {
	w := std(t)
	w.rec(2, "pkg/d").Record.Writes = sl("out")
	w.rec(1, "pkg/c").Record.AbsentObservations = sl("out") // c is order-dependent
	w.rec(1, "pkg/a").Status = "failed"                     // and a copy of a is truncated
	r := w.merge()
	if r.Emitted || len(r.Manifests) != 0 || len(r.Passes) != 0 || len(r.emittedActions()) != 0 {
		t.Fatal("the strict verdict must emit nothing here")
	}
	if !r.relaxedOK() || len(r.relaxedActions()) == 0 {
		t.Fatalf("the relaxed verdict should still see yield: %v", r.RelaxedRunVoid)
	}
	// And identical to a run where the relaxed fields are ignored: the strict
	// output does not depend on them.
	in := w.input()
	if r2 := Merge(in); len(r2.Manifests) != 0 {
		t.Fatal("manifests appeared")
	}
	// A clean world: relaxed and strict agree.
	c := std(t).merge()
	if len(c.relaxedActions()) != 4 || len(c.emittedActions()) != 4 || len(c.Manifests) != 4 {
		t.Errorf("%v %v", c.relaxedActions(), c.emittedActions())
	}
	// Hard causes void the relaxed verdict too.
	w = std(t)
	w.rec(2, "pkg/d").Unknown = "tracer:signal"
	if r := w.merge(); r.relaxedOK() || len(r.relaxedActions()) != 0 {
		t.Error("an unknown tracer is not relaxed")
	}
	// Missing key source: nothing can be emitted in either verdict.
	in = std(t).input()
	in.Keys = nil
	if r := Merge(in); len(r.relaxedActions()) != 0 || len(r.emittedActions()) != 0 {
		t.Error("emission without keys")
	}
}

// L-2 (PoC6): any clean:false step is an unknown write set.
func TestL2UncleanStepVoidsBothVerdicts(t *testing.T) {
	w := std(t)
	w.rec(2, "pkg/d").Steps = []recfmt.Step{{Clean: false}}
	r := w.merge()
	wantVoid(t, "unclean", r, "did not finish cleanly")
	if r.relaxedOK() {
		t.Error("not relaxed")
	}
	// And the step list must match steps_run (checked by recfmt, so by B4).
	w = std(t)
	w.rec(2, "pkg/d").Steps = []recfmt.Step{{Clean: false}, {Clean: false}, {Clean: false}}
	wantVoid(t, "three summaries for one step", w.merge(), "B4")
	w = std(t)
	w.rec(2, "pkg/d").Steps = nil
	wantVoid(t, "no summaries", w.merge(), "B4")
}

// L-3 (PoC11): a crafted record cannot make the merge quadratic.  The review's
// shape (1,500 nested probes over 60,000 writes, 4,500 byte paths) is refused
// at validation (depth and length caps), and nothing after it runs.
func TestL3CraftedRecordIsRefusedQuickly(t *testing.T) {
	w := std(t)
	const depth, leaves = 1500, 60000
	var chain, ws []string
	p := "z"
	for i := 0; i < depth; i++ {
		chain = append(chain, p)
		p += "/q"
	}
	for i := 0; i < leaves; i++ {
		ws = append(ws, p+"/"+strings.Repeat("0", 5-len(itoa(i)))+itoa(i))
	}
	w.rec(2, "pkg/d").Record.Writes = sl(ws...)
	w.rec(2, "pkg/d").Record.UntrackedObservations = sl(chain...)
	w.rec(2, "pkg/d").Record.Reasons = sl("daemon:x")
	w.inv(2, "z")
	start := time.Now()
	r := w.merge()
	took := time.Since(start)
	wantVoid(t, "quadratic", r, "B4")
	if took > time.Second {
		t.Errorf("took %v", took)
	}
}

// The same attack within the caps (depth 100, 20,000 writes) is bounded by the
// work budget instead: run-void "size".
func TestL3WorkBudgetStopsAWithinCapsQuadraticRecord(t *testing.T) {
	w := std(t)
	var chain, ws []string
	p := "z"
	for i := 0; i < 100; i++ {
		chain = append(chain, p)
		p += "/q"
	}
	for i := 0; i < 20000; i++ {
		ws = append(ws, p+"/"+strings.Repeat("0", 5-len(itoa(i)))+itoa(i))
	}
	w.rec(2, "pkg/d").Record.Writes = sl(ws...)
	w.rec(2, "pkg/d").Record.UntrackedObservations = sl(chain...)
	w.inv(2, "z")
	in := w.input()
	in.WorkBudget = 1_000_000
	start := time.Now()
	r := Merge(in)
	took := time.Since(start)
	wantVoid(t, "within caps", r, "work budget")
	if took > time.Second {
		t.Errorf("took %v", took)
	}
}

func itoa(i int) string {
	s := ""
	for ; i > 0; i /= 10 {
		s = string(rune('0'+i%10)) + s
	}
	if s == "" {
		return "0"
	}
	return s
}

func TestL3SmallBudgetAndRetainedCap(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/b").Record.AbsentObservations = sl("a/b/c/d", "a/b/c/e")
	in := w.input()
	in.WorkBudget = 3
	wantVoid(t, "tiny budget", Merge(in), "work budget")
	in = w.input()
	in.MaxRetainedBytes = 10
	r := Merge(in)
	wantVoid(t, "retained cap", r, "retained-text cap")
	if len(r.Outcomes) != 0 {
		t.Error("nothing should be analysed past the cap")
	}
	// A cap that is large enough changes nothing.
	in = w.input()
	in.MaxRetainedBytes = 1 << 20
	in.WorkBudget = 1 << 20
	if r := Merge(in); !r.Emitted {
		t.Errorf("%v", r.RunVoid)
	}
}

// L-4 (PoC7): cause text is deterministic.
func TestL4CauseTextIsDeterministic(t *testing.T) {
	seen := map[string]bool{}
	for i := 0; i < 200; i++ {
		w := std(t)
		b := &w.shards[0].Binding
		// Several malformed digests at once: which one is REPORTED must not
		// depend on map iteration order.
		b.KilnSHA256, b.BuildToolSHA256, b.SandboxConfigSHA256, b.GraphSHA256 = "x", "x", "x", "x"
		w.shards[0].Binding.Kernel, w.shards[0].Binding.StraceVersion = "a\nb", "c\nd"
		seen[strings.Join(w.merge().RunVoid, "|")] = true
	}
	if len(seen) != 1 {
		t.Errorf("%d distinct renderings for one input", len(seen))
	}
	// And the record validator's first complaint is stable.
	seen = map[string]bool{}
	for i := 0; i < 200; i++ {
		r := recfmt.ShardRecord{BindingID: "x", Action: "a", Status: "built", Language: "g\n", Refusal: "r\n", Gate: "g\n", Unknown: "u\n"}
		seen[r.Validate().Error()] = true
	}
	if len(seen) != 1 {
		t.Errorf("record validation is not deterministic: %v", seen)
	}
	seen = map[string]bool{}
	for i := 0; i < 200; i++ {
		r := recfmt.ShardRecord{BindingID: strings.Repeat("1", 64), Action: "a", Status: "built", Language: "g\n", Refusal: "r\n", Gate: "g\n", Unknown: "u\n"}
		r.Record.Package = "a"
		seen[r.Validate().Error()] = true
	}
	if len(seen) != 1 {
		t.Errorf("record text validation is not deterministic: %v", seen)
	}
}

// L-5 is a wording fix: M13 only claims the untracked-overlap and Binds match.
// A daemon whose server was reached through an unlisted bind (port 0, an
// abstract socket) with nothing else in common voids nobody; under per-step
// network namespaces (G10) no other action could have reached it.
func TestL5DaemonWithoutListedBindsHasNoPeers(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/b").Record.Reasons = sl("daemon:@abstract")
	r := w.merge()
	for _, o := range r.Outcomes {
		if o.Action != "pkg/b" && (o.hasKind("reads-product") || !o.RelaxedEmittable) {
			t.Errorf("%+v", o)
		}
	}
}

// L-6 (PoC9): a listing is changed by a write root that is the directory, is
// beneath it or contains it.
func TestL6ListingOverlapsAnyWriteRoot(t *testing.T) {
	for name, c := range map[string]struct {
		d, w string
		hit  bool
	}{
		"equal":      {"shared", "shared", true},
		"beneath":    {"shared", "shared/x", true},
		"above":      {"shared/sub", "shared", true},
		"sibling":    {"shared/sub", "shared/other", false},
		"prefix":     {"shared", "sharedx", false},
		"root":       {".", "anything", true},
		"root equal": {".", "a/b", true},
	} {
		w := std(t)
		w.entries(0, "pkg/b", "list:"+c.d)
		w.rec(2, "pkg/d").Record.Writes = sl(c.w)
		got := w.merge().out(t, "pkg/b")
		if got.hasKind("reads-product") != c.hit {
			t.Errorf("%s: %+v", name, got)
		}
	}
	// The tainted check likewise covers ancestors of the listed directory.
	w := std(t)
	w.entries(0, "pkg/b", "list:cfg/sub/deep")
	w.rec(2, "pkg/d").Record.TrackedWrites = sl("cfg/sub")
	if got := w.merge().out(t, "pkg/b"); !got.has("reads-product:cfg/sub") {
		t.Errorf("%+v", got)
	}
}

// INFO (PoC8): kiln_sha256, build_tool_sha256 and sandbox_config_sha256 are
// self-attested, so comparing them across shards proves nothing.  They are
// recorded for the report only; the guards are recorder == RecorderVersion
// plus B5 and B6.
func TestInfoBinaryDigestsAreReportOnly(t *testing.T) {
	w := std(t)
	b := &w.shards[1].Binding
	b.KilnSHA256 = strings.Repeat("9", 64)
	b.BuildToolSHA256 = strings.Repeat("8", 64)
	b.SandboxConfigSHA256 = strings.Repeat("7", 64)
	b.StraceVersion = "strace -- version 7.0"
	b.Kernel = "another kernel"
	r := w.merge()
	if !r.Emitted || len(r.emittedActions()) != 4 {
		t.Errorf("%v", r.RunVoid)
	}
}

// L-1 is in TestRecipeMismatchOnABuiltAction; L-7 is in the recfmt tests.

// Round 2, LOW-1: the work meter charges the owner lists a walk touches, so
// many actions sharing many write paths cannot dodge the budget.
func TestR2OwnerListWorkIsMetered(t *testing.T) {
	x := newPathIndex(&meter{max: 1 << 40})
	for id := 0; id < 400; id++ {
		x.add("z/y", id)
		x.add("z/y/x", id)
	}
	x.freeze()
	if n := spent(x, func() { x.ancestors("z/y/x", func(string, []int) bool { return true }) }); n < 800 {
		t.Errorf("ancestors charged %d for 2 nodes of 400 owners", n)
	}
	if n := spent(x, func() { x.descendants("z", true, func(string, []int) bool { return true }) }); n < 800 {
		t.Errorf("descendants charged %d for 2 nodes of 400 owners", n)
	}
	// The descendant comparison also costs the length of the query.
	long := strings.Repeat("q", 3200)
	y := newPathIndex(&meter{max: 1 << 40})
	y.add(long+"/a", 0)
	y.freeze()
	if n := spent(y, func() { y.descendants(long, true, func(string, []int) bool { return true }) }); n < 100 {
		t.Errorf("descendants ignored the query length: %d", n)
	}
}

func spent(x *pathIndex, walk func()) int64 { x.mt.spent = 0; walk(); return x.mt.spent }

// The review's world (400 actions x 2,000 shared writes, a daemon, three
// untracked observations) ends as run-void "size" under a budget the real
// work exceeds, whatever the clock says.
func TestR2ManyOwnersDoSIsBounded(t *testing.T) {
	var names []string
	for i := 0; i < 400; i++ {
		names = append(names, "pkg/p"+strings.Repeat("0", 4-len(itoa(i)))+itoa(i))
	}
	w := newWorld(t, shardSpec{names, names})
	var ws []string
	for i := 0; i < 2000; i++ {
		ws = append(ws, "z/y/x/"+strings.Repeat("0", 5-len(itoa(i)))+itoa(i))
	}
	ws = sl(ws...)
	for i := range w.shards[0].Records {
		r := &w.shards[0].Records[i]
		r.Record.Writes = ws
		r.Record.UntrackedObservations = sl("z", "z/y", "z/y/x")
		r.Record.Reasons = sl("daemon:x")
	}
	w.inv(0, "z")
	in := w.input()
	in.WorkBudget = 50_000_000
	start := time.Now()
	r := Merge(in)
	wantVoid(t, "owners", r, "work budget")
	if r.Emitted || len(r.Manifests) != 0 {
		t.Error("emitted")
	}
	if d := time.Since(start); d > 5*time.Second {
		t.Errorf("took %v", d)
	}
}

// Round 2, LOW-2: the M-3 allow-list.
func TestR2OwnReasonAllowList(t *testing.T) {
	intra := []string{"failed", "precondition:x", "network:curl", "host-read:/var/x", "host-read:setup-modified:/x",
		"state-write:/x", "writes-tracked:a", "vcs-read", "trace-visible:/proc/x", "unscrubbed",
		"trace-incomplete:size", "daemon-left", "manifest-too-large", "manifest-invalid:x"}
	for _, reason := range intra {
		w := std(t)
		w.honour = true
		yes := true
		w.rec(2, "pkg/d").Complete = &yes
		w.rec(2, "pkg/d").Record.Reasons = sl(reason)
		if r := w.merge(); r.out(t, "pkg/d").OrderDependent {
			t.Errorf("%q is intra-action but marked order-dependent", reason)
		}
	}
	for _, reason := range []string{"daemon:127.0.0.1:5432", "daemon:@abs", "daemon", "reads-product:x", "Reads-product:x",
		"reads-product", "unstable-path", "unstable-path:x", "brand-new-kind:x", "Network:x", "FAILED"} {
		w := std(t)
		w.rec(2, "pkg/d").Record.Reasons = sl(reason)
		r := w.merge()
		if !r.out(t, "pkg/d").OrderDependent {
			t.Errorf("%q was not marked order-dependent", reason)
		}
		if r.Emitted || len(r.Manifests) != 0 {
			t.Errorf("%q: emitted", reason)
		}
	}
	for _, k := range []string{"manifest-", "manifest-x"} {
		if !intraActionKind(k) {
			t.Error(k)
		}
	}
	if intraActionKind("manifest") || intraActionKind("") {
		t.Error("manifest kind prefix")
	}
}

// Security review MEDIUM-1: the reason-set trim is amortised.  Counted, not
// timed: a set that holds 60 kinds at 16 reasons each (960 entries) used to be
// re-sorted on every add once it passed 512.
func TestSRReasonTrimIsAmortised(t *testing.T) {
	mt := &meter{max: 1 << 60}
	r := reasons{mt: mt}
	const adds = 20000
	for i := 0; i < adds; i++ {
		r.add("k" + string(rune('A'+i%60)) + ":" + strings.Repeat("x", i%40) + string(rune('a'+i%26)) + itoa(i))
	}
	if r.trims > 40 {
		t.Errorf("%d trims for %d adds: the trim is not amortised", r.trims, adds)
	}
	// Total spend stays near linear: well under the n log n of re-sorting 960
	// entries on every add (about 20000 * 960 * 10).
	if mt.spent > 20_000_000 {
		t.Errorf("spent %d", mt.spent)
	}
	if !r.any || len(r.m) == 0 {
		t.Error("lost the set")
	}
	// A trim is charged.
	before := mt.spent
	small := reasons{mt: mt}
	for i := 0; i < 513; i++ {
		small.add("a:" + itoa(i))
	}
	if small.trims != 1 || mt.spent-before < 513*9 {
		t.Errorf("trims %d, charged %d", small.trims, mt.spent-before)
	}
}

// LOW-1: the cause set keeps the true distinct count and the smallest
// MaxCauses strings, whatever order the causes arrive in.
func TestSRCauseSetIsBoundedAndOrderIndependent(t *testing.T) {
	var all []string
	for i := 0; i < 1000; i++ {
		all = append(all, "cause "+itoa(i)+" "+strings.Repeat("z", i%7))
	}
	a, b := newCauseSet(), newCauseSet()
	for i, c := range all {
		a.add(c)
		a.add(c) // a repeat is not a new cause
		b.add(all[len(all)-1-i])
	}
	if a.count() != 1000 || b.count() != 1000 || len(a.top) != MaxCauses {
		t.Fatalf("%d %d %d", a.count(), b.count(), len(a.top))
	}
	if strings.Join(a.list(), "|") != strings.Join(b.list(), "|") {
		t.Error("the retained causes depend on arrival order")
	}
	for i := 1; i < len(a.top); i++ {
		if a.top[i-1] >= a.top[i] {
			t.Error("not sorted")
		}
	}
	if len(newCauseSet().list()) != 0 {
		t.Error("empty")
	}
}

// INFO: the meter saturates instead of wrapping.
func TestSRMeterSaturates(t *testing.T) {
	m := &meter{max: 100}
	m.spent = 1<<63 - 10
	if m.spend(1 << 40) {
		t.Error("overflow read as within budget")
	}
	if m.spent != 1<<63-1 || !m.over() {
		t.Errorf("%d", m.spent)
	}
	if m.spend(-5) || m.spent != 1<<63-1 {
		t.Error("a negative spend must saturate too")
	}
}

type spyKeys struct {
	fakeKeys
	images []string
}

func (s *spyKeys) Weak(f recfmt.HostFacts, a string) ([32]byte, string, error) {
	s.images = append(s.images, f.Image)
	return s.fakeKeys.Weak(f, a)
}

// LOW-3: a shard whose artifact index is outside the plan is never analysed:
// its (unvalidated) host facts never reach Keys.Weak.
func TestSRUnvalidatedShardNeverReachesKeys(t *testing.T) {
	w := std(t)
	c := *w.shards[0]
	c.ArtifactIndex = 9
	c.Facts.Image = "HOSTILE-IMAGE"
	c.Facts.Root = "" // invalid
	w.shards = append(w.shards, &c)
	in := w.input()
	spy := &spyKeys{}
	in.Keys = spy
	wantVoid(t, "out of range", Merge(in), "outside the plan")
	for _, img := range spy.images {
		if img == "HOSTILE-IMAGE" {
			t.Error("unvalidated facts reached Keys.Weak")
		}
	}
	if len(spy.images) != 0 {
		t.Errorf("Keys was consulted for a malformed run: %v", spy.images)
	}
}

// MEDIUM-2: the retained-text cap counts the host facts too.
func TestSRRetainedCapCountsHostFacts(t *testing.T) {
	w := std(t)
	var env []string
	for i := 0; i < 100; i++ {
		env = append(env, "V"+itoa(i)+"="+strings.Repeat("v", 1000))
	}
	w.shards[0].Facts.Env = env
	in := w.input()
	in.MaxRetainedBytes = 50_000
	wantVoid(t, "env", Merge(in), "retained-text cap")
}

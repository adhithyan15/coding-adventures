package recmerge

// A tiny world builder: shards, a plan, records and fake keys, all in memory.
// Everything it produces is VALID (it passes every B check), so a test breaks
// exactly the one thing it is about.

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"sort"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

const (
	dGraph = "1111111111111111111111111111111111111111111111111111111111111111"
	dSteps = "2222222222222222222222222222222222222222222222222222222222222222"
	rev    = "abcdefabcdefabcdefabcdefabcdefabcdefabcd"
)

// fakeKeys derives keys from the facts and the action, so a different image or
// action gives a different key, and counts nothing else.
type fakeKeys struct {
	refuse    map[string]string
	weakErr   error
	strongErr error
}

func weakOf(f recfmt.HostFacts, action string) [32]byte {
	return sha256.Sum256([]byte("w|" + f.Image + "|" + f.Root + "|" + strings.Join(f.Env, ",") + "|" + action))
}

func (k fakeKeys) Weak(f recfmt.HostFacts, action string) ([32]byte, string, error) {
	if k.weakErr != nil {
		return [32]byte{}, "", k.weakErr
	}
	if r, ok := k.refuse[action]; ok {
		return [32]byte{}, r, nil
	}
	return weakOf(f, action), "", nil
}

func (k fakeKeys) Strong(f recfmt.HostFacts, action string, names []string) ([32]byte, error) {
	if k.strongErr != nil {
		return [32]byte{}, k.strongErr
	}
	w := weakOf(f, action)
	return sha256.Sum256([]byte("s|" + hex.EncodeToString(w[:]) + "|" + strings.Join(names, ","))), nil
}

type shardSpec struct{ run, assigned []string }

type world struct {
	t      *testing.T
	plan   []recfmt.ShardPlan
	shards []*recfmt.Shard
	steps  map[string]string
	keys   fakeKeys
	run    Run
	honour bool
}

// std is the world most tests use:
//
//	shard 0 runs pkg/a pkg/b       assigned pkg/a pkg/b
//	shard 1 runs pkg/a pkg/c       assigned pkg/c          (pkg/a is a prerequisite copy)
//	shard 2 runs pkg/d             assigned pkg/d
func std(t *testing.T) *world {
	return newWorld(t,
		shardSpec{[]string{"pkg/a", "pkg/b"}, []string{"pkg/a", "pkg/b"}},
		shardSpec{[]string{"pkg/a", "pkg/c"}, []string{"pkg/c"}},
		shardSpec{[]string{"pkg/d"}, []string{"pkg/d"}})
}

func newWorld(t *testing.T, specs ...shardSpec) *world {
	t.Helper()
	w := &world{t: t, steps: map[string]string{}, keys: fakeKeys{},
		run: Run{Repository: "owner/repo", ID: "42", Attempt: 2, Revision: rev}}
	for i, s := range specs {
		w.plan = append(w.plan, recfmt.ShardPlan{Index: i, Packages: s.run, Assigned: s.assigned})
	}
	part, err := recfmt.PartitionDigest(w.plan)
	if err != nil {
		t.Fatal(err)
	}
	for i, s := range specs {
		facts := recfmt.HostFacts{Schema: recfmt.SchemaHostFacts, Image: fmt.Sprintf("img-%d", i),
			Root: "/work/r", Home: "/home/r", Tmpdir: "/tmp/kiln-tmp", ExecutorArgs: []string{}, Env: []string{"LANG=C.UTF-8"}}
		inv := recfmt.Inventory{Schema: recfmt.SchemaInventory, Paths: []string{}}
		fb, _ := recfmt.EncodeHostFacts(facts)
		ib, _ := recfmt.EncodeInventory(inv)
		b := recfmt.Binding{Schema: recfmt.SchemaBinding, Repository: "owner/repo", Run: "42", Attempt: 1,
			Revision: rev, ShardIndex: i, ShardCount: len(specs), GraphSHA256: dGraph, StepsSHA256: dSteps,
			PartitionSHA256: part, HostFactsSHA256: sha(fb), InventorySHA256: sha(ib),
			SandboxConfigSHA256: dGraph, KilnSHA256: dGraph, BuildToolSHA256: dGraph, Recorder: legacy.RecorderVersion,
			StraceVersion: "strace", Kernel: "k", JobStart: 1, PreparedAt: "2026-11-13T00:00:00Z"}
		bb, err := recfmt.EncodeBinding(b)
		if err != nil {
			t.Fatal(err)
		}
		sh := &recfmt.Shard{ArtifactIndex: i, Binding: b, BindingID: sha(bb), HostFactsSHA256: sha(fb),
			InventorySHA256: sha(ib), Facts: facts, Inventory: inv,
			Header: recfmt.ShardHeader{Schema: recfmt.SchemaShard, BindingID: sha(bb), ShardIndex: i,
				ShardCount: len(specs), Void: []string{}}}
		for _, pkg := range s.run {
			w.steps[pkg] = recfmt.StepsDigest([]string{"echo " + pkg})
			wk := weakOf(facts, pkg)
			sh.Records = append(sh.Records, recfmt.ShardRecord{
				BindingID: sh.BindingID, Action: pkg, Language: "go", Status: recfmt.StatusBuilt,
				StepsRun: 1, StepsTotal: 1, Steps: []recfmt.Step{{Clean: true}}, StepsSHA: w.steps[pkg],
				Weak:   hex.EncodeToString(wk[:]),
				Record: recfmt.Normalize(recfmt.RawRecord{Package: pkg, Entries: []string{"pkg:" + pkg}})})
		}
		w.shards = append(w.shards, sh)
	}
	return w
}

func sha(b []byte) string { d := sha256.Sum256(b); return hex.EncodeToString(d[:]) }

// sl is a sorted, duplicate-free list.
func sl(v ...string) []string {
	m := map[string]bool{}
	for _, s := range v {
		m[s] = true
	}
	out := make([]string, 0, len(m))
	for s := range m {
		out = append(out, s)
	}
	sort.Strings(out)
	return out
}

func (w *world) input() Input {
	return Input{Run: w.run, Expected: Expected{GraphSHA256: dGraph, StepsSHA256: dSteps, Plan: w.plan, Steps: w.steps},
		Shards: w.shards, Keys: w.keys, Now: time.Date(2026, 11, 13, 1, 2, 3, 999, time.UTC),
		Kiln: "kiln/0.1.0 traced/2 recorder/2", HonourWritesComplete: w.honour}
}

func (w *world) merge() Result { return Merge(w.input()) }

// rec is the copy of action in shard.
func (w *world) rec(shard int, action string) *recfmt.ShardRecord {
	w.t.Helper()
	for i := range w.shards[shard].Records {
		if w.shards[shard].Records[i].Action == action {
			return &w.shards[shard].Records[i]
		}
	}
	w.t.Fatalf("no record %s in shard %d", action, shard)
	return nil
}

// all applies fn to every copy of action.
func (w *world) all(action string, fn func(r *recfmt.ShardRecord)) {
	n := 0
	for _, s := range w.shards {
		for i := range s.Records {
			if s.Records[i].Action == action {
				fn(&s.Records[i])
				n++
			}
		}
	}
	if n == 0 {
		w.t.Fatalf("no copy of %s", action)
	}
}

func (w *world) inv(shard int, paths ...string) { w.shards[shard].Inventory.Paths = sl(paths...) }

func (w *world) entries(shard int, action string, e ...string) {
	r := w.rec(shard, action)
	r.Record.Entries = sl(append([]string{"pkg:" + action}, e...)...)
}

func (r Result) out(t *testing.T, action string) Outcome {
	t.Helper()
	for _, o := range r.Outcomes {
		if o.Action == action {
			return o
		}
	}
	t.Fatalf("no outcome for %s", action)
	return Outcome{}
}

func (o Outcome) has(reason string) bool {
	for _, r := range o.Reasons {
		if r == reason {
			return true
		}
	}
	return false
}

func (o Outcome) hasKind(kind string) bool {
	for _, r := range o.Reasons {
		if recfmt.ReasonKind(r) == kind {
			return true
		}
	}
	return false
}

func (r Result) cause(sub string) bool {
	for _, c := range r.RunVoid {
		if strings.Contains(c, sub) {
			return true
		}
	}
	return false
}

func (r Result) emittedActions() []string {
	var out []string
	for _, o := range r.Outcomes {
		if o.Emitted {
			out = append(out, o.Action)
		}
	}
	return out
}

// wantEmitted checks the exact emitted set.
func wantEmitted(t *testing.T, r Result, actions ...string) {
	t.Helper()
	got := r.emittedActions()
	if strings.Join(got, ",") != strings.Join(sl(actions...), ",") {
		t.Fatalf("emitted %v, want %v (run void: %v; outcomes %+v)", got, sl(actions...), r.RunVoid, r.Outcomes)
	}
}

// relaxedOK: the run is not void in the relaxed (report-only) verdict, where
// truncated executions and order dependence cost only their action.
func (r Result) relaxedOK() bool { return r.RelaxedRunVoidCount == 0 }

func (r Result) relaxedActions() []string {
	var out []string
	for _, o := range r.Outcomes {
		if o.RelaxedEmittable {
			out = append(out, o.Action)
		}
	}
	return out
}

// wantRelaxed checks the exact set of actions the relaxed verdict would emit.
func wantRelaxed(t *testing.T, r Result, actions ...string) {
	t.Helper()
	got := r.relaxedActions()
	if strings.Join(got, ",") != strings.Join(sl(actions...), ",") {
		t.Fatalf("relaxed-emittable %v, want %v (relaxed void: %v; outcomes %+v)", got, sl(actions...), r.RelaxedRunVoid, r.Outcomes)
	}
}

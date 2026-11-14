//go:build unix

package record

// Tests of `kiln record collect`, part 1: the clean run, the merge's acceptance of
// it, and every refusal (KILN04 section 4.8, stage 4 PR 2b-2b-ii). The per-action
// verdict table is collect_verdict_unix_test.go and the post-build checks are
// collect_post_unix_test.go.

import (
	"encoding/hex"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recmerge"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

func TestCollectCleanShard(t *testing.T) {
	f := newCfix(t)
	f.good()
	res, sh := f.collect()
	if res.Records != 3 || len(sh.Records) != 3 || len(res.Unknown) != 0 {
		t.Fatalf("%+v", res)
	}
	h := sh.Header
	if len(h.Void) != 0 || h.TrackedChangedUnexplained != 0 || h.PostInventoryUnexplained != 0 {
		t.Fatalf("a clean run has a clean header: %+v", h)
	}
	if h.BindingID != f.pre.BindingID || h.ShardIndex != 0 || h.ShardCount != 1 || sh.BindingID != f.pre.BindingID {
		t.Fatalf("header %+v", h)
	}
	for _, c := range []struct {
		action string
		steps  int
	}{{ca, 1}, {cb, 2}, {cz, 1}} {
		r := requireKnown(t, sh, c.action)
		pl, _ := f.pre.Action(c.action)
		if r.Status != "built" || r.StepsRun != c.steps || r.StepsTotal != c.steps || len(r.Steps) != c.steps {
			t.Errorf("%s: %+v", c.action, r)
		}
		if r.Complete == nil || !*r.Complete {
			t.Errorf("%s: writes_complete %v, the classifier said true", c.action, r.Complete)
		}
		if r.Weak != pl.Weak || r.Refusal != pl.Refusal || r.Language != pl.Language || r.Gate != "" {
			t.Errorf("%s: key fields %+v", c.action, r)
		}
		if r.StepsSHA != recfmt.StepsDigest(pl.Steps) || r.BindingID != f.pre.BindingID {
			t.Errorf("%s: digests", c.action)
		}
		for _, s := range r.Steps {
			if !s.Clean || s.WallMS != 2500 || s.TraceBytes == 0 {
				t.Errorf("%s: step %+v", c.action, s)
			}
		}
		if len(r.Record.Writes) != 1 || r.Record.Writes[0] != c.action+"/out" {
			t.Errorf("%s: the raw record was not carried: %+v", c.action, r.Record)
		}
	}
	// The file is whole, private and has no temporary sibling left.
	if st, err := os.Stat(f.out); err != nil || st.Mode().Perm() != 0o600 {
		t.Fatalf("%v %v", st, err)
	}
	if ents, _ := os.ReadDir(filepath.Dir(f.out)); len(ents) != 1 {
		t.Fatalf("a temporary file was left: %v", ents)
	}
}

// fakeKeys answers the merge's key questions from the sealed action files: what
// the merge job would recompute from its own tree.
type fakeKeys struct{ pre *Pre }

func (k fakeKeys) Weak(_ recfmt.HostFacts, action string) ([32]byte, string, error) {
	a, _ := k.pre.Action(action)
	var out [32]byte
	if a.Refusal != "" {
		return out, a.Refusal, nil
	}
	b, err := hex.DecodeString(a.Weak)
	copy(out[:], b)
	return out, "", err
}

func (k fakeKeys) Strong(_ recfmt.HostFacts, _ string, _ []string) ([32]byte, error) {
	return [32]byte{7}, nil
}

// mergeOf feeds a decoded shard to the merge as the whole run.
func mergeOf(f *cfix, sh *recfmt.Shard) recmerge.Result {
	steps := map[string]string{}
	var rels []string
	for _, n := range f.pre.Actions() {
		a, _ := f.pre.Action(n)
		steps[n] = recfmt.StepsDigest(a.Steps)
		rels = append(rels, n)
	}
	return recmerge.Merge(recmerge.Input{
		Run:      recmerge.Run{Repository: "owner/repo", ID: "43117337685", Attempt: 2, Revision: f.p.sha},
		Expected: recmerge.Expected{GraphSHA256: sh.Binding.GraphSHA256, StepsSHA256: sh.Binding.StepsSHA256, Plan: []recfmt.ShardPlan{{Index: 0, Packages: rels, Assigned: rels}}, Steps: steps},
		Shards:   []*recfmt.Shard{sh}, Keys: fakeKeys{f.pre},
		Now: time.Date(2026, 11, 14, 0, 0, 0, 0, time.UTC), Kiln: "kiln/test", HonourWritesComplete: true,
	})
}

// The shard `collect` writes is accepted by the merge: nothing void, every
// action emitted.
func TestCollectShardDecodesAndMerges(t *testing.T) {
	f := newCfix(t)
	f.good()
	_, sh := f.collect()
	m := mergeOf(f, sh)
	if !m.Emitted || m.RunVoidCount != 0 || len(m.Manifests) != 3 {
		t.Fatalf("the merge did not accept the shard: emitted %v, run-void %v, %d manifests, outcomes %+v", m.Emitted, m.RunVoid, len(m.Manifests), m.Outcomes)
	}
	// And a failed action (an unknown write set: nothing is believed) voids the
	// run in the merge, exactly as the verdict says.
	f2 := newCfix(t)
	f2.good()
	os.Remove(RecordFile(f2.p.dir, ca))
	_, sh2 := f2.collect()
	requireUnknown(t, sh2, ca, "no record.json")
	if m := mergeOf(f2, sh2); m.Emitted || m.RunVoidCount == 0 {
		t.Fatalf("an unknown write set must void the run: %+v", m)
	}
}

// ---------------------------------------------------------------------------
// Refusals: collect cannot say what the build did, so it writes nothing.

func TestCollectRefusesWhileAStepHoldsTheDirectory(t *testing.T) {
	f := newCfix(t)
	f.good()
	// A step (or a supervisor its dead build tool orphaned) holds the shared lock.
	unlock, err := LockDir(f.p.dir, false)
	if err != nil {
		t.Fatal(err)
	}
	f.refuse("cannot be locked exclusive")
	unlock()
	f.collect() // and once it is gone the same directory collects
}

func TestCollectLockIsExclusive(t *testing.T) {
	f := newCfix(t)
	f.good()
	// While Collect runs it holds the exclusive lock: a second collect, and a step
	// (shared), are refused. The seam runs inside the lock.
	var inner, stepLock error
	o := f.opts()
	o.Fsck = func(g *source.Git) error {
		_, inner = Collect(CollectOptions{Root: o.Root, Dir: o.Dir, Results: o.Results, Out: o.Out + "2", KilnExe: o.KilnExe})
		_, stepLock = LockDir(o.Dir, false)
		return nil
	}
	if _, err := Collect(o); err != nil {
		t.Fatal(err)
	}
	if inner == nil || !strings.Contains(inner.Error(), "cannot be locked") {
		t.Fatalf("a second collect was not refused: %v", inner)
	}
	if stepLock == nil {
		t.Fatal("a step could take the shared lock while collect held the exclusive one")
	}
}

func TestCollectRefusesATamperedPre(t *testing.T) {
	for name, tamper := range map[string]func(f *cfix){
		"a sealed file edited": func(f *cfix) {
			os.Chmod(filepath.Join(f.p.dir, "pre"), 0o755)
			p := filepath.Join(f.p.dir, "pre", "tools.json")
			os.Chmod(p, 0o644)
			data, _ := os.ReadFile(p)
			os.WriteFile(p, append(data, ' '), 0o644)
		},
		"SEALED missing": func(f *cfix) {
			os.Chmod(filepath.Join(f.p.dir, "pre"), 0o755)
			os.Remove(filepath.Join(f.p.dir, "pre", "SEALED"))
		},
		"a stray file in pre": func(f *cfix) {
			os.Chmod(filepath.Join(f.p.dir, "pre"), 0o755)
			os.WriteFile(filepath.Join(f.p.dir, "pre", "extra"), []byte("x"), 0o644)
		},
	} {
		t.Run(name, func(t *testing.T) {
			f := newCfixWith(t, nil, nil)
			f.good()
			tamper(f)
			f.refuse("pre/")
		})
	}
}

func TestCollectRefusesAnotherRootOrBinary(t *testing.T) {
	f := newCfix(t)
	f.good()
	o := f.opts()
	other := filepath.Join(f.p.base, "elsewhere")
	os.MkdirAll(other, 0o755)

	o.Root = other
	if _, err := Collect(o); err == nil || !strings.Contains(err.Error(), "not the checkout") {
		t.Fatalf("another root: %v", err)
	}
	// Another kiln binary: not the one prepare hashed.
	o = f.opts()
	o.KilnExe = filepath.Join(f.p.base, "bin", "other-kiln")
	os.WriteFile(o.KilnExe, []byte("another binary"), 0o755)
	if _, err := Collect(o); err == nil || !strings.Contains(err.Error(), "not the binary prepare hashed") {
		t.Fatalf("another kiln: %v", err)
	}
	// Relative and unclean paths.
	for _, mod := range []func(*CollectOptions){
		func(o *CollectOptions) { o.Root = "r" }, func(o *CollectOptions) { o.Dir = "d" },
		func(o *CollectOptions) { o.Results = "x" }, func(o *CollectOptions) { o.Out = "o" },
		func(o *CollectOptions) { o.KilnExe = "k" }, func(o *CollectOptions) { o.Out = o.Out + "/../x" },
	} {
		o = f.opts()
		mod(&o)
		if _, err := Collect(o); err == nil || !strings.Contains(err.Error(), "absolute, clean") {
			t.Fatalf("%v", err)
		}
	}
	// A record directory that is a symlink to the real one is not canonical.
	link := filepath.Join(f.p.base, "link")
	if err := os.Symlink(f.p.dir, link); err != nil {
		t.Skip(err)
	}
	o = f.opts()
	o.Dir = link
	if _, err := Collect(o); err == nil || !strings.Contains(err.Error(), "canonical") {
		t.Fatalf("a symlinked --dir: %v", err)
	}
}

// Another git than the pinned one is refused (its digest is in the binding), and
// git is never looked up on PATH: a planted `git` there never runs.
func TestCollectUsesOnlyThePinnedGit(t *testing.T) {
	f := newCfix(t)
	f.good()
	f2 := newCfixWith(t, nil, nil) // built before PATH changes: the fixture itself runs git
	f2.good()
	planted := t.TempDir()
	marker := filepath.Join(planted, "ran")
	os.WriteFile(filepath.Join(planted, "git"), []byte("#!/bin/sh\ntouch '"+marker+"'\nexit 1\n"), 0o755)
	t.Setenv("PATH", planted+string(os.PathListSeparator)+os.Getenv("PATH"))
	if _, err := Collect(f.opts()); err != nil {
		t.Fatal(err)
	}
	if _, err := os.Stat(marker); err == nil {
		t.Fatal("a git found on PATH was run")
	}
	// tools.json naming another program: a digest that is not the binding's.
	alt := filepath.Join(f2.p.base, "bin", "git2")
	os.WriteFile(alt, []byte("not git"), 0o755)
	tools := f2.pre.Tools
	tools.Git = alt
	data, err := marshalJSON(tools)
	if err != nil {
		t.Fatal(err)
	}
	(&stepFix{t: t, p: f2.p}).reseal(map[string][]byte{"tools.json": data}, false)
	f2.refuse("not the binary prepare hashed")
}

func TestCollectRefusesAToolsFileWithoutHome(t *testing.T) {
	f := newCfixWith(t, nil, nil)
	f.good()
	tools := f.pre.Tools
	var env []string
	for _, e := range tools.BuildToolEnv {
		if !strings.HasPrefix(e, "HOME=") {
			env = append(env, e)
		}
	}
	tools.BuildToolEnv = env
	data, _ := marshalJSON(tools)
	(&stepFix{t: t, p: f.p}).reseal(map[string][]byte{"tools.json": data}, false)
	f.refuse("no HOME")
}

func TestCollectRefusesAnUnusableResults(t *testing.T) {
	f := newCfix(t)
	f.good()
	good := f.results
	os.Remove(f.rpath)
	f.refuse("--results")
	f.setResults(append(good, ResultsPackage{Package: "code/packages/x/zz", Status: "built", StepsRun: 1, StepsTotal: 1})...)
	f.refuse("not an action of this shard's plan")
	f.writeResultsFile("kiln-legacy-results/9", good)
	f.refuse("schema")
	f.setResults(good...)
	f.collect()
}

func TestCollectRefusesOutPlacement(t *testing.T) {
	f := newCfix(t)
	f.good()
	for name, out := range map[string]string{
		"inside pre":      filepath.Join(f.p.dir, "pre", "records.jsonl"),
		"inside traces":   filepath.Join(f.p.dir, "traces", "records.jsonl"),
		"inside checkout": filepath.Join(f.p.root, "records.jsonl"),
		"missing parent":  filepath.Join(f.p.base, "nowhere", "records.jsonl"),
	} {
		o := f.opts()
		o.Out = out
		if _, err := Collect(o); err == nil {
			t.Errorf("%s: collect wrote %s", name, out)
		}
		if _, err := os.Lstat(out); err == nil {
			t.Errorf("%s: %s exists", name, out)
		}
	}
	// A parent that is a symlink is not canonical.
	real := filepath.Join(f.p.base, "realout")
	os.MkdirAll(real, 0o755)
	link := filepath.Join(f.p.base, "linkout")
	if err := os.Symlink(real, link); err != nil {
		t.Skip(err)
	}
	o := f.opts()
	o.Out = filepath.Join(link, "records.jsonl")
	if _, err := Collect(o); err == nil || !strings.Contains(err.Error(), "canonical") {
		t.Fatalf("%v", err)
	}
	// An existing file is never replaced.
	os.WriteFile(f.out, []byte("precious"), 0o600)
	if _, err := Collect(f.opts()); err == nil {
		t.Fatal("collect replaced an existing --out")
	}
	if data, _ := os.ReadFile(f.out); string(data) != "precious" {
		t.Fatalf("the existing file was changed: %q", data)
	}
}

// The same real git with an unreadable record directory ancestor etc. are
// covered by VerifyPre; here the seams prove the order: the lock first.
func TestCollectTakesTheLockBeforeReadingAnything(t *testing.T) {
	f := newCfix(t)
	f.good()
	var order []string
	o := f.opts()
	o.Lock = func(dir string, excl bool) (func(), error) {
		order = append(order, "lock")
		if !excl {
			t.Error("collect asked for a shared lock")
		}
		return LockDir(dir, excl)
	}
	o.VerifyTree = func(g *source.Git, root, want string) (Result, error) {
		order = append(order, "verify")
		return VerifyTreeWith(g, root, want)
	}
	if _, err := Collect(o); err != nil {
		t.Fatal(err)
	}
	if len(order) != 2 || order[0] != "lock" {
		t.Fatalf("%v", order)
	}
}

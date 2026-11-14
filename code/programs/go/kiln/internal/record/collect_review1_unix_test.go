//go:build unix

package record

// Tests added by the first review of `collect` (KILN04 stage 4 PR 2b-2b-ii).

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// ---------------------------------------------------------------------------
// HIGH-1: a package with an empty recipe is legitimate.

const cw = "code/packages/x/w"

// zeroPlan is a one-shard plan of four packages, the fourth (x/w) with no step.
const zeroPlan = `{
  "schema_version": 1, "force": true,
  "packages": [
    {"name": "x/a", "rel_path": "code/packages/x/a", "language": "x"},
    {"name": "x/b", "rel_path": "code/packages/x/b", "language": "x"},
    {"name": "x/w", "rel_path": "code/packages/x/w", "language": "x"},
    {"name": "x/z", "rel_path": "code/packages/x/z", "language": "x"}
  ],
  "shards": [
    {"index": 0, "name": "s0", "assigned_packages": ["x/a", "x/b", "x/w", "x/z"], "package_names": ["x/a", "x/b", "x/w", "x/z"]}
  ]
}`

// newCfixZero is newCfix with a fourth package, x/w, whose BUILD is only a
// comment: the build tool drops comments, blank lines and the `set -e` preamble,
// so its step list is empty and it reports built, 0 of 0.
func newCfixZero(t *testing.T) *cfix {
	t.Helper()
	return borrow(t, "zero", func(t *testing.T, base string) *cfix {
		p := newPrepAt(t, base)
		p.repoDest = filepath.Join(base, "repo")
		p.files[cw+"/BUILD"] = "# nothing to run\n"
		b := "BUILD"
		var pkgs []legacy.Package
		for _, n := range [][2]string{{"x/a", "code/packages/x/a"}, {"x/b", "code/packages/x/b"}, {"x/w", cw}, {"x/z", "code/packages/x/z"}} {
			pkgs = append(pkgs, legacy.Package{
				Name: n[0], RelPath: n[1], Language: "x",
				BuildFiles: map[string]*string{"linux": &b, "darwin": &b, "windows": &b},
				Deps:       map[string][]string{"linux": {}, "darwin": {}, "windows": {}},
				DevDeps:    []string{}, RepoInputs: []string{}, RepoInputGlobs: []string{}, ExtraToolchains: []string{},
			})
		}
		g, err := json.Marshal(legacy.Graph{Schema: legacy.GraphSchema, Packages: pkgs})
		if err != nil {
			t.Fatal(err)
		}
		p.graph = string(g)
		p.steps = `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[` +
			`{"package":"code/packages/x/a","steps":["echo a"]},` +
			`{"package":"code/packages/x/b","steps":["echo b1","echo b2"]},` +
			`{"package":"` + cw + `","steps":[]},` +
			`{"package":"code/packages/x/z","steps":["echo z"]}]}`
		p.commit()
		return finishCfix(t, p, base, zeroPlan)
	})
}

// goodZero is good() plus the empty package, built 0 of 0.
func (f *cfix) goodZero(w ResultsPackage) {
	f.t.Helper()
	f.good()
	f.setResults(
		ResultsPackage{Package: ca, Status: "built", StepsRun: 1, StepsTotal: 1},
		ResultsPackage{Package: cb, Status: "built", StepsRun: 2, StepsTotal: 2},
		w,
		ResultsPackage{Package: cz, Status: "built", StepsRun: 1, StepsTotal: 1},
	)
}

func TestCollectAnEmptyRecipeIsVacuouslyComplete(t *testing.T) {
	f := newCfixZero(t)
	f.goodZero(ResultsPackage{Package: cw, Status: "built", StepsRun: 0, StepsTotal: 0})
	res, sh := f.collect()
	w := requireKnown(t, sh, cw)
	if w.Status != "built" || w.StepsRun != 0 || w.StepsTotal != 0 || len(w.Steps) != 0 || w.Complete == nil || !*w.Complete {
		t.Fatalf("%+v", w)
	}
	if len(w.Record.Entries)+len(w.Record.Writes)+len(w.Record.TrackedWrites) != 0 {
		t.Fatalf("an empty recipe has an empty raw record: %+v", w.Record)
	}
	for _, u := range res.Unknown {
		if strings.Contains(u, cw) {
			t.Fatalf("reported as unknown: %v", res.Unknown)
		}
	}
	// The merge accepts the run, and does not emit the action that has nothing to
	// cache (a manifest needs its own pkg: entry).
	m := mergeOf(f, sh)
	if !m.Emitted || m.RunVoidCount != 0 {
		t.Fatalf("the merge voided the run for an empty recipe: %v", m.RunVoid)
	}
	for _, mf := range m.Manifests {
		if mf.Target == cw {
			t.Fatal("the empty-recipe action was emitted")
		}
	}
	if len(m.Manifests) != 3 {
		t.Fatalf("%d manifests, want the other three", len(m.Manifests))
	}
	for _, o := range m.Outcomes {
		if o.Action == cw && o.Emitted {
			t.Fatal("emitted")
		}
	}
}

func TestCollectAnEmptyRecipeIsAcceptedOnlyWhenEveryFactAgrees(t *testing.T) {
	for _, c := range []struct {
		name string
		w    ResultsPackage
		do   func(f *cfix)
		want string
	}{
		{"a trace directory exists", ResultsPackage{Package: cw, Status: "built", StepsRun: 0, StepsTotal: 0},
			func(f *cfix) { os.Mkdir(f.dirOf(cw), 0o700) }, "built without a traced step"},
		{"the build tool started a step", ResultsPackage{Package: cw, Status: "built", StepsRun: 1, StepsTotal: 1},
			func(f *cfix) {}, "the build tool says 1 steps, the plan has 0"},
		{"the build tool counts a step it never ran", ResultsPackage{Package: cw, Status: "built", StepsRun: 0, StepsTotal: 2},
			func(f *cfix) {}, "the build tool says 2 steps, the plan has 0"},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := newCfixZero(t)
			f.goodZero(c.w)
			c.do(f)
			_, sh := f.collect()
			requireUnknown(t, sh, cw, c.want)
		})
	}
	// Not built: it ran nothing, as any action that did not run.
	t.Run("not built", func(t *testing.T) {
		f := newCfixZero(t)
		f.goodZero(ResultsPackage{Package: cw, Status: "dep-skipped", StepsRun: 0, StepsTotal: 0})
		_, sh := f.collect()
		w := requireKnown(t, sh, cw)
		if w.Status != "dep-skipped" || w.Complete == nil || *w.Complete {
			t.Fatalf("%+v", w)
		}
	})
	// Not listed in the results: not-run, never vacuous.
	t.Run("absent from the results", func(t *testing.T) {
		f := newCfixZero(t)
		f.good()
		f.setResults(f.results[0], f.results[1], f.results[2]) // good() listed a, b, z
		_, sh := f.collect()
		if w := requireKnown(t, sh, cw); w.Status != "not-run" || *w.Complete {
			t.Fatalf("%+v", w)
		}
	})
	// The shard-wide order rules still override it.
	t.Run("the serial pass overrides", func(t *testing.T) {
		f := newCfixZero(t)
		f.goodZero(ResultsPackage{Package: cw, Status: "built", StepsRun: 0, StepsTotal: 0})
		f.setSeq("9\n")
		_, sh := f.collect()
		requireUnknown(t, sh, cw, "the counter reached 9")
	})
}

// ---------------------------------------------------------------------------
// MEDIUM-1: the report says why a post-build check failed.

func TestPostResultNoteIsBoundedAndPrintable(t *testing.T) {
	var r postResult
	for i := 0; i < 100; i++ {
		r.note("fsck: %s\n\x1b[31m::error::%d", strings.Repeat("x", 2000), i)
	}
	if len(r.problems) != maxPostProblems {
		t.Fatalf("%d entries", len(r.problems))
	}
	for _, p := range r.problems {
		if len(p) > maxPostProblemText || strings.ContainsAny(p, "\n\x1b") {
			t.Fatalf("%d bytes: %q", len(p), p[:60])
		}
	}
	if got := printableText("\u00e9\u00e9\u00e9", 5); got != "\u00e9\u00e9" {
		t.Fatalf("a cut inside a rune: %q", got)
	}
	if got := firstN([]string{"a", "b", "c"}, 2); len(got) != 2 {
		t.Fatal(got)
	}
}

func TestCollectPostProblemsNameEveryFailedCheck(t *testing.T) {
	f := newCfix(t)
	f.good()
	var paths []string
	var lines []string
	for i := 0; i < 40; i++ {
		paths = append(paths, fmt.Sprintf("dir/odd\nname%02d", i))
		lines = append(lines, fmt.Sprintf("dir/odd\nname%02d: content differs", i))
	}
	res, sh := f.collectWith(func(o *CollectOptions) {
		o.VerifyTree = func(*source.Git, string, string) (Result, error) {
			return Result{}, &TreeError{What: "working tree is not HEAD", Total: 40, Paths: paths, Problems: lines[:16]}
		}
		o.Fsck = func(*source.Git) error {
			return errors.New("git fsck: missing blob " + strings.Repeat("0", 40) + "\n::error::x")
		}
		o.Inventory = func(*source.Git, string) (recfmt.Inventory, error) {
			return recfmt.Inventory{}, errors.New("inventory has more than 2097152 entries")
		}
	})
	requireVoid(t, sh.Header, 41, 1, "tracked-changed")
	if len(res.PostProblems) != 3 {
		t.Fatalf("%q", res.PostProblems)
	}
	for i, want := range []string{"verify-tree: ", "fsck: ", "inventory: "} {
		p := res.PostProblems[i]
		if !strings.HasPrefix(p, want) || len(p) > maxPostProblemText || strings.ContainsAny(p, "\n\x1b") {
			t.Fatalf("%d: %q", i, p)
		}
	}
	// The tree entry names the check's own words and at most 16 paths.
	if v := res.PostProblems[0]; !strings.Contains(v, "40 unexplained") || !strings.Contains(v, "dir/odd?name00") {
		t.Fatalf("%q", v)
	}
	// The shard itself does not carry them.
	data, _ := os.ReadFile(f.out)
	if strings.Contains(string(data), "fsck") {
		t.Fatal("diagnostics reached the shard")
	}
}

func TestCollectPostProblemsNameTheNewPaths(t *testing.T) {
	f := newCfix(t)
	f.good()
	f.edit("code/packages/x/a/new.txt", "x")
	f.edit("code/packages/x/a/src.txt", "changed\n")
	res, _ := f.collect()
	joined := strings.Join(res.PostProblems, "\n")
	for _, want := range []string{"inventory: 1 new paths no record's writes cover: code/packages/x/a/new.txt", "verify-tree: ", "1 unexplained: code/packages/x/a/src.txt"} {
		if !strings.Contains(joined, want) {
			t.Errorf("lacks %q:\n%s", want, joined)
		}
	}
	// A clean run has nothing to say.
	g := newCfix(t)
	g.good()
	if res, _ := g.collect(); len(res.PostProblems) != 0 {
		t.Fatalf("%q", res.PostProblems)
	}
}

// ---------------------------------------------------------------------------
// MEDIUM (security): a new directory is not explained by a write beneath it.

func TestCollectAWriteBeneathANewDirectoryDoesNotExplainTheDirectory(t *testing.T) {
	f := newCfix(t)
	f.good()
	f.writesOf(nil, []string{"code/packages/x/a/nd/deep/b/c"})
	f.edit("code/packages/x/a/nd/deep/b/c", "x")
	f.edit("code/packages/x/a/nd/other", "nobody wrote this")
	_, sh := f.collect()
	requireVoid(t, sh.Header, 0, 1, "")
	_ = sh
}

// ---------------------------------------------------------------------------
// LOW 9: a partial TreeError.

func TestCollectAPartialTreeErrorCountsOneMore(t *testing.T) {
	const src = "code/packages/x/a/src.txt"
	for _, c := range []struct {
		name    string
		tracked []string
		want    int
	}{{"explained by a tracked write", []string{src}, 1}, {"unexplained", nil, 2}} {
		t.Run(c.name, func(t *testing.T) {
			f := newCfix(t)
			f.good()
			f.writesOf(c.tracked, []string{ca + "/out"})
			// Stage a deletion: the index no longer has the file the commit does,
			// so the check stops before it compares the disk.
			runGit(t, f.p.root, "rm", "-q", "--cached", src)
			_, err := VerifyTree(f.p.root, f.p.sha)
			var te *TreeError
			if !errors.As(err, &te) || !te.Partial {
				t.Fatalf("want a partial TreeError, got %v", err)
			}
			_, sh := f.collect()
			if sh.Header.TrackedChangedUnexplained != c.want {
				t.Fatalf("%+v, want %d", sh.Header, c.want)
			}
		})
	}
	// A change on the disk is not partial.
	f := newCfix(t)
	f.edit("code/packages/x/a/src.txt", "changed\n")
	_, err := VerifyTree(f.p.root, f.p.sha)
	var te *TreeError
	if !errors.As(err, &te) || te.Partial {
		t.Fatalf("%v", err)
	}
	if markPartial(errors.New("plain")) == nil {
		t.Fatal("markPartial swallowed an error")
	}
}

// ---------------------------------------------------------------------------
// LOW 10: duplicate and case-variant keys are refused.

func TestDecodersRefuseDuplicateAndCaseVariantKeys(t *testing.T) {
	f := newCfix(t)
	f.good()
	f.step(ca, 0, 0, nil)
	f.verdict(ca, 1, nil) // never overwrites: written by good() already, replaced here
	sidecar, _ := os.ReadFile(StepFile(f.p.dir, ca, 0, "json"))
	if _, err := DecodeStepSidecar(sidecar); err != nil {
		t.Fatal(err)
	}
	record, _ := os.ReadFile(RecordFile(f.p.dir, ca))
	if _, err := DecodeClassified(record); err != nil {
		t.Fatal(err)
	}
	for name, c := range map[string]struct {
		data   []byte
		decode func([]byte) error
		from   string
		to     string
	}{
		"a sidecar with a duplicate key":    {sidecar, func(b []byte) error { _, e := DecodeStepSidecar(b); return e }, `"seq":0`, `"seq":0,"seq":5`},
		"a sidecar with a case variant":     {sidecar, func(b []byte) error { _, e := DecodeStepSidecar(b); return e }, `"seq":0`, `"Seq":0`},
		"a record with a duplicate key":     {record, func(b []byte) error { _, e := DecodeClassified(b); return e }, `"steps_run":1`, `"steps_run":1,"steps_run":2`},
		"a record with a case variant":      {record, func(b []byte) error { _, e := DecodeClassified(b); return e }, `"parse_ok":true`, `"Parse_ok":true`},
		"a nested raw record duplicate key": {record, func(b []byte) error { _, e := DecodeClassified(b); return e }, `"entries":`, `"entries":[],"entries":`},
	} {
		mutated := strings.Replace(string(c.data), c.from, c.to, 1)
		if mutated == string(c.data) {
			t.Fatalf("%s: the fixture does not contain %s", name, c.from)
		}
		if err := c.decode([]byte(mutated)); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// And collect treats such a record as undecodable, not as a record.
	os.WriteFile(RecordFile(f.p.dir, ca), []byte(strings.Replace(string(record), `"steps_run":1`, `"steps_run":1,"steps_run":1`, 1)), 0o600)
	_, sh := f.collect()
	requireUnknown(t, sh, ca, "record.json does not decode")
}

// ---------------------------------------------------------------------------
// LOW 12 and 13: the arguments, and the tombstone.

func TestCollectRefusesResultsInsideTheCheckout(t *testing.T) {
	f := newCfix(t)
	f.good()
	o := f.opts()
	o.Results = filepath.Join(f.p.root, "results.json")
	if _, err := Collect(o); err == nil || !strings.Contains(err.Error(), "--results") || !strings.Contains(err.Error(), "inside the checkout") {
		t.Fatalf("%v", err)
	}
	if _, err := os.Lstat(filepath.Join(f.p.dir, "traces", CollectedMarker)); err == nil {
		t.Fatal("a refused argument used up the directory's collection")
	}
}

func TestCollectLeavesATombstoneAndStepRefusesAfterIt(t *testing.T) {
	f := newCfix(t)
	f.good()
	tomb := filepath.Join(f.p.dir, "traces", CollectedMarker)
	// Refusals about the arguments come first and leave no tombstone.
	os.Remove(f.rpath)
	f.refuse("--results")
	if _, err := os.Lstat(tomb); err == nil {
		t.Fatal("a refusal left the tombstone")
	}
	f.setResults(f.results...)
	// An --out in the way is refused before it too.
	os.WriteFile(f.out, []byte("x"), 0o600)
	if _, err := Collect(f.opts()); err == nil || !strings.Contains(err.Error(), "exists") {
		t.Fatalf("%v", err)
	}
	if _, err := os.Lstat(tomb); err == nil {
		t.Fatal("an --out in the way used up the collection")
	}
	os.Remove(f.out)

	f.collect()
	st, err := os.Lstat(tomb)
	if err != nil || !st.Mode().IsRegular() {
		t.Fatalf("no tombstone: %v", err)
	}
	// A directory is collected once.
	os.Remove(f.out)
	if _, err := Collect(f.opts()); err == nil || !strings.Contains(err.Error(), "collected once") {
		t.Fatalf("%v", err)
	}
	// A step that arrives afterwards runs nothing, takes no number and leaves no
	// marker.
	seqBefore, _ := os.ReadFile(filepath.Join(f.p.dir, "traces", ".seq"))
	entriesBefore, _ := os.ReadDir(f.dirOf(ca))
	env := append(append([]string{}, f.pre.Tools.BuildToolEnv...), EnvAction+"="+ca, EnvStep+"=0", EnvStepCount+"=1")
	_, err = RunStep(StepOptions{Dir: f.p.dir, Argv: []string{"sh", "-c", "echo a"}, Environ: env, Cwd: f.pre.Config.PackageDir(ca), KilnExe: f.p.kiln})
	if err == nil || !strings.Contains(err.Error(), "was collected") {
		t.Fatalf("%v", err)
	}
	seqAfter, _ := os.ReadFile(filepath.Join(f.p.dir, "traces", ".seq"))
	entriesAfter, _ := os.ReadDir(f.dirOf(ca))
	if string(seqBefore) != string(seqAfter) || len(entriesBefore) != len(entriesAfter) {
		t.Fatal("a step after collect changed the record directory")
	}
	// A stale tombstone is refused too.
	g := newCfix(t)
	g.good()
	os.WriteFile(filepath.Join(g.p.dir, "traces", CollectedMarker), nil, 0o600)
	g.refuse("collected once")
}

// ---------------------------------------------------------------------------
// LOW 6 and 8.

func TestCheckSidecarsIsBoundedByThePlan(t *testing.T) {
	f := newCfix(t)
	f.good()
	c := &collector{dir: f.p.dir, pre: f.pre}
	pl, _ := f.pre.Action(ca)
	s := dirScan{exists: true, sidecars: map[int]bool{}, traces: map[int]bool{999999: true}}
	var bad []string
	checkSidecars(c, ca, pl, s, &bad)
	if len(bad) > 4 {
		t.Fatalf("%d reasons for one orphan", len(bad))
	}
	if !strings.Contains(strings.Join(bad, ";"), "steps ran, the plan has 1") {
		t.Fatalf("%q", bad)
	}
}

func TestWriteAtomicUsesARandomNameAndNeverReplaces(t *testing.T) {
	d := canonTemp(t)
	final := filepath.Join(d, "record.json")
	// A leftover of a killed process with this pid must not stop the next write.
	os.WriteFile(fmt.Sprintf("%s.tmp-%d", final, os.Getpid()), []byte("stray"), 0o600)
	if err := writeAtomic(final, []byte("whole")); err != nil {
		t.Fatal(err)
	}
	if data, _ := os.ReadFile(final); string(data) != "whole" {
		t.Fatalf("%q", data)
	}
	if err := writeAtomic(final, []byte("second")); err == nil {
		t.Fatal("replaced")
	}
	if data, _ := os.ReadFile(final); string(data) != "whole" {
		t.Fatalf("%q", data)
	}
	ents, _ := os.ReadDir(d)
	if len(ents) != 2 { // the record and the stray one, nothing of ours
		t.Fatalf("temporary files were left: %v", ents)
	}
	// The parent cannot be written: an error, and nothing left behind.
	if err := writeAtomic(filepath.Join(d, "absent", "x"), nil); err == nil {
		t.Fatal("no error")
	}
	if err := syncDir(filepath.Join(d, "absent")); err != nil {
		t.Fatalf("an unopenable directory is not an error: %v", err)
	}
}

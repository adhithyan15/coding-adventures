//go:build unix

package record

// The fixture of the `collect` tests (KILN04 section 4.8, stage 4 PR 2b-2b-ii)
// ============================================================================
//
// `collect` reads what `kiln record step` left behind, so a test of it must put
// the same files in the same places. Nothing here runs strace or the sandbox
// (that is the Linux-only end-to-end test of the parent package): the fixture
// PREPARES a real shard (`Prepare`, a real committed repository, the real git)
// and then WRITES, by hand, exactly what a supervisor and its classification
// child would have written for a run of the three fixture packages:
//
//	x/a   one step              seq 0
//	x/b   two steps             seq 1, 2
//	x/z   one step              seq 3        (.seq is then 4)
//
// Each file is made with the production encoder (EncodeStepSidecar,
// EncodeClassified), so the tests exercise `collect` against well-formed input
// and then break ONE thing at a time. Breaking a thing is a function the test
// passes in, so every refusal and void path is a few lines of table.

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// singlePlan is a plan of ONE shard that runs and is assigned all three
// packages, so a decoded shard is the whole run and the merge can be given it.
const singlePlan = `{
  "schema_version": 1, "force": true,
  "packages": [
    {"name": "x/a", "rel_path": "code/packages/x/a", "language": "x"},
    {"name": "x/b", "rel_path": "code/packages/x/b", "language": "x"},
    {"name": "x/z", "rel_path": "code/packages/x/z", "language": "x"}
  ],
  "shards": [
    {"index": 0, "name": "s0", "assigned_packages": ["x/a", "x/b", "x/z"], "package_names": ["x/a", "x/b", "x/z"]}
  ]
}`

// cfix is a prepared one-shard record directory plus a way to fake its run.
type cfix struct {
	t       *testing.T
	p       *prep
	pre     *Pre
	results []ResultsPackage
	rpath   string // the results file
	out     string // where records.jsonl goes
}

// newCfix prepares a shard of the three fixture packages. Prepare is given the
// one-shard plan, so the shard is shard 0 of 1.
func newCfix(t *testing.T) *cfix {
	t.Helper()
	p := newPrep(t)
	p.steps = `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[` +
		`{"package":"code/packages/x/a","steps":["echo a"]},` +
		`{"package":"code/packages/x/b","steps":["echo b1","echo b2"]},` +
		`{"package":"code/packages/x/z","steps":["echo z"]}]}`
	p.commit()
	old := plan
	plan = singlePlan
	t.Cleanup(func() { plan = old })
	o := p.options()
	o.ShardCount = 1
	t.Cleanup(p.writable)
	if _, err := Prepare(o); err != nil {
		t.Fatalf("Prepare: %v", err)
	}
	pre, err := VerifyPre(p.dir)
	if err != nil {
		t.Fatal(err)
	}
	outDir := filepath.Join(p.base, "out")
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		t.Fatal(err)
	}
	return &cfix{t: t, p: p, pre: pre, rpath: filepath.Join(p.base, "results.json"), out: filepath.Join(outDir, "records.jsonl")}
}

func (f *cfix) opts() CollectOptions {
	return CollectOptions{Root: f.p.root, Dir: f.p.dir, Results: f.rpath, Out: f.out, KilnExe: f.p.kiln}
}

// collect runs Collect and requires success.
func (f *cfix) collect() (*CollectResult, *recfmt.Shard) {
	f.t.Helper()
	res, err := Collect(f.opts())
	if err != nil {
		f.t.Fatalf("Collect: %v", err)
	}
	return res, f.decode()
}

// refuse runs Collect and requires a refusal naming want, with nothing written.
func (f *cfix) refuse(want string) {
	f.t.Helper()
	res, err := Collect(f.opts())
	if err == nil {
		f.t.Fatalf("Collect succeeded, want a refusal naming %q: %+v", want, res)
	}
	if !strings.Contains(err.Error(), want) {
		f.t.Fatalf("refusal %q does not name %q", err, want)
	}
	if _, serr := os.Lstat(f.out); serr == nil {
		f.t.Fatalf("a refused collect wrote %s", f.out)
	}
}

// decode reads records.jsonl back the way the merge does.
func (f *cfix) decode() *recfmt.Shard {
	f.t.Helper()
	data, err := os.ReadFile(f.out)
	if err != nil {
		f.t.Fatal(err)
	}
	files := recfmt.ShardFiles{Records: strings.NewReader(string(data))}
	for _, x := range []struct {
		rel string
		dst *[]byte
	}{{"binding.json", &files.Binding}, {"host-facts.json", &files.HostFacts}, {"inventory.json", &files.Inventory}} {
		b, err := f.pre.ReadSealed(x.rel)
		if err != nil {
			f.t.Fatal(err)
		}
		*x.dst = b
	}
	sh, err := recfmt.DecodeShard(0, files, recfmt.DefaultLimits)
	if err != nil {
		f.t.Fatalf("the shard does not decode: %v", err)
	}
	return sh
}

func recOf(t *testing.T, sh *recfmt.Shard, action string) recfmt.ShardRecord {
	t.Helper()
	for _, r := range sh.Records {
		if r.Action == action {
			return r
		}
	}
	t.Fatalf("no record for %s", action)
	return recfmt.ShardRecord{}
}

// ---------------------------------------------------------------------------
// Writing what a run leaves.

func (f *cfix) dirOf(action string) string { return StepDir(f.p.dir, action) }

// step writes step j of action with the sequence number seq: a trace of one
// line and a sidecar that describes it. edit, when set, changes the sidecar
// BEFORE it is encoded (the trace is written first, so edit can also override
// the digest it holds).
func (f *cfix) step(action string, j, seq int, edit func(*StepSidecar)) {
	f.t.Helper()
	pl, _ := f.pre.Action(action)
	if err := os.MkdirAll(f.dirOf(action), 0o700); err != nil {
		f.t.Fatal(err)
	}
	trace := []byte(fmt.Sprintf("1 trace of %s step %d\n", action, j))
	if err := os.WriteFile(StepFile(f.p.dir, action, j, "trace"), trace, 0o600); err != nil {
		f.t.Fatal(err)
	}
	line := sha256.Sum256([]byte(pl.Steps[j]))
	sum := sha256.Sum256(trace)
	sc := StepSidecar{
		Schema: StepSchema, Action: action, Step: j, StepCount: len(pl.Steps), Seq: seq, BindingID: f.pre.BindingID,
		LineSHA256: hex.EncodeToString(line[:]), LineInPlan: true, Started: true,
		StartedAt: "2026-11-14T10:00:00Z", EndedAt: "2026-11-14T10:00:02.5Z",
		StraceStdoutEmpty: true, StraceStderrEmpty: true,
		TraceBytes: int64(len(trace)), TraceSHA256: hex.EncodeToString(sum[:]),
		StepNetNS: true, Clean: true, Unclean: []string{},
	}
	if edit != nil {
		edit(&sc)
	}
	data, err := EncodeStepSidecar(sc)
	if err != nil {
		f.t.Fatalf("sidecar: %v", err)
	}
	if err := os.WriteFile(StepFile(f.p.dir, action, j, "json"), data, 0o600); err != nil {
		f.t.Fatal(err)
	}
}

// rawFor is the classifier's output for an action in the fixture.
func rawFor(action string) *traced.RawRecord {
	return &traced.RawRecord{
		Package: action, Entries: []string{"pkg:" + action}, Writes: []string{action + "/out"},
		TrackedWrites: []string{}, AbsentObservations: []string{}, Binds: []string{},
		UntrackedObservations: []string{}, Traversed: []string{}, Reasons: []string{},
	}
}

// verdict writes record.json for action; steps is the number of steps the
// record claims started. edit changes the record before it is encoded.
func (f *cfix) verdict(action string, steps int, edit func(*ClassifiedRecord)) {
	f.t.Helper()
	c := ClassifiedRecord{
		Schema: ClassifiedSchema, Action: action, BindingID: f.pre.BindingID, SealedSHA256: f.pre.SealedSHA256,
		StepsRun: steps, StepNetNS: true, ParseOK: true, ParseReasons: []string{}, WritesComplete: true, Record: rawFor(action),
	}
	if edit != nil {
		edit(&c)
	}
	data, err := EncodeClassified(c)
	if err != nil {
		f.t.Fatalf("record.json: %v", err)
	}
	if err := os.WriteFile(RecordFile(f.p.dir, action), data, 0o600); err != nil {
		f.t.Fatal(err)
	}
}

func (f *cfix) setSeq(text string) {
	f.t.Helper()
	if err := os.WriteFile(filepath.Join(f.p.dir, "traces", ".seq"), []byte(text), 0o600); err != nil {
		f.t.Fatal(err)
	}
}

func (f *cfix) setResults(pkgs ...ResultsPackage) {
	f.t.Helper()
	f.results = pkgs
	f.writeResultsFile(ResultsSchema, pkgs)
}

func (f *cfix) writeResultsFile(schema string, pkgs []ResultsPackage) {
	f.t.Helper()
	data, err := json.Marshal(Results{Schema: schema, Packages: pkgs})
	if err != nil {
		f.t.Fatal(err)
	}
	if err := os.WriteFile(f.rpath, data, 0o600); err != nil {
		f.t.Fatal(err)
	}
}

const (
	ca = "code/packages/x/a"
	cb = "code/packages/x/b"
	cz = "code/packages/x/z"
)

// good writes a complete, clean run of all three actions.
func (f *cfix) good() {
	f.t.Helper()
	f.step(ca, 0, 0, nil)
	f.verdict(ca, 1, nil)
	f.step(cb, 0, 1, nil)
	f.step(cb, 1, 2, nil)
	f.verdict(cb, 2, nil)
	f.step(cz, 0, 3, nil)
	f.verdict(cz, 1, nil)
	f.setSeq("4\n")
	f.setResults(
		ResultsPackage{Package: ca, Status: "built", StepsRun: 1, StepsTotal: 1},
		ResultsPackage{Package: cb, Status: "built", StepsRun: 2, StepsTotal: 2},
		ResultsPackage{Package: cz, Status: "built", StepsRun: 1, StepsTotal: 1},
	)
}

// requireUnknown asserts the record of action is an unknown write set whose
// reason names want, and is a well-formed one: an empty raw record and an
// explicit writes_complete false.
func requireUnknown(t *testing.T, sh *recfmt.Shard, action, want string) {
	t.Helper()
	r := recOf(t, sh, action)
	if r.Unknown == "" || !strings.Contains(r.Unknown, want) {
		t.Fatalf("%s: unknown_writes %q does not name %q", action, r.Unknown, want)
	}
	if r.Complete == nil || *r.Complete {
		t.Fatalf("%s: writes_complete is %v on an unknown write set", action, r.Complete)
	}
	if len(r.Record.Entries)+len(r.Record.Writes)+len(r.Record.TrackedWrites) != 0 {
		t.Fatalf("%s: an unknown write set carries a raw record: %+v", action, r.Record)
	}
	if len(r.Unknown) > 200 {
		t.Fatalf("%s: unknown_writes is %d bytes", action, len(r.Unknown))
	}
}

func requireKnown(t *testing.T, sh *recfmt.Shard, action string) recfmt.ShardRecord {
	t.Helper()
	r := recOf(t, sh, action)
	if r.Unknown != "" {
		t.Fatalf("%s: unexpectedly unknown: %s", action, r.Unknown)
	}
	return r
}

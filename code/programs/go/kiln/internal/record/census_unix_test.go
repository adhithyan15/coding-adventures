//go:build unix

package record

// The census side file (census.go): what the child writes, that nothing it can
// do changes the verdict, and that the directory scans of `collect` and `step`
// leave it alone.  Unix only: the fixtures are POSIX paths and process-level
// helpers, so Windows skips them by not building them.

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// censusFix runs one action through the real step, so the real child leaves
// record.json (and census.json) behind, then removes both to classify again.
func censusFix(t *testing.T) (*stepFix, string) {
	t.Helper()
	line := playLine("", "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 0, cannedOpts{})
	f.must(actZ, 0, 1, line)
	want, err := os.ReadFile(RecordFile(f.p.dir, actZ))
	if err != nil {
		t.Fatal(err)
	}
	os.Remove(RecordFile(f.p.dir, actZ))
	os.Remove(CensusFile(f.p.dir, actZ))
	return f, string(want)
}

// A classifier that raises 40 host reads: the child's census holds all 40 and
// is a well-formed census.json; the file is whole (no temporary left).
func TestClassifyChildWritesTheCensus(t *testing.T) {
	f, _ := censusFix(t)
	h := ChildHooks{Classify: func(cfg traced.Config, in traced.Input) (*traced.RawRecord, error) {
		if in.Census == nil {
			t.Error("the child did not attach a census")
			return traced.Classify(cfg, in)
		}
		for i := 0; i < 40; i++ {
			in.Census.Note("host-read", fmt.Sprintf("/var/x/f%02d", i))
		}
		in.Census.Note("trace-incomplete:proc-path", "/proc/9/cmdline")
		return traced.Classify(cfg, in)
	}}
	if err := ClassifyChild(f.p.dir, actZ, 0, 0, h); err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(CensusFile(f.p.dir, actZ))
	if err != nil {
		t.Fatal(err)
	}
	var doc CensusDoc
	if err := json.Unmarshal(data, &doc); err != nil {
		t.Fatal(err)
	}
	if doc.Schema != "kiln-record-census/1" || doc.Action != actZ || doc.Total < 41 || len(doc.Entries) < 41 {
		t.Fatalf("census: %+v", doc)
	}
	ents, _ := os.ReadDir(StepDir(f.p.dir, actZ))
	for _, e := range ents {
		if strings.Contains(e.Name(), ".tmp-") {
			t.Fatalf("a temporary survived: %s", e.Name())
		}
	}
}

// A census that cannot be written changes neither the verdict nor the exit
// status: record.json is byte-for-byte the one made with a working census.
func TestCensusWriteFailureChangesNothing(t *testing.T) {
	f, _ := censusFix(t)
	if err := ClassifyChild(f.p.dir, actZ, 0, 0, ChildHooks{}); err != nil {
		t.Fatal(err)
	}
	good, _ := os.ReadFile(RecordFile(f.p.dir, actZ))
	if !exists(CensusFile(f.p.dir, actZ)) {
		t.Fatal("the census was not written")
	}
	os.Remove(RecordFile(f.p.dir, actZ))
	os.Remove(CensusFile(f.p.dir, actZ))

	for name, fail := range map[string]func(string, []byte) error{
		"an error": func(string, []byte) error { return fmt.Errorf("disk full") },
		"a panic":  func(string, []byte) error { panic("boom") },
	} {
		old := censusWrite
		censusWrite = fail
		err := ClassifyChild(f.p.dir, actZ, 0, 0, ChildHooks{})
		censusWrite = old
		if err != nil {
			t.Fatalf("%s: the child failed: %v", name, err)
		}
		got, _ := os.ReadFile(RecordFile(f.p.dir, actZ))
		if string(got) != string(good) {
			t.Fatalf("%s: the verdict changed", name)
		}
		if exists(CensusFile(f.p.dir, actZ)) {
			t.Fatalf("%s: a census appeared", name)
		}
		os.Remove(RecordFile(f.p.dir, actZ))
	}
}

// The real write is atomic: a failure (here, a directory in the way of the
// rename) leaves no temporary behind and no half file.
func TestCensusWriteIsAtomic(t *testing.T) {
	d := t.TempDir()
	final := filepath.Join(d, "census.json")
	if err := writeCensusFile(final, []byte("one")); err != nil {
		t.Fatal(err)
	}
	if err := writeCensusFile(final, []byte("two")); err != nil { // replaces whole
		t.Fatal(err)
	}
	if b, _ := os.ReadFile(final); string(b) != "two" {
		t.Fatalf("%q", b)
	}
	blocked := filepath.Join(d, "blocked")
	if err := os.MkdirAll(filepath.Join(blocked, "x"), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := writeCensusFile(blocked, []byte("x")); err == nil {
		t.Fatal("renamed over a directory")
	}
	ents, _ := os.ReadDir(d)
	for _, e := range ents {
		if strings.Contains(e.Name(), ".tmp-") {
			t.Fatalf("a temporary survived: %s", e.Name())
		}
	}
}

// The encoded file is bounded; what is shed is counted in overflow.
func TestEncodeCensusIsBounded(t *testing.T) {
	c := traced.NewCensus()
	for i := 0; i < traced.MaxCensusEntries; i++ {
		c.Note("host-read", fmt.Sprintf("/%s/%d", strings.Repeat("d", 150), i))
	}
	c.Note("host-read", "/a")
	data, err := EncodeCensus(actZ, c)
	if err != nil {
		t.Fatal(err)
	}
	if len(data) > MaxCensusBytes {
		t.Fatalf("%d bytes", len(data))
	}
	var doc CensusDoc
	if err := json.Unmarshal(data, &doc); err != nil {
		t.Fatal(err)
	}
	var held int64
	for _, e := range doc.Entries {
		held += e.Count
	}
	if doc.Overflow == 0 || held+doc.Overflow != doc.Total {
		t.Fatalf("held %d overflow %d total %d", held, doc.Overflow, doc.Total)
	}
	empty, _ := EncodeCensus(actZ, traced.NewCensus())
	if !strings.Contains(string(empty), `"entries":[]`) {
		t.Fatalf("%s", empty)
	}
}

// collect treats census.json (and a stray temporary of one) as nothing: the
// action still collects cleanly.
func TestCollectIgnoresTheCensus(t *testing.T) {
	f := newCfix(t)
	f.good()
	os.WriteFile(filepath.Join(f.dirOf(ca), "census.json"), []byte(`{"schema":"kiln-record-census/1"}`), 0o600)
	os.WriteFile(filepath.Join(f.dirOf(ca), "census.json.tmp-abcd"), []byte("{"), 0o600)
	_, sh := f.collect()
	if r := recOf(t, sh, ca); r.Unknown != "" {
		t.Fatalf("a census made the action unknown: %q", r.Unknown)
	}
}

// step's order check ignores it too, and still finds .rerun, .overlap and
// out-of-order steps.
func TestCheckOrderIgnoresTheCensus(t *testing.T) {
	d := canonTemp(t)
	act := "pkg/a"
	writeBytes(t, StepFile(d, act, 0, "json"), []byte("{}"), 0o600)
	writeBytes(t, CensusFile(d, act), []byte("{}"), 0o600)
	writeBytes(t, CensusFile(d, act)+".tmp-1", []byte("{"), 0o600)
	writeBytes(t, StepFile(d, act, 1, "overlap"), nil, 0o600)
	if err := checkOrder(d, act, 1); err != nil {
		t.Fatalf("step 1 after step 0 with a census present: %v", err)
	}
	// A step that did run is still caught.
	writeBytes(t, StepFile(d, act, 1, "json"), []byte("{}"), 0o600)
	if err := checkOrder(d, act, 1); err == nil || !strings.Contains(err.Error(), "already run") {
		t.Fatalf("%v", err)
	}
	if !exists(StepFile(d, act, 1, "rerun")) {
		t.Fatal("no .rerun marker")
	}
}

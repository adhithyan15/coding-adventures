package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	mb "github.com/adhithyan15/coding-adventures/code/packages/go/microbenchmark"
)

func testPlan() Plan {
	c := mb.Defaults()
	c.Pairs = 6
	c.TargetNS = 1
	c.MaxBatch = 1
	return Plan{1, c, []int{1024}, []int{128}, []int64{7}}
}
func encode(t *testing.T, v any) []byte {
	t.Helper()
	b, err := json.Marshal(v)
	if err != nil {
		t.Fatal(err)
	}
	return b
}
func planFile(t *testing.T, p Plan) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "plan.json")
	if err := os.WriteFile(path, encode(t, p), 0600); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestPlansAndBounds(t *testing.T) {
	base := string(encode(t, testPlan()))
	if _, err := parsePlan([]byte(base)); err != nil {
		t.Fatal(err)
	}
	for _, path := range []string{"../../../benchmarks/microbenchmarks/learning-plan.json", "../../../benchmarks/microbenchmarks/smoke-plan.json"} {
		data, err := os.ReadFile(path)
		if err != nil {
			t.Fatal(err)
		}
		if _, err := parsePlan(data); err != nil {
			t.Fatal(err)
		}
	}
	cases := []string{`null`, `[]`, `{`, base + "{}", strings.Repeat(" ", 65537), strings.Replace(base, `"version":1`, `"version":2`, 1), strings.Replace(base, `"version":1`, `"version":1,"\u0076ersion":1`, 1), strings.Replace(base, `"version":1`, `"extra":1`, 1), strings.Replace(base, `"seed":0`, `"seed":null`, 1), strings.Replace(base, `"pairs":6`, `"pairs":6,"pairs":6`, 1), strings.Replace(base, `"pairs":6`, `"pairs":6.0`, 1), strings.Replace(base, `"seeds":[7]`, `"seeds":[7,7]`, 1), strings.Replace(base, `"memory_bytes":[1024]`, `"memory_bytes":[1024,1024]`, 1)}
	for i, data := range cases {
		if _, err := parsePlan([]byte(data)); err == nil {
			t.Fatalf("accepted invalid plan %d", i)
		}
	}
	mutations := []func(*Plan){func(p *Plan) { p.Config.Pairs = 0 }, func(p *Plan) { p.Config.Pairs = 10001 }, func(p *Plan) { p.Config.ConfidencePercent = 0 }, func(p *Plan) { p.Config.MinimumFactor = 1 }, func(p *Plan) { p.Config.TargetNS = 0 }, func(p *Plan) { p.Config.MaxBatch = 1048577 }, func(p *Plan) { p.Seeds = nil }, func(p *Plan) { p.Seeds = make([]int64, 17) }, func(p *Plan) { p.MemoryBytes = nil; p.GrowthLengths = nil }, func(p *Plan) { p.MemoryBytes = []int{15} }, func(p *Plan) { p.MemoryBytes = []int{(32 << 20) + 1} }, func(p *Plan) { p.GrowthLengths = []int{0} }, func(p *Plan) { p.GrowthLengths = []int{32769} }, func(p *Plan) { p.MemoryBytes = make([]int, 33) }, func(p *Plan) { p.MemoryBytes = make([]int, 32); p.Seeds = make([]int64, 16) }}
	for i, mutate := range mutations {
		p := testPlan()
		mutate(&p)
		if _, err := parsePlan(encode(t, p)); err == nil {
			t.Fatalf("accepted invalid plan mutation%d", i)
		}
	}
}

func TestWorkloadsAndIndependentCounts(t *testing.T) {
	// 17 is coprime to 251, so one complete period permutes 0..250.
	for _, doubling := range []bool{false, true} {
		if sum := growth(251, doubling); sum != 31375 {
			t.Fatalf("growth period sum%d", sum)
		}
	}
	for n := 1; n <= 1000; n++ {
		fixed := growthCounts(n, false)
		allocs := (n + 31) / 32
		wantCopies := 32 * allocs * (allocs - 1) / 2
		if fixed.Allocations != allocs || fixed.Copied != wantCopies {
			t.Fatalf("fixed counts n%d: %+v", n, fixed)
		}
		power, allocs := 1, 1
		for power < n {
			power *= 2
			allocs++
		}
		doubled := growthCounts(n, true)
		if doubled.Allocations != allocs || doubled.Copied != power-1 {
			t.Fatalf("doubling counts n%d: %+v", n, doubled)
		}
	}
	for _, kind := range []string{"indexed", "links", "growth"} {
		c, err := prepare(kind, 1024, 7)
		if err != nil {
			t.Fatal(err)
		}
		for _, batch := range []mb.Batch{c.Baseline, c.Candidate} {
			for _, n := range []int{0, 1, 3} {
				got, err := batch(n)
				if err != nil || got != c.Expected*int64(n) {
					t.Fatalf("%s checksum n%d: %d %v", kind, n, got, err)
				}
			}
			for _, n := range []int{-1, 1048577} {
				if _, err := batch(n); err == nil {
					t.Fatalf("%s accepted bad count", kind)
				}
			}
		}
	}
	a := []int64{1, 2, 3}
	a[1] = 42
	for _, batch := range []func([]int64, int) (int64, error){indexed, ranged} {
		got, err := batch(a, 3)
		if err != nil || got != 138 {
			t.Fatal("mutable-array values not consumed")
		}
	}
	for _, bad := range []struct {
		kind string
		size int
	}{{"unknown", 1}, {"indexed", 15}, {"indexed", (32 << 20) + 1}, {"links", 0}, {"growth", 0}, {"growth", 32769}} {
		if _, err := prepare(bad.kind, bad.size, 0); err == nil {
			t.Fatal("accepted invalid preparation")
		}
	}
	valid := []node{{1, 1}, {2, 2}, {3, 0}}
	if err := cycleValid(valid, 6); err != nil {
		t.Fatal(err)
	}
	sum, err := traverse(valid, 2)
	if err != nil || sum != 12 {
		t.Fatal("incorrect dependent traversal")
	}
	for _, bad := range [][]node{nil, {{1, 0}, {2, 0}}, {{1, 5}}, {{1, 1}, {2, 1}}} {
		if err := cycleValid(bad, 3); err == nil {
			t.Fatal("accepted malformed cycle")
		}
	}
	if err := cycleValid(valid, 7); err == nil {
		t.Fatal("accepted wrong cycle checksum")
	}
}

func TestExecuteRetainsReportsAndCompletionManifest(t *testing.T) {
	p := testPlan()
	path := planFile(t, p)
	out := filepath.Join(t.TempDir(), "run")
	var tick int64
	now := func() int64 { tick += 10; return tick }
	var stdout bytes.Buffer
	if err := execute(path, out, strings.Repeat("a", 40), now, &stdout); err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(filepath.Join(out, "manifest.json"))
	if err != nil {
		t.Fatal(err)
	}
	var m manifest
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatal(err)
	}
	if len(m.Cases) != 3 || len(m.ExecutableSHA256) != 64 || m.PlanSHA256 != digest(encode(t, p)) || m.Finished == "" || stdout.Len() == 0 {
		t.Fatalf("invalid manifest %+v", m)
	}
	for _, c := range m.Cases {
		raw, err := os.ReadFile(filepath.Join(out, c.Report))
		if err != nil || digest(raw) != c.SHA256 {
			t.Fatalf("report digest: %v", err)
		}
		var core coreReport
		if err := json.Unmarshal(raw, &core); err != nil {
			t.Fatal(err)
		}
		analysis, err := mb.Analyze(core.Pairs, core.Config.ConfidencePercent, core.Config.MinimumFactor)
		if err != nil || analysis.Ratio != core.Analysis.Ratio || core.Config.Seed != c.Seed || core.Analysis.Verdict != "equivalent" {
			t.Fatalf("failed replay: %v", err)
		}
	}
	if err := execute(path, out, strings.Repeat("a", 40), now, io.Discard); err == nil {
		t.Fatal("overwrote completed output")
	}
	failed := filepath.Join(t.TempDir(), "failed")
	if err := execute(path, failed, strings.Repeat("a", 40), func() int64 { return 0 }, io.Discard); err == nil {
		t.Fatal("accepted zero-duration measurement")
	}
	if _, err := os.Stat(filepath.Join(failed, "manifest.json")); !os.IsNotExist(err) {
		t.Fatal("failed collection published completion manifest")
	}
}

type failedWriter struct{}

func (failedWriter) Write([]byte) (int, error) { return 0, fmt.Errorf("broken output") }
func TestIOAndCLIRejections(t *testing.T) {
	p := planFile(t, testPlan())
	revision := strings.Repeat("a", 40)
	for _, args := range [][]string{{"--unknown"}, {"extra"}, {"--plan", p, "--out", filepath.Join(t.TempDir(), "out"), "--revision", "not-a-sha"}, {"--help"}} {
		var stdout, stderr bytes.Buffer
		code := cli(args, &stdout, &stderr)
		if len(args) == 1 && args[0] == "--help" {
			if code != 0 {
				t.Fatal("help failed")
			}
		} else if code == 0 || stdout.Len() != 0 || stderr.Len() == 0 {
			t.Fatal("invalid CLI succeeded")
		}
	}
	if err := execute("missing", filepath.Join(t.TempDir(), "out"), revision, mb.MonotonicClock(), io.Discard); err == nil {
		t.Fatal("missing plan accepted")
	}
	if err := execute(p, filepath.Join(t.TempDir(), "missing", "out"), revision, mb.MonotonicClock(), io.Discard); err == nil {
		t.Fatal("missing parent accepted")
	}
	var tick int64
	if err := execute(p, filepath.Join(t.TempDir(), "out"), revision, func() int64 { tick += 10; return tick }, failedWriter{}); err == nil {
		t.Fatal("output error ignored")
	}
	file := filepath.Join(t.TempDir(), "new")
	if err := writeNew(file, []byte("first")); err != nil {
		t.Fatal(err)
	}
	if err := writeNew(file, []byte("second")); err == nil {
		t.Fatal("write replaced existing file")
	}
	if _, err := fileDigest("missing"); err == nil {
		t.Fatal("missing digest input accepted")
	}
}

type faultFile struct {
	n                  int
	writeErr, closeErr error
	closed             bool
}

func (f *faultFile) Write(p []byte) (int, error) { return f.n, f.writeErr }
func (f *faultFile) Close() error                { f.closed = true; return f.closeErr }
func TestCompletionPublicationAndWriteFailures(t *testing.T) {
	writeErr, closeErr := errors.New("write failure"), errors.New("close failure")
	for _, f := range []*faultFile{{0, writeErr, nil, false}, {0, nil, nil, false}, {3, nil, closeErr, false}} {
		err := writeClosed(f, []byte("abc"))
		if err == nil || !f.closed {
			t.Fatal("write/close failure ignored")
		}
	}
	dir := t.TempDir()
	if err := publishManifest(dir, []byte("complete")); err != nil {
		t.Fatal(err)
	}
	if err := publishManifest(dir, []byte("replacement")); err == nil {
		t.Fatal("existing marker overwritten")
	}
	data, err := os.ReadFile(filepath.Join(dir, "manifest.json"))
	if err != nil || string(data) != "complete" {
		t.Fatal("completion changed")
	}
	entries, err := os.ReadDir(dir)
	if err != nil || len(entries) != 1 {
		t.Fatal("staged file was not removed")
	}
	if err := publishManifest(filepath.Join(dir, "absent"), []byte("data")); err == nil {
		t.Fatal("missing output directory accepted")
	}
}

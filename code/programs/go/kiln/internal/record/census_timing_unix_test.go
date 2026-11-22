//go:build unix

package record

// G2 evidence by size, the host block, the cold-start parse, the directory lock,
// the cross-check size limit and the panic guard of `kiln record census`.
// Unix only (the step fixture and flock).

import (
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// G2 is judged per size bucket and at the census p99 size; the throughput and
// the cold start are separate numbers.
func TestCensusTimingBySize(t *testing.T) {
	a := newCensusAcc(nil)
	a.sizesAll = []int64{100, 100, 5000, 5000, 1 << 20}
	a.samples = []parseSample{
		{size: 100, ms: 1, share: 0.01}, {size: 100, ms: 3, share: 0.02},
		{size: 5000, ms: 10, share: 0.05}, {size: 5000, ms: 20, share: -1},
		{size: 1 << 20, ms: 6000, share: 0.5},
	}
	a.first = FirstParse{MS: 7, Bytes: 100}
	s := a.summary(10)
	tm := s.Timing
	if len(tm.BySize) != 3 || tm.BySize[0].Min != 64 || tm.BySize[0].N != 2 || tm.BySize[0].ParseMS.Max != 3 || tm.BySize[1].Min != 4096 || tm.BySize[1].ParseShare.N != 1 || tm.BySize[2].Min != 1<<20 {
		t.Errorf("by size: %+v", tm.BySize)
	}
	// p99 of five sizes is the largest: only the 1 MiB trace is at or above it, and
	// it fails both limbs.
	if g := tm.G2AtP99; g.ThresholdBytes != 1<<20 || g.Traces != 1 || g.Over10Pct != 1 || g.Over5s != 1 || g.Failing != 1 {
		t.Errorf("g2 at p99: %+v", g)
	}
	if tm.ParseMBPerS.N != 5 || tm.ParseMBPerS.Max <= 0 || tm.FirstParse.MS != 7 {
		t.Errorf("throughput %+v first %+v", tm.ParseMBPerS, tm.FirstParse)
	}
}

// The path-byte cap, like the distinct-path cap, drops and counts.
func TestCensusPathByteCap(t *testing.T) {
	a := newCensusAcc(nil)
	a.pathBytes = maxCensusPathBytes - 3
	a.addCensus(&CensusDoc{Entries: []tracedEntry{{Kind: "host-read", Path: "/a", Count: 1}, {Kind: "host-read", Path: "/bcd", Count: 1}}})
	s := a.summary(10)
	if s.Bounds.PathsDropped != 1 || s.Reasons[0].DistinctPaths != 1 {
		t.Errorf("%+v %+v", s.Bounds, s.Reasons[0])
	}
}

// The first parse, the host block and the sanitised CPU model of a real run.
func TestCensusHostAndColdStart(t *testing.T) {
	f := censusDirFix(t)
	cpu := filepath.Join(t.TempDir(), "cpuinfo")
	writeBytes(t, cpu, []byte("processor\t: 0\nmodel name\t: Evil CPU\n::error::x‮\nmodel name\t: second\n"), 0o644)
	_, s, _, mdPath := runCensus(t, f, func(o *CensusRunOptions) { o.CPUInfo = cpu })
	h := s.Host
	if h.ShardCount != f.pre.Binding.ShardCount || h.ShardIndex != f.pre.Binding.ShardIndex || h.StraceVersion == "" && f.pre.Binding.StraceVersion != "" {
		t.Errorf("host: %+v", h)
	}
	data, err := f.pre.ReadSealed("host-facts.json")
	if err != nil {
		t.Fatal(err)
	}
	hf, err := recfmt.DecodeHostFacts(data)
	if err != nil || h.ImageID != hf.Image || h.ImageID == "" {
		t.Errorf("image %q, want %q (%v)", h.ImageID, hf.Image, err)
	}
	if h.CPUModel != "Evil CPU" {
		t.Errorf("cpu model %q", h.CPUModel)
	}
	if !s.Timing.FirstParse.Accepted || s.Timing.FirstParse.MS <= 0 || s.Timing.FirstParse.Bytes <= 0 || len(s.Timing.BySize) == 0 || s.Timing.ParseMBPerS.N != 2 {
		t.Errorf("timing: %+v", s.Timing)
	}
	md, _ := os.ReadFile(mdPath)
	if !strings.Contains(string(md), "## Host") || !strings.Contains(string(md), "`Evil CPU`") {
		t.Error("the Markdown lacks the host block")
	}
	// A CPU model that is hostile is sanitised.
	writeBytes(t, cpu, []byte("model name\t: a‮b\x1b[31m\n"), 0o644)
	_, s, _, _ = runCensus(t, f, func(o *CensusRunOptions) { o.CPUInfo = cpu })
	if s.Host.CPUModel != "a?b?[31m" {
		t.Errorf("cpu model %q", s.Host.CPUModel)
	}
	// No such file: empty, not a failure.
	_, s, _, _ = runCensus(t, f, func(o *CensusRunOptions) { o.CPUInfo = "/nonexistent/cpuinfo" })
	if s.Host.CPUModel != "" {
		t.Errorf("cpu model %q", s.Host.CPUModel)
	}
}

// A recording in progress (a step holds the directory lock) is refused, and
// nothing is written; once it lets go the census runs.
func TestCensusRefusesWhileARecordingHoldsTheLock(t *testing.T) {
	f := censusDirFix(t)
	unlock, err := LockDir(f.p.dir, false) // what a step holds
	if err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(t.TempDir(), "o.json")
	_, err = RunCensus(CensusRunOptions{CensusOptions: CensusOptions{Dir: f.p.dir}, Out: out}, io.Discard)
	if err == nil || !strings.Contains(err.Error(), "in progress") {
		t.Fatalf("no refusal: %v", err)
	}
	if exists(out) {
		t.Error("a refused census wrote its output")
	}
	unlock()
	if res, _, _, _ := runCensus(t, f); res.Exit != 0 {
		t.Errorf("exit %d after the lock was released", res.Exit)
	}
}

// A trace over the cross-check limit is timed but not cross-checked, and counted;
// with nothing cross-checked the run is an infrastructure failure.
func TestCensusSkipsLargeCrossChecks(t *testing.T) {
	f := censusDirFix(t)
	res, s, _, _ := runCensus(t, f, func(o *CensusRunOptions) { o.CrossCheckMaxBytes = 1 })
	cc := s.CrossCheck
	if cc.Skipped != 2 || cc.SkippedBytes <= 0 || cc.Traces != 0 || s.Timing.ParseMS.N != 2 {
		t.Errorf("cross-check %+v, timing %+v", cc, s.Timing.ParseMS)
	}
	if res.Exit != CensusExitInfra || !strings.Contains(s.InfrastructureFailure, "no trace was cross-checked") {
		t.Errorf("exit %d, %q", res.Exit, s.InfrastructureFailure)
	}
}

// A panic in a parse is a counted problem, not a crash.
func TestCensusSurvivesAParsePanic(t *testing.T) {
	f := censusDirFix(t)
	res, s, _, _ := runCensus(t, f, func(o *CensusRunOptions) {
		o.CrossCheck = func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, trace.CrossCheckDetail) {
			panic("boom")
		}
	})
	// A panic is soak-relevant: exit 4, like a mismatch (not the infrastructure 1
	// that "nothing was cross-checked" would give).
	if s.Problems["parse_panic"] != 2 || res.Exit != CensusExitMismatch || s.TraceSizes.All.N != 2 {
		t.Errorf("exit %d problems %v", res.Exit, s.Problems)
	}
}

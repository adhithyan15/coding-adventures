package main

import (
	"bytes"
	"fmt"
	"os"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Gate G1 of the default flip (KILN04 section 4.9): the grammar path's PASS 1
// (the line reader) must read at a rate that would get a cap-sized action (2 GiB)
// through in a quarter of the classify deadline (10 minutes): 2048 MiB / 150 s =
// about 13.7 MB/s, rounded up to 14.  Two things make that a sound gate here:
//
//   - It is applied to the BEST (minimum) time of gateRepeats passes after a
//     warm-up pass, never to a single shot: a 50 KB trace is read in about two
//     milliseconds, so one sample is mostly timer, scheduler and neighbour noise,
//     and noise only ever adds time.  The minimum is the right statistic for a
//     capability floor.  (The single-shot figure is still logged, as
//     grammar_mb_s, for the census.)
//   - Traces under gateMinBytes (32 KB) are not gated at all.
//
// The rate on a ~50 KB trace or a 4 MiB repetition is a PROXY: it does not prove
// the rate at 2 GiB (the line reader is linear, but memory pressure at that size
// is not measured).
//
// The gate cannot silently go vacuous: every gated PROBE line says gated=1, and
// ci/assert_sandbox_tests.py fails the job unless at least one gated line was
// logged and the deterministic synthetic-4mib line is among them.
const (
	gateMinMBs   = 14.0
	gateMinBytes = 32 << 10
	gateRepeats  = 7
)

// probeTrace is probeTraceWall without a step wall time.
func probeTrace(t *testing.T, label, path string) {
	t.Helper()
	probeTraceWall(t, label, path, 0)
}

// probeTraceWall logs, with the "PROBE:" marker the CI script surfaces, the size of
// a trace real strace produced and how fast pass 1 reads it with the old scanner
// and with the grammar path (compact LineParser since P2 PR 5; D24, KILN04 section 4.9).
// grammar_mb_s is the single-shot figure (census); best_mb_s is the best of
// gateRepeats passes after a warm-up, and is the one gate G1 uses.  The
// measurement runs on a private copy of the bytes; it cannot change what the
// test goes on to assert.
//
// wall is the step's wall-clock time when the caller has it (0 when not).  It
// adds step_wall_ms, parse_ms and parse_share (grammar pass-1 time over wall) to
// the line, for the G2 census; G2 is report-only in PR 6, and needs wall time
// plumbed through the record-step and probe paths before it can block.
func probeTraceWall(t *testing.T, label, path string, wall time.Duration) {
	t.Helper()
	fh, err := os.Open(path)
	if err != nil {
		t.Logf("PROBE: trace %s: cannot open: %v", label, err)
		return
	}
	defer fh.Close()
	m, err := trace.MeasureBest(fh, true, gateRepeats)
	if err != nil {
		t.Logf("PROBE: trace %s: cannot measure: %v", label, err)
		return
	}
	// On a REAL trace the grammar path must accept everything the old scanner
	// does: a stricter-only refusal here would void the recording at the flip.
	if m.OldError == "" && m.NewError != "" {
		t.Errorf("trace %s: the grammar path refused (%s) a trace the old scanner accepted:\n%s", label, m.NewError, m.NewWindow)
	}
	t.Cleanup(func() {
		if n := trace.AllowedDivergences(); n != 0 {
			t.Errorf("the cross-check allowed %d stricter-only divergences on real traces; there must be none", n)
		}
	})
	mbs := func(d time.Duration) float64 {
		if d <= 0 {
			return 0
		}
		return float64(m.Bytes) / d.Seconds() / 1e6
	}
	gated := m.Bytes >= gateMinBytes && m.NewError == ""
	wallFacts := ""
	if wall > 0 {
		wallFacts = fmt.Sprintf(" step_wall_ms=%d parse_ms=%.2f parse_share=%.4f",
			wall.Milliseconds(), float64(m.NewTime.Microseconds())/1000, m.NewTime.Seconds()/wall.Seconds())
	}
	t.Logf("PROBE: trace %s bytes=%d lines=%d old_mb_s=%.1f grammar_mb_s=%.1f best_mb_s=%.1f gated=%d old_allocs=%d grammar_allocs=%d old_refusal=%q grammar_refusal=%q%s",
		label, m.Bytes, m.Lines, mbs(m.OldTime), mbs(m.NewTime), mbs(m.NewBest), b2i(gated), m.OldAllocs, m.NewAllocs, m.OldError, m.NewError, wallFacts)
	// G1 (blocking).
	if gated && mbs(m.NewBest) < gateMinMBs {
		t.Errorf("trace %s: the grammar path's best of %d passes read %d bytes at %.1f MB/s, below the %.0f MB/s floor (gate G1, KILN04 section 4.9)",
			label, gateRepeats, m.Bytes, mbs(m.NewBest), gateMinMBs)
	}
}

func b2i(b bool) int {
	if b {
		return 1
	}
	return 0
}

// TestSandboxTraceGateSynthetic is the deterministic gate-G1 input: the committed
// build.strace fixture repeated to at least 4 MiB.  Pass 1 must read all of it
// (a refusal fails the test), its whole-input best-of-gateRepeats rate is logged
// as whole_mb_s (census only), and the GATED figure is best_mb_s: the best of
// gateRepeats passes over a 64 KB slice of whole repetitions, against the same
// floor.  Gating the 4 MiB pass itself would make the floor depend on how the CPU
// is shared for the whole 200 ms of every pass (measured on an M2 Max with two
// busy threads per core: a 4 MiB pass fell to 12 MB/s while a 64 KB slice's best
// stayed at 20), and a sustained shortage of CPU is not something a best-of
// statistic can see through.  It needs no namespace or strace (it is named
// Sandbox so the kiln-sandbox job's `-run Sandbox` selects it and
// assert_sandbox_tests.py requires it), and it is what keeps G1 from going
// vacuous if every real trace in a run happens to be under 32 KB.
func TestSandboxTraceGateSynthetic(t *testing.T) {
	one, err := os.ReadFile("internal/trace/testdata/build.strace")
	if err != nil {
		t.Fatal(err)
	}
	if len(one) == 0 || one[len(one)-1] != '\n' {
		t.Fatal("the fixture must end with a newline to be repeatable")
	}
	whole := bytes.Repeat(one, (4<<20)/len(one)+1)
	slice := bytes.Repeat(one, (64<<10)/len(one)+1)
	wholeBest, refusal := trace.BestPass1(whole, true, gateRepeats)
	if refusal != "" {
		// The fixture must be repeatable: pass 1 must read every line.
		t.Fatalf("pass 1 refused the repeated fixture: %s", refusal)
	}
	best, refusal := trace.BestPass1(slice, true, gateRepeats)
	if refusal != "" {
		t.Fatalf("pass 1 refused the repeated fixture slice: %s", refusal)
	}
	mb := float64(len(slice)) / best.Seconds() / 1e6
	t.Logf("PROBE: trace synthetic-4mib bytes=%d lines=%d whole_mb_s=%.1f slice_bytes=%d best_mb_s=%.1f gated=1 repeats=%d",
		len(whole), bytes.Count(whole, []byte{'\n'}), float64(len(whole))/wholeBest.Seconds()/1e6, len(slice), mb, gateRepeats)
	if mb < gateMinMBs {
		t.Errorf("synthetic-4mib: the grammar path's best of %d passes read a %d byte slice at %.1f MB/s, below the %.0f MB/s floor (gate G1, KILN04 section 4.9)",
			gateRepeats, len(slice), mb, gateMinMBs)
	}
}

package main

import (
	"fmt"
	"os"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// gateMinMBs and gateMinBytes are gate G1 of the default flip (KILN04 section
// 4.9): the grammar path must parse a cap-sized action (2 GiB) within a quarter
// of the classify deadline (10 minutes), which is 2048 MiB / 150 s = about
// 13.7 MB/s, rounded up to 14.  Traces under 32 KB are timer noise (a few
// hundred microseconds), so only larger ones are held to it.
const (
	gateMinMBs   = 14.0
	gateMinBytes = 32 << 10
)

// probeTrace is probeTraceWall without a step wall time.
func probeTrace(t *testing.T, label, path string) {
	t.Helper()
	probeTraceWall(t, label, path, 0)
}

// probeTraceWall logs, with the "PROBE:" marker the CI script surfaces, the size of
// a trace real strace produced and how fast pass 1 reads it with the old scanner
// and with the grammar path (compact LineParser since P2 PR 5; D24, KILN04 section 4.9).  The default-flip gate is
// restated from these numbers, so every sandbox test that has a real trace
// reports it.  The measurement runs on a private copy of the bytes; it cannot
// change what the test goes on to assert.
func probeTraceWall(t *testing.T, label, path string, wall time.Duration) {
	t.Helper()
	fh, err := os.Open(path)
	if err != nil {
		t.Logf("PROBE: trace %s: cannot open: %v", label, err)
		return
	}
	defer fh.Close()
	m, err := trace.Measure(fh, true)
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
	mbs := func(secs float64) float64 {
		if secs <= 0 {
			return 0
		}
		return float64(m.Bytes) / secs / 1e6
	}
	gmbs := mbs(m.NewTime.Seconds())
	wallFacts := ""
	if wall > 0 {
		wallFacts = fmt.Sprintf(" step_wall_ms=%d parse_ms=%.2f parse_share=%.4f",
			wall.Milliseconds(), float64(m.NewTime.Microseconds())/1000, m.NewTime.Seconds()/wall.Seconds())
	}
	t.Logf("PROBE: trace %s bytes=%d lines=%d old_mb_s=%.1f grammar_mb_s=%.1f old_allocs=%d grammar_allocs=%d old_refusal=%q grammar_refusal=%q%s",
		label, m.Bytes, m.Lines, mbs(m.OldTime.Seconds()), gmbs, m.OldAllocs, m.NewAllocs, m.OldError, m.NewError, wallFacts)
	// G1 (blocking).
	if m.Bytes >= gateMinBytes && m.NewError == "" && gmbs < gateMinMBs {
		t.Errorf("trace %s: the grammar path read %d bytes at %.1f MB/s, below the %.0f MB/s floor (gate G1, KILN04 section 4.9)", label, m.Bytes, gmbs, gateMinMBs)
	}
}

package main

import (
	"os"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// probeTrace logs, with the "PROBE:" marker the CI script surfaces, the size of
// a trace real strace produced and how fast pass 1 reads it with the old scanner
// and with the grammar path (D24, KILN04 section 4.9).  The default-flip gate is
// restated from these numbers, so every sandbox test that has a real trace
// reports it.  The measurement runs on a private copy of the bytes; it cannot
// change what the test goes on to assert.
func probeTrace(t *testing.T, label, path string) {
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
	mbs := func(secs float64) float64 {
		if secs <= 0 {
			return 0
		}
		return float64(m.Bytes) / secs / 1e6
	}
	t.Logf("PROBE: trace %s bytes=%d lines=%d old_mb_s=%.1f grammar_mb_s=%.1f old_refusal=%q grammar_refusal=%q",
		label, m.Bytes, m.Lines, mbs(m.OldTime.Seconds()), mbs(m.NewTime.Seconds()), m.OldError, m.NewError)
}

package main

import (
	"bytes"
	"crypto/sha256"
	"os"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// TestG1DiagnosticSyntheticSlice profiles only the deterministic 64 KiB slice
// used by G1. Keep this separate from TestSandboxTraceGateSynthetic so neither
// the 4 MiB census nor the real sandbox tests enter the profile. The dedicated
// workflow runs this test with CPU and allocation profiling in separate
// processes; its instrumented timing is diagnostic and is never an admission
// receipt. The CPU run makes 20 BestPass1 calls (at least 5 seconds); the
// higher-overhead allocation run makes two calls. Each call is bounded by
// BestPass1's 200-pass cap, and the test process has a 30-second timeout.
func TestG1DiagnosticSyntheticSlice(t *testing.T) {
	one, err := os.ReadFile("internal/trace/testdata/build.strace")
	if err != nil {
		t.Fatal(err)
	}
	if len(one) == 0 || one[len(one)-1] != '\n' {
		t.Fatal("the fixture must end with a newline to be repeatable")
	}
	slice := bytes.Repeat(one, (64<<10)/len(one)+1)
	lines := bytes.Count(slice, []byte{'\n'})
	if len(slice) != 66600 || lines != 936 {
		t.Fatalf("G1 diagnostic input drifted: bytes=%d lines=%d, want 66600/936", len(slice), lines)
	}

	kind := os.Getenv("KILN_G1_PROFILE_KIND")
	calls := 0
	switch kind {
	case "cpu":
		calls = 20
	case "alloc":
		calls = 2
	default:
		t.Fatalf("KILN_G1_PROFILE_KIND must be cpu or alloc, got %q", kind)
	}

	started := time.Now()
	for i := 0; i < calls; i++ {
		if _, refusal := trace.BestPass1(slice, true, gateRepeats); refusal != "" {
			t.Fatalf("BestPass1 call %d refused the repeated fixture slice: %s", i+1, refusal)
		}
	}
	fixtureSHA := sha256.Sum256(one)
	sliceSHA := sha256.Sum256(slice)
	t.Logf("G1PROFILE: kind=%s fixture_sha256=%x slice_sha256=%x bytes=%d lines=%d helper_root=true gc=off_during_timing calls=%d timed_passes_min=%d timed_passes_max=%d min_span_ms_per_call=250 bestpass1_max_passes_per_call=200 elapsed_ms=%d",
		kind, fixtureSHA, sliceSHA, len(slice), lines, calls, calls*gateRepeats, calls*200, time.Since(started).Milliseconds())
}

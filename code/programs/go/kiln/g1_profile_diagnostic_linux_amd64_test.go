package main

import (
	"bytes"
	"crypto/sha256"
	"fmt"
	"os"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// TestG1DiagnosticSyntheticSlice profiles the exact gated 64 KiB input without
// changing the official admission test. Each workflow invocation is a separate
// process: CPU captures use 20 BestPass1 calls, and the allocation capture uses
// two calls with memprofilerate=1. The profile timings are diagnostic only.
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
	if len(one) != 1850 || len(slice) != 66600 || lines != 936 {
		t.Fatalf("G1 diagnostic input drifted: fixture=%d slice=%d/%d, want 1850/66600/936", len(one), len(slice), lines)
	}
	fixtureSHA := sha256.Sum256(one)
	sliceSHA := sha256.Sum256(slice)
	if got := fmt.Sprintf("%x", fixtureSHA); got != "dc8acd0de1d9b235c3924d0d71e2fcd565274e09ded9975570f429b548525aae" {
		t.Fatalf("fixture SHA-256 drifted: %s", got)
	}
	if got := fmt.Sprintf("%x", sliceSHA); got != "6cd61f63fdec55ce37294ad061705394b1babda80e5368b17ecb5bd38e2207af" {
		t.Fatalf("slice SHA-256 drifted: %s", got)
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
	t.Logf("G1PROFILE: kind=%s fixture_sha256=%x slice_sha256=%x bytes=%d lines=%d helper_root=true gc=off_during_timing calls=%d timed_passes_min=%d timed_passes_max=%d elapsed_ms=%d",
		kind, fixtureSHA, sliceSHA, len(slice), lines, calls, calls*gateRepeats, calls*200, time.Since(started).Milliseconds())
}

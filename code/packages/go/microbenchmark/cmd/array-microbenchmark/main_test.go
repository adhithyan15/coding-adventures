package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"strings"
	"testing"
)

func TestRunProducesJSONReport(t *testing.T) {
	var stdout, stderr bytes.Buffer
	tick := int64(0)
	err := run([]string{"array-microbenchmark", "--pairs", "2", "--target-ns", "1", "--max-batch", "1", "--revision", "abc"}, &stdout, &stderr, func() int64 { tick++; return tick })
	if err != nil {
		t.Fatal(err)
	}
	var report map[string]any
	if err := json.Unmarshal(stdout.Bytes(), &report); err != nil {
		t.Fatal(err)
	}
	for _, key := range []string{"workload", "invocation", "revision", "runtime", "os", "arch", "config", "batch_size", "pairs", "analysis"} {
		if _, ok := report[key]; !ok {
			t.Errorf("missing top-level field %q", key)
		}
	}
	if report["workload"] != "indexed-array-sum-4096" || report["revision"] != "abc" {
		t.Fatalf("unexpected metadata: %+v", report)
	}
}

func TestRunErrorsDoNotWritePartialReport(t *testing.T) {
	for _, args := range [][]string{{"cmd", "--unknown"}, {"cmd", "leftover"}} {
		var out, stderr bytes.Buffer
		err := run(args, &out, &stderr, func() int64 { return 0 })
		if err == nil || out.Len() != 0 {
			t.Fatalf("args %v: err=%v output=%q", args, err, out.String())
		}
	}
	var out, stderr bytes.Buffer
	err := run([]string{"cmd", "--pairs", "2", "--target-ns", "1", "--max-batch", "1"}, &out, &stderr, func() int64 { return 0 })
	if err == nil || out.Len() != 0 || !strings.Contains(err.Error(), "calibration limit") {
		t.Fatalf("err=%v output=%q", err, out.String())
	}
}

func TestRunHelpIsSuccessfulAndEmitsNoJSON(t *testing.T) {
	var out, stderr bytes.Buffer
	err := run([]string{"array-microbenchmark", "--help"}, &out, &stderr, func() int64 { return 0 })
	if err != nil {
		t.Fatal(err)
	}
	if out.Len() != 0 || !strings.Contains(stderr.String(), "Usage of array-microbenchmark") {
		t.Fatalf("unexpected help output: stdout=%q stderr=%q", out.String(), stderr.String())
	}
}

type brokenWriter struct{}

func (brokenWriter) Write([]byte) (int, error) { return 0, errors.New("write failed") }

func TestRunPropagatesOutputFailure(t *testing.T) {
	tick := int64(0)
	err := run([]string{"cmd", "--pairs", "1", "--target-ns", "1", "--max-batch", "1"}, brokenWriter{}, &bytes.Buffer{}, func() int64 { tick++; return tick })
	if err == nil || !strings.Contains(err.Error(), "write failed") {
		t.Fatalf("expected writer failure, got %v", err)
	}
}

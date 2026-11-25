package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"sort"
	"testing"
)

type steppedClock struct{ tick int64 }

func (clock *steppedClock) now() int64 {
	clock.tick += 100
	return clock.tick
}

func TestRunEmitsBalancedSelfReplayedReport(t *testing.T) {
	var stdout, stderr bytes.Buffer
	clock := &steppedClock{}
	args := []string{"heap-construction-benchmark", "--size", "7", "--pairs", "5", "--target-ns", "1", "--max-batch", "1", "--seed", "17", "--revision", "test-revision"}
	if err := run(args, &stdout, &stderr, clock.now); err != nil {
		t.Fatalf("run: %v; stderr=%s", err, stderr.String())
	}
	var result outputReport
	if err := json.Unmarshal(stdout.Bytes(), &result); err != nil {
		t.Fatal(err)
	}
	if result.Workload != workloadID || result.Revision != "test-revision" || len(result.Pairs) != 5 {
		t.Fatalf("unexpected report metadata or pair count: %+v", result)
	}
	if len(result.Invocation) != len(args)-1 || result.Invocation[0] != "--size" {
		t.Fatalf("invocation = %v", result.Invocation)
	}
	baselineFirst := 0
	for index, pair := range result.Pairs {
		if pair.BaselineNS <= 0 || pair.CandidateNS <= 0 {
			t.Fatalf("pair %d has a nonpositive duration: %+v", index, pair)
		}
		if pair.Order == "baseline-first" {
			baselineFirst++
		}
	}
	if baselineFirst != 3 {
		t.Fatalf("baseline-first count = %d, want ceil(5/2)", baselineFirst)
	}
}

func TestInputGenerationIsSeededAndOutputValidationIsIndependent(t *testing.T) {
	first := makeInput(128, 41)
	second := makeInput(128, 41)
	if !equalInts(first, second) {
		t.Fatal("same seed generated different input")
	}
	original := append([]int(nil), first...)
	sorted := append([]int(nil), first...)
	// This oracle uses the standard comparison sort and complete pop sequences,
	// not the heap's internal layout.
	sort.Ints(sorted)
	queue, err := buildByFloyd(first)
	if err != nil {
		t.Fatal(err)
	}
	if err := verifyCompleteHeap(queue, sorted); err != nil {
		t.Fatal(err)
	}
	if err := verifyInputUnchanged(first, original); err != nil {
		t.Fatal(err)
	}
}

func TestSetupValidationRejectsUnexpectedData(t *testing.T) {
	queue, err := buildByFloyd([]int{1, 2})
	if err != nil {
		t.Fatal(err)
	}
	if err := verifyCompleteHeap(queue, []int{1}); err == nil {
		t.Fatal("complete validation accepted the wrong expected length")
	}
	queue, err = buildByFloyd([]int{1, 2})
	if err != nil {
		t.Fatal(err)
	}
	if err := verifyCompleteHeap(queue, nil); err == nil {
		t.Fatal("complete validation accepted leftover elements")
	}
	queue, err = buildByFloyd([]int{1})
	if err != nil {
		t.Fatal(err)
	}
	if err := verifyCompleteHeap(queue, []int{2}); err == nil {
		t.Fatal("complete validation accepted an incorrect pop")
	}
	if err := verifyInputUnchanged([]int{1, 2}, []int{1}); err == nil {
		t.Fatal("input validation accepted a changed length")
	}
	if err := verifyInputUnchanged([]int{1, 3}, []int{1, 2}); err == nil {
		t.Fatal("input validation accepted a changed element")
	}
	empty, err := buildByFloyd(nil)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := observeHeap(empty); err == nil {
		t.Fatal("observation accepted an empty heap")
	}
}

func TestRunRejectsBadSizeFlagsAndClock(t *testing.T) {
	for _, args := range [][]string{
		{"heap-construction-benchmark", "--size", "0"},
		{"heap-construction-benchmark", "--size", fmt.Sprint(maxInputElements + 1)},
		{"heap-construction-benchmark", "extra"},
		{"heap-construction-benchmark", "--revision", ""},
	} {
		if err := run(args, io.Discard, io.Discard, (&steppedClock{}).now); err == nil {
			t.Fatalf("run(%v) unexpectedly succeeded", args)
		}
	}
	if err := run([]string{"heap-construction-benchmark"}, io.Discard, io.Discard, nil); err == nil {
		t.Fatal("run accepted a nil clock")
	}
	if err := run([]string{"heap-construction-benchmark", "--size", "5", "--pairs", "3", "--target-ns", "1", "--max-batch", "1"}, failingWriter{}, io.Discard, (&steppedClock{}).now); err == nil {
		t.Fatal("run suppressed an output error")
	}
	for _, args := range [][]string{
		{"heap-construction-benchmark", "--pairs", "0"},
		{"heap-construction-benchmark", "--confidence", "100"},
		{"heap-construction-benchmark", "--factor", "1"},
		{"heap-construction-benchmark", "--target-ns", "0"},
		{"heap-construction-benchmark", "--max-batch", "0"},
	} {
		if err := run(args, io.Discard, io.Discard, (&steppedClock{}).now); err == nil {
			t.Errorf("run(%v) accepted invalid collector configuration", args)
		}
	}
}

func TestNativeCLIEntrypoint(t *testing.T) {
	readEnd, writeEnd, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	oldArgs, oldStdout := os.Args, os.Stdout
	os.Args = []string{"heap-construction-benchmark", "--size", "16", "--pairs", "1", "--target-ns", "1", "--max-batch", "1", "--revision", "cli-test"}
	os.Stdout = writeEnd
	main()
	os.Args, os.Stdout = oldArgs, oldStdout
	if err := writeEnd.Close(); err != nil {
		t.Fatal(err)
	}
	output, err := io.ReadAll(readEnd)
	if err != nil {
		t.Fatal(err)
	}
	if err := readEnd.Close(); err != nil {
		t.Fatal(err)
	}
	var report outputReport
	if err := json.Unmarshal(output, &report); err != nil {
		t.Fatalf("native CLI report: %v\n%s", err, output)
	}
	if report.Workload != workloadID || report.Revision != "cli-test" || len(report.Pairs) != 1 {
		t.Fatalf("unexpected native CLI report: %+v", report)
	}
}

func TestWriteAllPropagatesBrokenOutput(t *testing.T) {
	if err := writeAll(failingWriter{}, []byte("report")); err == nil {
		t.Fatal("writeAll suppressed output error")
	}
	if err := writeAll(stalledWriter{}, []byte("report")); !errors.Is(err, io.ErrShortWrite) {
		t.Fatalf("writeAll stalled writer = %v, want io.ErrShortWrite", err)
	}
	if err := writeAll(byteWriter{}, []byte("report")); err != nil {
		t.Fatalf("writeAll partial writer: %v", err)
	}
}

type failingWriter struct{}

func (failingWriter) Write([]byte) (int, error) { return 0, errors.New("closed writer") }

type stalledWriter struct{}

func (stalledWriter) Write([]byte) (int, error) { return 0, nil }

type byteWriter struct{}

func (byteWriter) Write(value []byte) (int, error) {
	if len(value) == 0 {
		return 0, nil
	}
	return 1, nil
}

func equalInts(left, right []int) bool {
	if len(left) != len(right) {
		return false
	}
	for index := range left {
		if left[index] != right[index] {
			return false
		}
	}
	return true
}

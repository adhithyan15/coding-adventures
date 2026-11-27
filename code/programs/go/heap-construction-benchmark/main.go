// Command heap-construction-benchmark compares checked repeated insertion
// with bottom-up construction from an identical immutable input slice.
package main

import (
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"math/rand"
	"os"
	"runtime"
	"sort"

	"github.com/adhithyan15/coding-adventures/code/packages/go/heap"
	"github.com/adhithyan15/coding-adventures/code/packages/go/microbenchmark"
)

const (
	workloadID       = "heap-construction-repeated-push-vs-floyd"
	maxInputElements = 100_000
)

type outputReport struct {
	microbenchmark.Report
	Workload   string   `json:"workload"`
	Invocation []string `json:"invocation"`
	Revision   string   `json:"revision"`
	Runtime    string   `json:"runtime"`
	OS         string   `json:"os"`
	Arch       string   `json:"arch"`
}

func main() {
	if err := run(os.Args, os.Stdout, os.Stderr, microbenchmark.MonotonicClock()); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string, stdout, stderr io.Writer, clock microbenchmark.Clock) error {
	if len(args) == 0 {
		args = []string{"heap-construction-benchmark"}
	}
	config := microbenchmark.Defaults()
	size := 1024
	flags := flag.NewFlagSet("heap-construction-benchmark", flag.ContinueOnError)
	flags.SetOutput(stderr)
	flags.IntVar(&size, "size", size, "number of integers in each constructed heap (1 through 100000)")
	flags.IntVar(&config.Pairs, "pairs", config.Pairs, "number of paired observations")
	flags.IntVar(&config.ConfidencePercent, "confidence", config.ConfidencePercent, "confidence percentage")
	flags.Float64Var(&config.MinimumFactor, "factor", config.MinimumFactor, "minimum meaningful factor")
	flags.Int64Var(&config.Seed, "seed", config.Seed, "input and schedule seed")
	flags.Int64Var(&config.TargetNS, "target-ns", config.TargetNS, "minimum calibration batch duration")
	flags.IntVar(&config.MaxBatch, "max-batch", config.MaxBatch, "maximum calibration batch size")
	revision := flags.String("revision", "unknown", "source revision")
	if err := flags.Parse(args[1:]); errors.Is(err, flag.ErrHelp) {
		return nil
	} else if err != nil {
		return err
	}
	if flags.NArg() != 0 {
		return errors.New("unexpected positional arguments")
	}
	if size < 1 || size > maxInputElements {
		return fmt.Errorf("size must be from 1 to %d", maxInputElements)
	}
	if *revision == "" {
		return errors.New("revision must be nonempty")
	}
	if clock == nil {
		return errors.New("clock is required")
	}

	// Input generation and the complete correctness comparison happen before
	// the collector starts its clock. Both timed variants share this slice.
	values := makeInput(size, config.Seed)
	inputSnapshot := append([]int(nil), values...)
	expectedSorted := append([]int(nil), values...)
	sort.Ints(expectedSorted)
	repeated, err := buildByRepeatedPush(values)
	if err != nil {
		return err
	}
	floyd, err := buildByFloyd(values)
	if err != nil {
		return err
	}
	if err := verifyCompleteHeap(repeated, expectedSorted); err != nil {
		return fmt.Errorf("repeated-push validation: %w", err)
	}
	if err := verifyCompleteHeap(floyd, expectedSorted); err != nil {
		return fmt.Errorf("Floyd validation: %w", err)
	}
	if err := verifyInputUnchanged(values, inputSnapshot); err != nil {
		return err
	}

	checksum := heapChecksum(size, expectedSorted[0])
	baseline := func(repetitions int) (int64, error) {
		var total int64
		for range repetitions {
			built, buildErr := buildByRepeatedPush(values)
			if buildErr != nil {
				return 0, buildErr
			}
			observed, observeErr := observeHeap(built)
			if observeErr != nil {
				return 0, observeErr
			}
			total += observed
		}
		return total, nil
	}
	candidate := func(repetitions int) (int64, error) {
		var total int64
		for range repetitions {
			built, buildErr := buildByFloyd(values)
			if buildErr != nil {
				return 0, buildErr
			}
			observed, observeErr := observeHeap(built)
			if observeErr != nil {
				return 0, observeErr
			}
			total += observed
		}
		return total, nil
	}
	report, err := microbenchmark.Collect(microbenchmark.CollectOptions{
		Config: config, ExpectedChecksum: checksum,
		Baseline: baseline, Candidate: candidate, Now: clock,
	})
	if err != nil {
		return err
	}
	result := outputReport{
		Report: report, Workload: workloadID,
		Invocation: append([]string{}, args[1:]...), Revision: *revision,
		Runtime: runtime.Version(), OS: runtime.GOOS, Arch: runtime.GOARCH,
	}
	encoded, err := json.MarshalIndent(result, "", "  ")
	if err != nil {
		return fmt.Errorf("encode report: %w", err)
	}
	encoded = append(encoded, '\n')
	return writeAll(stdout, encoded)
}

func makeInput(size int, seed int64) []int {
	random := rand.New(rand.NewSource(seed))
	values := make([]int, size)
	for index := range values {
		values[index] = random.Intn(1_000_000)
	}
	return values
}

func buildByRepeatedPush(values []int) (*heap.BoundedHeap[int], error) {
	queue, err := heap.NewBoundedHeap(len(values), func(a, b int) bool { return a < b })
	if err != nil {
		return nil, err
	}
	for _, value := range values {
		if err := queue.Push(value); err != nil {
			return nil, err
		}
	}
	return queue, nil
}

func buildByFloyd(values []int) (*heap.BoundedHeap[int], error) {
	return heap.NewBoundedHeapFromSlice(values, len(values), func(a, b int) bool { return a < b })
}

// verifyCompleteHeap drains only the two setup heaps, outside timed collection.
func verifyCompleteHeap(queue *heap.BoundedHeap[int], expected []int) error {
	if queue.Len() != len(expected) {
		return fmt.Errorf("length = %d, want %d", queue.Len(), len(expected))
	}
	for index, want := range expected {
		got, ok := queue.Pop()
		if !ok || got != want {
			return fmt.Errorf("pop %d = %d,%v, want %d", index, got, ok, want)
		}
	}
	if _, ok := queue.Peek(); ok {
		return errors.New("queue remained nonempty after complete validation")
	}
	return nil
}

func verifyInputUnchanged(values, snapshot []int) error {
	if len(values) != len(snapshot) {
		return errors.New("benchmark input length changed")
	}
	for index := range values {
		if values[index] != snapshot[index] {
			return fmt.Errorf("benchmark input changed at index %d", index)
		}
	}
	return nil
}

// observeHeap reads a constant-size view rather than draining a heap inside the
// timed construction callback. Full contents were checked in setup first.
func observeHeap(queue *heap.BoundedHeap[int]) (int64, error) {
	minimum, ok := queue.Peek()
	if !ok {
		return 0, errors.New("constructed heap is empty")
	}
	return heapChecksum(queue.Len(), minimum), nil
}

func heapChecksum(length, minimum int) int64 {
	return int64(length)*1_000_003 + int64(minimum)
}

func writeAll(writer io.Writer, content []byte) error {
	for len(content) > 0 {
		written, err := writer.Write(content)
		if err != nil {
			return err
		}
		if written <= 0 || written > len(content) {
			return io.ErrShortWrite
		}
		content = content[written:]
	}
	return nil
}

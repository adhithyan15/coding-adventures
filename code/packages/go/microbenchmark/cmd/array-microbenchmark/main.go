// Command array-microbenchmark runs the reference indexed-array experiment.
package main

import (
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/microbenchmark"
)

type output struct {
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
	c := microbenchmark.Defaults()
	flags := flag.NewFlagSet("array-microbenchmark", flag.ContinueOnError)
	flags.SetOutput(stderr)
	flags.IntVar(&c.Pairs, "pairs", c.Pairs, "number of paired observations")
	flags.IntVar(&c.ConfidencePercent, "confidence", c.ConfidencePercent, "confidence percentage")
	flags.Float64Var(&c.MinimumFactor, "factor", c.MinimumFactor, "minimum meaningful factor")
	flags.Int64Var(&c.Seed, "seed", c.Seed, "schedule random seed")
	flags.Int64Var(&c.TargetNS, "target-ns", c.TargetNS, "minimum calibration batch duration")
	flags.IntVar(&c.MaxBatch, "max-batch", c.MaxBatch, "maximum calibration batch size")
	revision := flags.String("revision", "unknown", "source revision")
	if err := flags.Parse(args[1:]); errors.Is(err, flag.ErrHelp) {
		return nil
	} else if err != nil {
		return err
	}
	if flags.NArg() != 0 {
		return fmt.Errorf("unexpected positional arguments")
	}
	array := make([]int64, 4096)
	var checksum int64
	for i := range array {
		array[i] = int64((17*i + 3) % 251)
		checksum += array[i]
	}
	if checksum != 511533 {
		return fmt.Errorf("internal indexed-array checksum mismatch")
	}
	baseline := func(n int) (int64, error) {
		var sum int64
		for j := 0; j < n; j++ {
			for i := 0; i < len(array); i++ {
				sum += array[i]
			}
		}
		return sum, nil
	}
	candidate := func(n int) (int64, error) {
		var sum int64
		for j := 0; j < n; j++ {
			for _, v := range array {
				sum += v
			}
		}
		return sum, nil
	}
	report, err := microbenchmark.Collect(microbenchmark.CollectOptions{Config: c, ExpectedChecksum: checksum, Baseline: baseline, Candidate: candidate, Now: clock})
	if err != nil {
		return err
	}
	result := output{Report: report, Workload: "indexed-array-sum-4096", Invocation: args, Revision: *revision, Runtime: runtime.Version(), OS: runtime.GOOS, Arch: runtime.GOARCH}
	enc := json.NewEncoder(stdout)
	enc.SetIndent("", "  ")
	return enc.Encode(result)
}

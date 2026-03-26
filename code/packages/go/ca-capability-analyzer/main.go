package main

// main.go is the CLI entry point for ca-capability-analyzer.
//
// Usage:
//
//	ca-capability-analyzer [--dir <path>] [--verbose]
//
// Options:
//
//	--dir      Path to the Go package directory to analyze. Default: current directory.
//	--verbose  Print detected capabilities even when the analysis passes.
//
// Exit codes:
//
//	0  All detected capabilities are declared in the manifest; no banned constructs.
//	1  One or more violations found (undeclared capability or banned construct).
//	2  Tool error (directory not found, parse error, internal failure).
//
// # Architecture: main() vs run()
//
// The logic is split into two functions:
//   - main() handles flag parsing and os.Exit. It cannot be unit-tested because
//     os.Exit terminates the process.
//   - run() does the actual work and returns an exit code. It is unit-testable.

import (
	"flag"
	"fmt"
	"io"
	"os"
)

// run executes the analyzer with the given arguments and writes output to stdout/stderr.
//
// It is separated from main() so that tests can call it directly without
// triggering os.Exit. Returns the exit code: 0 = pass, 1 = violations, 2 = error.
func run(dir string, verbose bool, stdout, stderr io.Writer) int {
	result, err := AnalyzeDir(dir)
	if err != nil {
		fmt.Fprintf(stderr, "ca-capability-analyzer: %v\n", err)
		return 2
	}

	// Always print parse errors to stderr. A file that could not be parsed is
	// not analyzed, so violations within it go undetected. Surfacing this as
	// a visible warning prevents a false-clean exit code 0.
	for _, pe := range result.ParseErrors {
		fmt.Fprintf(stderr, "ca-capability-analyzer: warning: %s\n", pe)
	}

	// With --verbose (or when violations exist), print all detected capabilities
	// so the developer can see what was found even on a passing run.
	if verbose || !result.Passed() {
		if len(result.Detected) > 0 {
			fmt.Fprintln(stdout, "Detected capabilities:")
			for _, d := range result.Detected {
				fmt.Fprintf(stdout, "  %s:%d: %s (%s)\n", d.File, d.Line, d.Capability, d.Evidence)
			}
		}
		if len(result.Banned) > 0 {
			fmt.Fprintln(stdout, "Banned constructs:")
			for _, b := range result.Banned {
				fmt.Fprintf(stdout, "  %s:%d: %s\n", b.File, b.Line, b.Kind)
			}
		}
	}

	// Always print violations.
	for _, v := range result.Violations {
		fmt.Fprintln(stdout, v.Format())
	}

	if !result.Passed() {
		return 1
	}

	if verbose {
		fmt.Fprintf(stdout, "ca-capability-analyzer: %s passed\n", result.Dir)
	}
	return 0
}

func main() {
	dir := flag.String("dir", ".", "path to Go package directory to analyze")
	verbose := flag.Bool("verbose", false, "show detected capabilities even when passing")
	flag.Parse()

	os.Exit(run(*dir, *verbose, os.Stdout, os.Stderr))
}

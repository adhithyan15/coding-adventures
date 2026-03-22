// =========================================================================
// nproc — Print Number of Processing Units
// =========================================================================
//
// The `nproc` utility prints the number of processing units (CPUs/cores)
// available to the current process. This is commonly used in build scripts
// to parallelize compilation:
//
//   make -j$(nproc)     # use all available CPUs for building
//
// # Available vs. installed CPUs
//
// On most systems, all installed CPUs are available. However, some
// environments restrict CPU access:
//
//   - cgroups (Docker containers): may limit to fewer CPUs
//   - taskset: pins a process to specific CPUs
//   - CPU hotplug: some CPUs may be offline
//
// The `--all` flag shows the total installed count, while the default
// shows the count available to the current process. In Go, runtime.NumCPU()
// returns the number of CPUs available to the process, which respects
// cgroup limits on Linux.
//
// # The --ignore flag
//
// Sometimes you want to leave some CPUs free for other tasks:
//
//   make -j$(nproc --ignore=2)   # use all but 2 CPUs
//
// The result is always at least 1 (you can't have zero CPUs).
//
// # Architecture
//
//   nproc.json (spec)          nproc_tool.go (this file)
//   ┌──────────────────┐     ┌──────────────────────────────┐
//   │ flag: --all       │     │ get CPU count (NumCPU)       │
//   │ flag: --ignore N  │────>│ subtract ignore value         │
//   │ help, version     │     │ clamp to minimum 1            │
//   └──────────────────┘     └──────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"runtime"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// calculateNproc — compute the number of available processors
// =========================================================================
//
// This function computes the number of processors to report, taking into
// account the --ignore flag. The result is always at least 1.
//
// Parameters:
//   - totalCPUs: the total number of CPUs (from runtime.NumCPU())
//   - ignore: number of CPUs to subtract (from --ignore flag, 0 if not set)
//
// Returns:
//   The adjusted CPU count, with a minimum of 1.
//
// Examples:
//   calculateNproc(8, 0)  => 8   (no adjustment)
//   calculateNproc(8, 2)  => 6   (subtract 2)
//   calculateNproc(4, 10) => 1   (clamped to minimum 1)
//   calculateNproc(1, 0)  => 1   (single CPU)

func calculateNproc(totalCPUs int, ignore int) int {
	result := totalCPUs - ignore
	if result < 1 {
		return 1
	}
	return result
}

// =========================================================================
// runNproc — the testable entry point for the nproc tool
// =========================================================================
//
// The nproc tool:
//   1. Parses arguments using cli-builder.
//   2. Gets the CPU count via runtime.NumCPU().
//   3. Subtracts the --ignore value (if provided).
//   4. Prints the result (clamped to minimum 1).

func runNproc(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "nproc: %s\n", err)
		return 1
	}

	// Step 2: Parse the arguments.
	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 1
	}

	// Step 3: Handle the result.
	switch r := result.(type) {

	case *clibuilder.HelpResult:
		fmt.Fprintln(stdout, r.Text)
		return 0

	case *clibuilder.VersionResult:
		fmt.Fprintln(stdout, r.Version)
		return 0

	case *clibuilder.ParseResult:
		// Get the total CPU count.
		// In Go, runtime.NumCPU() already respects cgroup limits on Linux,
		// so --all and default give the same result in most Go programs.
		totalCPUs := runtime.NumCPU()

		// Extract the --ignore flag value.
		// The cli-builder returns integer flags as int64.
		ignore := 0
		if v, ok := r.Flags["ignore"].(int64); ok {
			ignore = int(v)
		}

		// Calculate and print the result.
		count := calculateNproc(totalCPUs, ignore)
		fmt.Fprintln(stdout, count)
		return 0

	default:
		fmt.Fprintf(stderr, "nproc: unexpected result type: %T\n", result)
		return 1
	}
}

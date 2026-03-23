// =========================================================================
// tee — Read from stdin, Write to stdout and Files
// =========================================================================
//
// The `tee` utility copies standard input to standard output AND to one
// or more files simultaneously. Its name comes from the T-splitter used
// in plumbing — input flows in one end and out two (or more) ends.
//
// # Basic usage
//
//   echo "hello" | tee file.txt             Write to stdout AND file.txt
//   echo "hello" | tee file1.txt file2.txt  Write to stdout AND two files
//   echo "hello" | tee -a file.txt          Append instead of overwrite
//
// # Why is tee useful?
//
// tee is invaluable in pipelines where you want to save intermediate
// results while still passing data to the next command:
//
//   cat log.txt | grep ERROR | tee errors.txt | wc -l
//   (saves matching lines to errors.txt AND counts them)
//
// # Flags
//
//   -a    Append to the given files instead of overwriting them.
//   -i    Ignore interrupt signals (SIGINT). Useful in pipelines where
//         you don't want Ctrl-C to stop tee before it finishes writing.
//
// # Architecture
//
//   tee.json (spec)            tee_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -a, -i    │      │ open all output files          │
//   │ variadic FILE    │─────>│ read stdin                     │
//   │ help, version    │      │ write to stdout + all files    │
//   └──────────────────┘      └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// runTee — the testable core of the tee tool
// =========================================================================

func runTee(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runTeeWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runTeeWithStdin is the inner implementation that accepts a custom stdin.

func runTeeWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "tee: %s\n", err)
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
		// Extract flags.
		appendMode := getBool(r.Flags, "append")

		// Extract file paths.
		files := getStringSlice(r.Arguments, "files")

		// Open all output files.
		//
		// We collect all writers (stdout + files) into a slice.
		// io.MultiWriter will write to all of them simultaneously.
		writers := []io.Writer{stdout}
		exitCode := 0

		for _, file := range files {
			flag := os.O_WRONLY | os.O_CREATE
			if appendMode {
				flag |= os.O_APPEND
			} else {
				flag |= os.O_TRUNC
			}

			f, err := os.OpenFile(file, flag, 0644)
			if err != nil {
				fmt.Fprintf(stderr, "tee: %s: %s\n", file, err)
				exitCode = 1
				continue
			}
			defer f.Close()
			writers = append(writers, f)
		}

		// Create a MultiWriter that writes to all destinations.
		//
		// io.MultiWriter is like a plumbing T-junction: everything
		// written to it gets duplicated to all underlying writers.
		multiWriter := io.MultiWriter(writers...)

		// Copy stdin to all writers.
		if _, err := io.Copy(multiWriter, stdin); err != nil {
			fmt.Fprintf(stderr, "tee: %s\n", err)
			return 1
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "tee: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// tail — Output the Last Part of Files
// =========================================================================
//
// The `tail` utility displays the end of a file. By default, it prints
// the last 10 lines of each specified file. It is the complement of `head`.
//
// # Basic usage
//
//   tail file.txt              Print last 10 lines of file.txt
//   tail -n 5 file.txt         Print last 5 lines
//   tail -n +5 file.txt        Print starting from line 5
//   tail -c 100 file.txt       Print last 100 bytes
//   tail file1.txt file2.txt   Print last 10 lines of each file
//   tail                       Read from standard input
//
// # The +NUM syntax
//
// A unique feature of tail is the +NUM prefix. While -n 5 means "last 5
// lines", -n +5 means "starting from line 5" (i.e., skip the first 4
// lines and print the rest). This is why -n has type "string" — it needs
// to parse the optional + prefix.
//
//   tail -n 5 file.txt    =>  last 5 lines
//   tail -n +5 file.txt   =>  everything from line 5 onward
//
// # Flags
//
//   -n NUM    Output the last NUM lines (default 10). +NUM outputs
//             starting from line NUM.
//   -c NUM    Output the last NUM bytes. +NUM outputs starting from
//             byte NUM.
//   -f        Follow — output appended data as the file grows.
//             (Not fully implemented in this version.)
//   --retry   Keep trying to open a file if it is inaccessible.
//   -q        Never output headers giving file names.
//   -v        Always output headers giving file names.
//   -z        Line delimiter is NUL, not newline.
//
// # Architecture
//
//   tail.json (spec)           tail_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -n,-c     │      │ for each file:                 │
//   │ -f,--retry       │─────>│   read all lines into memory   │
//   │ -q,-v,-z         │      │   output last N lines/bytes    │
//   │ variadic FILE    │      │   write to stdout              │
//   └──────────────────┘      └────────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// tailLines — output the last N lines from a reader
// =========================================================================
//
// To get the last N lines, we need to read the entire input first (since
// we don't know how many lines there are until we reach EOF). We use a
// circular buffer of size N to keep only the last N lines in memory.
//
// This is the classic "ring buffer" approach:
//
//   Input lines:  [a, b, c, d, e, f, g]   with N=3
//   Ring buffer:  [e, f, g]  (positions wrap around)
//
// For +NUM mode (fromStart=true), we skip the first (num-1) lines and
// print everything else.

func tailLines(reader io.Reader, stdout io.Writer, num int, fromStart bool, delimiter byte) error {
	scanner := bufio.NewScanner(reader)
	if delimiter == 0 {
		scanner.Split(splitOnNUL)
	}

	buf := make([]byte, 0, 1024*1024)
	scanner.Buffer(buf, 1024*1024)

	if fromStart {
		// +NUM mode: skip the first (num-1) lines, print the rest.
		lineIndex := 1
		for scanner.Scan() {
			if lineIndex >= num {
				fmt.Fprint(stdout, scanner.Text())
				stdout.Write([]byte{delimiter})
			}
			lineIndex++
		}
		return scanner.Err()
	}

	// Normal mode: collect all lines, then print the last N.
	// We use a simple slice here. For very large files, a ring buffer
	// would be more memory-efficient, but for correctness and clarity
	// this approach is preferred.
	var lines []string
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	if err := scanner.Err(); err != nil {
		return err
	}

	// Determine the starting index.
	start := len(lines) - num
	if start < 0 {
		start = 0
	}

	for i := start; i < len(lines); i++ {
		fmt.Fprint(stdout, lines[i])
		stdout.Write([]byte{delimiter})
	}

	return nil
}

// =========================================================================
// tailBytes — output the last N bytes from a reader
// =========================================================================
//
// Similar to tailLines, we read the entire input and then output the
// last N bytes. For +NUM mode, we skip the first (num-1) bytes.

func tailBytes(reader io.Reader, stdout io.Writer, num int, fromStart bool) error {
	data, err := io.ReadAll(reader)
	if err != nil {
		return err
	}

	if fromStart {
		// +NUM mode: output starting from byte position (num-1).
		start := num - 1
		if start < 0 {
			start = 0
		}
		if start < len(data) {
			stdout.Write(data[start:])
		}
		return nil
	}

	// Normal mode: output the last num bytes.
	start := len(data) - num
	if start < 0 {
		start = 0
	}
	stdout.Write(data[start:])
	return nil
}

// =========================================================================
// parseTailNum — parse a tail count argument like "10", "+5", "-3"
// =========================================================================
//
// tail's -n and -c flags accept an optional + prefix:
//   "10"  => count=10, fromStart=false  (last 10)
//   "+5"  => count=5,  fromStart=true   (from line/byte 5)
//   "-3"  => count=3,  fromStart=false  (last 3, same as "3")
//
// Returns the parsed count and whether it's a "from start" specification.

func parseTailNum(s string) (int, bool, error) {
	s = strings.TrimSpace(s)
	if s == "" {
		return 10, false, nil
	}

	fromStart := false
	if strings.HasPrefix(s, "+") {
		fromStart = true
		s = s[1:]
	} else if strings.HasPrefix(s, "-") {
		s = s[1:]
	}

	n, err := strconv.Atoi(s)
	if err != nil {
		return 0, false, fmt.Errorf("invalid number: %q", s)
	}

	return n, fromStart, nil
}

// =========================================================================
// runTail — the testable core of the tail tool
// =========================================================================

func runTail(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runTailWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runTailWithStdin is the inner implementation that accepts a custom stdin.

func runTailWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "tail: %s\n", err)
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
		quiet := getBool(r.Flags, "quiet")
		verbose := getBool(r.Flags, "verbose")
		zeroTerminated := getBool(r.Flags, "zero_terminated")

		// Determine the line delimiter.
		delimiter := byte('\n')
		if zeroTerminated {
			delimiter = 0
		}

		// Determine whether we're counting lines or bytes.
		byteMode := false
		byteCount := 0
		byteFromStart := false
		lineCount := 10
		lineFromStart := false

		if v, ok := r.Flags["bytes"]; ok {
			if s, ok := v.(string); ok && s != "" {
				n, fs, err := parseTailNum(s)
				if err != nil {
					fmt.Fprintf(stderr, "tail: %s\n", err)
					return 1
				}
				byteMode = true
				byteCount = n
				byteFromStart = fs
			}
		}

		if !byteMode {
			if v, ok := r.Flags["lines"]; ok {
				if s, ok := v.(string); ok && s != "" {
					n, fs, err := parseTailNum(s)
					if err != nil {
						fmt.Fprintf(stderr, "tail: %s\n", err)
						return 1
					}
					lineCount = n
					lineFromStart = fs
				}
			}
		}

		// Extract file paths.
		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			files = []string{"-"}
		}

		// Determine whether to show headers.
		showHeaders := len(files) > 1
		if quiet {
			showHeaders = false
		}
		if verbose {
			showHeaders = true
		}

		// Process each file.
		exitCode := 0
		for i, file := range files {
			var reader io.Reader

			if file == "-" {
				reader = stdin
				if showHeaders {
					printHeader(stdout, "standard input", i == 0)
				}
			} else {
				f, err := os.Open(file)
				if err != nil {
					fmt.Fprintf(stderr, "tail: cannot open '%s' for reading: %s\n", file, err)
					exitCode = 1
					continue
				}
				defer f.Close()
				reader = f
				if showHeaders {
					printHeader(stdout, file, i == 0)
				}
			}

			if byteMode {
				if err := tailBytes(reader, stdout, byteCount, byteFromStart); err != nil {
					fmt.Fprintf(stderr, "tail: error reading '%s': %s\n", file, err)
					exitCode = 1
				}
			} else {
				if err := tailLines(reader, stdout, lineCount, lineFromStart, delimiter); err != nil {
					fmt.Fprintf(stderr, "tail: error reading '%s': %s\n", file, err)
					exitCode = 1
				}
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "tail: unexpected result type: %T\n", result)
		return 1
	}
}

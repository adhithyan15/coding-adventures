// =========================================================================
// head — Output the First Part of Files
// =========================================================================
//
// The `head` utility displays the beginning of a file. By default, it
// prints the first 10 lines of each specified file. If more than one file
// is specified, each is preceded by a header giving the file name.
//
// # Basic usage
//
//   head file.txt              Print first 10 lines of file.txt
//   head -n 5 file.txt         Print first 5 lines
//   head -c 100 file.txt       Print first 100 bytes
//   head file1.txt file2.txt   Print first 10 lines of each file
//   head                       Read from standard input
//
// # Flags
//
//   -n NUM    Print the first NUM lines (default 10).
//   -c NUM    Print the first NUM bytes (overrides -n).
//   -q        Never print headers giving file names (quiet mode).
//   -v        Always print headers giving file names (verbose mode).
//   -z        Use NUL as the line delimiter instead of newline.
//
// # Headers
//
// When multiple files are given, head prints a header before each file:
//
//   ==> filename <==
//
// With -q, headers are suppressed. With -v, headers are always shown
// even for a single file. By default, headers are shown only when there
// are multiple files.
//
// # Architecture
//
//   head.json (spec)           head_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -n,-c     │      │ for each file:                 │
//   │ -q,-v,-z         │─────>│   print header if needed       │
//   │ variadic FILE    │      │   output first N lines/bytes   │
//   │ help, version    │      │   write to stdout              │
//   └──────────────────┘      └────────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// headReader — output the first N lines from a reader
// =========================================================================
//
// Reads from the given reader and writes the first `count` lines to stdout.
// A "line" is defined by the delimiter byte (usually '\n', or '\0' with -z).
//
// Parameters:
//   - reader: the input source (file or stdin)
//   - stdout: where to write the output
//   - count: how many lines to output
//   - delimiter: the line delimiter byte ('\n' or '\0')

func headLines(reader io.Reader, stdout io.Writer, count int, delimiter byte) error {
	scanner := bufio.NewScanner(reader)

	// Configure the scanner to split on our delimiter.
	// By default, bufio.Scanner splits on '\n'. For zero-terminated mode,
	// we need a custom split function.
	if delimiter == 0 {
		scanner.Split(splitOnNUL)
	}

	// Increase buffer size for very long lines.
	buf := make([]byte, 0, 1024*1024)
	scanner.Buffer(buf, 1024*1024)

	linesWritten := 0
	for scanner.Scan() {
		if linesWritten >= count {
			break
		}
		fmt.Fprint(stdout, scanner.Text())
		stdout.Write([]byte{delimiter})
		linesWritten++
	}

	return scanner.Err()
}

// =========================================================================
// splitOnNUL — custom scanner split function for NUL-delimited lines
// =========================================================================
//
// This function tells bufio.Scanner to split input on NUL bytes (\0)
// instead of newlines. It follows the same signature as bufio.SplitFunc.
//
// The split function contract:
//   - advance: how many bytes to consume from the input
//   - token: the bytes to return as the next token
//   - err: any error encountered

func splitOnNUL(data []byte, atEOF bool) (advance int, token []byte, err error) {
	if atEOF && len(data) == 0 {
		return 0, nil, nil
	}
	for i, b := range data {
		if b == 0 {
			return i + 1, data[:i], nil
		}
	}
	if atEOF {
		return len(data), data, nil
	}
	return 0, nil, nil
}

// =========================================================================
// headBytes — output the first N bytes from a reader
// =========================================================================
//
// Reads exactly `count` bytes from the reader (or until EOF, whichever
// comes first) and writes them to stdout.

func headBytes(reader io.Reader, stdout io.Writer, count int) error {
	_, err := io.CopyN(stdout, reader, int64(count))
	if err == io.EOF {
		// EOF before we read `count` bytes is not an error — the file
		// is simply shorter than the requested byte count.
		return nil
	}
	return err
}

// =========================================================================
// printHeader — print the "==> filename <==" header
// =========================================================================
//
// This header format matches GNU coreutils head/tail. The header is
// printed before each file's content when multiple files are processed.

func printHeader(stdout io.Writer, filename string, isFirst bool) {
	if !isFirst {
		fmt.Fprintln(stdout)
	}
	fmt.Fprintf(stdout, "==> %s <==\n", filename)
}

// =========================================================================
// runHead — the testable core of the head tool
// =========================================================================

func runHead(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runHeadWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runHeadWithStdin is the inner implementation that accepts a custom stdin.

func runHeadWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "head: %s\n", err)
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
		// The -c (bytes) flag takes precedence when set.
		//
		// The cli-builder returns integer flags as int64, and nil for
		// flags that weren't specified on the command line.
		byteMode := false
		byteCount := 0
		lineCount := 10 // default

		if v := r.Flags["bytes"]; v != nil {
			if n, ok := v.(int64); ok && n > 0 {
				parsedByteCount, err := intFromInt64(n)
				if err != nil {
					fmt.Fprintf(stderr, "head: invalid byte count: %s\n", err)
					return 1
				}
				byteMode = true
				byteCount = parsedByteCount
			}
		}

		if !byteMode {
			if v := r.Flags["lines"]; v != nil {
				if n, ok := v.(int64); ok {
					parsedLineCount, err := intFromInt64(n)
					if err != nil {
						fmt.Fprintf(stderr, "head: invalid line count: %s\n", err)
						return 1
					}
					lineCount = parsedLineCount
				}
			}
		}

		// Extract file paths.
		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			files = []string{"-"}
		}

		// Determine whether to show headers.
		// Default: show headers when there are multiple files.
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
					fmt.Fprintf(stderr, "head: cannot open '%s' for reading: %s\n", file, err)
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
				if err := headBytes(reader, stdout, byteCount); err != nil {
					fmt.Fprintf(stderr, "head: error reading '%s': %s\n", file, err)
					exitCode = 1
				}
			} else {
				if err := headLines(reader, stdout, lineCount, delimiter); err != nil {
					fmt.Fprintf(stderr, "head: error reading '%s': %s\n", file, err)
					exitCode = 1
				}
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "head: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// paste — Merge Lines of Files
// =========================================================================
//
// The `paste` utility merges corresponding lines from multiple files,
// joining them with a delimiter (tab by default).
//
// # How paste works
//
// Think of paste as "horizontal cat". While cat concatenates files
// vertically (top to bottom), paste concatenates them horizontally
// (side by side).
//
//   file1.txt:    file2.txt:    paste file1 file2:
//   Alice         25            Alice   25
//   Bob           30            Bob     30
//   Carol         22            Carol   22
//
// # Serial mode (-s)
//
// In serial mode, paste reads each file separately and merges all its
// lines into a single output line:
//
//   paste -s file1.txt → Alice  Bob  Carol
//   paste -s file2.txt → 25  30  22
//
// # Custom delimiters (-d)
//
// The -d flag specifies a list of delimiters to use. If multiple
// delimiters are given, they cycle:
//
//   paste -d ',;' f1 f2 f3 → line1,line1;line1
//
// # Architecture
//
//   paste.json (spec)            paste_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -d,-s,-z   │       │ open all files                   │
//   │ arg: FILES...     │──────>│ read lines round-robin           │
//   │                   │       │ join with delimiter               │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// pasteFiles — merge lines from multiple readers
// =========================================================================
//
// In parallel mode (default): reads one line from each reader per output
// line, joining them with the delimiter.
//
// In serial mode (-s): reads all lines from each reader and joins them
// into a single output line.

func pasteFiles(readers []io.Reader, delimiters string, serial bool) []string {
	if delimiters == "" {
		delimiters = "\t"
	}

	delimRunes := []rune(delimiters)

	if serial {
		return pasteSerial(readers, delimRunes)
	}
	return pasteParallel(readers, delimRunes)
}

// =========================================================================
// pasteParallel — merge corresponding lines from all files
// =========================================================================
//
// Algorithm:
//   1. Create a scanner for each reader
//   2. Repeat until all scanners are exhausted:
//      a. Read one line from each scanner
//      b. Join them with cycling delimiters
//      c. Append to output

func pasteParallel(readers []io.Reader, delimRunes []rune) []string {
	scanners := make([]*bufio.Scanner, len(readers))
	for i, r := range readers {
		scanners[i] = bufio.NewScanner(r)
	}

	var output []string

	for {
		// Read one line from each scanner.
		var parts []string
		anyAlive := false

		for i, scanner := range scanners {
			_ = i
			if scanner.Scan() {
				parts = append(parts, scanner.Text())
				anyAlive = true
			} else {
				parts = append(parts, "")
			}
		}

		if !anyAlive {
			break
		}

		// Join parts with cycling delimiters.
		var line strings.Builder
		for i, part := range parts {
			if i > 0 {
				delimIdx := (i - 1) % len(delimRunes)
				line.WriteRune(delimRunes[delimIdx])
			}
			line.WriteString(part)
		}

		output = append(output, line.String())
	}

	return output
}

// =========================================================================
// pasteSerial — merge all lines from each file into one output line
// =========================================================================

func pasteSerial(readers []io.Reader, delimRunes []rune) []string {
	var output []string

	for _, r := range readers {
		scanner := bufio.NewScanner(r)
		var lines []string
		for scanner.Scan() {
			lines = append(lines, scanner.Text())
		}

		// Join all lines from this file with cycling delimiters.
		var line strings.Builder
		for i, l := range lines {
			if i > 0 {
				delimIdx := (i - 1) % len(delimRunes)
				line.WriteRune(delimRunes[delimIdx])
			}
			line.WriteString(l)
		}
		output = append(output, line.String())
	}

	return output
}

// =========================================================================
// runPaste — the testable core of the paste tool
// =========================================================================

func runPaste(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runPasteWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runPasteWithStdin is the inner implementation with injectable stdin.

func runPasteWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "paste: %s\n", err)
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
		delimiters := "\t"
		if d, ok := r.Flags["delimiters"].(string); ok && d != "" {
			delimiters = d
		}
		serial := getBool(r.Flags, "serial")

		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			files = []string{"-"}
		}

		// Open all files.
		readers := make([]io.Reader, 0, len(files))
		for _, filename := range files {
			if filename == "-" {
				readers = append(readers, stdin)
			} else {
				f, err := os.Open(filename)
				if err != nil {
					fmt.Fprintf(stderr, "paste: %s: %s\n", filename, err)
					return 1
				}
				defer f.Close()
				readers = append(readers, f)
			}
		}

		// Merge and output.
		outputLines := pasteFiles(readers, delimiters, serial)
		for _, line := range outputLines {
			fmt.Fprintln(stdout, line)
		}

		return 0

	default:
		fmt.Fprintf(stderr, "paste: unexpected result type: %T\n", result)
		return 1
	}
}

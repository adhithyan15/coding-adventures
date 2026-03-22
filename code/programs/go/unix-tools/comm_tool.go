// =========================================================================
// comm — Compare Two Sorted Files Line by Line
// =========================================================================
//
// The `comm` utility compares two sorted files and produces three columns
// of output:
//
//   Column 1: Lines unique to FILE1
//   Column 2: Lines unique to FILE2
//   Column 3: Lines common to both files
//
// # Visual example
//
//   FILE1:      FILE2:      comm FILE1 FILE2:
//   apple       banana      apple           ← only in FILE1
//   banana      cherry              banana  ← only in FILE2 (wait, no...)
//   cherry      date
//   fig
//
// Actually, let me show a proper example:
//
//   FILE1 (sorted):   FILE2 (sorted):   Output:
//   apple             banana            apple
//   banana            cherry                    banana
//   cherry                                      cherry
//   fig                                 fig
//
// Column 1 has no leading tabs.
// Column 2 has one leading tab.
// Column 3 has two leading tabs.
//
// # Suppression flags
//
//   -1  Suppress column 1 (lines unique to FILE1)
//   -2  Suppress column 2 (lines unique to FILE2)
//   -3  Suppress column 3 (lines common to both)
//
// Common combinations:
//   comm -12 FILE1 FILE2  → show only common lines
//   comm -23 FILE1 FILE2  → show only lines unique to FILE1
//   comm -13 FILE1 FILE2  → show only lines unique to FILE2
//
// # Architecture
//
//   comm.json (spec)             comm_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -1,-2,-3   │       │ merge-compare two sorted lists   │
//   │ --output-delimiter│──────>│ classify each line into column   │
//   │ args: FILE1 FILE2 │       │ format with tab prefixes         │
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
// CommOptions — configuration for the comm comparison
// =========================================================================

type CommOptions struct {
	SuppressCol1    bool   // -1: don't show lines unique to FILE1
	SuppressCol2    bool   // -2: don't show lines unique to FILE2
	SuppressCol3    bool   // -3: don't show lines common to both
	OutputDelimiter string // default: tab
}

// =========================================================================
// CommLine — a single line of comm output with its column assignment
// =========================================================================

type CommLine struct {
	Text   string // the actual line text
	Column int    // which column: 1, 2, or 3
}

// =========================================================================
// compareFiles — the core comm algorithm
// =========================================================================
//
// This implements a merge-compare on two sorted sequences, similar to
// the merge step of merge sort. We walk through both lists simultaneously:
//
//   - If line1 < line2: line1 is unique to FILE1 (column 1), advance FILE1
//   - If line1 > line2: line2 is unique to FILE2 (column 2), advance FILE2
//   - If line1 == line2: common line (column 3), advance both
//
// This is O(n + m) where n and m are the lengths of the two files.

func compareFiles(lines1, lines2 []string) []CommLine {
	var result []CommLine
	i, j := 0, 0

	for i < len(lines1) && j < len(lines2) {
		if lines1[i] < lines2[j] {
			// Line is unique to FILE1.
			result = append(result, CommLine{Text: lines1[i], Column: 1})
			i++
		} else if lines1[i] > lines2[j] {
			// Line is unique to FILE2.
			result = append(result, CommLine{Text: lines2[j], Column: 2})
			j++
		} else {
			// Line is common to both files.
			result = append(result, CommLine{Text: lines1[i], Column: 3})
			i++
			j++
		}
	}

	// Remaining lines in FILE1 are unique to FILE1.
	for ; i < len(lines1); i++ {
		result = append(result, CommLine{Text: lines1[i], Column: 1})
	}

	// Remaining lines in FILE2 are unique to FILE2.
	for ; j < len(lines2); j++ {
		result = append(result, CommLine{Text: lines2[j], Column: 2})
	}

	return result
}

// =========================================================================
// formatCommOutput — format comm results with tab prefixes
// =========================================================================
//
// Each column gets a prefix:
//   Column 1: no prefix
//   Column 2: one tab (or one delimiter)
//   Column 3: two tabs (or two delimiters)
//
// When columns are suppressed, the prefix width adjusts accordingly.
// For example, if column 1 is suppressed, column 2 gets no prefix and
// column 3 gets one tab.

func formatCommOutput(lines []CommLine, opts CommOptions) string {
	delim := opts.OutputDelimiter
	if delim == "" {
		delim = "\t"
	}

	var result strings.Builder

	for _, line := range lines {
		switch line.Column {
		case 1:
			if !opts.SuppressCol1 {
				result.WriteString(line.Text)
				result.WriteString("\n")
			}
		case 2:
			if !opts.SuppressCol2 {
				// Column 2 prefix: one delimiter for each non-suppressed
				// column before it.
				if !opts.SuppressCol1 {
					result.WriteString(delim)
				}
				result.WriteString(line.Text)
				result.WriteString("\n")
			}
		case 3:
			if !opts.SuppressCol3 {
				// Column 3 prefix: one delimiter for each non-suppressed
				// column before it.
				if !opts.SuppressCol1 {
					result.WriteString(delim)
				}
				if !opts.SuppressCol2 {
					result.WriteString(delim)
				}
				result.WriteString(line.Text)
				result.WriteString("\n")
			}
		}
	}

	return result.String()
}

// =========================================================================
// runComm — the testable core of the comm tool
// =========================================================================

func runComm(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runCommWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runCommWithStdin is the inner implementation with injectable stdin.

func runCommWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "comm: %s\n", err)
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
		opts := CommOptions{
			SuppressCol1: getBool(r.Flags, "suppress_col1"),
			SuppressCol2: getBool(r.Flags, "suppress_col2"),
			SuppressCol3: getBool(r.Flags, "suppress_col3"),
		}
		if od, ok := r.Flags["output_delimiter"].(string); ok {
			opts.OutputDelimiter = od
		}

		// Read FILE1.
		file1Name, _ := r.Arguments["file1"].(string)
		file2Name, _ := r.Arguments["file2"].(string)

		lines1, err := readFileLines(file1Name, stdin)
		if err != nil {
			fmt.Fprintf(stderr, "comm: %s: %s\n", file1Name, err)
			return 1
		}

		lines2, err := readFileLines(file2Name, stdin)
		if err != nil {
			fmt.Fprintf(stderr, "comm: %s: %s\n", file2Name, err)
			return 1
		}

		// Compare and output.
		commLines := compareFiles(lines1, lines2)
		output := formatCommOutput(commLines, opts)
		fmt.Fprint(stdout, output)

		return 0

	default:
		fmt.Fprintf(stderr, "comm: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// readFileLines — read all lines from a file or stdin
// =========================================================================

func readFileLines(filename string, stdin io.Reader) ([]string, error) {
	var reader io.Reader
	if filename == "-" {
		reader = stdin
	} else {
		f, err := os.Open(filename)
		if err != nil {
			return nil, err
		}
		defer f.Close()
		reader = f
	}

	scanner := bufio.NewScanner(reader)
	var lines []string
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	return lines, scanner.Err()
}

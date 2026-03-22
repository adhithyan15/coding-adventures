// =========================================================================
// unexpand — Convert Spaces to Tabs
// =========================================================================
//
// The `unexpand` utility is the inverse of `expand`: it converts sequences
// of spaces back into tab characters at tab stop boundaries.
//
// # Basic usage
//
//   unexpand file.txt            Convert leading spaces to tabs (8-column)
//   unexpand -a file.txt         Convert ALL spaces, not just leading ones
//   unexpand -t 4 file.txt       Use 4-column tab stops
//   unexpand --first-only file   Convert only leading blanks (default)
//
// # How it works
//
// unexpand looks for sequences of spaces that align with tab stop
// boundaries. When it finds spaces that span from one position to the
// next tab stop, it replaces them with a single tab character.
//
// Example (with 8-column tab stops):
//
//   Input:  "        hello"  (8 spaces + "hello")
//   Output: "\thello"        (1 tab + "hello")
//
// # Architecture
//
//   unexpand.json (spec)         unexpand_tool.go (this file)
//   ┌──────────────────┐       ┌────────────────────────────────┐
//   │ flags: -a,-t     │       │ for each input line:           │
//   │ --first-only     │──────>│   walk chars, track column     │
//   │ variadic FILE    │       │   replace space runs with tabs │
//   └──────────────────┘       └────────────────────────────────┘

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
// unexpandLine — convert spaces to tabs in a single line
// =========================================================================
//
// This function walks through the line character by character, tracking
// the current column position. When it encounters a sequence of spaces
// that reaches a tab stop, it replaces them with a tab character.
//
// Parameters:
//   - line: the input line (without the newline)
//   - tabStops: parsed tab stop positions
//   - allBlanks: if true, convert spaces everywhere; if false, only leading

func unexpandLine(line string, tabStops []int, allBlanks bool) string {
	var result strings.Builder
	column := 0
	spaceCount := 0
	spaceStartCol := 0
	seenNonBlank := false

	for _, ch := range line {
		if ch == ' ' {
			if !allBlanks && seenNonBlank {
				// Not converting non-leading spaces.
				result.WriteRune(' ')
				column++
				continue
			}

			if spaceCount == 0 {
				spaceStartCol = column
			}
			spaceCount++
			column++

			// Check if we've reached a tab stop.
			tabWidth := nextTabStop(spaceStartCol, tabStops)
			if spaceCount >= tabWidth {
				// Replace the accumulated spaces with a tab.
				result.WriteRune('\t')
				spaceCount = 0
			}
		} else {
			// Flush any remaining spaces that didn't reach a tab stop.
			for i := 0; i < spaceCount; i++ {
				result.WriteRune(' ')
			}
			spaceCount = 0

			if ch != '\t' {
				seenNonBlank = true
			}
			result.WriteRune(ch)
			if ch == '\t' {
				// Tab advances to next tab stop.
				column += nextTabStop(column, tabStops)
			} else {
				column++
			}
		}
	}

	// Flush any trailing spaces.
	for i := 0; i < spaceCount; i++ {
		result.WriteRune(' ')
	}

	return result.String()
}

// =========================================================================
// runUnexpand — the testable core of the unexpand tool
// =========================================================================

func runUnexpand(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runUnexpandWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runUnexpandWithStdin is the inner implementation that accepts a custom stdin.

func runUnexpandWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "unexpand: %s\n", err)
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
		allBlanks := getBool(r.Flags, "all")
		firstOnly := getBool(r.Flags, "first_only")

		// --first-only overrides -a.
		if firstOnly {
			allBlanks = false
		}

		// Parse tab stops.
		tabsStr, _ := r.Flags["tabs"].(string)
		tabStops, err := parseTabStops(tabsStr)
		if err != nil {
			fmt.Fprintf(stderr, "unexpand: %s\n", err)
			return 1
		}

		// Extract file paths.
		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			files = []string{"-"}
		}

		exitCode := 0
		for _, file := range files {
			var reader io.Reader

			if file == "-" {
				reader = stdin
			} else {
				f, err := os.Open(file)
				if err != nil {
					fmt.Fprintf(stderr, "unexpand: %s: %s\n", file, err)
					exitCode = 1
					continue
				}
				defer f.Close()
				reader = f
			}

			scanner := bufio.NewScanner(reader)
			buf := make([]byte, 0, 1024*1024)
			scanner.Buffer(buf, 1024*1024)

			for scanner.Scan() {
				unexpanded := unexpandLine(scanner.Text(), tabStops, allBlanks)
				fmt.Fprintln(stdout, unexpanded)
			}

			if err := scanner.Err(); err != nil {
				fmt.Fprintf(stderr, "unexpand: error reading '%s': %s\n", file, err)
				exitCode = 1
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "unexpand: unexpected result type: %T\n", result)
		return 1
	}
}

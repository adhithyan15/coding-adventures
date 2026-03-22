// =========================================================================
// expand — Convert Tabs to Spaces
// =========================================================================
//
// The `expand` utility converts tab characters to the appropriate number
// of spaces, preserving column alignment. By default, tab stops are every
// 8 columns.
//
// # How tab expansion works
//
// A tab character doesn't always represent the same number of spaces.
// It advances the cursor to the NEXT tab stop. With tab stops every
// 8 columns:
//
//   Column:  0  1  2  3  4  5  6  7  8  9  10 ...
//   Tab at column 0 => 8 spaces (advances to column 8)
//   Tab at column 3 => 5 spaces (advances to column 8)
//   Tab at column 7 => 1 space  (advances to column 8)
//   Tab at column 8 => 8 spaces (advances to column 16)
//
// # Basic usage
//
//   expand file.txt              Convert tabs to spaces (8-column stops)
//   expand -t 4 file.txt         Use 4-column tab stops
//   expand -t 2,4,8 file.txt     Variable tab stops at columns 2, 4, 8
//   expand -i file.txt           Only expand initial (leading) tabs
//
// # Architecture
//
//   expand.json (spec)           expand_tool.go (this file)
//   ┌──────────────────┐       ┌────────────────────────────────┐
//   │ flags: -i,-t     │       │ for each input line:           │
//   │ variadic FILE    │──────>│   walk chars, track column     │
//   │ help, version    │       │   replace tabs with spaces     │
//   └──────────────────┘       └────────────────────────────────┘

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
// parseTabStops — parse the -t flag value into a list of tab stop positions
// =========================================================================
//
// The -t flag can be:
//   - A single number: "4" means tab stops every 4 columns
//   - A comma-separated list: "2,4,8" means tab stops at those columns
//
// For a single number, we return just that number (the caller will use
// it as a regular interval). For a list, we return all positions.

func parseTabStops(tabsStr string) ([]int, error) {
	if tabsStr == "" {
		return []int{8}, nil
	}

	parts := strings.Split(tabsStr, ",")
	stops := make([]int, 0, len(parts))
	for _, p := range parts {
		n, err := strconv.Atoi(strings.TrimSpace(p))
		if err != nil || n <= 0 {
			return nil, fmt.Errorf("invalid tab stop: '%s'", p)
		}
		stops = append(stops, n)
	}

	return stops, nil
}

// =========================================================================
// nextTabStop — calculate the next tab stop position from the current column
// =========================================================================
//
// Given the current column and the tab stop configuration, returns how
// many spaces are needed to reach the next tab stop.

func nextTabStop(column int, tabStops []int) int {
	if len(tabStops) == 1 {
		// Regular interval: tab stops every N columns.
		interval := tabStops[0]
		return interval - (column % interval)
	}

	// Variable tab stops: find the first stop past the current column.
	for _, stop := range tabStops {
		if stop > column {
			return stop - column
		}
	}

	// Past all defined stops — use last interval as repeating.
	if len(tabStops) >= 2 {
		lastInterval := tabStops[len(tabStops)-1] - tabStops[len(tabStops)-2]
		return lastInterval - ((column - tabStops[len(tabStops)-1]) % lastInterval)
	}

	// Fallback: 1 space.
	return 1
}

// =========================================================================
// expandLine — expand tabs in a single line
// =========================================================================

func expandLine(line string, tabStops []int, initialOnly bool) string {
	var result strings.Builder
	column := 0
	seenNonBlank := false

	for _, ch := range line {
		if ch == '\t' {
			if initialOnly && seenNonBlank {
				// -i flag: don't expand tabs after non-blank characters.
				result.WriteRune(ch)
				column++
			} else {
				// Replace the tab with the appropriate number of spaces.
				spaces := nextTabStop(column, tabStops)
				for i := 0; i < spaces; i++ {
					result.WriteRune(' ')
				}
				column += spaces
			}
		} else {
			if ch != ' ' {
				seenNonBlank = true
			}
			result.WriteRune(ch)
			column++
		}
	}

	return result.String()
}

// =========================================================================
// runExpand — the testable core of the expand tool
// =========================================================================

func runExpand(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runExpandWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runExpandWithStdin is the inner implementation that accepts a custom stdin.

func runExpandWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "expand: %s\n", err)
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
		initialOnly := getBool(r.Flags, "initial")

		// Parse tab stops.
		tabsStr, _ := r.Flags["tabs"].(string)
		tabStops, err := parseTabStops(tabsStr)
		if err != nil {
			fmt.Fprintf(stderr, "expand: %s\n", err)
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
					fmt.Fprintf(stderr, "expand: %s: %s\n", file, err)
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
				expanded := expandLine(scanner.Text(), tabStops, initialOnly)
				fmt.Fprintln(stdout, expanded)
			}

			if err := scanner.Err(); err != nil {
				fmt.Fprintf(stderr, "expand: error reading '%s': %s\n", file, err)
				exitCode = 1
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "expand: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// fold — Wrap Each Input Line to Fit in Specified Width
// =========================================================================
//
// The `fold` utility wraps each input line so it is no wider than a
// specified width (default 80 columns). Unlike `fmt`, which reformats
// paragraphs, fold does a simple line-length wrap — it doesn't understand
// words or sentences by default.
//
// # Basic usage
//
//   fold file.txt                Wrap at 80 columns
//   fold -w 40 file.txt         Wrap at 40 columns
//   fold -s file.txt            Break at spaces (word-aware)
//   fold -b file.txt            Count bytes, not columns
//
// # The -s flag (break at spaces)
//
// Without -s, fold breaks at exactly the width boundary, even in the
// middle of words:
//
//   Input:  "The quick brown fox"    (width=10)
//   Output: "The quick \nbrown fox\n"
//
// With -s, fold breaks at the last space before the width boundary:
//
//   Input:  "The quick brown fox"    (width=10)
//   Output: "The quick \nbrown fox\n"
//
// # Architecture
//
//   fold.json (spec)             fold_tool.go (this file)
//   ┌──────────────────┐       ┌────────────────────────────────┐
//   │ flags: -b,-s,-w  │       │ for each input line:           │
//   │ variadic FILE    │──────>│   break at width boundary      │
//   │ help, version    │       │   respect -s for spaces        │
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
// foldLine — fold a single line to fit within the specified width
// =========================================================================
//
// This function takes a single line (without the trailing newline) and
// returns the folded result with newlines inserted at the appropriate
// positions.
//
// Parameters:
//   - line: the input line (no trailing newline)
//   - width: maximum column width
//   - breakAtSpaces: if true, try to break at spaces instead of mid-word
//   - countBytes: if true, count bytes instead of characters

func foldLine(line string, width int, breakAtSpaces bool, countBytes bool) string {
	if width <= 0 {
		return line
	}

	var result strings.Builder
	runes := []rune(line)
	start := 0

	for start < len(runes) {
		end := start + width
		if end >= len(runes) {
			// The remaining text fits within the width.
			result.WriteString(string(runes[start:]))
			break
		}

		if breakAtSpaces {
			// Look for the last space within the width boundary.
			breakPoint := -1
			for i := end; i > start; i-- {
				if runes[i] == ' ' {
					breakPoint = i
					break
				}
			}

			if breakPoint > start {
				// Break at the space.
				result.WriteString(string(runes[start:breakPoint]))
				result.WriteRune('\n')
				start = breakPoint + 1 // skip the space
				continue
			}
		}

		// Hard break at the width boundary.
		result.WriteString(string(runes[start:end]))
		result.WriteRune('\n')
		start = end
	}

	return result.String()
}

// =========================================================================
// runFold — the testable core of the fold tool
// =========================================================================

func runFold(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runFoldWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runFoldWithStdin is the inner implementation that accepts a custom stdin.

func runFoldWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "fold: %s\n", err)
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
		countBytes := getBool(r.Flags, "bytes")
		breakAtSpaces := getBool(r.Flags, "spaces")

		// Extract width with default of 80.
		width := 80
		if v := r.Flags["width"]; v != nil {
			if n, ok := v.(int64); ok {
				width = int(n)
			}
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
					fmt.Fprintf(stderr, "fold: %s: %s\n", file, err)
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
				folded := foldLine(scanner.Text(), width, breakAtSpaces, countBytes)
				fmt.Fprintln(stdout, folded)
			}

			if err := scanner.Err(); err != nil {
				fmt.Fprintf(stderr, "fold: error reading '%s': %s\n", file, err)
				exitCode = 1
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "fold: unexpected result type: %T\n", result)
		return 1
	}
}

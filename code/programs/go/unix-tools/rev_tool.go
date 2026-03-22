// =========================================================================
// rev — Reverse Lines Characterwise
// =========================================================================
//
// The `rev` utility reverses each line of input. Each line's characters
// are reversed, but the order of lines is preserved.
//
// # Basic usage
//
//   echo "hello" | rev         =>  "olleh"
//   rev file.txt               Reverse each line in file.txt
//
// # Examples
//
//   Input:      Output:
//   ┌────────┐  ┌────────┐
//   │ hello  │  │ olleh  │
//   │ world  │  │ dlrow  │
//   │ abc    │  │ cba    │
//   └────────┘  └────────┘
//
// # Unicode handling
//
// rev operates on Unicode code points (runes), not bytes. This means
// multi-byte characters like emoji or accented letters are reversed
// correctly as whole characters:
//
//   "cafe\u0301" reversed properly keeps the accent with its base char
//   (though combining characters are a known edge case)
//
// # Architecture
//
//   rev.json (spec)            rev_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ no flags          │      │ for each file/stdin:           │
//   │ variadic FILE    │─────>│   read line by line            │
//   │ help, version    │      │   reverse runes in each line   │
//   └──────────────────┘      │   output reversed line         │
//                             └────────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// reverseRunes — reverse a string by its Unicode code points
// =========================================================================
//
// This function converts the string to a rune slice (which correctly
// handles multi-byte UTF-8 characters), reverses the slice in place,
// and converts back to a string.
//
// The reversal algorithm uses the classic "swap from both ends" approach:
//
//   Before: [h, e, l, l, o]
//           ^              ^
//           i              j
//
//   After:  [o, l, l, e, h]
//
// Time complexity: O(n) where n is the number of runes.
// Space complexity: O(n) for the rune slice copy.

func reverseRunes(s string) string {
	runes := []rune(s)
	for i, j := 0, len(runes)-1; i < j; i, j = i+1, j-1 {
		runes[i], runes[j] = runes[j], runes[i]
	}
	return string(runes)
}

// =========================================================================
// revReader — reverse each line from a reader and write to stdout
// =========================================================================
//
// Reads from the given reader line by line, reverses each line, and
// writes the result to stdout.

func revReader(reader io.Reader, stdout io.Writer) error {
	scanner := bufio.NewScanner(reader)

	// Increase buffer size for very long lines.
	buf := make([]byte, 0, 1024*1024)
	scanner.Buffer(buf, 1024*1024)

	for scanner.Scan() {
		reversed := reverseRunes(scanner.Text())
		fmt.Fprintln(stdout, reversed)
	}

	return scanner.Err()
}

// =========================================================================
// runRev — the testable core of the rev tool
// =========================================================================

func runRev(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runRevWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runRevWithStdin is the inner implementation that accepts a custom stdin.

func runRevWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "rev: %s\n", err)
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
		// Extract file paths.
		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			files = []string{"-"}
		}

		// Process each file.
		exitCode := 0
		for _, file := range files {
			var reader io.Reader

			if file == "-" {
				reader = stdin
			} else {
				f, err := os.Open(file)
				if err != nil {
					fmt.Fprintf(stderr, "rev: cannot open '%s': %s\n", file, err)
					exitCode = 1
					continue
				}
				defer f.Close()
				reader = f
			}

			if err := revReader(reader, stdout); err != nil {
				fmt.Fprintf(stderr, "rev: error reading '%s': %s\n", file, err)
				exitCode = 1
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "rev: unexpected result type: %T\n", result)
		return 1
	}
}

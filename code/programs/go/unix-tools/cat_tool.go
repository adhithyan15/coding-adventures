// =========================================================================
// cat — Concatenate Files and Print to Standard Output
// =========================================================================
//
// The `cat` utility reads files sequentially and writes their contents
// to standard output. Its name comes from "concatenate" — its primary
// purpose is joining multiple files together.
//
// # Basic usage
//
//   cat file.txt                Read one file
//   cat file1.txt file2.txt     Concatenate two files
//   cat                         Read from standard input (stdin)
//   cat -                       Same as above (explicit stdin)
//   cat file1.txt - file2.txt   Mix files and stdin
//
// # Flags
//
// cat has several display-modifying flags. None of them change the actual
// content — they only affect how it appears on screen:
//
//   -n    Number all output lines (starting from 1).
//   -b    Number only non-blank lines (overrides -n).
//   -s    Squeeze repeated blank lines into a single blank line.
//   -T    Show tab characters as ^I (caret notation).
//   -E    Show a $ at the end of each line (makes trailing spaces visible).
//   -v    Show non-printing characters using ^ and M- notation.
//   -A    Show all: equivalent to -vET.
//
// # How line numbering works
//
//	Input:           -n output:        -b output:
//	┌────────────┐   ┌──────────────┐  ┌──────────────┐
//	│ hello      │   │ 1  hello     │  │ 1  hello     │
//	│            │   │ 2            │  │              │
//	│ world      │   │ 3  world     │  │ 2  world     │
//	└────────────┘   └──────────────┘  └──────────────┘
//
// Notice that -b skips numbering the blank line (line 2 in -n mode).
// The -b flag overrides -n: if both are given, -b wins.
//
// # Squeeze blank lines (-s)
//
//	Input:           -s output:
//	┌────────────┐   ┌────────────┐
//	│ hello      │   │ hello      │
//	│            │   │            │
//	│            │   │ world      │
//	│            │   │            │
//	│ world      │   └────────────┘
//	└────────────┘
//
// Multiple consecutive blank lines are collapsed into one.
//
// # Architecture
//
//	cat.json (spec)          cat_tool.go (this file)
//	┌──────────────────┐     ┌──────────────────────────────┐
//	│ flags: -n,-b,-s  │     │ for each file:               │
//	│ -T,-E,-v,-A      │────>│   read lines                 │
//	│ variadic FILE    │     │   apply transforms            │
//	│ help, version    │     │   write to stdout             │
//	└──────────────────┘     └──────────────────────────────┘

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
// catOptions — configuration for the cat operation
// =========================================================================
//
// We bundle all the boolean flags into a struct for clarity. This avoids
// passing 7 separate booleans to functions, which would be error-prone
// and hard to read.

type catOptions struct {
	number        bool // -n: number all lines
	numberNonblank bool // -b: number only non-blank lines (overrides -n)
	squeezeBlank  bool // -s: squeeze consecutive blank lines
	showTabs      bool // -T: display tabs as ^I
	showEnds      bool // -E: display $ at end of each line
	showNonprint  bool // -v: show non-printing characters
}

// =========================================================================
// showNonPrintingChar — convert a byte to its ^ or M- representation
// =========================================================================
//
// Non-printing characters (control characters, DEL, and characters with
// the high bit set) are displayed using a notation that makes them visible:
//
//   Control characters (0x00-0x1F): ^@ through ^_ (except TAB and LFD)
//   DEL (0x7F): ^?
//   High-bit characters (0x80-0xFF): M-<char> where <char> is the
//     low-7-bit representation
//
// This is the same notation used by `cat -v` in GNU coreutils.
//
// Examples:
//   0x00 (NUL)  => ^@
//   0x01 (SOH)  => ^A
//   0x1B (ESC)  => ^[
//   0x7F (DEL)  => ^?
//   0x80        => M-^@
//   0xC0        => M-@

func showNonPrintingChar(b byte) string {
	// Characters with the high bit set (128-255) get the M- prefix.
	// We strip the high bit, process the low 7 bits, then prepend M-.
	if b >= 128 {
		// Strip the high bit and recurse on the low 7 bits.
		if b == 128+127 {
			return "M-^?"
		}
		low := b - 128
		if low < 32 {
			return fmt.Sprintf("M-^%c", low+'@')
		}
		return fmt.Sprintf("M-%c", low)
	}

	// DEL character (127) is shown as ^?
	if b == 127 {
		return "^?"
	}

	// Control characters (0-31) are shown as ^@ through ^_
	// Exception: TAB (9) and LFD/newline (10) are NOT converted —
	// they're handled separately by -T and normal line processing.
	if b < 32 && b != '\t' && b != '\n' {
		return fmt.Sprintf("^%c", b+'@')
	}

	// Regular printable character — return as-is.
	return string(b)
}

// =========================================================================
// processLine — apply display transforms to a single line
// =========================================================================
//
// This function takes a raw line (without the trailing newline) and
// applies the requested display transforms in the correct order:
//   1. Show non-printing characters (-v)
//   2. Show tabs as ^I (-T)
//   3. Show line ending as $ (-E)
//
// The transforms are applied per-character, building a new string.

func processLine(line string, opts catOptions) string {
	// Fast path: if no character-level transforms are needed, return as-is.
	if !opts.showNonprint && !opts.showTabs && !opts.showEnds {
		return line
	}

	var result strings.Builder

	for i := 0; i < len(line); i++ {
		b := line[i]

		// Handle TAB characters.
		if b == '\t' {
			if opts.showTabs {
				// Show tabs as ^I (caret notation for horizontal tab).
				// The "I" comes from: TAB is the 9th character (0-indexed),
				// and '@' + 9 = 'I' in ASCII.
				result.WriteString("^I")
			} else {
				result.WriteByte('\t')
			}
			continue
		}

		// Handle non-printing characters.
		if opts.showNonprint && (b < 32 || b >= 127) && b != '\t' && b != '\n' {
			result.WriteString(showNonPrintingChar(b))
			continue
		}

		// Regular character — pass through.
		result.WriteByte(b)
	}

	// Append $ at the end of the line if -E is set.
	if opts.showEnds {
		result.WriteByte('$')
	}

	return result.String()
}

// =========================================================================
// catReader — process a single reader (file or stdin) with cat options
// =========================================================================
//
// This function reads from the given reader line by line and writes to
// stdout with the requested transforms applied. The lineNum pointer
// allows line numbering to continue across multiple files (cat file1 file2
// should have continuous line numbers).
//
// Parameters:
//   - reader: the input source (file or stdin)
//   - stdout: where to write the output
//   - opts: which display transforms to apply
//   - lineNum: pointer to the current line number (shared across files)
//   - prevBlank: pointer to whether the previous line was blank (for -s)

func catReader(reader io.Reader, stdout io.Writer, opts catOptions, lineNum *int, prevBlank *bool) error {
	scanner := bufio.NewScanner(reader)

	// Increase the scanner buffer size to handle very long lines.
	// The default is 64KB, which is usually enough, but some files
	// (like minified JavaScript) can have lines much longer than that.
	buf := make([]byte, 0, 1024*1024)
	scanner.Buffer(buf, 1024*1024)

	for scanner.Scan() {
		line := scanner.Text()
		isBlank := len(line) == 0

		// Handle -s (squeeze blank lines).
		// If this line is blank and the previous line was also blank,
		// skip this line entirely.
		if opts.squeezeBlank && isBlank && *prevBlank {
			continue
		}
		*prevBlank = isBlank

		// Apply character-level transforms to the line content.
		processed := processLine(line, opts)

		// Handle line numbering.
		if opts.numberNonblank {
			// -b: number only non-blank lines.
			if !isBlank {
				*lineNum++
				fmt.Fprintf(stdout, "%6d\t%s\n", *lineNum, processed)
			} else {
				fmt.Fprintln(stdout, processed)
			}
		} else if opts.number {
			// -n: number all lines.
			*lineNum++
			fmt.Fprintf(stdout, "%6d\t%s\n", *lineNum, processed)
		} else {
			// No numbering — just print the line.
			fmt.Fprintln(stdout, processed)
		}
	}

	return scanner.Err()
}

// =========================================================================
// runCat — the testable core of the cat tool
// =========================================================================
//
// The cat tool processes each file in order, applying the requested
// display transforms. If no files are specified, it reads from stdin.
// The special filename "-" also means stdin.
//
// Line numbering continues across files: if file1 has 3 lines and file2
// has 2 lines, the total numbering goes 1-5.

func runCat(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runCatWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runCatWithStdin is the inner implementation that accepts a custom stdin
// reader. This allows tests to provide mock stdin input without modifying
// the real os.Stdin.

func runCatWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "cat: %s\n", err)
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
		// Extract flags into our options struct.
		opts := catOptions{
			number:        getBool(r.Flags, "number"),
			numberNonblank: getBool(r.Flags, "number_nonblank"),
			squeezeBlank:  getBool(r.Flags, "squeeze_blank"),
			showTabs:      getBool(r.Flags, "show_tabs"),
			showEnds:      getBool(r.Flags, "show_ends"),
			showNonprint:  getBool(r.Flags, "show_nonprinting"),
		}

		// -A (show all) is equivalent to -vET.
		if getBool(r.Flags, "show_all") {
			opts.showNonprint = true
			opts.showEnds = true
			opts.showTabs = true
		}

		// -b overrides -n. If both are set, only number non-blank lines.
		if opts.numberNonblank {
			opts.number = false
		}

		// Extract file paths from arguments.
		files := getStringSlice(r.Arguments, "files")

		// If no files specified, default to stdin.
		if len(files) == 0 {
			files = []string{"-"}
		}

		// Process each file in order.
		lineNum := 0
		prevBlank := false
		exitCode := 0

		for _, file := range files {
			var reader io.Reader

			if file == "-" {
				// "-" means read from standard input.
				reader = stdin
			} else {
				// Open the file for reading.
				f, err := os.Open(file)
				if err != nil {
					fmt.Fprintf(stderr, "cat: %s: %s\n", file, err)
					exitCode = 1
					continue
				}
				defer f.Close()
				reader = f
			}

			if err := catReader(reader, stdout, opts, &lineNum, &prevBlank); err != nil {
				fmt.Fprintf(stderr, "cat: %s: %s\n", file, err)
				exitCode = 1
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "cat: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// Helper functions for extracting typed values from parsed results
// =========================================================================

// getBool safely extracts a boolean value from a map.
// Returns false if the key doesn't exist or the value isn't a bool.
func getBool(m map[string]interface{}, key string) bool {
	v, ok := m[key].(bool)
	if !ok {
		return false
	}
	return v
}

// getStringSlice extracts a string slice from a parsed arguments map.
//
// cli-builder returns variadic arguments as []interface{}, where each
// element is a string. This function handles the type conversion safely.
func getStringSlice(m map[string]interface{}, key string) []string {
	raw, ok := m[key]
	if !ok {
		return nil
	}

	// Try []interface{} first (what cli-builder returns for variadic args).
	if slice, ok := raw.([]interface{}); ok {
		result := make([]string, 0, len(slice))
		for _, v := range slice {
			if s, ok := v.(string); ok {
				result = append(result, s)
			}
		}
		return result
	}

	// Try a single string (for non-variadic args or single values).
	if s, ok := raw.(string); ok {
		return []string{s}
	}

	return nil
}

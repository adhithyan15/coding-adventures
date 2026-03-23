// =========================================================================
// wc — Word Count
// =========================================================================
//
// The `wc` utility counts lines, words, and bytes (or characters) in
// files. By default, it displays all three counts. Individual flags
// let you select specific counts.
//
// # Basic usage
//
//   wc file.txt         Print lines, words, bytes for file.txt
//   wc -l file.txt      Print only the line count
//   wc -w file.txt      Print only the word count
//   wc -c file.txt      Print only the byte count
//   wc -m file.txt      Print only the character count
//   wc -L file.txt      Print the length of the longest line
//   wc                   Read from standard input
//   wc file1 file2      Print counts for each file, plus a total
//
// # What counts as a "line"?
//
// A line is terminated by a newline character (\n). The count is the
// number of newline characters in the file. This means a file that
// doesn't end with a newline has one fewer "line" than you might expect:
//
//   "hello\nworld\n"  => 2 lines  (two newlines)
//   "hello\nworld"    => 1 line   (one newline)
//   "hello"           => 0 lines  (no newlines)
//
// # What counts as a "word"?
//
// A word is a maximal sequence of non-whitespace characters. Whitespace
// includes spaces, tabs, newlines, carriage returns, form feeds, and
// vertical tabs. This definition matches the POSIX spec.
//
// # Bytes vs. characters (-c vs -m)
//
// For ASCII text, bytes and characters are the same. But for UTF-8 text:
//   - Byte count (-c): the number of raw bytes in the file
//   - Character count (-m): the number of Unicode code points
//
// For example, the string "cafe\u0301" (cafe with accent) has 6 bytes
// but 5 characters in UTF-8.
//
// These flags are mutually exclusive — you can't use -c and -m together.
//
// # Output format
//
//   Each count is right-aligned in a field. When multiple files are
//   given, a "total" line is appended at the end.
//
//   Example with two files:
//     3   10   42 file1.txt
//     7   25  118 file2.txt
//    10   35  160 total
//
// # Architecture
//
//	wc.json (spec)          wc_tool.go (this file)
//	┌──────────────────┐     ┌──────────────────────────────┐
//	│ flags: -l,-w,-c  │     │ for each file:               │
//	│ -m,-L            │────>│   count lines/words/bytes    │
//	│ variadic FILE    │     │   accumulate totals          │
//	│ help, version    │     │   format and print           │
//	└──────────────────┘     └──────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"unicode/utf8"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// wcCounts — holds the counts for a single file
// =========================================================================
//
// We track all five possible metrics (lines, words, bytes, chars, and
// max line length) regardless of which flags are set. This simplifies
// the counting logic — we always count everything, and only display
// what was requested.

type wcCounts struct {
	lines         int // number of newline characters
	words         int // number of whitespace-delimited words
	bytes         int // number of raw bytes
	chars         int // number of Unicode code points (runes)
	maxLineLength int // length of the longest line (in characters)
}

// =========================================================================
// wcOptions — which counts to display
// =========================================================================
//
// When no flags are specified, wc displays lines, words, and bytes
// (the default set). When any flag is specified, only the requested
// counts are shown.

type wcOptions struct {
	showLines         bool
	showWords         bool
	showBytes         bool
	showChars         bool
	showMaxLineLength bool
}

// =========================================================================
// countReader — count lines, words, bytes, chars for a reader
// =========================================================================
//
// This function reads from the given reader and counts all metrics
// simultaneously in a single pass. This is more efficient than reading
// the data multiple times for different counts.
//
// The algorithm works byte by byte, tracking whether we're currently
// inside a word (a sequence of non-whitespace characters). When we
// transition from whitespace to non-whitespace, we've found a new word.
//
// State machine for word counting:
//
//	┌─────────────────┐    non-whitespace    ┌──────────────────┐
//	│   NOT IN WORD   │ ──────────────────> │    IN WORD       │
//	│   (inWord=false)│ <────────────────── │    (inWord=true)  │
//	└─────────────────┘    whitespace        └──────────────────┘
//	                       (words++)

func countReader(reader io.Reader) (wcCounts, error) {
	var counts wcCounts
	inWord := false
	currentLineLength := 0

	// We read in chunks for efficiency. Reading byte by byte would be
	// extremely slow due to syscall overhead. bufio.Reader gives us
	// buffered reading while still letting us process byte by byte.
	br := bufio.NewReader(reader)

	for {
		b, err := br.ReadByte()
		if err != nil {
			if err == io.EOF {
				// End of input. If we were in a word, it's complete.
				if inWord {
					counts.words++
				}
				// Update max line length for the final line (if it
				// didn't end with a newline).
				if currentLineLength > counts.maxLineLength {
					counts.maxLineLength = currentLineLength
				}
				break
			}
			return counts, err
		}

		// Count every byte.
		counts.bytes++

		// Count characters (Unicode code points).
		//
		// In UTF-8, the first byte of a character tells us how many
		// bytes the character uses:
		//   0xxxxxxx  => 1 byte  (ASCII)
		//   110xxxxx  => 2 bytes (start of 2-byte sequence)
		//   1110xxxx  => 3 bytes (start of 3-byte sequence)
		//   11110xxx  => 4 bytes (start of 4-byte sequence)
		//   10xxxxxx  => continuation byte (NOT a new character)
		//
		// utf8.RuneStart() returns true for all bytes EXCEPT continuation
		// bytes. So we count one character per start byte.
		if utf8.RuneStart(b) {
			counts.chars++
		}

		// Check if this byte is whitespace.
		isWhitespace := b == ' ' || b == '\t' || b == '\n' ||
			b == '\r' || b == '\f' || b == '\v'

		if isWhitespace {
			if inWord {
				// We just finished a word.
				counts.words++
				inWord = false
			}
		} else {
			// Non-whitespace character — we're in a word.
			inWord = true
		}

		// Handle newlines for line counting and max line length.
		if b == '\n' {
			counts.lines++
			if currentLineLength > counts.maxLineLength {
				counts.maxLineLength = currentLineLength
			}
			currentLineLength = 0
		} else {
			// Count characters in the current line (for -L).
			// We count bytes here for simplicity; a proper implementation
			// would count display width (considering wide characters).
			// For ASCII text this is the same.
			if utf8.RuneStart(b) {
				currentLineLength++
			}
		}
	}

	return counts, nil
}

// =========================================================================
// formatCounts — format a single row of wc output
// =========================================================================
//
// This function formats the counts for a single file (or the total line)
// according to the requested options. Each count is right-aligned in a
// field of at least `width` characters.
//
// The width parameter ensures that columns align when displaying counts
// for multiple files. The width is determined by the largest count across
// all files.

func formatCounts(counts wcCounts, opts wcOptions, width int, name string) string {
	var parts []string

	// The order of fields matches GNU wc: lines, words, bytes/chars, max-line-length.
	if opts.showLines {
		parts = append(parts, fmt.Sprintf("%*d", width, counts.lines))
	}
	if opts.showWords {
		parts = append(parts, fmt.Sprintf("%*d", width, counts.words))
	}
	if opts.showBytes {
		parts = append(parts, fmt.Sprintf("%*d", width, counts.bytes))
	}
	if opts.showChars {
		parts = append(parts, fmt.Sprintf("%*d", width, counts.chars))
	}
	if opts.showMaxLineLength {
		parts = append(parts, fmt.Sprintf("%*d", width, counts.maxLineLength))
	}

	// Build the output line.
	result := ""
	for i, p := range parts {
		if i > 0 {
			result += " "
		}
		result += p
	}

	// Append the filename (empty for stdin with no name).
	if name != "" {
		result += " " + name
	}

	return result
}

// =========================================================================
// maxCount — find the largest count across all results for column alignment
// =========================================================================
//
// We need to know the widest number so we can right-align all columns.
// This scans all counts that will be displayed and returns the maximum.

func maxCount(allCounts []wcCounts, opts wcOptions) int {
	max := 0
	for _, c := range allCounts {
		if opts.showLines && c.lines > max {
			max = c.lines
		}
		if opts.showWords && c.words > max {
			max = c.words
		}
		if opts.showBytes && c.bytes > max {
			max = c.bytes
		}
		if opts.showChars && c.chars > max {
			max = c.chars
		}
		if opts.showMaxLineLength && c.maxLineLength > max {
			max = c.maxLineLength
		}
	}
	return max
}

// =========================================================================
// digitWidth — how many digits does a number have?
// =========================================================================
//
// This determines the minimum column width needed to display a number.
// For example: 0 needs 1 digit, 42 needs 2, 1000 needs 4.

func digitWidth(n int) int {
	if n == 0 {
		return 1
	}
	width := 0
	for n > 0 {
		width++
		n /= 10
	}
	return width
}

// =========================================================================
// runWc — the testable core of the wc tool
// =========================================================================

func runWc(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runWcWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runWcWithStdin is the inner implementation that accepts a custom stdin.

func runWcWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "wc: %s\n", err)
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
		// Extract which counts to display.
		opts := wcOptions{
			showLines:         getBool(r.Flags, "lines"),
			showWords:         getBool(r.Flags, "words"),
			showBytes:         getBool(r.Flags, "bytes"),
			showChars:         getBool(r.Flags, "chars"),
			showMaxLineLength: getBool(r.Flags, "max_line_length"),
		}

		// If no specific flags were set, show the default set: lines,
		// words, and bytes. This matches POSIX wc behavior.
		if !opts.showLines && !opts.showWords && !opts.showBytes &&
			!opts.showChars && !opts.showMaxLineLength {
			opts.showLines = true
			opts.showWords = true
			opts.showBytes = true
		}

		// Extract file paths from arguments.
		files := getStringSlice(r.Arguments, "files")

		// If no files specified, default to stdin.
		if len(files) == 0 {
			files = []string{"-"}
		}

		// Process each file, collecting counts.
		var allCounts []wcCounts
		var allNames []string
		exitCode := 0
		var total wcCounts

		for _, file := range files {
			var reader io.Reader
			var name string

			if file == "-" {
				reader = stdin
				name = "" // stdin has no name in single-file mode
				if len(files) > 1 {
					name = "-"
				}
			} else {
				f, err := os.Open(file)
				if err != nil {
					fmt.Fprintf(stderr, "wc: %s: %s\n", file, err)
					exitCode = 1
					continue
				}
				defer f.Close()
				reader = f
				name = file
			}

			counts, err := countReader(reader)
			if err != nil {
				fmt.Fprintf(stderr, "wc: %s: %s\n", file, err)
				exitCode = 1
				continue
			}

			allCounts = append(allCounts, counts)
			allNames = append(allNames, name)

			// Accumulate totals.
			total.lines += counts.lines
			total.words += counts.words
			total.bytes += counts.bytes
			total.chars += counts.chars
			if counts.maxLineLength > total.maxLineLength {
				total.maxLineLength = counts.maxLineLength
			}
		}

		// If we have counts for multiple files, add the total row.
		if len(allCounts) > 1 {
			allCounts = append(allCounts, total)
			allNames = append(allNames, "total")
		}

		// Determine column width based on the largest count.
		maxVal := maxCount(allCounts, opts)
		width := digitWidth(maxVal)
		if width < 1 {
			width = 1
		}

		// Print each row.
		for i, counts := range allCounts {
			line := formatCounts(counts, opts, width, allNames[i])
			fmt.Fprintln(stdout, line)
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "wc: unexpected result type: %T\n", result)
		return 1
	}
}

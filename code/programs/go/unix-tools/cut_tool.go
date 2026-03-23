// =========================================================================
// cut — Remove Sections from Each Line of Files
// =========================================================================
//
// The `cut` utility extracts selected portions from each line of input.
// It can select by byte position, character position, or field number.
//
// # Three selection modes
//
//   Mode          Flag   Example
//   ────────────  ────   ────────────────────────────────
//   Bytes         -b     cut -b 1-5        (bytes 1-5)
//   Characters    -c     cut -c 3,7-       (char 3, then 7+)
//   Fields        -f     cut -f 2 -d ','   (2nd CSV field)
//
// Only one mode can be used at a time.
//
// # LIST format
//
// The LIST argument specifies which elements to select. It's a comma-
// separated list of ranges:
//
//   Format    Meaning
//   ────────  ─────────────────────────
//   N         Just element N
//   N-M       Elements N through M
//   N-        Elements N through end
//   -M        Elements 1 through M
//
// Examples:
//   "1,3,5"     → elements 1, 3, and 5
//   "1-3,7-"    → elements 1-3 and 7 onwards
//   "-5"        → elements 1-5
//
// # Architecture
//
//   cut.json (spec)              cut_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -b,-c,-f  │       │ parse LIST into ranges           │
//   │ -d,-s,--output-  │──────>│ for each line, select elements   │
//   │ delimiter         │       │ join with output delimiter       │
//   │ arg: FILES...     │       │                                  │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"math"
	"os"
	"strconv"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// CutRange — a single range in a LIST specification
// =========================================================================
//
// Represents a range like "3-7" where Start=3, End=7.
// For open-ended ranges like "5-", End is set to math.MaxInt.
// Positions are 1-indexed (matching the Unix convention).

type CutRange struct {
	Start int // 1-indexed start position (inclusive)
	End   int // 1-indexed end position (inclusive), MaxInt for open-ended
}

// =========================================================================
// CutOptions — configuration for the cut operation
// =========================================================================

type CutOptions struct {
	Mode            string     // "bytes", "characters", or "fields"
	Ranges          []CutRange // parsed LIST of ranges
	Delimiter       string     // field delimiter (default: tab)
	OutputDelimiter string     // output delimiter
	OnlyDelimited   bool       // -s: skip lines without delimiter
	Complement      bool       // --complement: invert selection
}

// =========================================================================
// parseList — parse a LIST string into CutRange slices
// =========================================================================
//
// Examples:
//   "1,3,5"  → [{1,1}, {3,3}, {5,5}]
//   "1-3"    → [{1,3}]
//   "5-"     → [{5, MaxInt}]
//   "-3"     → [{1, 3}]

func parseList(list string) ([]CutRange, error) {
	var ranges []CutRange

	for _, part := range strings.Split(list, ",") {
		part = strings.TrimSpace(part)
		if part == "" {
			continue
		}

		if strings.Contains(part, "-") {
			// Range: "N-M", "N-", or "-M"
			sides := strings.SplitN(part, "-", 2)

			start := 1
			end := math.MaxInt

			if sides[0] != "" {
				n, err := strconv.Atoi(sides[0])
				if err != nil {
					return nil, fmt.Errorf("invalid range: %s", part)
				}
				start = n
			}

			if sides[1] != "" {
				n, err := strconv.Atoi(sides[1])
				if err != nil {
					return nil, fmt.Errorf("invalid range: %s", part)
				}
				end = n
			}

			if start < 1 {
				return nil, fmt.Errorf("invalid range: fields are 1-indexed")
			}

			ranges = append(ranges, CutRange{Start: start, End: end})
		} else {
			// Single position: "N"
			n, err := strconv.Atoi(part)
			if err != nil {
				return nil, fmt.Errorf("invalid position: %s", part)
			}
			if n < 1 {
				return nil, fmt.Errorf("invalid position: fields are 1-indexed")
			}
			ranges = append(ranges, CutRange{Start: n, End: n})
		}
	}

	if len(ranges) == 0 {
		return nil, fmt.Errorf("empty list")
	}

	return ranges, nil
}

// =========================================================================
// isInRanges — check if a 1-indexed position is selected
// =========================================================================

func isInRanges(pos int, ranges []CutRange, complement bool) bool {
	inRange := false
	for _, r := range ranges {
		if pos >= r.Start && pos <= r.End {
			inRange = true
			break
		}
	}
	if complement {
		return !inRange
	}
	return inRange
}

// =========================================================================
// cutLineByChars — extract characters from a line by position
// =========================================================================
//
// Operates on runes (Unicode code points), so it works correctly
// with multi-byte characters.

func cutLineByChars(line string, ranges []CutRange, complement bool, outDelim string) string {
	runes := []rune(line)
	var selected []string

	for i, r := range runes {
		pos := i + 1 // Convert to 1-indexed.
		if isInRanges(pos, ranges, complement) {
			selected = append(selected, string(r))
		}
	}

	if outDelim != "" {
		return strings.Join(selected, outDelim)
	}
	return strings.Join(selected, "")
}

// =========================================================================
// cutLineByBytes — extract bytes from a line by position
// =========================================================================

func cutLineByBytes(line string, ranges []CutRange, complement bool, outDelim string) string {
	bs := []byte(line)
	var selected []string

	for i, b := range bs {
		pos := i + 1
		if isInRanges(pos, ranges, complement) {
			selected = append(selected, string(b))
		}
	}

	if outDelim != "" {
		return strings.Join(selected, outDelim)
	}
	return strings.Join(selected, "")
}

// =========================================================================
// cutLineByFields — extract fields from a line by field number
// =========================================================================

func cutLineByFields(line string, opts CutOptions) (string, bool) {
	delim := opts.Delimiter

	// If the line doesn't contain the delimiter and -s is set, skip it.
	if !strings.Contains(line, delim) {
		if opts.OnlyDelimited {
			return "", false // Signal to skip this line.
		}
		return line, true // Print the line as-is.
	}

	fields := strings.Split(line, delim)

	var selected []string
	for i, field := range fields {
		pos := i + 1
		if isInRanges(pos, opts.Ranges, opts.Complement) {
			selected = append(selected, field)
		}
	}

	outDelim := opts.OutputDelimiter
	if outDelim == "" {
		outDelim = delim
	}

	return strings.Join(selected, outDelim), true
}

// =========================================================================
// cutLine — the main line-processing function
// =========================================================================
//
// Dispatches to the appropriate cutting function based on mode.
// Returns the cut line and whether it should be printed.

func cutLine(line string, opts CutOptions) (string, bool) {
	switch opts.Mode {
	case "bytes":
		return cutLineByBytes(line, opts.Ranges, opts.Complement, opts.OutputDelimiter), true
	case "characters":
		return cutLineByChars(line, opts.Ranges, opts.Complement, opts.OutputDelimiter), true
	case "fields":
		return cutLineByFields(line, opts)
	default:
		return line, true
	}
}

// =========================================================================
// runCut — the testable core of the cut tool
// =========================================================================

func runCut(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runCutWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runCutWithStdin is the inner implementation with injectable stdin.

func runCutWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "cut: %s\n", err)
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
		// Determine the mode and LIST.
		var mode, list string
		if v, ok := r.Flags["bytes"].(string); ok && v != "" {
			mode = "bytes"
			list = v
		} else if v, ok := r.Flags["characters"].(string); ok && v != "" {
			mode = "characters"
			list = v
		} else if v, ok := r.Flags["fields"].(string); ok && v != "" {
			mode = "fields"
			list = v
		} else {
			fmt.Fprintf(stderr, "cut: you must specify a list of bytes, characters, or fields\n")
			return 1
		}

		// Parse the LIST.
		ranges, err := parseList(list)
		if err != nil {
			fmt.Fprintf(stderr, "cut: %s\n", err)
			return 1
		}

		// Build options.
		opts := CutOptions{
			Mode:          mode,
			Ranges:        ranges,
			Delimiter:     "\t", // Default delimiter is tab.
			OnlyDelimited: getBool(r.Flags, "only_delimited"),
			Complement:    getBool(r.Flags, "complement"),
		}

		if d, ok := r.Flags["delimiter"].(string); ok && d != "" {
			opts.Delimiter = d
		}
		if od, ok := r.Flags["output_delimiter"].(string); ok {
			opts.OutputDelimiter = od
		}

		// Process files.
		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			files = []string{"-"}
		}

		for _, filename := range files {
			var reader io.Reader
			if filename == "-" {
				reader = stdin
			} else {
				f, err := os.Open(filename)
				if err != nil {
					fmt.Fprintf(stderr, "cut: %s: %s\n", filename, err)
					return 1
				}
				defer f.Close()
				reader = f
			}

			scanner := bufio.NewScanner(reader)
			for scanner.Scan() {
				line := scanner.Text()
				output, shouldPrint := cutLine(line, opts)
				if shouldPrint {
					fmt.Fprintln(stdout, output)
				}
			}
			if err := scanner.Err(); err != nil {
				fmt.Fprintf(stderr, "cut: %s: %s\n", filename, err)
				return 1
			}
		}

		return 0

	default:
		fmt.Fprintf(stderr, "cut: unexpected result type: %T\n", result)
		return 1
	}
}

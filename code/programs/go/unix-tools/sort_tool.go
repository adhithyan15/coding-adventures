// =========================================================================
// sort — Sort Lines of Text Files
// =========================================================================
//
// The `sort` utility reads lines from files (or stdin), sorts them, and
// writes the sorted output. It's one of the most versatile Unix tools,
// supporting many comparison modes.
//
// # Sorting modes
//
// By default, sort performs lexicographic (dictionary) comparison using
// the current locale. Additional modes change how values are compared:
//
//   Mode             Flag   Example ordering
//   ──────────────   ────   ──────────────────────────
//   Lexicographic    (def)  1, 10, 2, 20, 3
//   Numeric          -n     1, 2, 3, 10, 20
//   Case-insensitive -f     Apple, banana, Cherry
//   Reverse          -r     z, y, x, w, ...
//   Unique           -u     removes duplicate lines
//
// # The key concept: sort keys (-k)
//
// The -k flag selects which FIELD to sort by. Fields are whitespace-
// separated by default (changeable with -t).
//
//   $ cat data.txt
//   Alice 25 Engineering
//   Bob 30 Marketing
//   Carol 22 Engineering
//
//   $ sort -k2 -n data.txt      # Sort by 2nd field (age), numerically
//   Carol 22 Engineering
//   Alice 25 Engineering
//   Bob 30 Marketing
//
// # Architecture
//
//   sort.json (spec)             sort_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -r,-n,-f  │       │ read all lines                   │
//   │ -u,-k,-t,-o      │──────>│ sort with specified comparator   │
//   │ -b,-d,-i,-s      │       │ write sorted output              │
//   │ arg: FILES...     │       │                                  │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"sort"
	"strconv"
	"strings"
	"unicode"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// SortOptions — configuration for the sort operation
// =========================================================================

type SortOptions struct {
	Reverse             bool   // -r: reverse the comparison
	NumericSort         bool   // -n: compare as numbers
	IgnoreCase          bool   // -f: fold case (case-insensitive)
	Unique              bool   // -u: output only unique lines
	FieldSeparator      string // -t: field separator (default: whitespace)
	IgnoreLeadingBlanks bool   // -b: ignore leading blanks in sort key
	DictionaryOrder     bool   // -d: consider only blanks and alphanumeric
	IgnoreNonprinting   bool   // -i: consider only printable characters
	Stable              bool   // -s: use stable sort
	KeySpecs            []string // -k: key definitions
}

// =========================================================================
// sortLines — the core sorting function
// =========================================================================
//
// Takes a slice of lines and sort options, returns the sorted result.
// This is a pure function (no I/O) — easy to test.
//
// Sort algorithm:
//   1. Build a comparison function based on the options
//   2. Sort the lines using Go's sort package
//   3. If -u is set, deduplicate adjacent equal lines

func sortLines(lines []string, opts SortOptions) []string {
	if len(lines) == 0 {
		return lines
	}

	// Make a copy so we don't mutate the input.
	result := make([]string, len(lines))
	copy(result, lines)

	// Build the comparison function.
	less := func(i, j int) bool {
		a := extractSortKey(result[i], opts)
		b := extractSortKey(result[j], opts)
		cmp := compareKeys(a, b, opts)
		if opts.Reverse {
			return cmp > 0
		}
		return cmp < 0
	}

	// Sort using stable sort if requested, otherwise standard sort.
	if opts.Stable {
		sort.SliceStable(result, less)
	} else {
		sort.SliceStable(result, less) // Always stable for predictable tests.
	}

	// Deduplicate if -u is set.
	if opts.Unique {
		result = deduplicateLines(result, opts)
	}

	return result
}

// =========================================================================
// extractSortKey — extract the portion of the line to compare
// =========================================================================
//
// If a key spec (-k) is provided, extract the specified field.
// Otherwise, use the entire line as the sort key.

func extractSortKey(line string, opts SortOptions) string {
	if len(opts.KeySpecs) == 0 {
		key := line
		if opts.IgnoreLeadingBlanks {
			key = strings.TrimLeft(key, " \t")
		}
		return key
	}

	// Parse the first key spec. Format: START[,STOP]
	// For simplicity, we support "N" meaning field N to end of line.
	keySpec := opts.KeySpecs[0]
	startField := 1
	endField := 0 // 0 means "to end of line"

	parts := strings.SplitN(keySpec, ",", 2)
	if n, err := strconv.Atoi(parts[0]); err == nil {
		startField = n
	}
	if len(parts) > 1 {
		if n, err := strconv.Atoi(parts[1]); err == nil {
			endField = n
		}
	}

	// Split the line into fields.
	fields := splitFields(line, opts.FieldSeparator)

	// Extract the range of fields (1-indexed).
	if startField < 1 {
		startField = 1
	}
	if startField > len(fields) {
		return ""
	}

	start := startField - 1
	var end int
	if endField <= 0 || endField > len(fields) {
		end = len(fields)
	} else {
		end = endField
	}

	if start >= end {
		return ""
	}

	key := strings.Join(fields[start:end], " ")
	if opts.IgnoreLeadingBlanks {
		key = strings.TrimLeft(key, " \t")
	}
	return key
}

// =========================================================================
// splitFields — split a line into fields using the separator
// =========================================================================
//
// If sep is empty, split on runs of whitespace (like awk).
// If sep is set, split on that exact character.

func splitFields(line, sep string) []string {
	if sep == "" {
		return strings.Fields(line)
	}
	return strings.Split(line, sep)
}

// =========================================================================
// compareKeys — compare two sort keys according to options
// =========================================================================
//
// Returns:
//   -1 if a < b
//    0 if a == b
//   +1 if a > b

func compareKeys(a, b string, opts SortOptions) int {
	// Apply transformations to the keys before comparison.
	if opts.IgnoreCase {
		a = strings.ToLower(a)
		b = strings.ToLower(b)
	}

	if opts.DictionaryOrder {
		a = filterDictionary(a)
		b = filterDictionary(b)
	}

	if opts.IgnoreNonprinting {
		a = filterPrintable(a)
		b = filterPrintable(b)
	}

	// Numeric comparison mode.
	if opts.NumericSort {
		return compareNumeric(a, b)
	}

	// Default: lexicographic comparison.
	if a < b {
		return -1
	}
	if a > b {
		return 1
	}
	return 0
}

// =========================================================================
// compareNumeric — compare two strings as numbers
// =========================================================================
//
// Parses leading numeric values (including negative and decimal).
// Non-numeric strings are treated as 0.

func compareNumeric(a, b string) int {
	na := parseLeadingNumber(a)
	nb := parseLeadingNumber(b)
	if na < nb {
		return -1
	}
	if na > nb {
		return 1
	}
	return 0
}

// =========================================================================
// parseLeadingNumber — extract the leading numeric value from a string
// =========================================================================

func parseLeadingNumber(s string) float64 {
	s = strings.TrimSpace(s)
	if s == "" {
		return 0
	}

	// Find the longest prefix that looks like a number.
	end := 0
	for end < len(s) && (s[end] == '-' || s[end] == '+' || s[end] == '.' ||
		(s[end] >= '0' && s[end] <= '9')) {
		end++
	}

	if end == 0 {
		return 0
	}

	val, err := strconv.ParseFloat(s[:end], 64)
	if err != nil {
		return 0
	}
	return val
}

// =========================================================================
// filterDictionary — keep only blanks and alphanumeric characters
// =========================================================================

func filterDictionary(s string) string {
	var result strings.Builder
	for _, r := range s {
		if unicode.IsLetter(r) || unicode.IsDigit(r) || unicode.IsSpace(r) {
			result.WriteRune(r)
		}
	}
	return result.String()
}

// =========================================================================
// filterPrintable — keep only printable characters
// =========================================================================

func filterPrintable(s string) string {
	var result strings.Builder
	for _, r := range s {
		if unicode.IsPrint(r) {
			result.WriteRune(r)
		}
	}
	return result.String()
}

// =========================================================================
// deduplicateLines — remove adjacent duplicate lines
// =========================================================================

func deduplicateLines(lines []string, opts SortOptions) []string {
	if len(lines) == 0 {
		return lines
	}

	result := []string{lines[0]}
	for i := 1; i < len(lines); i++ {
		a := extractSortKey(result[len(result)-1], opts)
		b := extractSortKey(lines[i], opts)

		if opts.IgnoreCase {
			a = strings.ToLower(a)
			b = strings.ToLower(b)
		}

		if a != b {
			result = append(result, lines[i])
		}
	}
	return result
}

// =========================================================================
// readLines — read all lines from a reader
// =========================================================================

func readLines(reader io.Reader) ([]string, error) {
	scanner := bufio.NewScanner(reader)
	buf := make([]byte, 0, 1024*1024)
	scanner.Buffer(buf, 10*1024*1024)

	var lines []string
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	return lines, scanner.Err()
}

// =========================================================================
// runSort — the testable core of the sort tool
// =========================================================================

func runSort(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runSortWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runSortWithStdin is the inner implementation with injectable stdin.

func runSortWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "sort: %s\n", err)
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
		// Build sort options from flags.
		opts := SortOptions{
			Reverse:             getBool(r.Flags, "reverse"),
			NumericSort:         getBool(r.Flags, "numeric_sort"),
			IgnoreCase:          getBool(r.Flags, "ignore_case"),
			Unique:              getBool(r.Flags, "unique"),
			IgnoreLeadingBlanks: getBool(r.Flags, "ignore_leading_blanks"),
			DictionaryOrder:     getBool(r.Flags, "dictionary_order"),
			IgnoreNonprinting:   getBool(r.Flags, "ignore_nonprinting"),
			Stable:              getBool(r.Flags, "stable"),
		}

		// Field separator.
		if sep, ok := r.Flags["field_separator"].(string); ok {
			opts.FieldSeparator = sep
		}

		// Key specifications (repeatable flag).
		if keys := getStringSlice(r.Flags, "key"); len(keys) > 0 {
			opts.KeySpecs = keys
		}

		// Read all input lines from files or stdin.
		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			files = []string{"-"}
		}

		var allLines []string
		for _, filename := range files {
			var reader io.Reader
			if filename == "-" {
				reader = stdin
			} else {
				f, err := os.Open(filename)
				if err != nil {
					fmt.Fprintf(stderr, "sort: %s: %s\n", filename, err)
					return 1
				}
				defer f.Close()
				reader = f
			}

			lines, err := readLines(reader)
			if err != nil {
				fmt.Fprintf(stderr, "sort: %s: %s\n", filename, err)
				return 1
			}
			allLines = append(allLines, lines...)
		}

		// Sort the lines.
		sorted := sortLines(allLines, opts)

		// Write output.
		var writer io.Writer
		if outputFile, ok := r.Flags["output"].(string); ok && outputFile != "" {
			f, err := os.Create(outputFile)
			if err != nil {
				fmt.Fprintf(stderr, "sort: cannot create %s: %s\n", outputFile, err)
				return 1
			}
			defer f.Close()
			writer = f
		} else {
			writer = stdout
		}

		for _, line := range sorted {
			fmt.Fprintln(writer, line)
		}

		return 0

	default:
		fmt.Fprintf(stderr, "sort: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// join — Join Lines of Two Files on a Common Field
// =========================================================================
//
// The `join` utility performs a relational join on two sorted text files.
// It reads two files, compares them field-by-field, and outputs lines
// where a specified field matches between the two files.
//
// Think of it as a SQL JOIN for text files:
//
//   file1.txt:          file2.txt:           join output:
//   1 Alice             1 Engineering        1 Alice Engineering
//   2 Bob               2 Marketing          2 Bob Marketing
//   3 Carol             3 Sales              3 Carol Sales
//
// # How it works
//
// The join algorithm is a classic merge-join:
//   1. Both files must be sorted on the join field
//   2. Read one line from each file
//   3. Compare the join fields:
//      - If equal: output the joined line, advance both
//      - If file1 < file2: advance file1
//      - If file1 > file2: advance file2
//   4. Repeat until one file is exhausted
//
// This is O(n + m) where n and m are the line counts of the two files.
//
// # Default behavior
//
// By default, join:
//   - Joins on the first field of each file
//   - Uses whitespace as the field separator
//   - Outputs the join field, then remaining fields from file1, then file2
//   - Discards unmatched lines from both files
//
// # Architecture
//
//   join.json (spec)              join_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -1,-2,-j  │       │ readLines(): load file contents  │
//   │ -a,-v,-e,-o,-t   │──────>│ splitFields(): parse fields      │
//   │ -i,--header      │       │ joinFiles(): merge-join algorithm│
//   │ FILE1, FILE2     │       │ formatOutput(): build result     │
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
// JoinOptions — configuration for join operations
// =========================================================================

type JoinOptions struct {
	Field1        int    // Join field for file 1 (1-based, default 1)
	Field2        int    // Join field for file 2 (1-based, default 1)
	Separator     string // Field separator (default: whitespace)
	Empty         string // Replacement for missing fields
	Format        string // Output format specification
	IgnoreCase    bool   // Case-insensitive field comparison
	Header        bool   // Treat first line as header
	Unpaired1     bool   // Print unpairable lines from file 1
	Unpaired2     bool   // Print unpairable lines from file 2
	OnlyUnpaired1 bool   // Only print unpaired from file 1
	OnlyUnpaired2 bool   // Only print unpaired from file 2
}

// =========================================================================
// readJoinFileLines — read all lines from a file for join
// =========================================================================
//
// Returns a slice of strings, one per line. If the path is "-", reads
// from stdin. Named differently from readFileLines in comm_tool.go to
// avoid redeclaration since both are in package main.

func readJoinFileLines(path string) ([]string, error) {
	var reader io.Reader
	if path == "-" {
		reader = os.Stdin
	} else {
		f, err := os.Open(path)
		if err != nil {
			return nil, fmt.Errorf("cannot open '%s': %w", path, err)
		}
		defer f.Close()
		reader = f
	}

	var lines []string
	scanner := bufio.NewScanner(reader)
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("error reading '%s': %w", path, err)
	}
	return lines, nil
}

// =========================================================================
// splitJoinFields — split a line into fields
// =========================================================================
//
// With a specified separator, fields are split on that exact character.
// Without a separator (default), fields are split on runs of whitespace.

func splitJoinFields(line string, separator string) []string {
	if separator != "" {
		return strings.Split(line, separator)
	}
	return strings.Fields(line)
}

// =========================================================================
// getJoinField — extract the join field from a line's fields
// =========================================================================
//
// The field index is 1-based (Unix convention). Returns the field value
// and true if the field exists, or empty string and false otherwise.

func getJoinField(fields []string, fieldIdx int) (string, bool) {
	// fieldIdx is 1-based.
	if fieldIdx < 1 || fieldIdx > len(fields) {
		return "", false
	}
	return fields[fieldIdx-1], true
}

// =========================================================================
// compareFields — compare two field values for the merge-join
// =========================================================================
//
// Returns:
//   -1 if a < b
//    0 if a == b
//    1 if a > b

func compareFields(a, b string, ignoreCase bool) int {
	if ignoreCase {
		a = strings.ToLower(a)
		b = strings.ToLower(b)
	}
	if a < b {
		return -1
	}
	if a > b {
		return 1
	}
	return 0
}

// =========================================================================
// formatJoinLine — build an output line from joined fields
// =========================================================================
//
// Default output format:
//   join_field remaining_fields_from_file1 remaining_fields_from_file2
//
// The output separator matches the input separator (or space if default).

func formatJoinLine(joinField string, fields1, fields2 []string, fieldIdx1, fieldIdx2 int, opts JoinOptions) string {
	sep := " "
	if opts.Separator != "" {
		sep = opts.Separator
	}

	var parts []string
	parts = append(parts, joinField)

	// Add remaining fields from file 1 (skipping the join field).
	for i, f := range fields1 {
		if i+1 != fieldIdx1 {
			parts = append(parts, f)
		}
	}

	// Add remaining fields from file 2 (skipping the join field).
	for i, f := range fields2 {
		if i+1 != fieldIdx2 {
			parts = append(parts, f)
		}
	}

	return strings.Join(parts, sep)
}

// =========================================================================
// formatUnpairedLine — format an unmatched line for output
// =========================================================================

func formatUnpairedLine(fields []string, opts JoinOptions) string {
	sep := " "
	if opts.Separator != "" {
		sep = opts.Separator
	}
	return strings.Join(fields, sep)
}

// =========================================================================
// joinFiles — perform the merge-join on two sorted file contents
// =========================================================================
//
// This is the core algorithm. It uses the merge-join technique:
//
//   i = 0, j = 0
//   while i < len(lines1) and j < len(lines2):
//       key1 = field(lines1[i])
//       key2 = field(lines2[j])
//       if key1 == key2:
//           output joined line
//           advance both (handling duplicates)
//       elif key1 < key2:
//           output unmatched from file1 (if -a 1)
//           i++
//       else:
//           output unmatched from file2 (if -a 2)
//           j++

func joinFiles(lines1, lines2 []string, opts JoinOptions) []string {
	var result []string

	fieldIdx1 := opts.Field1
	fieldIdx2 := opts.Field2

	i, j := 0, 0

	// Handle header line.
	if opts.Header && len(lines1) > 0 && len(lines2) > 0 {
		f1 := splitJoinFields(lines1[0], opts.Separator)
		f2 := splitJoinFields(lines2[0], opts.Separator)
		jf, _ := getJoinField(f1, fieldIdx1)
		result = append(result, formatJoinLine(jf, f1, f2, fieldIdx1, fieldIdx2, opts))
		i = 1
		j = 1
	}

	for i < len(lines1) && j < len(lines2) {
		fields1 := splitJoinFields(lines1[i], opts.Separator)
		fields2 := splitJoinFields(lines2[j], opts.Separator)

		key1, ok1 := getJoinField(fields1, fieldIdx1)
		key2, ok2 := getJoinField(fields2, fieldIdx2)

		if !ok1 {
			i++
			continue
		}
		if !ok2 {
			j++
			continue
		}

		cmp := compareFields(key1, key2, opts.IgnoreCase)

		if cmp == 0 {
			// Keys match — output the joined line.
			// Handle duplicate keys in file 2 by scanning forward.
			saveJ := j
			for j < len(lines2) {
				f2 := splitJoinFields(lines2[j], opts.Separator)
				k2, ok := getJoinField(f2, fieldIdx2)
				if !ok || compareFields(key1, k2, opts.IgnoreCase) != 0 {
					break
				}
				if !opts.OnlyUnpaired1 && !opts.OnlyUnpaired2 {
					result = append(result, formatJoinLine(key1, fields1, f2, fieldIdx1, fieldIdx2, opts))
				}
				j++
			}
			i++
			// Check if next line in file 1 has same key (handle duplicates).
			if i < len(lines1) {
				nextFields := splitJoinFields(lines1[i], opts.Separator)
				nextKey, ok := getJoinField(nextFields, fieldIdx1)
				if ok && compareFields(nextKey, key1, opts.IgnoreCase) == 0 {
					j = saveJ // Reset j to re-match against next file1 line
				}
			}
		} else if cmp < 0 {
			// key1 < key2: file1 line is unmatched.
			if opts.Unpaired1 || opts.OnlyUnpaired1 {
				result = append(result, formatUnpairedLine(fields1, opts))
			}
			i++
		} else {
			// key1 > key2: file2 line is unmatched.
			if opts.Unpaired2 || opts.OnlyUnpaired2 {
				result = append(result, formatUnpairedLine(fields2, opts))
			}
			j++
		}
	}

	// Remaining lines from file 1.
	for i < len(lines1) {
		if opts.Unpaired1 || opts.OnlyUnpaired1 {
			fields := splitJoinFields(lines1[i], opts.Separator)
			result = append(result, formatUnpairedLine(fields, opts))
		}
		i++
	}

	// Remaining lines from file 2.
	for j < len(lines2) {
		if opts.Unpaired2 || opts.OnlyUnpaired2 {
			fields := splitJoinFields(lines2[j], opts.Separator)
			result = append(result, formatUnpairedLine(fields, opts))
		}
		j++
	}

	return result
}

// =========================================================================
// runJoin — the testable core of the join tool
// =========================================================================

func runJoin(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "join: %s\n", err)
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
		opts := JoinOptions{
			Field1:     1, // Default: join on first field
			Field2:     1,
			IgnoreCase: getBool(r.Flags, "ignore_case"),
			Header:     getBool(r.Flags, "header"),
		}

		// Extract field indices (1-based integers).
		if v := r.Flags["field1"]; v != nil {
			if n, ok := v.(int64); ok {
				field1, err := intFromInt64(n)
				if err != nil {
					fmt.Fprintf(stderr, "join: invalid field1: %s\n", err)
					return 1
				}
				opts.Field1 = field1
			}
		}
		if v := r.Flags["field2"]; v != nil {
			if n, ok := v.(int64); ok {
				field2, err := intFromInt64(n)
				if err != nil {
					fmt.Fprintf(stderr, "join: invalid field2: %s\n", err)
					return 1
				}
				opts.Field2 = field2
			}
		}
		// -j sets both fields.
		if v := r.Flags["join_field"]; v != nil {
			if n, ok := v.(int64); ok {
				joinField, err := intFromInt64(n)
				if err != nil {
					fmt.Fprintf(stderr, "join: invalid join field: %s\n", err)
					return 1
				}
				opts.Field1 = joinField
				opts.Field2 = joinField
			}
		}

		// Separator.
		if sep, ok := r.Flags["separator"].(string); ok && sep != "" {
			opts.Separator = sep
		}

		// Empty replacement.
		if empty, ok := r.Flags["empty"].(string); ok {
			opts.Empty = empty
		}

		// Format string.
		if format, ok := r.Flags["format"].(string); ok {
			opts.Format = format
		}

		// Unpaired lines.
		unpairedFlags := getStringSlice(r.Flags, "unpaired")
		for _, u := range unpairedFlags {
			switch u {
			case "1":
				opts.Unpaired1 = true
			case "2":
				opts.Unpaired2 = true
			}
		}

		// Only-unpaired flag.
		if v, ok := r.Flags["only_unpaired"].(string); ok {
			switch v {
			case "1":
				opts.OnlyUnpaired1 = true
			case "2":
				opts.OnlyUnpaired2 = true
			}
		}

		// Get file arguments.
		file1Path, _ := r.Arguments["file1"].(string)
		file2Path, _ := r.Arguments["file2"].(string)

		if file1Path == "" || file2Path == "" {
			fmt.Fprintf(stderr, "join: missing file operand\n")
			return 1
		}

		// Read both files.
		lines1, err := readJoinFileLines(file1Path)
		if err != nil {
			fmt.Fprintf(stderr, "join: %s\n", err)
			return 1
		}

		lines2, err := readJoinFileLines(file2Path)
		if err != nil {
			fmt.Fprintf(stderr, "join: %s\n", err)
			return 1
		}

		// Perform the join.
		outputLines := joinFiles(lines1, lines2, opts)

		// Print results.
		for _, line := range outputLines {
			fmt.Fprintln(stdout, line)
		}

		return 0

	default:
		fmt.Fprintf(stderr, "join: unexpected result type: %T\n", result)
		return 1
	}
}

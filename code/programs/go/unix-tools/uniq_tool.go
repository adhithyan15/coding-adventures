// =========================================================================
// uniq — Report or Omit Repeated Lines
// =========================================================================
//
// The `uniq` utility filters out (or reports) adjacent duplicate lines.
// Key word: ADJACENT. Unlike `sort -u`, uniq only compares consecutive
// lines. To find all duplicates, sort the input first: `sort | uniq`.
//
// # Basic usage
//
//   uniq file.txt               Remove adjacent duplicate lines
//   uniq -c file.txt            Count occurrences of each line
//   uniq -d file.txt            Show only duplicated lines
//   uniq -u file.txt            Show only unique lines
//   uniq -i file.txt            Case-insensitive comparison
//
// # How adjacency works
//
//   Input:        Output (default):   Output (-c):
//   apple         apple               2 apple
//   apple         banana              1 banana
//   banana        apple               1 apple
//   apple
//
// Notice "apple" appears twice in the output because the two groups
// of "apple" are separated by "banana".
//
// # Architecture
//
//   uniq.json (spec)            uniq_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -c,-d,-u  │      │ read lines from input          │
//   │ -i,-f,-s,-w,-z   │─────>│ group adjacent identical lines │
//   │ args: IN, OUT    │      │ filter/format per flags        │
//   └──────────────────┘      └────────────────────────────────┘

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
// uniqGroup — represents a group of adjacent identical lines
// =========================================================================

type uniqGroup struct {
	line  string // the original line text
	count int    // how many consecutive times it appeared
}

// =========================================================================
// compareKey — extract the comparison key from a line
// =========================================================================
//
// This function applies the skip-fields (-f), skip-chars (-s), and
// check-chars (-w) transformations to produce the portion of the line
// that should be compared.
//
// Processing order:
//   1. Skip the first N fields (whitespace-delimited)
//   2. Skip the first M characters from what remains
//   3. Take only the first W characters from what remains

func compareKey(line string, skipFields, skipChars, checkChars int, ignoreCase bool) string {
	result := line

	// Step 1: Skip fields.
	if skipFields > 0 {
		remaining := result
		for i := 0; i < skipFields; i++ {
			// Skip leading whitespace.
			remaining = strings.TrimLeft(remaining, " \t")
			// Skip the non-whitespace field.
			idx := strings.IndexAny(remaining, " \t")
			if idx < 0 {
				remaining = ""
				break
			}
			remaining = remaining[idx:]
		}
		result = strings.TrimLeft(remaining, " \t")
	}

	// Step 2: Skip characters.
	runes := []rune(result)
	if skipChars > 0 && skipChars < len(runes) {
		runes = runes[skipChars:]
	} else if skipChars >= len(runes) {
		runes = nil
	}

	// Step 3: Limit to check-chars characters.
	if checkChars > 0 && checkChars < len(runes) {
		runes = runes[:checkChars]
	}

	result = string(runes)

	// Step 4: Case folding.
	if ignoreCase {
		result = strings.ToLower(result)
	}

	return result
}

// =========================================================================
// processUniq — the core uniq algorithm
// =========================================================================
//
// Reads lines, groups adjacent duplicates, then filters and formats
// according to the options.

func processUniq(content string, showCount, showRepeated, showUnique, ignoreCase bool,
	skipFields, skipChars, checkChars int) string {

	lines := strings.Split(content, "\n")

	// Remove trailing empty element from the split (if content ends with \n).
	if len(lines) > 0 && lines[len(lines)-1] == "" {
		lines = lines[:len(lines)-1]
	}

	if len(lines) == 0 {
		return ""
	}

	// Group adjacent identical lines.
	var groups []uniqGroup
	currentGroup := uniqGroup{line: lines[0], count: 1}

	for i := 1; i < len(lines); i++ {
		key1 := compareKey(currentGroup.line, skipFields, skipChars, checkChars, ignoreCase)
		key2 := compareKey(lines[i], skipFields, skipChars, checkChars, ignoreCase)

		if key1 == key2 {
			// Same group — increment count.
			currentGroup.count++
		} else {
			// New group — save the current one and start a new one.
			groups = append(groups, currentGroup)
			currentGroup = uniqGroup{line: lines[i], count: 1}
		}
	}
	groups = append(groups, currentGroup)

	// Filter and format the output.
	var result strings.Builder
	for _, g := range groups {
		// Apply filters.
		if showRepeated && g.count < 2 {
			continue // -d: skip non-duplicates
		}
		if showUnique && g.count > 1 {
			continue // -u: skip duplicates
		}

		// Format the output line.
		if showCount {
			fmt.Fprintf(&result, "%7d %s\n", g.count, g.line)
		} else {
			result.WriteString(g.line + "\n")
		}
	}

	return result.String()
}

// =========================================================================
// runUniq — the testable core of the uniq tool
// =========================================================================

func runUniq(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runUniqWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runUniqWithStdin is the inner implementation that accepts a custom stdin.

func runUniqWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "uniq: %s\n", err)
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
		showCount := getBool(r.Flags, "count")
		showRepeated := getBool(r.Flags, "repeated")
		showUnique := getBool(r.Flags, "unique")
		ignoreCase := getBool(r.Flags, "ignore_case")

		// Extract integer flags with defaults of 0.
		skipFields := 0
		if v, ok := r.Flags["skip_fields"].(int64); ok {
			skipFields = int(v)
		}
		skipChars := 0
		if v, ok := r.Flags["skip_chars"].(int64); ok {
			skipChars = int(v)
		}
		checkChars := 0
		if v, ok := r.Flags["check_chars"].(int64); ok {
			checkChars = int(v)
		}

		// Determine input source.
		var reader io.Reader
		inputFile, _ := r.Arguments["input"].(string)
		if inputFile != "" && inputFile != "-" {
			f, err := os.Open(inputFile)
			if err != nil {
				fmt.Fprintf(stderr, "uniq: %s: %s\n", inputFile, err)
				return 1
			}
			defer f.Close()
			reader = f
		} else {
			reader = stdin
		}

		// Read all input.
		scanner := bufio.NewScanner(reader)
		buf := make([]byte, 0, 1024*1024)
		scanner.Buffer(buf, 1024*1024)

		var lines []string
		for scanner.Scan() {
			lines = append(lines, scanner.Text())
		}
		if err := scanner.Err(); err != nil {
			fmt.Fprintf(stderr, "uniq: error reading input: %s\n", err)
			return 1
		}

		content := strings.Join(lines, "\n")
		if len(lines) > 0 {
			content += "\n"
		}

		// Process and output.
		output := processUniq(content, showCount, showRepeated, showUnique, ignoreCase,
			skipFields, skipChars, checkChars)

		// Determine output destination.
		var writer io.Writer
		outputFile, _ := r.Arguments["output"].(string)
		if outputFile != "" && outputFile != "-" {
			f, err := os.Create(outputFile)
			if err != nil {
				fmt.Fprintf(stderr, "uniq: %s: %s\n", outputFile, err)
				return 1
			}
			defer f.Close()
			writer = f
		} else {
			writer = stdout
		}

		fmt.Fprint(writer, output)
		return 0

	default:
		fmt.Fprintf(stderr, "uniq: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// diff — Compare Files Line by Line
// =========================================================================
//
// The `diff` utility compares two files (or directories) and reports
// the differences between them. It is one of the most important tools
// in the Unix toolbox, forming the backbone of version control systems,
// code review workflows, and patch distribution.
//
// # How diff works
//
// At its core, diff solves the "Longest Common Subsequence" (LCS) problem:
// given two sequences of lines, find the longest subsequence that appears
// in both. The lines NOT in the LCS are the differences.
//
// For example, given:
//
//   File A:      File B:
//   apple        apple
//   banana       cherry
//   cherry       cherry
//   date         fig
//
//   LCS: apple, cherry (these lines are in both files)
//   Differences: banana (only in A), date (only in A), fig (only in B)
//
// # Output formats
//
// diff supports several output formats:
//
//   Format     Flag   Description
//   ─────────  ────   ──────────────────────────────────
//   Normal     (def)  Classic format: "2d1", "< banana"
//   Unified    -u     Modern format with @@ markers
//   Context    -c     Old format with *** and --- markers
//   Brief      -q     Just "Files X and Y differ"
//
// # Flags overview
//
//   -u, --unified       Unified diff format (most common)
//   -c, --context        Context diff format
//   -q, --brief          Only report whether files differ
//   -r, --recursive      Recursively compare directories
//   -i, --ignore-case    Case-insensitive comparison
//   -b, --ignore-space-change  Ignore whitespace changes
//   -w, --ignore-all-space     Ignore all whitespace
//   -B, --ignore-blank-lines   Ignore blank line changes
//
// # Architecture
//
//   diff.json (spec)             diff_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -u,-c,-q  │       │ read both files into lines       │
//   │ -r,-i,-b,-w,-B   │──────>│ compute LCS via dynamic prog     │
//   │ args: FILE1 FILE2│       │ format differences as output     │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// getInt — safely extract an integer from a parsed flags/arguments map
// =========================================================================
//
// The cli-builder parser may return integer values as int, int64, or
// float64 depending on the parsing path. This helper handles all cases,
// returning the integer value and a boolean indicating success.

func getInt(m map[string]interface{}, key string) (int, bool) {
	v, ok := m[key]
	if !ok || v == nil {
		return 0, false
	}
	switch n := v.(type) {
	case int:
		return n, true
	case int64:
		return int(n), true
	case float64:
		return int(n), true
	default:
		return 0, false
	}
}

// =========================================================================
// DiffOptions — configuration for the diff operation
// =========================================================================
//
// This struct bundles all the flag values that control how differences
// are computed and displayed.

type DiffOptions struct {
	Unified          int  // -u: lines of unified context (0 = not unified)
	Context          int  // -c: lines of context format (0 = not context)
	UseUnified       bool // whether -u was explicitly requested
	UseContext       bool // whether -c was explicitly requested
	Brief            bool // -q: only report whether files differ
	Recursive        bool // -r: compare directories recursively
	IgnoreCase       bool // -i: case-insensitive comparison
	IgnoreSpaceChg   bool // -b: ignore changes in whitespace amount
	IgnoreAllSpace   bool // -w: ignore all whitespace
	IgnoreBlankLines bool // -B: ignore blank line changes
}

// =========================================================================
// diffEdit — represents a single edit operation in the diff
// =========================================================================
//
// A diff is essentially a sequence of edit operations that transform
// file A into file B. Each edit is one of:
//
//   - Equal: the line exists in both files (no change)
//   - Insert: the line was added in file B
//   - Delete: the line was removed from file A
//
// This is the standard "edit script" representation used by most
// diff implementations.

type diffEdit struct {
	Op   byte   // '=' for equal, '+' for insert, '-' for delete
	Line string // the actual line content
	IdxA int    // line index in file A (0-based, -1 if insert)
	IdxB int    // line index in file B (0-based, -1 if delete)
}

// =========================================================================
// readDiffLines — read all lines from a file
// =========================================================================
//
// Reads a file and returns its contents as a slice of strings,
// one per line. This is distinct from the sort tool's readLines
// to avoid name collisions.

func readDiffLines(filename string) ([]string, error) {
	f, err := os.Open(filename)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	var lines []string
	scanner := bufio.NewScanner(f)
	buf := make([]byte, 0, 1024*1024)
	scanner.Buffer(buf, 10*1024*1024)
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	return lines, scanner.Err()
}

// =========================================================================
// normalizeDiffLine — apply comparison options to a line
// =========================================================================
//
// Before comparing lines, we may need to normalize them based on
// the user's flags. For example, -i makes comparison case-insensitive
// by lowercasing both lines before comparison.
//
// Truth table:
//
//   Flag             Transformation
//   ──────────────   ──────────────────────────
//   -i (ignore case) strings.ToLower
//   -w (all space)   remove all whitespace
//   -b (space chg)   collapse runs of whitespace to single space

func normalizeDiffLine(line string, opts DiffOptions) string {
	if opts.IgnoreCase {
		line = strings.ToLower(line)
	}
	if opts.IgnoreAllSpace {
		var b strings.Builder
		for _, r := range line {
			if r != ' ' && r != '\t' && r != '\r' {
				b.WriteRune(r)
			}
		}
		return b.String()
	}
	if opts.IgnoreSpaceChg {
		// Collapse runs of whitespace to a single space and trim.
		fields := strings.Fields(line)
		return strings.Join(fields, " ")
	}
	return line
}

// =========================================================================
// computeDiffEdits — compute the edit script between two files
// =========================================================================
//
// This implements the classic LCS-based diff algorithm using dynamic
// programming. The algorithm works in two phases:
//
// Phase 1: Build the LCS table
//   We create a 2D table where lcs[i][j] is the length of the longest
//   common subsequence of linesA[0..i-1] and linesB[0..j-1].
//
// Phase 2: Backtrack to produce the edit script
//   Starting from lcs[m][n], we trace back through the table:
//   - If lines match, it's an Equal edit
//   - If we came from above, it's a Delete
//   - If we came from the left, it's an Insert
//
// Time complexity: O(m * n) where m and n are the file lengths
// Space complexity: O(m * n) for the LCS table

func computeDiffEdits(linesA, linesB []string, opts DiffOptions) []diffEdit {
	m := len(linesA)
	n := len(linesB)

	// Phase 1: Build the LCS table.
	// lcs[i][j] = length of LCS of linesA[0..i-1] and linesB[0..j-1]
	lcs := make([][]int, m+1)
	for i := 0; i <= m; i++ {
		lcs[i] = make([]int, n+1)
	}
	for i := 1; i <= m; i++ {
		for j := 1; j <= n; j++ {
			normA := normalizeDiffLine(linesA[i-1], opts)
			normB := normalizeDiffLine(linesB[j-1], opts)
			if normA == normB {
				lcs[i][j] = lcs[i-1][j-1] + 1
			} else if lcs[i-1][j] >= lcs[i][j-1] {
				lcs[i][j] = lcs[i-1][j]
			} else {
				lcs[i][j] = lcs[i][j-1]
			}
		}
	}

	// Phase 2: Backtrack to produce the edit script.
	var edits []diffEdit
	i, j := m, n
	for i > 0 || j > 0 {
		if i > 0 && j > 0 {
			normA := normalizeDiffLine(linesA[i-1], opts)
			normB := normalizeDiffLine(linesB[j-1], opts)
			if normA == normB {
				edits = append(edits, diffEdit{'=', linesA[i-1], i - 1, j - 1})
				i--
				j--
				continue
			}
		}
		if i > 0 && (j == 0 || lcs[i-1][j] >= lcs[i][j-1]) {
			edits = append(edits, diffEdit{'-', linesA[i-1], i - 1, -1})
			i--
		} else {
			edits = append(edits, diffEdit{'+', linesB[j-1], -1, j - 1})
			j--
		}
	}

	// The edits are in reverse order from the backtracking. Reverse them.
	for left, right := 0, len(edits)-1; left < right; left, right = left+1, right-1 {
		edits[left], edits[right] = edits[right], edits[left]
	}

	return edits
}

// =========================================================================
// filterBlankLineEdits — remove edits that only affect blank lines
// =========================================================================
//
// When -B (ignore blank lines) is set, we convert any insert or delete
// of a blank line into an equal edit, effectively hiding those changes.

func filterBlankLineEdits(edits []diffEdit) []diffEdit {
	result := make([]diffEdit, len(edits))
	copy(result, edits)
	for i := range result {
		if result[i].Op != '=' && strings.TrimSpace(result[i].Line) == "" {
			result[i].Op = '='
		}
	}
	return result
}

// =========================================================================
// formatNormalDiff — format edits in the classic "normal" diff format
// =========================================================================
//
// Normal diff format looks like:
//
//   2d1
//   < banana
//   3a3
//   > fig
//
// Each "hunk" header uses the format:
//   La,Lb{a|c|d}Ra,Rb
// Where L = left line range, R = right line range, and the letter
// indicates add (a), change (c), or delete (d).

func formatNormalDiff(edits []diffEdit, w io.Writer) {
	i := 0
	for i < len(edits) {
		if edits[i].Op == '=' {
			i++
			continue
		}

		// Collect a contiguous group of changes (deletes then inserts).
		delStart := i
		var dels, ins []diffEdit
		for i < len(edits) && edits[i].Op == '-' {
			dels = append(dels, edits[i])
			i++
		}
		for i < len(edits) && edits[i].Op == '+' {
			ins = append(ins, edits[i])
			i++
		}
		_ = delStart

		// Build the hunk header.
		if len(dels) > 0 && len(ins) > 0 {
			// Change
			fmt.Fprintf(w, "%sc%s\n",
				diffRange(dels[0].IdxA+1, dels[len(dels)-1].IdxA+1),
				diffRange(ins[0].IdxB+1, ins[len(ins)-1].IdxB+1))
			for _, d := range dels {
				fmt.Fprintf(w, "< %s\n", d.Line)
			}
			fmt.Fprintln(w, "---")
			for _, a := range ins {
				fmt.Fprintf(w, "> %s\n", a.Line)
			}
		} else if len(dels) > 0 {
			// Delete
			afterLineB := 0
			// Find the line number in B after the deletion point
			for j := delStart - 1; j >= 0; j-- {
				if edits[j].IdxB >= 0 {
					afterLineB = edits[j].IdxB + 1
					break
				}
			}
			fmt.Fprintf(w, "%sd%d\n",
				diffRange(dels[0].IdxA+1, dels[len(dels)-1].IdxA+1),
				afterLineB)
			for _, d := range dels {
				fmt.Fprintf(w, "< %s\n", d.Line)
			}
		} else if len(ins) > 0 {
			// Add
			afterLineA := 0
			for j := delStart - 1; j >= 0; j-- {
				if edits[j].IdxA >= 0 {
					afterLineA = edits[j].IdxA + 1
					break
				}
			}
			fmt.Fprintf(w, "%da%s\n",
				afterLineA,
				diffRange(ins[0].IdxB+1, ins[len(ins)-1].IdxB+1))
			for _, a := range ins {
				fmt.Fprintf(w, "> %s\n", a.Line)
			}
		}
	}
}

// diffRange formats a line range: "3" for a single line, "3,5" for a range.
func diffRange(start, end int) string {
	if start == end {
		return fmt.Sprintf("%d", start)
	}
	return fmt.Sprintf("%d,%d", start, end)
}

// =========================================================================
// formatUnifiedDiff — format edits in unified diff format
// =========================================================================
//
// Unified diff is the most commonly used format today. It looks like:
//
//   --- file1.txt
//   +++ file2.txt
//   @@ -1,4 +1,4 @@
//    apple
//   -banana
//   -cherry
//   +cherry
//   +fig
//
// Context lines (unchanged) appear with a space prefix.
// Deleted lines get a '-' prefix, added lines get a '+' prefix.
// The @@ header shows line ranges for both files.

func formatUnifiedDiff(edits []diffEdit, fileA, fileB string, ctxLines int, w io.Writer) {
	fmt.Fprintf(w, "--- %s\n", fileA)
	fmt.Fprintf(w, "+++ %s\n", fileB)

	// Group edits into hunks. A hunk is a contiguous region of changes
	// plus surrounding context lines.
	hunks := buildDiffHunks(edits, ctxLines)

	for _, hunk := range hunks {
		// Calculate line ranges.
		startA, countA := 0, 0
		startB, countB := 0, 0
		firstA, firstB := true, true

		for _, e := range hunk {
			switch e.Op {
			case '=':
				if firstA {
					startA = e.IdxA + 1
					firstA = false
				}
				if firstB {
					startB = e.IdxB + 1
					firstB = false
				}
				countA++
				countB++
			case '-':
				if firstA {
					startA = e.IdxA + 1
					firstA = false
				}
				countA++
			case '+':
				if firstB {
					startB = e.IdxB + 1
					firstB = false
				}
				countB++
			}
		}

		if firstA {
			startA = 1
		}
		if firstB {
			startB = 1
		}

		fmt.Fprintf(w, "@@ -%d,%d +%d,%d @@\n", startA, countA, startB, countB)

		for _, e := range hunk {
			switch e.Op {
			case '=':
				fmt.Fprintf(w, " %s\n", e.Line)
			case '-':
				fmt.Fprintf(w, "-%s\n", e.Line)
			case '+':
				fmt.Fprintf(w, "+%s\n", e.Line)
			}
		}
	}
}

// =========================================================================
// formatContextDiff — format edits in context diff format
// =========================================================================
//
// Context diff format (older) looks like:
//
//   *** file1.txt
//   --- file2.txt
//   ***************
//   *** 1,4 ****
//   ...
//   --- 1,4 ----
//   ...

func formatContextDiff(edits []diffEdit, fileA, fileB string, ctxLines int, w io.Writer) {
	fmt.Fprintf(w, "*** %s\n", fileA)
	fmt.Fprintf(w, "--- %s\n", fileB)

	hunks := buildDiffHunks(edits, ctxLines)

	for _, hunk := range hunks {
		fmt.Fprintln(w, "***************")

		// Compute ranges for the *** section (file A lines).
		aStart, aEnd := 0, 0
		bStart, bEnd := 0, 0
		firstA, firstB := true, true

		for _, e := range hunk {
			if e.IdxA >= 0 {
				if firstA {
					aStart = e.IdxA + 1
					firstA = false
				}
				aEnd = e.IdxA + 1
			}
			if e.IdxB >= 0 {
				if firstB {
					bStart = e.IdxB + 1
					firstB = false
				}
				bEnd = e.IdxB + 1
			}
		}

		if firstA {
			aStart = 1
			aEnd = 0
		}
		if firstB {
			bStart = 1
			bEnd = 0
		}

		// Print file A section.
		fmt.Fprintf(w, "*** %s ****\n", diffRange(aStart, aEnd))
		for _, e := range hunk {
			if e.Op == '+' {
				continue
			}
			prefix := "  "
			if e.Op == '-' {
				prefix = "- "
			}
			fmt.Fprintf(w, "%s%s\n", prefix, e.Line)
		}

		// Print file B section.
		fmt.Fprintf(w, "--- %s ----\n", diffRange(bStart, bEnd))
		for _, e := range hunk {
			if e.Op == '-' {
				continue
			}
			prefix := "  "
			if e.Op == '+' {
				prefix = "+ "
			}
			fmt.Fprintf(w, "%s%s\n", prefix, e.Line)
		}
	}
}

// =========================================================================
// buildDiffHunks — group edits into hunks with context
// =========================================================================
//
// A hunk is a contiguous group of changes surrounded by context lines.
// If two change groups are separated by fewer than 2*ctxLines of equal
// lines, they are merged into a single hunk.

func buildDiffHunks(edits []diffEdit, ctxLines int) [][]diffEdit {
	if len(edits) == 0 {
		return nil
	}

	// Find the indices of all change edits.
	var changeIndices []int
	for i, e := range edits {
		if e.Op != '=' {
			changeIndices = append(changeIndices, i)
		}
	}

	if len(changeIndices) == 0 {
		return nil
	}

	// Build hunks by expanding context around each change.
	var hunks [][]diffEdit
	hunkStart := changeIndices[0] - ctxLines
	if hunkStart < 0 {
		hunkStart = 0
	}
	hunkEnd := changeIndices[0]

	for ci := 0; ci < len(changeIndices); ci++ {
		idx := changeIndices[ci]

		// Extend hunkEnd past this change.
		hunkEnd = idx + 1

		// Check if next change is close enough to merge.
		if ci+1 < len(changeIndices) {
			nextIdx := changeIndices[ci+1]
			gap := nextIdx - hunkEnd
			if gap <= 2*ctxLines {
				// Merge: extend hunk to include the gap and next change.
				continue
			}
		}

		// Close the current hunk with trailing context.
		trailEnd := hunkEnd + ctxLines
		if trailEnd > len(edits) {
			trailEnd = len(edits)
		}

		hunks = append(hunks, edits[hunkStart:trailEnd])

		// Start the next hunk.
		if ci+1 < len(changeIndices) {
			hunkStart = changeIndices[ci+1] - ctxLines
			if hunkStart < 0 {
				hunkStart = 0
			}
		}
	}

	return hunks
}

// =========================================================================
// hasDiffChanges — check if there are any non-equal edits
// =========================================================================

func hasDiffChanges(edits []diffEdit) bool {
	for _, e := range edits {
		if e.Op != '=' {
			return true
		}
	}
	return false
}

// =========================================================================
// diffDirectory — recursively compare two directories
// =========================================================================

func diffDirectory(dirA, dirB string, opts DiffOptions, stdout, stderr io.Writer) int {
	exitCode := 0

	entriesA, err := os.ReadDir(dirA)
	if err != nil {
		fmt.Fprintf(stderr, "diff: %s: %s\n", dirA, err)
		return 2
	}
	entriesB, err := os.ReadDir(dirB)
	if err != nil {
		fmt.Fprintf(stderr, "diff: %s: %s\n", dirB, err)
		return 2
	}

	// Build maps for quick lookup.
	mapA := make(map[string]os.DirEntry)
	for _, e := range entriesA {
		mapA[e.Name()] = e
	}
	mapB := make(map[string]os.DirEntry)
	for _, e := range entriesB {
		mapB[e.Name()] = e
	}

	// Collect all unique names, sorted.
	allNames := make(map[string]bool)
	for _, e := range entriesA {
		allNames[e.Name()] = true
	}
	for _, e := range entriesB {
		allNames[e.Name()] = true
	}

	// Sort names.
	var names []string
	for n := range allNames {
		names = append(names, n)
	}
	sortDiffNames(names)

	for _, name := range names {
		pathA := filepath.Join(dirA, name)
		pathB := filepath.Join(dirB, name)

		_, inA := mapA[name]
		_, inB := mapB[name]

		if inA && !inB {
			fmt.Fprintf(stdout, "Only in %s: %s\n", dirA, name)
			if exitCode < 1 {
				exitCode = 1
			}
			continue
		}
		if !inA && inB {
			fmt.Fprintf(stdout, "Only in %s: %s\n", dirB, name)
			if exitCode < 1 {
				exitCode = 1
			}
			continue
		}

		eA := mapA[name]
		eB := mapB[name]

		if eA.IsDir() && eB.IsDir() {
			rc := diffDirectory(pathA, pathB, opts, stdout, stderr)
			if rc > exitCode {
				exitCode = rc
			}
		} else if !eA.IsDir() && !eB.IsDir() {
			rc := diffFiles(pathA, pathB, opts, stdout, stderr)
			if rc > exitCode {
				exitCode = rc
			}
		} else {
			fmt.Fprintf(stderr, "diff: %s is a %s while %s is a %s\n",
				pathA, diffFileType(eA), pathB, diffFileType(eB))
			if exitCode < 1 {
				exitCode = 1
			}
		}
	}

	return exitCode
}

func diffFileType(e os.DirEntry) string {
	if e.IsDir() {
		return "directory"
	}
	return "regular file"
}

// sortDiffNames sorts a slice of strings in place.
func sortDiffNames(names []string) {
	for i := 1; i < len(names); i++ {
		key := names[i]
		j := i - 1
		for j >= 0 && names[j] > key {
			names[j+1] = names[j]
			j--
		}
		names[j+1] = key
	}
}

// =========================================================================
// diffFiles — compare two specific files
// =========================================================================

func diffFiles(fileA, fileB string, opts DiffOptions, stdout, stderr io.Writer) int {
	linesA, err := readDiffLines(fileA)
	if err != nil {
		fmt.Fprintf(stderr, "diff: %s: %s\n", fileA, err)
		return 2
	}
	linesB, err := readDiffLines(fileB)
	if err != nil {
		fmt.Fprintf(stderr, "diff: %s: %s\n", fileB, err)
		return 2
	}

	edits := computeDiffEdits(linesA, linesB, opts)

	if opts.IgnoreBlankLines {
		edits = filterBlankLineEdits(edits)
	}

	if !hasDiffChanges(edits) {
		return 0
	}

	// Brief mode: just report that files differ.
	if opts.Brief {
		fmt.Fprintf(stdout, "Files %s and %s differ\n", fileA, fileB)
		return 1
	}

	// Format the output.
	if opts.UseUnified {
		formatUnifiedDiff(edits, fileA, fileB, opts.Unified, stdout)
	} else if opts.UseContext {
		formatContextDiff(edits, fileA, fileB, opts.Context, stdout)
	} else {
		formatNormalDiff(edits, stdout)
	}

	return 1
}

// =========================================================================
// diffArgvHasFlag — check if any of the given flags appear in argv
// =========================================================================
//
// Because the JSON spec defines defaults for integer flags like -u and -c,
// they're always present in the parsed flags map. We need to check argv
// directly to know if the user explicitly requested a particular format.

func diffArgvHasFlag(argv []string, flags ...string) bool {
	for _, arg := range argv {
		for _, flag := range flags {
			if arg == flag || strings.HasPrefix(arg, flag+"=") {
				return true
			}
		}
	}
	return false
}

// =========================================================================
// runDiff — the testable core of the diff tool
// =========================================================================
//
// The diff tool:
//   1. Parses arguments using cli-builder.
//   2. Reads both files into memory.
//   3. Computes the edit script using LCS.
//   4. Formats the output according to the selected format.
//   5. Returns 0 if files are identical, 1 if different, 2 on error.

func runDiff(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "diff: %s\n", err)
		return 2
	}

	// Step 2: Parse the arguments.
	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 2
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
		// Build diff options from flags.
		opts := DiffOptions{
			Brief:            getBool(r.Flags, "brief"),
			Recursive:        getBool(r.Flags, "recursive"),
			IgnoreCase:       getBool(r.Flags, "ignore_case"),
			IgnoreSpaceChg:   getBool(r.Flags, "ignore_space_change"),
			IgnoreAllSpace:   getBool(r.Flags, "ignore_all_space"),
			IgnoreBlankLines: getBool(r.Flags, "ignore_blank_lines"),
		}

		// Determine output format.
		// Since the spec has default values for -u and -c, they're always
		// present in the flags map. We scan argv directly to detect if
		// the user explicitly requested unified or context format.
		opts.UseUnified = diffArgvHasFlag(argv, "-u", "--unified")
		opts.UseContext = diffArgvHasFlag(argv, "-c", "--context")

		if opts.UseUnified {
			if n, ok := getInt(r.Flags, "unified"); ok {
				opts.Unified = n
			} else {
				opts.Unified = 3
			}
		}
		if opts.UseContext {
			if n, ok := getInt(r.Flags, "context_format"); ok {
				opts.Context = n
			} else {
				opts.Context = 3
			}
		}

		// Extract file arguments.
		fileA, _ := r.Arguments["file1"].(string)
		fileB, _ := r.Arguments["file2"].(string)

		if fileA == "" || fileB == "" {
			fmt.Fprintf(stderr, "diff: missing operand\n")
			return 2
		}

		// Check if we're comparing directories.
		infoA, errA := os.Stat(fileA)
		infoB, errB := os.Stat(fileB)

		if errA != nil {
			fmt.Fprintf(stderr, "diff: %s: %s\n", fileA, errA)
			return 2
		}
		if errB != nil {
			fmt.Fprintf(stderr, "diff: %s: %s\n", fileB, errB)
			return 2
		}

		if infoA.IsDir() && infoB.IsDir() {
			if !opts.Recursive {
				fmt.Fprintf(stderr, "diff: %s is a directory\n", fileA)
				return 2
			}
			return diffDirectory(fileA, fileB, opts, stdout, stderr)
		}

		if infoA.IsDir() || infoB.IsDir() {
			fmt.Fprintf(stderr, "diff: cannot compare directory to file\n")
			return 2
		}

		return diffFiles(fileA, fileB, opts, stdout, stderr)

	default:
		fmt.Fprintf(stderr, "diff: unexpected result type: %T\n", result)
		return 2
	}
}

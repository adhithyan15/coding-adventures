// =========================================================================
// grep — Print Lines That Match Patterns
// =========================================================================
//
// The `grep` utility searches input files for lines matching a regular
// expression pattern. It is one of the most powerful and commonly used
// Unix text-processing commands.
//
// The name "grep" comes from the ed editor command g/re/p — "globally
// search for a regular expression and print matching lines."
//
// # Basic usage
//
//   grep "pattern" file.txt           Search for pattern in a file
//   grep -i "hello" file.txt          Case-insensitive search
//   grep -r "TODO" src/               Recursive search in a directory
//   grep -n "error" log.txt           Show line numbers
//   grep -c "test" file.txt           Count matches
//   grep -v "comment" file.txt        Show non-matching lines (invert)
//   grep -l "pattern" *.txt           List files containing pattern
//   grep -o "word" file.txt           Show only the matching parts
//
// # How grep works
//
//   1. Compile the pattern into a regexp.Regexp
//   2. For each input file (or stdin):
//      a. Read the file line by line
//      b. Test each line against the pattern
//      c. Apply modifiers (invert, word, line match, etc.)
//      d. Format and print matching lines
//   3. Return exit code: 0 if matches found, 1 if none, 2 on error
//
// # Architecture
//
//   grep.json (spec)              grep_tool.go (this file)
//   ┌──────────────────┐        ┌─────────────────────────────────┐
//   │ flags: -i,-v,-c  │        │ compilePattern(): build regexp  │
//   │ -l,-n,-o,-r      │───────>│ grepFile(): search one file     │
//   │ -A,-B,-C,-w,-x   │        │ grepLine(): test one line       │
//   │ PATTERN + FILES  │        │ formatMatch(): build output      │
//   └──────────────────┘        └─────────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// GrepOptions — configuration for grep operations
// =========================================================================

type GrepOptions struct {
	IgnoreCase     bool // -i: case-insensitive matching
	InvertMatch    bool // -v: select non-matching lines
	Count          bool // -c: print count of matching lines
	FilesWithMatch bool // -l: print only filenames with matches
	FilesWithout   bool // -L: print only filenames without matches
	LineNumber     bool // -n: prefix with line number
	WithFilename   bool // -H: print filename for each match
	NoFilename     bool // -h: suppress filename prefix
	OnlyMatching   bool // -o: print only the matched parts
	Quiet          bool // -q: suppress all output
	WordRegexp     bool // -w: match whole words only
	LineRegexp     bool // -x: match whole lines only
	FixedStrings   bool // -F: treat pattern as literal string
	Recursive      bool // -r/-R: search directories recursively
	MaxCount       int  // -m: stop after this many matches per file
	AfterContext   int  // -A: lines of context after match
	BeforeContext  int  // -B: lines of context before match
}

// =========================================================================
// GrepMatch — a single matching line with its metadata
// =========================================================================
//
// This struct represents one match found by grep. It includes the line
// content, line number, and filename — everything needed to format output.

type GrepMatch struct {
	Filename string // Source file name
	LineNum  int    // 1-based line number
	Line     string // The full line content
	IsMatch  bool   // true for actual matches, false for context lines
}

// =========================================================================
// compilePattern — build a regexp from the user's pattern string
// =========================================================================
//
// This function handles the various pattern interpretation modes:
//
//   Default:    basic regular expression
//   -E:         extended regular expression (Go's regexp is already ERE)
//   -F:         fixed string (escape all regex metacharacters)
//   -w:         wrap pattern in \b...\b for word boundaries
//   -x:         wrap pattern in ^...$ for full line match
//   -i:         add (?i) prefix for case-insensitive matching

func compilePattern(pattern string, opts GrepOptions) (*regexp.Regexp, error) {
	// Fixed strings mode: escape all regex metacharacters.
	if opts.FixedStrings {
		pattern = regexp.QuoteMeta(pattern)
	}

	// Word boundary matching: wrap in \b...\b.
	// \b matches the boundary between a word character and a non-word char.
	if opts.WordRegexp {
		pattern = `\b(?:` + pattern + `)\b`
	}

	// Line matching: anchor to start and end of line.
	if opts.LineRegexp {
		pattern = "^(?:" + pattern + ")$"
	}

	// Case-insensitive matching: prepend (?i) flag.
	if opts.IgnoreCase {
		pattern = "(?i)" + pattern
	}

	re, err := regexp.Compile(pattern)
	if err != nil {
		return nil, fmt.Errorf("invalid pattern: %w", err)
	}

	return re, nil
}

// =========================================================================
// grepLine — test whether a single line matches the pattern
// =========================================================================
//
// This function returns true if the line matches (or doesn't match,
// when invert mode is active).
//
// Truth table:
//
//   | re.Match | invert | result |
//   |----------|--------|--------|
//   | true     | false  | true   |
//   | true     | true   | false  |
//   | false    | false  | false  |
//   | false    | true   | true   |

func grepLine(line string, pattern *regexp.Regexp, opts GrepOptions) bool {
	matches := pattern.MatchString(line)
	if opts.InvertMatch {
		return !matches
	}
	return matches
}

// =========================================================================
// grepFile — search a single file for matching lines
// =========================================================================
//
// This function reads a file line by line and returns all matching lines
// as GrepMatch structs. It handles context lines (-A, -B, -C) by
// maintaining a circular buffer of recent lines.

func grepFile(path string, pattern *regexp.Regexp, opts GrepOptions) ([]GrepMatch, error) {
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

	scanner := bufio.NewScanner(reader)
	var matches []GrepMatch
	var allLines []string
	matchCount := 0

	// First, read all lines (needed for before-context).
	for scanner.Scan() {
		allLines = append(allLines, scanner.Text())
	}
	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("error reading '%s': %w", path, err)
	}

	// Track which lines have been output as context to avoid duplicates.
	outputLines := make(map[int]bool)

	// Now search through all lines.
	for i, line := range allLines {
		if opts.MaxCount > 0 && matchCount >= opts.MaxCount {
			break
		}

		if grepLine(line, pattern, opts) {
			matchCount++

			// Add before-context lines.
			if opts.BeforeContext > 0 {
				start := i - opts.BeforeContext
				if start < 0 {
					start = 0
				}
				for j := start; j < i; j++ {
					if !outputLines[j] {
						outputLines[j] = true
						matches = append(matches, GrepMatch{
							Filename: path,
							LineNum:  j + 1,
							Line:     allLines[j],
							IsMatch:  false,
						})
					}
				}
			}

			// Add the matching line itself.
			if !outputLines[i] {
				outputLines[i] = true
				matches = append(matches, GrepMatch{
					Filename: path,
					LineNum:  i + 1,
					Line:     line,
					IsMatch:  true,
				})
			}

			// Add after-context lines.
			if opts.AfterContext > 0 {
				end := i + opts.AfterContext
				if end >= len(allLines) {
					end = len(allLines) - 1
				}
				for j := i + 1; j <= end; j++ {
					if !outputLines[j] {
						outputLines[j] = true
						matches = append(matches, GrepMatch{
							Filename: path,
							LineNum:  j + 1,
							Line:     allLines[j],
							IsMatch:  false,
						})
					}
				}
			}
		}
	}

	return matches, nil
}

// =========================================================================
// grepRecursive — collect files to search from a directory tree
// =========================================================================

func grepRecursive(dir string) ([]string, error) {
	var files []string
	err := filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil // Skip files we can't access
		}
		if !info.IsDir() {
			files = append(files, path)
		}
		return nil
	})
	return files, err
}

// =========================================================================
// runGrep — the testable core of the grep tool
// =========================================================================
//
// Exit codes follow the grep convention:
//   0 — matches were found
//   1 — no matches were found
//   2 — an error occurred

func runGrep(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "grep: %s\n", err)
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
		opts := GrepOptions{
			IgnoreCase:     getBool(r.Flags, "ignore_case"),
			InvertMatch:    getBool(r.Flags, "invert_match"),
			Count:          getBool(r.Flags, "count"),
			FilesWithMatch: getBool(r.Flags, "files_with_matches"),
			FilesWithout:   getBool(r.Flags, "files_without_match"),
			LineNumber:     getBool(r.Flags, "line_number"),
			WithFilename:   getBool(r.Flags, "with_filename"),
			NoFilename:     getBool(r.Flags, "no_filename"),
			OnlyMatching:   getBool(r.Flags, "only_matching"),
			Quiet:          getBool(r.Flags, "quiet"),
			WordRegexp:     getBool(r.Flags, "word_regexp"),
			LineRegexp:     getBool(r.Flags, "line_regexp"),
			FixedStrings:   getBool(r.Flags, "fixed_strings"),
			Recursive:      getBool(r.Flags, "recursive") || getBool(r.Flags, "dereference_recursive"),
		}

		// Extract integer flags (cli-builder returns these as int64).
		if v := r.Flags["max_count"]; v != nil {
			if n, ok := v.(int64); ok {
				maxCount, err := intFromInt64(n)
				if err != nil {
					fmt.Fprintf(stderr, "grep: invalid max count: %s\n", err)
					return 2
				}
				opts.MaxCount = maxCount
			}
		}
		if v := r.Flags["after_context"]; v != nil {
			if n, ok := v.(int64); ok {
				afterContext, err := intFromInt64(n)
				if err != nil {
					fmt.Fprintf(stderr, "grep: invalid after-context: %s\n", err)
					return 2
				}
				opts.AfterContext = afterContext
			}
		}
		if v := r.Flags["before_context"]; v != nil {
			if n, ok := v.(int64); ok {
				beforeContext, err := intFromInt64(n)
				if err != nil {
					fmt.Fprintf(stderr, "grep: invalid before-context: %s\n", err)
					return 2
				}
				opts.BeforeContext = beforeContext
			}
		}
		if v := r.Flags["context"]; v != nil {
			if n, ok := v.(int64); ok {
				contextLines, err := intFromInt64(n)
				if err != nil {
					fmt.Fprintf(stderr, "grep: invalid context: %s\n", err)
					return 2
				}
				opts.AfterContext = contextLines
				opts.BeforeContext = contextLines
			}
		}

		// Get the pattern. It can come from:
		//   1. The -e flag (repeatable)
		//   2. The first positional argument
		patternStr := ""
		if ePatterns := getStringSlice(r.Flags, "regexp"); len(ePatterns) > 0 {
			// Multiple -e patterns are ORed together.
			patternStr = strings.Join(ePatterns, "|")
		} else if p, ok := r.Arguments["pattern"].(string); ok && p != "" {
			patternStr = p
		} else {
			fmt.Fprintf(stderr, "grep: no pattern specified\n")
			return 2
		}

		// Compile the pattern.
		re, err := compilePattern(patternStr, opts)
		if err != nil {
			fmt.Fprintf(stderr, "grep: %s\n", err)
			return 2
		}

		// Collect files to search.
		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			files = []string{"-"} // Read from stdin by default.
		}

		// Expand recursive directories.
		if opts.Recursive {
			var expanded []string
			for _, f := range files {
				info, err := os.Stat(f)
				if err != nil {
					fmt.Fprintf(stderr, "grep: %s: %s\n", f, err)
					continue
				}
				if info.IsDir() {
					recFiles, err := grepRecursive(f)
					if err != nil {
						fmt.Fprintf(stderr, "grep: %s: %s\n", f, err)
						continue
					}
					expanded = append(expanded, recFiles...)
				} else {
					expanded = append(expanded, f)
				}
			}
			files = expanded
		}

		// Determine filename display mode.
		// Default: show filename when searching multiple files.
		showFilename := len(files) > 1
		if opts.WithFilename {
			showFilename = true
		}
		if opts.NoFilename {
			showFilename = false
		}

		totalMatches := 0
		hasError := false

		for _, file := range files {
			matches, err := grepFile(file, re, opts)
			if err != nil {
				fmt.Fprintf(stderr, "grep: %s\n", err)
				hasError = true
				continue
			}

			// Count actual matches (not context lines).
			fileMatchCount := 0
			for _, m := range matches {
				if m.IsMatch {
					fileMatchCount++
				}
			}

			// Files-with-matches mode: just print the filename.
			if opts.FilesWithMatch {
				if fileMatchCount > 0 {
					fmt.Fprintln(stdout, file)
					totalMatches++
				}
				continue
			}

			// Files-without-match mode: print filenames with no matches.
			if opts.FilesWithout {
				if fileMatchCount == 0 {
					fmt.Fprintln(stdout, file)
				}
				continue
			}

			// Count mode: print the count per file.
			if opts.Count {
				if showFilename {
					fmt.Fprintf(stdout, "%s:%d\n", file, fileMatchCount)
				} else {
					fmt.Fprintf(stdout, "%d\n", fileMatchCount)
				}
				totalMatches += fileMatchCount
				continue
			}

			// Quiet mode: don't print anything, just track matches.
			if opts.Quiet {
				totalMatches += fileMatchCount
				if totalMatches > 0 {
					return 0
				}
				continue
			}

			// Normal mode: print matching lines.
			totalMatches += fileMatchCount
			prevLineNum := 0
			for _, m := range matches {
				// Print group separator between non-contiguous context groups.
				if prevLineNum > 0 && m.LineNum > prevLineNum+1 &&
					(opts.AfterContext > 0 || opts.BeforeContext > 0) {
					fmt.Fprintln(stdout, "--")
				}
				prevLineNum = m.LineNum

				// Build the output line.
				var prefix string
				separator := ":"
				if !m.IsMatch {
					separator = "-"
				}

				if showFilename {
					prefix += file + separator
				}
				if opts.LineNumber {
					prefix += fmt.Sprintf("%d%s", m.LineNum, separator)
				}

				if opts.OnlyMatching && m.IsMatch {
					// Print only the matching parts of the line.
					allMatches := re.FindAllString(m.Line, -1)
					for _, match := range allMatches {
						fmt.Fprintf(stdout, "%s%s\n", prefix, match)
					}
				} else {
					fmt.Fprintf(stdout, "%s%s\n", prefix, m.Line)
				}
			}
		}

		// Exit code: 0 if matches found, 1 if none, 2 if error.
		if hasError && totalMatches == 0 {
			return 2
		}
		if totalMatches > 0 {
			return 0
		}
		return 1

	default:
		fmt.Fprintf(stderr, "grep: unexpected result type: %T\n", result)
		return 2
	}
}

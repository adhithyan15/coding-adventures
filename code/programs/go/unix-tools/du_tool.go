// =========================================================================
// du — Estimate File Space Usage
// =========================================================================
//
// The `du` utility reports the disk space used by files and directories.
// By default, it recursively walks directory trees and reports the size
// of each directory.
//
// # How du works
//
// du walks the file tree using filepath.Walk, accumulating file sizes.
// For each directory, it reports the total size of all files within it
// (including subdirectories).
//
// # Common usage patterns
//
//   du              Show sizes of all subdirectories (current dir)
//   du -s           Summary: show only the total for each argument
//   du -h           Human-readable sizes (K, M, G)
//   du -sh .        One-line summary of current directory
//   du -a           Show all files, not just directories
//   du -d 1         Show only one level deep
//
// # Size reporting
//
// By default, du reports sizes in 1024-byte blocks (kilobytes).
// With -h, sizes are shown in human-readable format.
//
// # Architecture
//
//   du.json (spec)               du_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -a,-h,-s  │       │ walk directory tree               │
//   │ -c,-d,-B,-L,-P   │──────>│ accumulate file sizes             │
//   │ --exclude         │       │ report per-directory totals       │
//   │ arg: FILES...     │       │                                  │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// DuOptions — configuration for disk usage estimation
// =========================================================================

type DuOptions struct {
	ShowAll       bool     // -a: show all files, not just directories
	HumanReadable bool     // -h: human-readable sizes
	SI            bool     // --si: powers of 1000
	Summarize     bool     // -s: show only total for each argument
	ShowTotal     bool     // -c: produce a grand total
	MaxDepth      int      // -d: maximum directory depth (-1 = unlimited)
	Excludes      []string // --exclude: patterns to exclude
}

// =========================================================================
// DuEntry — one line of du output
// =========================================================================

type DuEntry struct {
	Path  string // the file or directory path
	Bytes int64  // size in bytes
}

// =========================================================================
// diskUsage — walk a path and compute disk usage
// =========================================================================
//
// Returns a list of entries (path + size). The list depends on options:
//   - Default: one entry per directory
//   - -a: one entry per file and directory
//   - -s: one entry for the root path only

func diskUsage(path string, opts DuOptions) ([]DuEntry, error) {
	// Track directory sizes.
	dirSizes := make(map[string]int64)
	var allEntries []DuEntry

	err := filepath.Walk(path, func(p string, info os.FileInfo, err error) error {
		if err != nil {
			return nil // Skip inaccessible files.
		}

		// Check exclusion patterns.
		for _, pattern := range opts.Excludes {
			if matched, _ := filepath.Match(pattern, filepath.Base(p)); matched {
				if info.IsDir() {
					return filepath.SkipDir
				}
				return nil
			}
		}

		// Check max depth.
		if opts.MaxDepth >= 0 {
			rel, _ := filepath.Rel(path, p)
			depth := 0
			if rel != "." {
				depth = strings.Count(rel, string(filepath.Separator)) + 1
			}
			if depth > opts.MaxDepth {
				if info.IsDir() {
					return filepath.SkipDir
				}
				return nil
			}
		}

		size := info.Size()

		// Add this file's size to its parent directory.
		dir := filepath.Dir(p)
		if !info.IsDir() {
			dirSizes[dir] += size
		}

		// Add to all entries if -a is set.
		if opts.ShowAll && !info.IsDir() {
			allEntries = append(allEntries, DuEntry{Path: p, Bytes: size})
		}

		// Record directory paths.
		if info.IsDir() {
			if _, exists := dirSizes[p]; !exists {
				dirSizes[p] = 0
			}
		}

		return nil
	})

	if err != nil {
		return nil, err
	}

	// Propagate sizes upward: each directory's total includes all
	// files and subdirectories within it.
	propagatedSizes := make(map[string]int64)
	for dir, size := range dirSizes {
		propagatedSizes[dir] += size
		// Walk up to the root path and add to each parent.
		current := dir
		for {
			parent := filepath.Dir(current)
			if parent == current || !strings.HasPrefix(current, path) {
				break
			}
			propagatedSizes[parent] += size
			current = parent
		}
	}

	// Build the output entries.
	var result []DuEntry

	if opts.Summarize {
		// Only the root path.
		totalSize := propagatedSizes[path]
		result = append(result, DuEntry{Path: path, Bytes: totalSize})
	} else {
		// Include file entries if -a is set.
		result = append(result, allEntries...)

		// Add directory entries.
		// Walk again to get them in order.
		filepath.Walk(path, func(p string, info os.FileInfo, err error) error {
			if err != nil {
				return nil
			}

			// Check max depth.
			if opts.MaxDepth >= 0 {
				rel, _ := filepath.Rel(path, p)
				depth := 0
				if rel != "." {
					depth = strings.Count(rel, string(filepath.Separator)) + 1
				}
				if depth > opts.MaxDepth {
					if info.IsDir() {
						return filepath.SkipDir
					}
					return nil
				}
			}

			if info.IsDir() {
				size := propagatedSizes[p]
				result = append(result, DuEntry{Path: p, Bytes: size})
			}
			return nil
		})
	}

	return result, nil
}

// =========================================================================
// formatDuSize — format a byte count for du output
// =========================================================================
//
// Default: 1K-blocks (bytes / 1024, minimum 1).

func formatDuSize(bytes int64, humanReadable, si bool) string {
	if humanReadable {
		return formatHumanReadable(uint64(bytes), 1024)
	}
	if si {
		return formatHumanReadable(uint64(bytes), 1000)
	}
	// Default: 1K-blocks, rounded up.
	blocks := bytes / 1024
	if bytes%1024 != 0 {
		blocks++
	}
	if blocks == 0 && bytes > 0 {
		blocks = 1
	}
	return fmt.Sprintf("%d", blocks)
}

// =========================================================================
// runDu — the testable core of the du tool
// =========================================================================

func runDu(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "du: %s\n", err)
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
		opts := DuOptions{
			ShowAll:       getBool(r.Flags, "all"),
			HumanReadable: getBool(r.Flags, "human_readable"),
			SI:            getBool(r.Flags, "si"),
			Summarize:     getBool(r.Flags, "summarize"),
			ShowTotal:     getBool(r.Flags, "total"),
			MaxDepth:      -1, // Unlimited by default.
		}

		if depth, ok := r.Flags["max_depth"].(int64); ok {
			maxDepth, err := intFromInt64(depth)
			if err != nil {
				fmt.Fprintf(stderr, "du: invalid max depth: %s\n", err)
				return 1
			}
			opts.MaxDepth = maxDepth
		}

		opts.Excludes = getStringSlice(r.Flags, "exclude")

		// Get paths.
		paths := getStringSlice(r.Arguments, "files")
		if len(paths) == 0 {
			paths = []string{"."}
		}

		var grandTotal int64
		exitCode := 0

		for _, path := range paths {
			entries, err := diskUsage(path, opts)
			if err != nil {
				fmt.Fprintf(stderr, "du: %s: %s\n", path, err)
				exitCode = 1
				continue
			}

			for _, entry := range entries {
				sizeStr := formatDuSize(entry.Bytes, opts.HumanReadable, opts.SI)
				fmt.Fprintf(stdout, "%s\t%s\n", sizeStr, entry.Path)
				grandTotal += entry.Bytes
			}
		}

		if opts.ShowTotal {
			sizeStr := formatDuSize(grandTotal, opts.HumanReadable, opts.SI)
			fmt.Fprintf(stdout, "%s\ttotal\n", sizeStr)
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "du: unexpected result type: %T\n", result)
		return 1
	}
}

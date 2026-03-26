// =========================================================================
// ls — List Directory Contents
// =========================================================================
//
// The `ls` utility lists information about files and directories. It is
// perhaps the most frequently used Unix command — you'll type `ls` dozens
// of times in any terminal session.
//
// # Basic usage
//
//   ls                        List current directory
//   ls -l                     Long format (permissions, size, date, etc.)
//   ls -a                     Show hidden files (starting with .)
//   ls -la                    Long format + hidden files (very common combo)
//   ls -R                     Recursive listing
//   ls -lh                    Long format with human-readable sizes
//   ls -1                     One entry per line
//   ls -F                     Append type indicators (/ for dirs, * for exec)
//
// # How ls works internally
//
//   1. Read directory entries using os.ReadDir (or stat a single file)
//   2. Filter entries (hide dotfiles unless -a)
//   3. Sort entries (by name, size, time, etc.)
//   4. Format output (simple names, or long format with metadata)
//
// # Architecture
//
//   ls.json (spec)               ls_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -l,-a,-R  │       │ listDirectory(): read entries     │
//   │ -S,-t,-r,-h,-1   │──────>│ sortEntries(): apply sort order  │
//   │ -d,-F,-i         │       │ formatEntries(): build output    │
//   │ optional FILE    │       │ handle -R: recurse into subdirs  │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// LsOptions — configuration for listing operations
// =========================================================================

type LsOptions struct {
	All           bool // Show hidden files
	AlmostAll     bool // Show hidden files except . and ..
	Long          bool // Long listing format
	HumanReadable bool // Human-readable sizes
	Reverse       bool // Reverse sort order
	Recursive     bool // List subdirectories recursively
	SortBySize    bool // Sort by file size
	SortByTime    bool // Sort by modification time
	OnePerLine    bool // One entry per line
	Directory     bool // List directories themselves, not contents
	Classify      bool // Append type indicator
	Inode         bool // Show inode numbers
	NoGroup       bool // Don't show group in long listing
	NumericUID    bool // Show numeric UID/GID
	Unsorted      bool // Don't sort
}

// =========================================================================
// FileEntry — information about a single file/directory
// =========================================================================
//
// This struct holds all the metadata we need to display a file entry.
// We pre-compute this information so that formatting and sorting can
// work with a clean, uniform data structure.

type FileEntry struct {
	Name    string      // Base name of the file
	Path    string      // Full path (for recursive listing)
	Info    fs.FileInfo // File metadata (size, mode, time, etc.)
	IsDir   bool        // Convenience field
	LinkDst string      // Symlink target, if applicable
}

// =========================================================================
// listDirectory — read and filter directory entries
// =========================================================================
//
// This function reads the contents of a directory and returns a slice
// of FileEntry structs, filtered according to the options.
//
// Filtering rules:
//   - Without -a/-A: hide entries starting with "."
//   - With -A: show dotfiles but exclude "." and ".."
//   - With -a: show everything including "." and ".."

func listDirectory(path string, opts LsOptions) ([]FileEntry, error) {
	dirEntries, err := os.ReadDir(path)
	if err != nil {
		return nil, fmt.Errorf("cannot access '%s': %w", path, err)
	}

	var entries []FileEntry

	// With -a, include "." and ".." entries.
	if opts.All {
		dotInfo, err := os.Stat(path)
		if err == nil {
			entries = append(entries, FileEntry{
				Name:  ".",
				Path:  path,
				Info:  dotInfo,
				IsDir: true,
			})
		}
		parentPath := filepath.Dir(path)
		parentInfo, err := os.Stat(parentPath)
		if err == nil {
			entries = append(entries, FileEntry{
				Name:  "..",
				Path:  parentPath,
				Info:  parentInfo,
				IsDir: true,
			})
		}
	}

	for _, de := range dirEntries {
		name := de.Name()

		// Filter dotfiles.
		if strings.HasPrefix(name, ".") && !opts.All && !opts.AlmostAll {
			continue
		}

		fullPath := filepath.Join(path, name)
		info, err := de.Info()
		if err != nil {
			continue
		}

		entry := FileEntry{
			Name:  name,
			Path:  fullPath,
			Info:  info,
			IsDir: de.IsDir(),
		}

		// Check if it's a symlink and get the target.
		if info.Mode()&os.ModeSymlink != 0 {
			target, err := os.Readlink(fullPath)
			if err == nil {
				entry.LinkDst = target
			}
		}

		entries = append(entries, entry)
	}

	return entries, nil
}

// =========================================================================
// sortEntries — sort file entries according to options
// =========================================================================
//
// Default sort is alphabetical by name (case-insensitive).
// With -S, sort by size (largest first).
// With -t, sort by modification time (newest first).
// With -r, reverse the sort order.
// With -U, don't sort at all.

func sortEntries(entries []FileEntry, opts LsOptions) {
	if opts.Unsorted {
		return
	}

	sort.SliceStable(entries, func(i, j int) bool {
		if opts.SortBySize {
			if entries[i].Info.Size() != entries[j].Info.Size() {
				return entries[i].Info.Size() > entries[j].Info.Size()
			}
		}
		if opts.SortByTime {
			ti := entries[i].Info.ModTime()
			tj := entries[j].Info.ModTime()
			if !ti.Equal(tj) {
				return ti.After(tj)
			}
		}
		// Default: alphabetical by name.
		return strings.ToLower(entries[i].Name) < strings.ToLower(entries[j].Name)
	})

	if opts.Reverse {
		// Reverse the sorted slice in-place.
		for i, j := 0, len(entries)-1; i < j; i, j = i+1, j-1 {
			entries[i], entries[j] = entries[j], entries[i]
		}
	}
}

// =========================================================================
// humanizeSize — format a byte count as a human-readable string
// =========================================================================
//
// This function converts byte counts to human-readable format:
//
//   1023      -> "1023"
//   1024      -> "1.0K"
//   1048576   -> "1.0M"
//   1073741824 -> "1.0G"
//
// We use binary prefixes (powers of 1024), matching `ls -h` behavior.

func humanizeSize(size int64) string {
	units := []string{"", "K", "M", "G", "T", "P"}
	fsize := float64(size)
	unitIdx := 0

	for fsize >= 1024 && unitIdx < len(units)-1 {
		fsize /= 1024
		unitIdx++
	}

	if unitIdx == 0 {
		return fmt.Sprintf("%d", size)
	}

	// Use one decimal place for human-readable output.
	if fsize >= 10 {
		return fmt.Sprintf("%.0f%s", fsize, units[unitIdx])
	}
	return fmt.Sprintf("%.1f%s", fsize, units[unitIdx])
}

// =========================================================================
// classifySuffix — return a type indicator character for -F
// =========================================================================
//
// The -F flag appends an indicator after each entry:
//
//   /   directory
//   *   executable
//   @   symbolic link
//   |   FIFO (named pipe)
//   =   socket
//       (nothing for regular files)

func classifySuffix(info fs.FileInfo) string {
	mode := info.Mode()
	switch {
	case mode.IsDir():
		return "/"
	case mode&os.ModeSymlink != 0:
		return "@"
	case mode&os.ModeNamedPipe != 0:
		return "|"
	case mode&os.ModeSocket != 0:
		return "="
	case mode&0111 != 0: // Any execute bit set
		return "*"
	default:
		return ""
	}
}

// =========================================================================
// formatLong — format a single entry in long listing format
// =========================================================================
//
// Long format shows: mode, nlink, owner, group, size, date, name
//
// Example:
//   -rw-r--r--  1 alice  staff  4096 Jan  5 12:30 file.txt
//   drwxr-xr-x  3 alice  staff   96 Jan  5 12:30 subdir/

func formatLong(entry FileEntry, opts LsOptions) string {
	info := entry.Info
	var parts []string

	// Inode number (optional, -i flag).
	if opts.Inode {
		parts = append(parts, fmt.Sprintf("%8d", getInode(info)))
	}

	// File mode string (e.g., "-rw-r--r--").
	parts = append(parts, info.Mode().String())

	// Number of hard links.
	parts = append(parts, fmt.Sprintf("%3d", getNlink(info)))

	// Owner and group.
	ownerName, groupName := getOwnerGroup(info, opts.NumericUID)
	parts = append(parts, fmt.Sprintf("%-8s", ownerName))
	if !opts.NoGroup {
		parts = append(parts, fmt.Sprintf("%-8s", groupName))
	}

	// File size.
	if opts.HumanReadable {
		parts = append(parts, fmt.Sprintf("%5s", humanizeSize(info.Size())))
	} else {
		parts = append(parts, fmt.Sprintf("%8d", info.Size()))
	}

	// Modification time.
	modTime := info.ModTime()
	parts = append(parts, modTime.Format("Jan _2 15:04"))

	// File name.
	name := entry.Name
	if opts.Classify {
		name += classifySuffix(info)
	}
	if entry.LinkDst != "" {
		name += " -> " + entry.LinkDst
	}
	parts = append(parts, name)

	return strings.Join(parts, " ")
}

// =========================================================================
// formatEntries — format all entries for output
// =========================================================================
//
// This function produces the final output string based on the format options:
//
//   - Long format (-l): one entry per line with full metadata
//   - One-per-line (-1): just names, one per line
//   - Default: names separated by double spaces (simplified; real ls uses
//     column formatting, but that requires terminal width detection)

func formatEntries(entries []FileEntry, opts LsOptions) string {
	if len(entries) == 0 {
		return ""
	}

	var lines []string

	if opts.Long || opts.NumericUID {
		for _, entry := range entries {
			lines = append(lines, formatLong(entry, opts))
		}
		return strings.Join(lines, "\n")
	}

	// Non-long format.
	for _, entry := range entries {
		name := entry.Name
		if opts.Inode {
			name = fmt.Sprintf("%8d %s", getInode(entry.Info), name)
		}
		if opts.Classify {
			name += classifySuffix(entry.Info)
		}
		lines = append(lines, name)
	}

	if opts.OnePerLine || opts.Long {
		return strings.Join(lines, "\n")
	}

	// Default: space-separated (simplified from real ls column format).
	return strings.Join(lines, "  ")
}

// =========================================================================
// runLs — the testable core of the ls tool
// =========================================================================

func runLs(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "ls: %s\n", err)
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
		opts := LsOptions{
			All:           getBool(r.Flags, "all"),
			AlmostAll:     getBool(r.Flags, "almost_all"),
			Long:          getBool(r.Flags, "long"),
			HumanReadable: getBool(r.Flags, "human_readable"),
			Reverse:       getBool(r.Flags, "reverse"),
			Recursive:     getBool(r.Flags, "recursive"),
			SortBySize:    getBool(r.Flags, "sort_by_size"),
			SortByTime:    getBool(r.Flags, "sort_by_time"),
			OnePerLine:    getBool(r.Flags, "one_per_line"),
			Directory:     getBool(r.Flags, "directory"),
			Classify:      getBool(r.Flags, "classify"),
			Inode:         getBool(r.Flags, "inode"),
			NoGroup:       getBool(r.Flags, "no_group"),
			NumericUID:    getBool(r.Flags, "numeric_uid_gid"),
			Unsorted:      getBool(r.Flags, "unsorted"),
		}

		// Extract file paths.
		paths := getStringSlice(r.Arguments, "files")
		if len(paths) == 0 {
			paths = []string{"."}
		}

		exitCode := 0
		showDirName := len(paths) > 1 || opts.Recursive

		for idx, path := range paths {
			info, err := os.Lstat(path)
			if err != nil {
				fmt.Fprintf(stderr, "ls: cannot access '%s': %s\n", path, err)
				exitCode = 1
				continue
			}

			// If -d flag or path is a file, list just that entry.
			if opts.Directory || !info.IsDir() {
				entry := FileEntry{
					Name:  filepath.Base(path),
					Path:  path,
					Info:  info,
					IsDir: info.IsDir(),
				}
				output := formatEntries([]FileEntry{entry}, opts)
				fmt.Fprintln(stdout, output)
				continue
			}

			// Directory listing.
			if showDirName {
				if idx > 0 {
					fmt.Fprintln(stdout)
				}
				fmt.Fprintf(stdout, "%s:\n", path)
			}

			entries, err := listDirectory(path, opts)
			if err != nil {
				fmt.Fprintf(stderr, "ls: %s\n", err)
				exitCode = 1
				continue
			}

			sortEntries(entries, opts)
			output := formatEntries(entries, opts)
			if output != "" {
				fmt.Fprintln(stdout, output)
			}

			// Recursive: list subdirectories.
			if opts.Recursive {
				for _, entry := range entries {
					if entry.IsDir && entry.Name != "." && entry.Name != ".." {
						fmt.Fprintln(stdout)
						exitCode = lsRecursive(entry.Path, opts, stdout, stderr, exitCode)
					}
				}
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "ls: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// lsRecursive — helper for recursive directory listing
// =========================================================================

func lsRecursive(path string, opts LsOptions, stdout io.Writer, stderr io.Writer, exitCode int) int {
	fmt.Fprintf(stdout, "%s:\n", path)

	entries, err := listDirectory(path, opts)
	if err != nil {
		fmt.Fprintf(stderr, "ls: %s\n", err)
		return 1
	}

	sortEntries(entries, opts)
	output := formatEntries(entries, opts)
	if output != "" {
		fmt.Fprintln(stdout, output)
	}

	for _, entry := range entries {
		if entry.IsDir && entry.Name != "." && entry.Name != ".." {
			fmt.Fprintln(stdout)
			exitCode = lsRecursive(entry.Path, opts, stdout, stderr, exitCode)
		}
	}

	return exitCode
}

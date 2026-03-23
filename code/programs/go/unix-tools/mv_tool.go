// =========================================================================
// mv — Move (Rename) Files and Directories
// =========================================================================
//
// The `mv` utility moves or renames files and directories. Under the hood,
// moving a file on the same filesystem is an instant metadata operation
// (just updating directory entries). Moving across filesystems requires
// copying the data and then deleting the original.
//
// # Basic usage
//
//   mv old.txt new.txt              Rename a file
//   mv file.txt dir/                Move a file into a directory
//   mv a.txt b.txt dir/             Move multiple files into a directory
//   mv -v file.txt backup/          Verbose: print what's being done
//
// # How moving works
//
// On the same filesystem:
//   os.Rename(src, dst) is an atomic operation that updates the directory
//   entry. The file's inode (and therefore its data) stays in place.
//
// Across filesystems:
//   os.Rename fails with EXDEV ("cross-device link"). In this case, we
//   fall back to copy + delete: copy the file's data to the new location,
//   then remove the original.
//
// # Flags overview
//
//   -f, --force         Do not prompt before overwriting
//   -i, --interactive   Prompt before overwrite (not implemented)
//   -n, --no-clobber    Do not overwrite existing files
//   -u, --update        Move only when source is newer or dest is missing
//   -v, --verbose       Print each operation
//
// # Architecture
//
//   mv.json (spec)               mv_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -f,-n     │       │ parse args: sources + dest       │
//   │ -u,-v            │──────>│ for each source:                 │
//   │ variadic SOURCE  │       │   moveFile() with fallback       │
//   └──────────────────┘       │   apply flag behaviors            │
//                              └──────────────────────────────────┘

package main

import (
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"syscall"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// MvOptions — configuration for move operations
// =========================================================================

type MvOptions struct {
	Force     bool // Do not prompt before overwriting
	NoClobber bool // Do not overwrite existing files
	Update    bool // Move only when source is newer
	Verbose   bool // Print each operation
}

// =========================================================================
// moveFile — move a single file or directory from src to dst
// =========================================================================
//
// This function tries os.Rename first. If that fails with EXDEV
// (cross-device), it falls back to copy + delete.
//
// The copy + delete fallback handles both files and directories:
//   - For files: copy bytes, then remove original
//   - For directories: recursively copy, then remove original tree

func moveFile(src, dst string, opts MvOptions) error {
	// Try the fast path: os.Rename.
	err := os.Rename(src, dst)
	if err == nil {
		return nil
	}

	// Check if the error is EXDEV (cross-device link).
	// If not, it's a real error we should report.
	var linkErr *os.LinkError
	if !errors.As(err, &linkErr) {
		return fmt.Errorf("cannot move '%s' to '%s': %w", src, dst, err)
	}

	// Check for EXDEV specifically.
	if !errors.Is(linkErr.Err, syscall.EXDEV) {
		return fmt.Errorf("cannot move '%s' to '%s': %w", src, dst, err)
	}

	// Cross-device fallback: copy then delete.
	srcInfo, statErr := os.Lstat(src)
	if statErr != nil {
		return fmt.Errorf("cannot stat '%s': %w", src, statErr)
	}

	if srcInfo.IsDir() {
		// Copy directory recursively.
		cpOpts := CpOptions{Recursive: true}
		copyErr := copyDir(src, dst, cpOpts, io.Discard)
		if copyErr != nil {
			return copyErr
		}
		return os.RemoveAll(src)
	}

	// Copy single file, then remove original.
	cpOpts := CpOptions{}
	copyErr := copyFile(src, dst, cpOpts)
	if copyErr != nil {
		return copyErr
	}
	return os.Remove(src)
}

// =========================================================================
// shouldSkipMove — decide whether to skip moving a file
// =========================================================================
//
// This mirrors the logic in shouldSkipCopy but for move operations.
//
//   -n (no-clobber): skip if destination exists
//   -u (update):     skip if destination is newer than source

func shouldSkipMove(src, dst string, opts MvOptions) bool {
	dstInfo, err := os.Stat(dst)
	if err != nil {
		// Destination doesn't exist — always move.
		return false
	}

	// No-clobber: never overwrite existing files.
	if opts.NoClobber {
		return true
	}

	// Update: only move if source is newer.
	if opts.Update {
		srcInfo, err := os.Stat(src)
		if err != nil {
			return false
		}
		if !srcInfo.ModTime().After(dstInfo.ModTime()) {
			return true
		}
	}

	_ = dstInfo
	return false
}

// =========================================================================
// runMv — the testable core of the mv tool
// =========================================================================

func runMv(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "mv: %s\n", err)
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
		opts := MvOptions{
			Force:     getBool(r.Flags, "force"),
			NoClobber: getBool(r.Flags, "no_clobber"),
			Update:    getBool(r.Flags, "update"),
			Verbose:   getBool(r.Flags, "verbose"),
		}

		// Extract positional arguments: sources + destination.
		args := getStringSlice(r.Arguments, "sources")
		if len(args) < 2 {
			fmt.Fprintf(stderr, "mv: missing destination operand\n")
			return 1
		}

		sources := args[:len(args)-1]
		dest := args[len(args)-1]

		// Check if destination is a directory.
		destInfo, destErr := os.Stat(dest)
		destIsDir := destErr == nil && destInfo.IsDir()

		// Multiple sources require a directory destination.
		if len(sources) > 1 && !destIsDir {
			fmt.Fprintf(stderr, "mv: target '%s' is not a directory\n", dest)
			return 1
		}

		exitCode := 0
		for _, src := range sources {
			actualDest := dest
			if destIsDir {
				actualDest = filepath.Join(dest, filepath.Base(src))
			}

			// Check skip conditions.
			if shouldSkipMove(src, actualDest, opts) {
				continue
			}

			// Remove destination first if force is set.
			if opts.Force {
				os.Remove(actualDest)
			}

			err := moveFile(src, actualDest, opts)
			if err != nil {
				fmt.Fprintf(stderr, "mv: %s\n", err)
				exitCode = 1
				continue
			}

			if opts.Verbose {
				fmt.Fprintf(stdout, "renamed '%s' -> '%s'\n", src, actualDest)
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "mv: unexpected result type: %T\n", result)
		return 1
	}
}

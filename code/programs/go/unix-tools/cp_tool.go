// =========================================================================
// cp — Copy Files and Directories
// =========================================================================
//
// The `cp` utility copies files and directories. It is one of the most
// fundamental file-management commands in Unix.
//
// # Basic usage
//
//   cp source.txt dest.txt          Copy one file
//   cp a.txt b.txt dir/             Copy multiple files into a directory
//   cp -R srcdir/ destdir/          Copy a directory recursively
//   cp -v file.txt backup/          Verbose: print what's being done
//
// # How copying works
//
// At its core, copying a file means:
//   1. Open the source file for reading
//   2. Create (or truncate) the destination file for writing
//   3. Read bytes from the source, write them to the destination
//   4. Close both files
//
// Go's io.Copy does exactly this — it reads from an io.Reader and writes
// to an io.Writer, using an internal buffer (typically 32KB).
//
// # Flags overview
//
//   -f, --force         Remove existing destination if it cannot be opened
//   -i, --interactive   Prompt before overwrite (not implemented in this version)
//   -n, --no-clobber    Do not overwrite existing files
//   -R, --recursive     Copy directories recursively
//   -v, --verbose       Print each file as it's copied
//   -u, --update        Copy only when source is newer or dest is missing
//   -l, --link          Hard link instead of copying
//   -s, --symbolic-link Create symbolic links instead of copying
//
// # Architecture
//
//   cp.json (spec)               cp_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -f,-R,-n  │       │ parse args: sources + dest       │
//   │ -v,-u,-l,-s      │──────>│ for each source:                 │
//   │ variadic SOURCE  │       │   copyFile() or copyDir()        │
//   └──────────────────┘       │   apply flag behaviors            │
//                              └──────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// CpOptions — configuration for copy operations
// =========================================================================
//
// This struct bundles all the flag values that control copy behavior.
// By passing a struct rather than individual booleans, we keep function
// signatures clean and make it easy to add new options.

type CpOptions struct {
	Force      bool // Remove destination if can't be opened
	NoClobber  bool // Don't overwrite existing files
	Recursive  bool // Copy directories recursively
	Verbose    bool // Print each operation
	Update     bool // Only copy if source is newer
	Link       bool // Hard link instead of copy
	SymLink    bool // Symbolic link instead of copy
	Dereference bool // Follow symlinks in source
}

// =========================================================================
// copyFile — copy a single file from src to dst
// =========================================================================
//
// This function handles the core file-copying logic:
//   1. Open the source file
//   2. Create the destination file
//   3. Copy bytes using io.Copy
//
// If the Link flag is set, we create a hard link instead.
// If the SymLink flag is set, we create a symbolic link instead.
//
// Returns nil on success, or an error describing what went wrong.

func copyFile(src, dst string, opts CpOptions) error {
	// Hard link mode: os.Link creates a new directory entry pointing
	// to the same inode as the source. No data is copied — both names
	// refer to the same physical file on disk.
	if opts.Link {
		return os.Link(src, dst)
	}

	// Symbolic link mode: os.Symlink creates a special file that
	// contains a path to the source. The symlink is a pointer, not a copy.
	if opts.SymLink {
		return os.Symlink(src, dst)
	}

	// Regular copy: open source, create dest, copy bytes.
	srcFile, err := os.Open(src)
	if err != nil {
		return fmt.Errorf("cannot open '%s': %w", src, err)
	}
	defer srcFile.Close()

	// Get source file info for permissions.
	srcInfo, err := srcFile.Stat()
	if err != nil {
		return fmt.Errorf("cannot stat '%s': %w", src, err)
	}

	// Create destination with same permissions as source.
	dstFile, err := os.OpenFile(dst, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, srcInfo.Mode())
	if err != nil {
		return fmt.Errorf("cannot create '%s': %w", dst, err)
	}
	defer dstFile.Close()

	// io.Copy reads from srcFile and writes to dstFile in chunks.
	// It returns the number of bytes copied and any error.
	_, err = io.Copy(dstFile, srcFile)
	if err != nil {
		return fmt.Errorf("error copying to '%s': %w", dst, err)
	}

	return nil
}

// =========================================================================
// copyDir — recursively copy a directory tree
// =========================================================================
//
// Directory copying works by walking the source tree depth-first:
//   1. Create the destination directory
//   2. Read all entries in the source directory
//   3. For each entry:
//      - If it's a directory, recurse (copyDir)
//      - If it's a file, copy it (copyFile)
//
// This mirrors the behavior of `cp -R`.

func copyDir(src, dst string, opts CpOptions, stdout io.Writer) error {
	// Get source directory info for permissions.
	srcInfo, err := os.Stat(src)
	if err != nil {
		return fmt.Errorf("cannot stat '%s': %w", src, err)
	}

	// Create the destination directory with the same permissions.
	err = os.MkdirAll(dst, srcInfo.Mode())
	if err != nil {
		return fmt.Errorf("cannot create directory '%s': %w", dst, err)
	}

	// Read all entries in the source directory.
	entries, err := os.ReadDir(src)
	if err != nil {
		return fmt.Errorf("cannot read directory '%s': %w", src, err)
	}

	for _, entry := range entries {
		srcPath := filepath.Join(src, entry.Name())
		dstPath := filepath.Join(dst, entry.Name())

		if entry.IsDir() {
			// Recurse into subdirectories.
			err = copyDir(srcPath, dstPath, opts, stdout)
			if err != nil {
				return err
			}
		} else {
			// Check no-clobber and update conditions before copying.
			if shouldSkipCopy(srcPath, dstPath, opts) {
				continue
			}

			err = copyFile(srcPath, dstPath, opts)
			if err != nil {
				return err
			}

			if opts.Verbose {
				fmt.Fprintf(stdout, "'%s' -> '%s'\n", srcPath, dstPath)
			}
		}
	}

	return nil
}

// =========================================================================
// shouldSkipCopy — decide whether to skip copying a file
// =========================================================================
//
// This function checks the no-clobber and update flags:
//
//   -n (no-clobber): skip if destination exists
//   -u (update):     skip if destination is newer than source
//
// Truth table for skip decisions:
//
//   | dst exists | no-clobber | update | src newer | skip? |
//   |------------|------------|--------|-----------|-------|
//   | no         | any        | any    | n/a       | no    |
//   | yes        | yes        | any    | any       | YES   |
//   | yes        | no         | yes    | yes       | no    |
//   | yes        | no         | yes    | no        | YES   |
//   | yes        | no         | no     | any       | no    |

func shouldSkipCopy(src, dst string, opts CpOptions) bool {
	dstInfo, err := os.Stat(dst)
	if err != nil {
		// Destination doesn't exist — always copy.
		return false
	}

	// No-clobber: never overwrite existing files.
	if opts.NoClobber {
		return true
	}

	// Update: only copy if source is newer.
	if opts.Update {
		srcInfo, err := os.Stat(src)
		if err != nil {
			return false
		}
		// Skip if destination is same age or newer than source.
		if !srcInfo.ModTime().After(dstInfo.ModTime()) {
			return true
		}
	}

	return false
}

// =========================================================================
// runCp — the testable core of the cp tool
// =========================================================================
//
// The cp tool:
//   1. Parses arguments using cli-builder.
//   2. Extracts flags and positional arguments.
//   3. Determines the destination (last argument).
//   4. For each source, copies to the destination.
//   5. Returns 0 on success, 1 on any error.

func runCp(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "cp: %s\n", err)
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
		// Extract flags into an options struct.
		opts := CpOptions{
			Force:       getBool(r.Flags, "force"),
			NoClobber:   getBool(r.Flags, "no_clobber"),
			Recursive:   getBool(r.Flags, "recursive") || getBool(r.Flags, "archive"),
			Verbose:     getBool(r.Flags, "verbose"),
			Update:      getBool(r.Flags, "update"),
			Link:        getBool(r.Flags, "link"),
			SymLink:     getBool(r.Flags, "symbolic_link"),
			Dereference: getBool(r.Flags, "dereference"),
		}

		// Extract positional arguments.
		// The spec defines a variadic "sources" argument with min 2.
		// The last element is the destination, all others are sources.
		args := getStringSlice(r.Arguments, "sources")
		if len(args) < 2 {
			fmt.Fprintf(stderr, "cp: missing destination operand\n")
			return 1
		}

		sources := args[:len(args)-1]
		dest := args[len(args)-1]

		// Determine if destination is an existing directory.
		destInfo, destErr := os.Stat(dest)
		destIsDir := destErr == nil && destInfo.IsDir()

		// If multiple sources, destination must be a directory.
		if len(sources) > 1 && !destIsDir {
			fmt.Fprintf(stderr, "cp: target '%s' is not a directory\n", dest)
			return 1
		}

		exitCode := 0
		for _, src := range sources {
			// Determine the actual destination path.
			// If dest is a directory, place the source inside it.
			actualDest := dest
			if destIsDir {
				actualDest = filepath.Join(dest, filepath.Base(src))
			}

			// Check if source is a directory.
			srcInfo, err := os.Lstat(src)
			if err != nil {
				fmt.Fprintf(stderr, "cp: cannot stat '%s': %s\n", src, err)
				exitCode = 1
				continue
			}

			if srcInfo.IsDir() {
				if !opts.Recursive {
					fmt.Fprintf(stderr, "cp: -R not specified; omitting directory '%s'\n", src)
					exitCode = 1
					continue
				}
				err = copyDir(src, actualDest, opts, stdout)
				if err != nil {
					fmt.Fprintf(stderr, "cp: %s\n", err)
					exitCode = 1
					continue
				}
				if opts.Verbose {
					fmt.Fprintf(stdout, "'%s' -> '%s'\n", src, actualDest)
				}
			} else {
				// Check skip conditions.
				if shouldSkipCopy(src, actualDest, opts) {
					continue
				}

				// If force and destination exists but can't be opened,
				// remove it first.
				if opts.Force {
					if _, err := os.Lstat(actualDest); err == nil {
						os.Remove(actualDest)
					}
				}

				err = copyFile(src, actualDest, opts)
				if err != nil {
					fmt.Fprintf(stderr, "cp: %s\n", err)
					exitCode = 1
					continue
				}

				if opts.Verbose {
					fmt.Fprintf(stdout, "'%s' -> '%s'\n", src, actualDest)
				}
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "cp: unexpected result type: %T\n", result)
		return 1
	}
}

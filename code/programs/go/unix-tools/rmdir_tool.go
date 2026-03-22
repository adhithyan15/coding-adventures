// =========================================================================
// rmdir — Remove Empty Directories
// =========================================================================
//
// The `rmdir` utility removes empty directories. Unlike `rm -r`, it refuses
// to remove directories that contain files — this is a safety feature.
//
// # Basic usage
//
//   rmdir empty_dir             Remove a single empty directory
//   rmdir -p a/b/c              Remove c, then b, then a (all must be empty)
//   rmdir -v dir1 dir2          Remove dir1 and dir2, printing each
//
// # The -p flag (parents)
//
// With -p, rmdir removes the directory AND its ancestors, working from
// the deepest to the shallowest:
//
//   rmdir -p a/b/c
//   Step 1: remove a/b/c
//   Step 2: remove a/b
//   Step 3: remove a
//
// Each ancestor must also be empty (after removing its child).
//
// # Architecture
//
//   rmdir.json (spec)           rmdir_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -p,-v     │      │ for each directory arg:        │
//   │ --ignore-fail... │─────>│   remove dir                   │
//   │ variadic DIR     │      │   if -p, remove ancestors      │
//   └──────────────────┘      │   print if verbose             │
//                             └────────────────────────────────┘

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
// runRmdir — the testable core of the rmdir tool
// =========================================================================

func runRmdir(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "rmdir: %s\n", err)
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
		parents := getBool(r.Flags, "parents")
		verbose := getBool(r.Flags, "verbose")
		ignoreFail := getBool(r.Flags, "ignore_fail")

		// Extract directory paths.
		dirs := getStringSlice(r.Arguments, "directories")

		exitCode := 0
		for _, dir := range dirs {
			// Try to remove the directory.
			err := os.Remove(dir)
			if err != nil {
				// Check if we should ignore non-empty errors.
				if ignoreFail && isNonEmptyError(err) {
					continue
				}
				fmt.Fprintf(stderr, "rmdir: failed to remove '%s': %s\n", dir, err)
				exitCode = 1
				continue
			}

			if verbose {
				fmt.Fprintf(stdout, "rmdir: removing directory, '%s'\n", dir)
			}

			// With -p, also remove parent directories.
			if parents {
				current := dir
				for {
					parent := filepath.Dir(current)
					// Stop when we reach the root or current directory.
					if parent == current || parent == "." || parent == "/" {
						break
					}

					err := os.Remove(parent)
					if err != nil {
						if ignoreFail && isNonEmptyError(err) {
							break
						}
						fmt.Fprintf(stderr, "rmdir: failed to remove '%s': %s\n", parent, err)
						exitCode = 1
						break
					}

					if verbose {
						fmt.Fprintf(stdout, "rmdir: removing directory, '%s'\n", parent)
					}

					current = parent
				}
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "rmdir: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// isNonEmptyError — check if an error is because the directory isn't empty
// =========================================================================
//
// On Unix systems, trying to remove a non-empty directory returns ENOTEMPTY.
// We check the error message string as a portable approach.

func isNonEmptyError(err error) bool {
	return strings.Contains(err.Error(), "not empty") ||
		strings.Contains(err.Error(), "directory not empty")
}

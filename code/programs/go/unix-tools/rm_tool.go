// =========================================================================
// rm — Remove Files or Directories
// =========================================================================
//
// The `rm` utility removes files and directories. It is one of the most
// dangerous Unix commands — there is no "undo" or "trash" for rm.
//
// # Basic usage
//
//   rm file.txt                 Remove a single file
//   rm -f file.txt              Force: don't error if file doesn't exist
//   rm -r dir/                  Recursively remove directory and contents
//   rm -rf dir/                 Force recursive removal (common pattern)
//   rm -v file.txt              Verbose: print what's being removed
//
// # The -r flag (recursive)
//
// Without -r, rm refuses to remove directories:
//
//   rm mydir/    => ERROR: mydir is a directory
//
// With -r, rm walks the directory tree depth-first, removing files and
// subdirectories before removing the parent:
//
//   rm -r mydir/
//     remove mydir/file1.txt
//     remove mydir/sub/file2.txt
//     remove mydir/sub/
//     remove mydir/
//
// # Architecture
//
//   rm.json (spec)              rm_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -f,-r,-d  │      │ for each file arg:             │
//   │ -i,-I,-v         │─────>│   check type (file/dir)        │
//   │ variadic FILE    │      │   remove with appropriate fn   │
//   └──────────────────┘      │   print if verbose             │
//                             └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// runRm — the testable core of the rm tool
// =========================================================================

func runRm(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "rm: %s\n", err)
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
		force := getBool(r.Flags, "force")
		recursive := getBool(r.Flags, "recursive")
		removeDir := getBool(r.Flags, "dir")
		verbose := getBool(r.Flags, "verbose")

		// Extract file paths.
		files := getStringSlice(r.Arguments, "files")

		exitCode := 0
		for _, file := range files {
			// Check if the file/dir exists.
			info, err := os.Lstat(file)
			if err != nil {
				if force {
					// With -f, silently ignore nonexistent files.
					continue
				}
				fmt.Fprintf(stderr, "rm: cannot remove '%s': %s\n", file, err)
				exitCode = 1
				continue
			}

			// Handle directories.
			if info.IsDir() {
				if recursive {
					// os.RemoveAll removes the directory and everything inside it.
					// This is the equivalent of `rm -r`.
					err = os.RemoveAll(file)
				} else if removeDir {
					// os.Remove only works on empty directories.
					// This is the equivalent of `rm -d` (same as rmdir).
					err = os.Remove(file)
				} else {
					fmt.Fprintf(stderr, "rm: cannot remove '%s': Is a directory\n", file)
					exitCode = 1
					continue
				}
			} else {
				// Regular file, symlink, etc. — just remove it.
				err = os.Remove(file)
			}

			if err != nil {
				if !force {
					fmt.Fprintf(stderr, "rm: cannot remove '%s': %s\n", file, err)
					exitCode = 1
				}
				continue
			}

			if verbose {
				fmt.Fprintf(stdout, "removed '%s'\n", file)
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "rm: unexpected result type: %T\n", result)
		return 1
	}
}

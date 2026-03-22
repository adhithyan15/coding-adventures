// =========================================================================
// realpath — Print the Resolved Absolute File Name
// =========================================================================
//
// The `realpath` utility resolves symbolic links, `.` and `..` references,
// and extra slashes to produce the canonical absolute path.
//
// # Basic usage
//
//   realpath file.txt            Print absolute path of file.txt
//   realpath ../foo              Resolve the .. and print absolute path
//   realpath -e file.txt         Error if file doesn't exist
//   realpath -m /no/such/path    Print resolved path even if it doesn't exist
//   realpath -s link             Don't follow symlinks
//
// # Modes
//
// realpath has three canonicalization modes:
//
//   Default:  last component need not exist, others must
//   -e:       ALL components must exist (strict)
//   -m:       NO components need exist (lenient)
//
// # Architecture
//
//   realpath.json (spec)         realpath_tool.go (this file)
//   ┌──────────────────┐       ┌────────────────────────────────┐
//   │ flags: -e,-m,-s  │       │ for each file arg:             │
//   │ -q,-z            │──────>│   resolve path                 │
//   │ variadic FILE    │       │   apply canonicalization mode   │
//   └──────────────────┘       │   print result                 │
//                              └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// runRealpath — the testable core of the realpath tool
// =========================================================================

func runRealpath(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "realpath: %s\n", err)
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
		canonExisting := getBool(r.Flags, "canonicalize_existing")
		canonMissing := getBool(r.Flags, "canonicalize_missing")
		noSymlinks := getBool(r.Flags, "no_symlinks")
		quiet := getBool(r.Flags, "quiet")
		zero := getBool(r.Flags, "zero")

		// Determine the line terminator.
		terminator := "\n"
		if zero {
			terminator = "\x00"
		}

		// Extract file paths.
		files := getStringSlice(r.Arguments, "files")

		exitCode := 0
		for _, file := range files {
			var resolved string
			var resolveErr error

			if noSymlinks {
				// Don't follow symlinks — just make the path absolute
				// and clean it.
				resolved, resolveErr = filepath.Abs(file)
				if resolveErr == nil {
					resolved = filepath.Clean(resolved)
				}
			} else {
				// Full canonicalization: resolve symlinks and clean.
				resolved, resolveErr = filepath.EvalSymlinks(file)
				if resolveErr == nil {
					resolved, resolveErr = filepath.Abs(resolved)
				}
			}

			// Apply canonicalization mode constraints.
			if canonExisting {
				// -e: ALL components must exist.
				if _, err := os.Stat(file); err != nil {
					if !quiet {
						fmt.Fprintf(stderr, "realpath: %s: No such file or directory\n", file)
					}
					exitCode = 1
					continue
				}
			} else if canonMissing {
				// -m: no components need exist. We just compute the
				// absolute path without checking existence.
				absPath, err := filepath.Abs(file)
				if err != nil {
					if !quiet {
						fmt.Fprintf(stderr, "realpath: %s: %s\n", file, err)
					}
					exitCode = 1
					continue
				}
				resolved = filepath.Clean(absPath)
				resolveErr = nil
			}

			if resolveErr != nil {
				if !quiet {
					fmt.Fprintf(stderr, "realpath: %s: %s\n", file, resolveErr)
				}
				exitCode = 1
				continue
			}

			fmt.Fprint(stdout, resolved+terminator)
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "realpath: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// mkdir — Make Directories
// =========================================================================
//
// The `mkdir` utility creates directories. By default, it creates a single
// directory at the given path. With `-p`, it creates parent directories as
// needed — like building a path one segment at a time.
//
// # Basic usage
//
//   mkdir new_dir               Create a single directory
//   mkdir -p a/b/c              Create a/b/c and all parents
//   mkdir -v dir1 dir2          Create dir1 and dir2, printing each
//   mkdir -m 0755 dir           Create dir with specific permissions
//
// # The -p flag (parents)
//
// Without -p, creating "a/b/c" fails if "a/b" doesn't exist:
//
//   mkdir a/b/c     => ERROR: a/b does not exist
//
// With -p, all intermediate directories are created automatically:
//
//   mkdir -p a/b/c  => creates a/, a/b/, a/b/c/
//
// Also, with -p, it's NOT an error if the directory already exists.
// Without -p, trying to create an existing directory is an error.
//
// # Architecture
//
//   mkdir.json (spec)           mkdir_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -p,-m,-v  │      │ for each directory arg:        │
//   │ variadic DIR     │─────>│   create dir (with parents?)   │
//   │ help, version    │      │   apply mode if specified      │
//   └──────────────────┘      │   print if verbose             │
//                             └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"strconv"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// runMkdir — the testable core of the mkdir tool
// =========================================================================

func runMkdir(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "mkdir: %s\n", err)
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

		// Extract the mode flag. Default is 0777 (before umask).
		// The mode string is in octal format like "0755".
		perm := os.FileMode(0777)
		if modeStr, ok := r.Flags["mode"].(string); ok && modeStr != "" {
			parsed, err := strconv.ParseUint(modeStr, 8, 32)
			if err != nil {
				fmt.Fprintf(stderr, "mkdir: invalid mode '%s'\n", modeStr)
				return 1
			}
			perm = os.FileMode(parsed)
		}

		// Extract directory paths.
		dirs := getStringSlice(r.Arguments, "directories")

		exitCode := 0
		for _, dir := range dirs {
			var mkErr error
			if parents {
				// MkdirAll creates the full path, including parents.
				// It does NOT error if the directory already exists.
				mkErr = os.MkdirAll(dir, perm)
			} else {
				// Mkdir creates only the leaf directory.
				// It errors if the parent doesn't exist or if the
				// directory already exists.
				mkErr = os.Mkdir(dir, perm)
			}

			if mkErr != nil {
				fmt.Fprintf(stderr, "mkdir: cannot create directory '%s': %s\n", dir, mkErr)
				exitCode = 1
				continue
			}

			if verbose {
				fmt.Fprintf(stdout, "mkdir: created directory '%s'\n", dir)
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "mkdir: unexpected result type: %T\n", result)
		return 1
	}
}

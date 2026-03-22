// =========================================================================
// dirname — Strip Last Component from File Name
// =========================================================================
//
// The `dirname` utility strips the last component (usually the filename)
// from a path, leaving just the directory portion. It's the complement
// of `basename`.
//
// # Basic usage
//
//   dirname /usr/bin/sort       =>  "/usr/bin"
//   dirname /usr/bin/           =>  "/usr"
//   dirname stdio.h             =>  "."
//   dirname /                   =>  "/"
//
// # How it works
//
// The POSIX algorithm:
//   1. If the string is empty, return "." (current directory).
//   2. Remove trailing slashes.
//   3. If no slashes remain, return "." (the path was just a filename).
//   4. Remove the trailing non-slash component.
//   5. Remove trailing slashes from the result.
//   6. If the result is empty, return "/".
//
// Go's filepath.Dir handles most of these cases correctly.
//
// # Flags
//
//   -z    End each output line with NUL instead of newline.
//
// # Architecture
//
//   dirname.json (spec)        dirname_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -z         │      │ for each name:                │
//   │ variadic NAME    │─────>│   strip last component         │
//   │ help, version    │      │   output with terminator       │
//   └──────────────────┘      └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"path/filepath"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// runDirname — the testable core of the dirname tool
// =========================================================================

func runDirname(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "dirname: %s\n", err)
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
		zero := getBool(r.Flags, "zero")

		// Determine the line terminator.
		terminator := "\n"
		if zero {
			terminator = "\x00"
		}

		// Extract positional arguments.
		names := getStringSlice(r.Arguments, "names")
		if len(names) == 0 {
			fmt.Fprintf(stderr, "dirname: missing operand\n")
			return 1
		}

		// Process each name.
		//
		// filepath.Dir() implements the POSIX dirname algorithm:
		//   filepath.Dir("/usr/bin/sort")  =>  "/usr/bin"
		//   filepath.Dir("sort")           =>  "."
		//   filepath.Dir("/")              =>  "/"
		for _, name := range names {
			dir := filepath.Dir(name)
			fmt.Fprint(stdout, dir)
			fmt.Fprint(stdout, terminator)
		}

		return 0

	default:
		fmt.Fprintf(stderr, "dirname: unexpected result type: %T\n", result)
		return 1
	}
}

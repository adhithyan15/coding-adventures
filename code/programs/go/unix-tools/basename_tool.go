// =========================================================================
// basename — Strip Directory and Suffix from Filenames
// =========================================================================
//
// The `basename` utility strips the directory path and optionally a
// suffix from a filename. It's commonly used in shell scripts to extract
// just the filename from a full path.
//
// # Basic usage
//
//   basename /usr/bin/sort      =>  "sort"
//   basename /home/user/file.txt .txt  =>  "file"
//   basename -s .txt file1.txt file2.txt  =>  "file1\nfile2"
//
// # How it works
//
// The algorithm follows POSIX rules:
//   1. If the string is "//", the result is implementation-defined
//      (we return "/").
//   2. Remove trailing slashes.
//   3. If no slashes remain, the string IS the basename.
//   4. Otherwise, remove everything up to and including the last slash.
//   5. If a suffix is specified and the basename ends with it
//      (and is not equal to it), remove the suffix.
//
// # Multiple mode (-a / -s SUFFIX)
//
// In traditional usage, basename takes NAME [SUFFIX] as two positional
// arguments. With -a, it treats all positional arguments as NAMEs and
// processes each one. The -s flag implies -a and specifies the suffix
// to strip from each name.
//
//   basename -a /usr/bin/sort /usr/bin/head
//   =>  "sort\nhead"
//
//   basename -s .go main.go test.go
//   =>  "main\ntest"
//
// # Architecture
//
//   basename.json (spec)       basename_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -a,-s,-z  │      │ for each name:                 │
//   │ variadic NAME    │─────>│   strip directory              │
//   │ help, version    │      │   strip suffix                 │
//   └──────────────────┘      │   output with terminator       │
//                             └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"path/filepath"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// stripBasename — extract the basename from a path
// =========================================================================
//
// This implements the POSIX basename algorithm:
//   1. Remove trailing slashes (unless the string is all slashes).
//   2. Return everything after the last slash.
//
// We use Go's filepath.Base which handles most edge cases correctly,
// including trailing slashes, multiple slashes, and the root path "/".

func stripBasename(name string, suffix string) string {
	// filepath.Base handles the core logic:
	//   "/usr/bin/sort" => "sort"
	//   "/"             => "/"
	//   "."             => "."
	base := filepath.Base(name)

	// Strip the suffix if specified.
	// The suffix is only removed if:
	//   1. The base ends with the suffix
	//   2. The base is NOT equal to the suffix (avoid stripping everything)
	if suffix != "" && base != suffix && strings.HasSuffix(base, suffix) {
		base = base[:len(base)-len(suffix)]
	}

	return base
}

// =========================================================================
// runBasename — the testable core of the basename tool
// =========================================================================

func runBasename(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "basename: %s\n", err)
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
		multiple := getBool(r.Flags, "multiple")
		zero := getBool(r.Flags, "zero")

		// Extract the suffix flag. If -s is given, it implies -a (multiple).
		suffix := ""
		if v, ok := r.Flags["suffix"]; ok {
			if s, ok := v.(string); ok && s != "" {
				suffix = s
				multiple = true
			}
		}

		// Determine the line terminator.
		terminator := "\n"
		if zero {
			terminator = "\x00"
		}

		// Extract positional arguments.
		names := getStringSlice(r.Arguments, "names")
		if len(names) == 0 {
			fmt.Fprintf(stderr, "basename: missing operand\n")
			return 1
		}

		if multiple {
			// Multiple mode (-a or -s): treat all arguments as names.
			for _, name := range names {
				fmt.Fprint(stdout, stripBasename(name, suffix))
				fmt.Fprint(stdout, terminator)
			}
		} else {
			// Traditional mode: first arg is NAME, second (optional) is SUFFIX.
			name := names[0]
			if len(names) > 1 {
				suffix = names[1]
			}
			fmt.Fprint(stdout, stripBasename(name, suffix))
			fmt.Fprint(stdout, terminator)
		}

		return 0

	default:
		fmt.Fprintf(stderr, "basename: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// printenv — Print All or Part of Environment
// =========================================================================
//
// The `printenv` utility prints the values of specified environment
// variables. If no arguments are given, it prints all environment
// variables in the form NAME=VALUE.
//
// # Basic usage
//
//   printenv                   Print all environment variables
//   printenv PATH              Print just the value of PATH
//   printenv HOME USER         Print the values of HOME and USER
//   printenv -0 PATH           Print PATH value terminated by NUL
//
// # Difference from env
//
// printenv and env are similar but different:
//   - `env` runs a command in a modified environment
//   - `printenv` just prints environment variables
//   - When called with no args, both print all variables
//   - `printenv VAR` prints just the value; `env` always shows NAME=VALUE
//
// # Exit code
//
// printenv returns 0 if all specified variables exist, and 1 if any
// are not set. When called with no arguments, it always returns 0.
//
// # Architecture
//
//   printenv.json (spec)       printenv_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -0         │      │ if no args:                   │
//   │ variadic VARIABLE │─────>│   print all env vars          │
//   │ help, version    │      │ else:                          │
//   └──────────────────┘      │   print requested vars         │
//                             └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"sort"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// runPrintenv — the testable core of the printenv tool
// =========================================================================

func runPrintenv(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "printenv: %s\n", err)
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
		null := getBool(r.Flags, "null")

		// Determine the line terminator.
		terminator := "\n"
		if null {
			terminator = "\x00"
		}

		// Extract variable names.
		variables := getStringSlice(r.Arguments, "variables")

		if len(variables) == 0 {
			// No arguments: print all environment variables.
			//
			// os.Environ() returns a slice of "KEY=VALUE" strings.
			// We sort them for deterministic output (matching GNU printenv).
			env := os.Environ()
			sort.Strings(env)
			for _, e := range env {
				fmt.Fprint(stdout, e)
				fmt.Fprint(stdout, terminator)
			}
			return 0
		}

		// Print the requested variables.
		//
		// If any requested variable is not set, we return exit code 1
		// (matching GNU printenv behavior). But we still print the ones
		// that DO exist.
		exitCode := 0
		for _, name := range variables {
			value, exists := os.LookupEnv(name)
			if !exists {
				exitCode = 1
				continue
			}

			// When printing specific variables, we print just the value
			// (not NAME=VALUE). This differs from the "no args" mode.
			fmt.Fprint(stdout, value)
			fmt.Fprint(stdout, terminator)
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "printenv: unexpected result type: %T\n", result)
		return 1
	}
}

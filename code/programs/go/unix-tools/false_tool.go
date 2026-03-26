// =========================================================================
// false — Do Nothing, Unsuccessfully
// =========================================================================
//
// The `false` utility is the counterpart to `true`. It does absolutely
// nothing and exits with status code 1 (failure). Like `true`, it plays
// an important role in shell scripting despite its simplicity.
//
// # Why does `false` exist?
//
// Shell scripts often need a command that always fails. Common uses:
//
//   1. Testing error handling:
//        if false; then
//          echo "this never runs"
//        else
//          echo "error handling works"
//        fi
//
//   2. Disabling features:
//        CAN_FLY=false
//        if $CAN_FLY; then
//          launch_rocket
//        fi
//
//   3. Breaking out of loops:
//        while true; do
//          if should_stop; then
//            false   # sets exit status to 1
//            break
//          fi
//        done
//
//   4. Triggering set -e exits:
//        set -e        # exit on any error
//        false         # script exits here
//        echo "unreachable"
//
// # Relationship to `true`
//
// `true` and `false` are mirror images:
//
//	┌──────────┬─────────────┬──────────┐
//	│ Command  │ Exit Code   │ Meaning  │
//	├──────────┼─────────────┼──────────┤
//	│ true     │ 0           │ success  │
//	│ false    │ 1           │ failure  │
//	└──────────┴─────────────┴──────────┘
//
// The implementation is identical except for the return value.

package main

import (
	"fmt"
	"io"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// runFalse — the testable core of the false tool
// =========================================================================
//
// Like `true`, we parse arguments for --help and --version support.
// The only difference: normal operation returns 1 instead of 0.

func runFalse(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "false: %s\n", err)
		return 1
	}

	// Step 2: Parse the arguments.
	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 1
	}

	// Step 3: Handle the result.
	//
	// --help and --version exit 0 (they succeeded at their task).
	// Normal operation exits 1 — that's the whole point of `false`.
	switch r := result.(type) {

	case *clibuilder.HelpResult:
		// Even `false` exits 0 for --help. The help request succeeded.
		fmt.Fprintln(stdout, r.Text)
		return 0

	case *clibuilder.VersionResult:
		// Same for --version: the version query succeeded.
		fmt.Fprintln(stdout, r.Version)
		return 0

	case *clibuilder.ParseResult:
		// The core behavior of `false`: do nothing, return failure.
		// This is the ONLY difference from `true`.
		return 1

	default:
		fmt.Fprintf(stderr, "false: unexpected result type: %T\n", result)
		return 1
	}
}

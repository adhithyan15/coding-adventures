// =========================================================================
// true — Do Nothing, Successfully
// =========================================================================
//
// The `true` utility is one of the simplest programs in Unix. It does
// absolutely nothing and exits with status code 0 (success). Despite its
// simplicity, it serves an important role in shell scripting.
//
// # Why does `true` exist?
//
// Shell scripts often need a command that always succeeds. Common uses:
//
//   1. Infinite loops:
//        while true; do
//          echo "running..."
//          sleep 1
//        done
//
//   2. Placeholder commands:
//        if some_condition; then
//          true   # TODO: implement this branch later
//        fi
//
//   3. Resetting exit status:
//        false          # sets $? to 1
//        true           # resets $? to 0
//
//   4. No-op in conditionals:
//        some_command || true   # ignore failures
//
// # Implementation
//
// Our implementation goes beyond the minimal POSIX spec by supporting
// --help and --version flags (via cli-builder). The POSIX spec says
// `true` should accept and ignore any arguments, but GNU coreutils
// added --help and --version, and that's what most users expect.
//
// The business logic is trivial: parse args, then return 0. The only
// interesting part is that we still parse arguments so that --help and
// --version work correctly.

package main

import (
	"fmt"
	"io"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// runTrue — the testable core of the true tool
// =========================================================================
//
// Even though `true` does nothing, we still parse arguments so that
// --help prints usage information and --version prints the version.
// This follows the GNU coreutils convention.
//
// The function signature matches our standard tool pattern:
//   - specPath: path to the JSON spec file
//   - argv: the full argument vector (argv[0] is the program name)
//   - stdout: where to write normal output (help text, version)
//   - stderr: where to write error messages
//   - returns: exit code (always 0 for true, unless there's a spec error)

func runTrue(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	//
	// Even though `true` ignores arguments, we need the parser to handle
	// --help and --version. Without parsing, the user couldn't discover
	// what this tool does.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "true: %s\n", err)
		return 1
	}

	// Step 2: Parse the arguments.
	result, err := parser.Parse()
	if err != nil {
		// Parse errors shouldn't happen for `true` (no flags to conflict),
		// but we handle them for robustness.
		fmt.Fprintf(stderr, "%s\n", err)
		return 1
	}

	// Step 3: Handle the result.
	//
	// For --help and --version, we print and exit 0.
	// For normal operation, we just exit 0.
	// `true` always exits 0 — that's its entire purpose.
	switch r := result.(type) {

	case *clibuilder.HelpResult:
		fmt.Fprintln(stdout, r.Text)
		return 0

	case *clibuilder.VersionResult:
		fmt.Fprintln(stdout, r.Version)
		return 0

	case *clibuilder.ParseResult:
		// The core behavior of `true`: do nothing, return success.
		return 0

	default:
		fmt.Fprintf(stderr, "true: unexpected result type: %T\n", result)
		return 1
	}
}

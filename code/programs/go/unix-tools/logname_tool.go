// =========================================================================
// logname — Print the User's Login Name
// =========================================================================
//
// The `logname` utility prints the name of the user who logged in to the
// current session. This is subtly different from `whoami`:
//
//   - `whoami` prints the *effective* user (which changes with sudo/su)
//   - `logname` prints the *login* user (who originally logged in)
//
// # Example showing the difference
//
//   $ whoami        # => alice
//   $ logname       # => alice
//   $ sudo whoami   # => root
//   $ sudo logname  # => alice  (still the original login user!)
//
// This distinction matters for audit logging: you want to know WHO
// ran the sudo command, not just that root executed something.
//
// # Implementation
//
// On most Unix systems, the login name is available via:
//   1. The LOGNAME environment variable (set by login/sshd/etc.)
//   2. The USER environment variable (as a fallback)
//
// POSIX specifies that LOGNAME should be set by the login process
// and should not be changed by sudo or su. In practice, some systems
// don't strictly follow this, but $LOGNAME is the standard source.
//
// # Architecture
//
//   logname.json (spec)       logname_tool.go (this file)
//   ┌──────────────────┐     ┌──────────────────────────────┐
//   │ no flags          │     │ check $LOGNAME               │
//   │ no arguments      │────>│ fallback to $USER            │
//   │ help, version     │     │ print login name or error    │
//   └──────────────────┘     └──────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// getLoginName — retrieve the login name from the environment
// =========================================================================
//
// Checks $LOGNAME first (the POSIX-standard variable set by login),
// then falls back to $USER. Returns empty string if neither is set.

func getLoginName() string {
	if logname := os.Getenv("LOGNAME"); logname != "" {
		return logname
	}
	return os.Getenv("USER")
}

// =========================================================================
// runLogname — the testable core of the logname tool
// =========================================================================
//
// The logname tool:
//   1. Parses arguments using cli-builder (only --help and --version).
//   2. Retrieves the login name from the environment.
//   3. Prints it to stdout, or prints an error and exits 1 if unavailable.

func runLogname(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "logname: %s\n", err)
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
		loginName := getLoginName()
		if loginName == "" {
			fmt.Fprintln(stderr, "logname: no login name")
			return 1
		}

		fmt.Fprintln(stdout, loginName)
		return 0

	default:
		fmt.Fprintf(stderr, "logname: unexpected result type: %T\n", result)
		return 1
	}
}

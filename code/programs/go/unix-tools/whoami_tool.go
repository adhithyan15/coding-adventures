// =========================================================================
// whoami — Print Effective User Name
// =========================================================================
//
// The `whoami` utility prints the user name associated with the current
// effective user ID. It's essentially a shorthand for `id -un`.
//
// # What is the "effective" user?
//
// On Unix systems, every process runs with a user identity. There are
// actually several user IDs:
//
//   - Real UID:      who you logged in as
//   - Effective UID: who you're acting as right now
//   - Saved UID:     the previous effective UID (for switching back)
//
// Most of the time, real == effective. But when you run a setuid program
// (like `sudo` or `passwd`), the effective UID changes to the program
// owner (often root). `whoami` reports the effective UID's username.
//
// # Examples
//
//   $ whoami
//   alice
//
//   $ sudo whoami
//   root
//
// # Implementation
//
// We use os/user.Current() which returns the effective user. As a
// fallback, we check the $USER environment variable.
//
// # Architecture
//
//   whoami.json (spec)         whoami_tool.go (this file)
//   ┌──────────────────┐     ┌──────────────────────────────┐
//   │ no flags          │     │ get current user              │
//   │ no arguments      │────>│ print username                │
//   │ help, version     │     │ exit 0                        │
//   └──────────────────┘     └──────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"os/user"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// getEffectiveUsername — retrieve the current effective username
// =========================================================================
//
// This function tries two approaches:
//   1. os/user.Current() — the authoritative source, queries the system
//      user database (passwd/LDAP/etc.)
//   2. $USER environment variable — a fallback for environments where
//      the user database is unavailable (containers, chroots, etc.)
//
// Returns empty string and error if neither method works.

func getEffectiveUsername() (string, error) {
	// Try the authoritative source first.
	u, err := user.Current()
	if err == nil && u.Username != "" {
		return u.Username, nil
	}

	// Fallback to $USER environment variable.
	username := os.Getenv("USER")
	if username != "" {
		return username, nil
	}

	return "", fmt.Errorf("cannot determine current user")
}

// =========================================================================
// runWhoami — the testable core of the whoami tool
// =========================================================================
//
// The whoami tool:
//   1. Parses arguments using cli-builder (only --help and --version).
//   2. Retrieves the current effective username.
//   3. Prints it to stdout.
//   4. Returns 0 on success, 1 on error.

func runWhoami(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "whoami: %s\n", err)
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
		username, err := getEffectiveUsername()
		if err != nil {
			fmt.Fprintf(stderr, "whoami: %s\n", err)
			return 1
		}

		fmt.Fprintln(stdout, username)
		return 0

	default:
		fmt.Fprintf(stderr, "whoami: unexpected result type: %T\n", result)
		return 1
	}
}

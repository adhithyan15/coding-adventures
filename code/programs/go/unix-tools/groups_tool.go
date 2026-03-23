// =========================================================================
// groups — Print Group Memberships
// =========================================================================
//
// The `groups` utility prints the groups a user belongs to. It's a
// simpler alternative to `id -Gn`.
//
// # Examples
//
//   $ groups
//   staff everyone localaccounts
//
//   $ groups alice
//   alice : staff everyone
//
// # How it works
//
// On Unix, every user belongs to a primary group (set in /etc/passwd)
// and zero or more supplementary groups (set in /etc/group). The
// `groups` command prints all of them.
//
// # Architecture
//
//   groups.json (spec)           groups_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ no flags          │       │ look up user via os/user         │
//   │ arg: USERNAME...  │──────>│ get group IDs, resolve names     │
//   │ help, version     │       │ print space-separated names      │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os/user"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// getGroups — look up group names for a user
// =========================================================================
//
// If username is empty, looks up the current user's groups.
// Returns a slice of group names (not IDs).

func getGroups(username string) ([]string, error) {
	var u *user.User
	var err error

	if username == "" {
		u, err = user.Current()
	} else {
		u, err = user.Lookup(username)
	}
	if err != nil {
		return nil, err
	}

	// Get all group IDs for this user.
	groupIds, err := u.GroupIds()
	if err != nil {
		return nil, fmt.Errorf("cannot get groups for %s: %w", u.Username, err)
	}

	// Resolve each group ID to a group name.
	names := make([]string, 0, len(groupIds))
	for _, gid := range groupIds {
		if g, err := user.LookupGroupId(gid); err == nil {
			names = append(names, g.Name)
		} else {
			// If we can't resolve the name, use the numeric ID.
			names = append(names, gid)
		}
	}

	return names, nil
}

// =========================================================================
// runGroups — the testable core of the groups tool
// =========================================================================

func runGroups(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "groups: %s\n", err)
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
		// Get the optional username arguments.
		usernames := getStringSlice(r.Arguments, "users")

		exitCode := 0

		if len(usernames) == 0 {
			// No arguments — show current user's groups.
			groups, err := getGroups("")
			if err != nil {
				fmt.Fprintf(stderr, "groups: %s\n", err)
				return 1
			}
			fmt.Fprintln(stdout, strings.Join(groups, " "))
		} else {
			// One or more usernames specified.
			for _, username := range usernames {
				groups, err := getGroups(username)
				if err != nil {
					fmt.Fprintf(stderr, "groups: %s\n", err)
					exitCode = 1
					continue
				}
				// When a username is given, prefix with "username : ".
				fmt.Fprintf(stdout, "%s : %s\n", username, strings.Join(groups, " "))
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "groups: unexpected result type: %T\n", result)
		return 1
	}
}

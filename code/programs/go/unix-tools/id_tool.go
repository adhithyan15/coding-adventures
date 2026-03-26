// =========================================================================
// id — Print Real and Effective User and Group IDs
// =========================================================================
//
// The `id` utility prints identity information about the current user
// (or a specified user). By default, it shows the user ID (uid), primary
// group ID (gid), and all supplementary group IDs (groups).
//
// # Default output format
//
//   uid=501(alice) gid=20(staff) groups=20(staff),501(access_bpf)
//
// # Common flags
//
//   id              Full output (uid, gid, groups)
//   id -u           Print only effective user ID (number)
//   id -un          Print only effective username (name)
//   id -g           Print only effective group ID (number)
//   id -gn          Print only effective group name
//   id -G           Print all group IDs
//   id -Gn          Print all group names
//
// # Architecture
//
//   id.json (spec)               id_tool.go (this file)
//   ┌──────────────────┐       ┌────────────────────────────────┐
//   │ flags: -u,-g,-G  │       │ look up user via os/user       │
//   │ -n,-r,-z         │──────>│ format based on flags          │
//   │ arg: USER        │       │ print to stdout                │
//   └──────────────────┘       └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os/user"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// IdInfo — holds all the identity information for a user
// =========================================================================

type IdInfo struct {
	Uid      string   // user ID (numeric)
	Username string   // user name
	Gid      string   // primary group ID (numeric)
	Gname    string   // primary group name
	Groups   []string // all group IDs (numeric)
	Gnames   []string // all group names
}

// =========================================================================
// getUserInfo — look up a user's identity information
// =========================================================================
//
// If username is empty, we look up the current user.
// Otherwise, we look up the specified user by name.

func getUserInfo(username string) (*IdInfo, error) {
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

	// Look up the primary group name.
	gname := u.Gid // Default to GID if lookup fails.
	if g, err := user.LookupGroupId(u.Gid); err == nil {
		gname = g.Name
	}

	// Look up all group IDs for this user.
	groupIds, err := u.GroupIds()
	if err != nil {
		// If group lookup fails, at least include the primary group.
		groupIds = []string{u.Gid}
	}

	// Look up group names for all group IDs.
	gnames := make([]string, len(groupIds))
	for i, gid := range groupIds {
		if g, err := user.LookupGroupId(gid); err == nil {
			gnames[i] = g.Name
		} else {
			gnames[i] = gid // Fall back to numeric ID.
		}
	}

	return &IdInfo{
		Uid:      u.Uid,
		Username: u.Username,
		Gid:      u.Gid,
		Gname:    gname,
		Groups:   groupIds,
		Gnames:   gnames,
	}, nil
}

// =========================================================================
// formatId — format the id output based on flags
// =========================================================================
//
// Formatting rules:
//   - No flags:         uid=NN(name) gid=NN(name) groups=NN(name),...
//   - -u:               print UID number
//   - -u -n:            print username
//   - -g:               print GID number
//   - -g -n:            print group name
//   - -G:               print all group IDs (space-separated)
//   - -G -n:            print all group names (space-separated)

func formatId(info *IdInfo, showUser, showGroup, showGroups, showName bool) string {
	if showUser {
		if showName {
			return info.Username
		}
		return info.Uid
	}

	if showGroup {
		if showName {
			return info.Gname
		}
		return info.Gid
	}

	if showGroups {
		if showName {
			return strings.Join(info.Gnames, " ")
		}
		return strings.Join(info.Groups, " ")
	}

	// Default: full output.
	// Format: uid=NN(name) gid=NN(name) groups=NN(name),NN(name),...
	var groupParts []string
	for i, gid := range info.Groups {
		groupParts = append(groupParts, fmt.Sprintf("%s(%s)", gid, info.Gnames[i]))
	}

	return fmt.Sprintf("uid=%s(%s) gid=%s(%s) groups=%s",
		info.Uid, info.Username,
		info.Gid, info.Gname,
		strings.Join(groupParts, ","))
}

// =========================================================================
// runId — the testable core of the id tool
// =========================================================================

func runId(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "id: %s\n", err)
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
		// Get the optional username argument.
		username, _ := r.Arguments["user_name"].(string)

		info, err := getUserInfo(username)
		if err != nil {
			fmt.Fprintf(stderr, "id: %s\n", err)
			return 1
		}

		output := formatId(info,
			getBool(r.Flags, "user"),
			getBool(r.Flags, "group"),
			getBool(r.Flags, "groups"),
			getBool(r.Flags, "name"),
		)

		fmt.Fprintln(stdout, output)
		return 0

	default:
		fmt.Fprintf(stderr, "id: unexpected result type: %T\n", result)
		return 1
	}
}

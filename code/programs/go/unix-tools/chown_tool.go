// =========================================================================
// chown — Change File Owner and Group
// =========================================================================
//
// The `chown` utility changes the user and/or group ownership of files.
// Ownership is a fundamental Unix security concept — it determines
// which user "owns" a file and which group it belongs to.
//
// # How Unix ownership works
//
// Every file has two ownership attributes:
//
//   Attribute   Description                     Example
//   ─────────   ─────────────────────────────   ────────
//   User (UID)  The file's owner                alice
//   Group (GID) The file's group                staff
//
// Only the superuser (root) can change file ownership. Regular users
// can only change the group to a group they belong to.
//
// # Specifying ownership
//
// chown supports several formats:
//
//   Format        Meaning
//   ──────────    ──────────────────────────────
//   OWNER         Change owner only
//   OWNER:GROUP   Change both owner and group
//   OWNER:        Change owner, set group to owner's login group
//   :GROUP        Change group only
//   OWNER.GROUP   Alternate separator (deprecated)
//
// # Note on non-root systems
//
// On most systems, only root can chown files. Our implementation
// parses the ownership spec and attempts the operation, but tests
// handle the expected EPERM errors gracefully.
//
// # Architecture
//
//   chown.json (spec)            chown_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -R,-v,-c  │       │ parse OWNER[:GROUP]              │
//   │ -h,-f,--reference│──────>│ lookup UID/GID                   │
//   │ args: OWNER FILES│       │ for each file: os.Lchown()       │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"os/user"
	"path/filepath"
	"strconv"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// ChownOptions — configuration for the chown operation
// =========================================================================

type ChownOptions struct {
	Recursive    bool // -R: operate recursively
	Verbose      bool // -v: output a diagnostic for every file
	Changes      bool // -c: report only when a change is made
	Silent       bool // -f: suppress most error messages
	NoDereference bool // -h: affect symlinks, not their targets
}

// =========================================================================
// ChownSpec — parsed owner and group specification
// =========================================================================
//
// After parsing the OWNER[:GROUP] argument, we store the results here.
// A value of -1 means "don't change this attribute".

type ChownSpec struct {
	UID int // User ID to set (-1 = don't change)
	GID int // Group ID to set (-1 = don't change)
	OwnerName string // Original owner name (for diagnostics)
	GroupName string // Original group name (for diagnostics)
}

// =========================================================================
// parseChownSpec — parse the OWNER[:GROUP] specification
// =========================================================================
//
// This function handles all the ownership specification formats:
//
//   "alice"         → change owner to alice
//   "alice:staff"   → change owner to alice, group to staff
//   "alice:"        → change owner to alice (group from alice's login)
//   ":staff"        → change group to staff only
//   "1000"          → change owner to UID 1000
//   "1000:100"      → change owner to UID 1000, group to GID 100
//
// Returns a ChownSpec with resolved UIDs and GIDs.

func parseChownSpec(spec string) (ChownSpec, error) {
	result := ChownSpec{UID: -1, GID: -1}

	// Split on ':' (preferred) or '.' (deprecated alternate).
	var ownerPart, groupPart string
	hasGroup := false

	if idx := strings.IndexByte(spec, ':'); idx >= 0 {
		ownerPart = spec[:idx]
		groupPart = spec[idx+1:]
		hasGroup = true
	} else if idx := strings.IndexByte(spec, '.'); idx >= 0 {
		ownerPart = spec[:idx]
		groupPart = spec[idx+1:]
		hasGroup = true
	} else {
		ownerPart = spec
	}

	// Resolve owner.
	if ownerPart != "" {
		result.OwnerName = ownerPart
		uid, err := resolveChownUser(ownerPart)
		if err != nil {
			return result, fmt.Errorf("invalid user: '%s'", ownerPart)
		}
		result.UID = uid
	}

	// Resolve group.
	if hasGroup && groupPart != "" {
		result.GroupName = groupPart
		gid, err := resolveChownGroup(groupPart)
		if err != nil {
			return result, fmt.Errorf("invalid group: '%s'", groupPart)
		}
		result.GID = gid
	}

	return result, nil
}

// =========================================================================
// resolveChownUser — resolve a username or UID string to a numeric UID
// =========================================================================

func resolveChownUser(name string) (int, error) {
	// Try numeric UID first.
	if uid, err := strconv.Atoi(name); err == nil {
		return uid, nil
	}

	// Look up by username.
	u, err := user.Lookup(name)
	if err != nil {
		return -1, err
	}
	return strconv.Atoi(u.Uid)
}

// =========================================================================
// resolveChownGroup — resolve a group name or GID string to a numeric GID
// =========================================================================

func resolveChownGroup(name string) (int, error) {
	// Try numeric GID first.
	if gid, err := strconv.Atoi(name); err == nil {
		return gid, nil
	}

	// Look up by group name.
	g, err := user.LookupGroup(name)
	if err != nil {
		return -1, err
	}
	return strconv.Atoi(g.Gid)
}

// =========================================================================
// chownFile — change ownership of a single file
// =========================================================================

func chownFile(path string, spec ChownSpec, opts ChownOptions, stdout, stderr io.Writer) int {
	// Get current ownership for comparison and diagnostics.
	info, err := os.Lstat(path)
	if err != nil {
		if !opts.Silent {
			fmt.Fprintf(stderr, "chown: cannot access '%s': %s\n", path, err)
		}
		return 1
	}

	oldUID, oldGID := getFileOwnership(info)

	uid := spec.UID
	gid := spec.GID

	// Perform the ownership change.
	var chownErr error
	if opts.NoDereference {
		chownErr = os.Lchown(path, uid, gid)
	} else {
		chownErr = os.Chown(path, uid, gid)
	}

	if chownErr != nil {
		if !opts.Silent {
			fmt.Fprintf(stderr, "chown: changing ownership of '%s': %s\n", path, chownErr)
		}
		return 1
	}

	// Report changes.
	changed := (uid >= 0 && uid != oldUID) || (gid >= 0 && gid != oldGID)
	ownerStr := formatChownOwner(spec)

	if opts.Verbose {
		fmt.Fprintf(stdout, "ownership of '%s' set to %s\n", path, ownerStr)
	} else if opts.Changes && changed {
		fmt.Fprintf(stdout, "changed ownership of '%s' to %s\n", path, ownerStr)
	}

	return 0
}

// formatChownOwner formats the ownership spec for diagnostic output.
func formatChownOwner(spec ChownSpec) string {
	parts := []string{}
	if spec.OwnerName != "" {
		parts = append(parts, spec.OwnerName)
	} else if spec.UID >= 0 {
		parts = append(parts, strconv.Itoa(spec.UID))
	}
	if spec.GroupName != "" {
		if len(parts) > 0 {
			return parts[0] + ":" + spec.GroupName
		}
		return ":" + spec.GroupName
	} else if spec.GID >= 0 {
		if len(parts) > 0 {
			return parts[0] + ":" + strconv.Itoa(spec.GID)
		}
		return ":" + strconv.Itoa(spec.GID)
	}
	if len(parts) > 0 {
		return parts[0]
	}
	return ""
}

// =========================================================================
// chownRecursive — recursively change ownership
// =========================================================================

func chownRecursive(path string, spec ChownSpec, opts ChownOptions, stdout, stderr io.Writer) int {
	exitCode := 0

	err := filepath.Walk(path, func(p string, info os.FileInfo, err error) error {
		if err != nil {
			if !opts.Silent {
				fmt.Fprintf(stderr, "chown: cannot access '%s': %s\n", p, err)
			}
			exitCode = 1
			return nil
		}

		rc := chownFile(p, spec, opts, stdout, stderr)
		if rc != 0 {
			exitCode = rc
		}
		return nil
	})

	if err != nil && !opts.Silent {
		fmt.Fprintf(stderr, "chown: %s\n", err)
		return 1
	}

	return exitCode
}

// =========================================================================
// runChown — the testable core of the chown tool
// =========================================================================

func runChown(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "chown: %s\n", err)
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
		opts := ChownOptions{
			Recursive:     getBool(r.Flags, "recursive"),
			Verbose:       getBool(r.Flags, "verbose"),
			Changes:       getBool(r.Flags, "changes"),
			Silent:        getBool(r.Flags, "silent"),
			NoDereference: getBool(r.Flags, "no_dereference"),
		}

		// Parse the ownership specification.
		var spec ChownSpec
		if refFile, ok := r.Flags["reference"].(string); ok && refFile != "" {
			// --reference mode: use the ownership of the reference file.
			refInfo, err := os.Stat(refFile)
			if err != nil {
				fmt.Fprintf(stderr, "chown: cannot stat reference file '%s': %s\n", refFile, err)
				return 1
			}
			if refSpec, ok := getRefFileOwnership(refInfo); ok {
				spec = refSpec
			}
		} else {
			ownerGroup, _ := r.Arguments["owner_group"].(string)
			if ownerGroup == "" {
				fmt.Fprintf(stderr, "chown: missing operand\n")
				return 1
			}
			spec, err = parseChownSpec(ownerGroup)
			if err != nil {
				fmt.Fprintf(stderr, "chown: %s\n", err)
				return 1
			}
		}

		// Get file list.
		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			fmt.Fprintf(stderr, "chown: missing operand\n")
			return 1
		}

		exitCode := 0
		for _, file := range files {
			var rc int
			if opts.Recursive {
				rc = chownRecursive(file, spec, opts, stdout, stderr)
			} else {
				rc = chownFile(file, spec, opts, stdout, stderr)
			}
			if rc != 0 {
				exitCode = rc
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "chown: unexpected result type: %T\n", result)
		return 1
	}
}

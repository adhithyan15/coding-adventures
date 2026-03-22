// =========================================================================
// ln — Make Links Between Files
// =========================================================================
//
// The `ln` utility creates links between files. There are two types:
//
// # Hard links (default)
//
// A hard link is a second name for the same file data on disk. Both
// names point to the same inode (the filesystem's internal file record):
//
//   ┌─────────┐     ┌─────────────┐
//   │ "orig"  │────>│  inode 42   │
//   │         │     │  data: ...  │
//   │ "link"  │────>│             │
//   └─────────┘     └─────────────┘
//
// Deleting one name doesn't affect the other. The data is only freed
// when ALL names are removed.
//
// # Symbolic links (-s)
//
// A symbolic link (symlink) is a special file that contains a path to
// another file. It's like a shortcut:
//
//   ┌─────────┐     ┌─────────────┐     ┌─────────────┐
//   │ "link"  │────>│ symlink     │────>│ "target"    │
//   │         │     │ -> "target" │     │ data: ...   │
//   └─────────┘     └─────────────┘     └─────────────┘
//
// # Basic usage
//
//   ln target link              Create hard link
//   ln -s target link           Create symbolic link
//   ln -sf target link          Force: remove existing link first
//
// # Architecture
//
//   ln.json (spec)              ln_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -s,-f,-n  │      │ determine link type            │
//   │ -i,-r,-v         │─────>│ remove existing if -f          │
//   │ variadic TARGET  │      │ create hard or symbolic link   │
//   └──────────────────┘      └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// runLn — the testable core of the ln tool
// =========================================================================

func runLn(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "ln: %s\n", err)
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
		symbolic := getBool(r.Flags, "symbolic")
		force := getBool(r.Flags, "force")
		verbose := getBool(r.Flags, "verbose")

		// Extract targets. The last argument is the link name when
		// there are exactly two args. With more args, the last is a
		// directory and the rest are targets.
		targets := getStringSlice(r.Arguments, "targets")

		if len(targets) < 1 {
			fmt.Fprintf(stderr, "ln: missing file operand\n")
			return 1
		}

		// Simple case: two arguments — target and link name.
		if len(targets) == 2 {
			target := targets[0]
			linkName := targets[1]
			return createLink(target, linkName, symbolic, force, verbose, stdout, stderr)
		}

		// Single argument: create link in current directory with same name.
		if len(targets) == 1 {
			target := targets[0]
			linkName := filepath.Base(target)
			return createLink(target, linkName, symbolic, force, verbose, stdout, stderr)
		}

		// Multiple targets: last arg must be a directory.
		dir := targets[len(targets)-1]
		info, err := os.Stat(dir)
		if err != nil || !info.IsDir() {
			fmt.Fprintf(stderr, "ln: target '%s' is not a directory\n", dir)
			return 1
		}

		exitCode := 0
		for _, target := range targets[:len(targets)-1] {
			linkName := filepath.Join(dir, filepath.Base(target))
			code := createLink(target, linkName, symbolic, force, verbose, stdout, stderr)
			if code != 0 {
				exitCode = code
			}
		}
		return exitCode

	default:
		fmt.Fprintf(stderr, "ln: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// createLink — create a single hard or symbolic link
// =========================================================================
//
// This function handles the actual link creation, including force-removal
// of existing files and verbose output.

func createLink(target, linkName string, symbolic, force, verbose bool, stdout, stderr io.Writer) int {
	// If force mode, remove existing link/file first.
	if force {
		os.Remove(linkName) // ignore error — file may not exist
	}

	var err error
	if symbolic {
		// os.Symlink creates a symbolic link. The target is stored
		// as-is in the symlink — it's not resolved at creation time.
		err = os.Symlink(target, linkName)
	} else {
		// os.Link creates a hard link. Both names must be on the
		// same filesystem, and the target must exist.
		err = os.Link(target, linkName)
	}

	if err != nil {
		fmt.Fprintf(stderr, "ln: failed to create link '%s': %s\n", linkName, err)
		return 1
	}

	if verbose {
		if symbolic {
			fmt.Fprintf(stdout, "'%s' -> '%s'\n", linkName, target)
		} else {
			fmt.Fprintf(stdout, "'%s' => '%s'\n", linkName, target)
		}
	}

	return 0
}

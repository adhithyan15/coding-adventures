// =========================================================================
// pwd — Print Working Directory (business logic)
// =========================================================================
//
// This file contains the business logic for the `pwd` tool, extracted from
// the original main.go. The pwd command prints the absolute pathname of
// the current working directory.
//
// # How POSIX pwd works
//
// The `pwd` command has two modes:
//
//   -L (logical)   Print the value of $PWD, which preserves the path the
//                  user navigated through — including symlinks. This is
//                  the default behavior.
//
//   -P (physical)  Print the actual filesystem path with all symlinks
//                  resolved. This is what you'd get if you followed every
//                  directory component to its real inode.
//
// # Why the distinction matters
//
// Consider:
//
//	/home/user/projects -> /mnt/ssd/projects  (symlink)
//
// If you `cd /home/user/projects`, then:
//
//	pwd -L  =>  /home/user/projects   (the logical path you typed)
//	pwd -P  =>  /mnt/ssd/projects     (the real filesystem path)
//
// The logical path is friendlier (it matches what the user typed); the
// physical path is authoritative (it's where the bytes actually live).

package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Logical vs. physical path helpers
// =========================================================================
//
// These two functions encapsulate the core business logic of pwd.

// getLogicalPath returns the logical current working directory.
//
// The logical path comes from the $PWD environment variable, which the
// shell maintains as the user navigates. It preserves symlinks and is
// the "friendly" path the user expects to see.
//
// However, $PWD can be stale, missing, or tampered with. We validate it
// by checking that it points to the same directory as os.Getwd(). If
// validation fails, we fall back to os.Getwd().
//
// Why validate? Consider:
//
//	export PWD=/tmp          # user manually sets $PWD
//	cd /home/user
//	pwd -L                   # should NOT print /tmp!
//
// The POSIX spec says: "If the PWD environment variable is an absolute
// pathname that does not contain the filenames dot or dot-dot, and if
// it refers to the same directory as the current working directory, it
// shall be considered to be the current working directory."
func getLogicalPath() (string, error) {
	// Step 1: Try $PWD from the environment.
	pwd := os.Getenv("PWD")

	// Step 2: If $PWD is empty, fall back to os.Getwd() immediately.
	// This happens in environments where $PWD is not set (some cron jobs,
	// Docker containers, etc.).
	if pwd == "" {
		return os.Getwd()
	}

	// Step 3: Validate that $PWD actually points to the current directory.
	// We do this by resolving both paths to their physical locations and
	// comparing. If they match, the logical $PWD is trustworthy.
	pwdReal, err := filepath.EvalSymlinks(pwd)
	if err != nil {
		// $PWD points to a path that doesn't exist or can't be resolved.
		// Fall back to os.Getwd().
		return os.Getwd()
	}

	cwd, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("cannot determine current directory: %w", err)
	}

	cwdReal, err := filepath.EvalSymlinks(cwd)
	if err != nil {
		// If we can't resolve the real cwd, just use cwd as-is.
		cwdReal = cwd
	}

	// Step 4: Compare the resolved paths. If they point to the same place,
	// $PWD is valid — return the logical (possibly symlinked) path.
	if pwdReal == cwdReal {
		return pwd, nil
	}

	// $PWD is stale or wrong. Fall back to os.Getwd().
	return cwd, nil
}

// getPhysicalPath returns the physical current working directory with all
// symlinks resolved.
//
// We use filepath.EvalSymlinks on the result of os.Getwd() to resolve any
// remaining symlinks. os.Getwd() itself usually returns a physical path on
// most systems, but filepath.EvalSymlinks provides the guarantee.
func getPhysicalPath() (string, error) {
	cwd, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("cannot determine current directory: %w", err)
	}

	// EvalSymlinks resolves every symlink component in the path.
	// For example: /home/user/link -> /mnt/real becomes /mnt/real.
	resolved, err := filepath.EvalSymlinks(cwd)
	if err != nil {
		// If symlink resolution fails (rare), fall back to the raw cwd.
		return cwd, nil
	}

	return resolved, nil
}

// =========================================================================
// runPwd — the testable core of the pwd tool
// =========================================================================
//
// This function handles pwd's argument parsing and business logic. It:
//
//   1. Creates a parser from the spec file and argv.
//   2. Parses the arguments.
//   3. Type-switches on the result to handle each case.
//   4. Returns the exit code (0 for success, 1 for errors).

func runPwd(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "pwd: %s\n", err)
		return 1
	}

	// Step 2: Parse argv against the spec.
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
		// Check which mode the user requested.
		// "logical" and "physical" are mutually exclusive (declared in
		// pwd.json), so cli-builder guarantees at most one is true.
		physical, _ := r.Flags["physical"].(bool)

		var path string
		if physical {
			path, err = getPhysicalPath()
		} else {
			path, err = getLogicalPath()
		}

		if err != nil {
			fmt.Fprintf(stderr, "pwd: %s\n", err)
			return 1
		}

		fmt.Fprintln(stdout, path)
		return 0

	default:
		fmt.Fprintf(stderr, "pwd: unexpected result type: %T\n", result)
		return 1
	}
}

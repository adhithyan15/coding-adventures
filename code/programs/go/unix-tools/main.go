// =========================================================================
// pwd — Print Working Directory
// =========================================================================
//
// This program reimplements the POSIX `pwd` utility using the cli-builder
// package from this repository. It demonstrates the simplest possible CLI
// tool built on cli-builder: the entire interface (flags, help text, version
// output, error messages) is declared in `pwd.json`, and this file contains
// only the business logic — reading and printing the current directory.
//
// # How POSIX pwd works
//
// The `pwd` command prints the absolute pathname of the current working
// directory. It has two modes:
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
//
// # Architecture
//
//	pwd.json (spec)          main.go (this file)
//	┌──────────────────┐     ┌──────────────────────────────┐
//	│ flags: -L, -P    │     │ if physical:                 │
//	│ mutual exclusion │────>│     print(resolve_symlinks)  │
//	│ help, version    │     │ else:                        │
//	│ error messages   │     │     print($PWD or fallback)  │
//	└──────────────────┘     └──────────────────────────────┘
//	    CLI Builder               Your code
//	  handles all this          handles only this

package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec file resolution
// =========================================================================
//
// The pwd.json spec file lives alongside the compiled binary. We resolve
// its path relative to the executable, NOT relative to the current working
// directory. This is critical because:
//
//   1. The user might invoke `pwd` from any directory on the system.
//   2. If we resolved relative to cwd, we'd look for pwd.json in the
//      user's current directory — which almost certainly doesn't have it.
//   3. os.Executable() gives us the path to the running binary, and the
//      spec file is always deployed alongside it.
//
// This pattern is standard for tools that carry configuration next to
// their binary (like how a .app bundle works on macOS).

func resolveSpecPath() (string, error) {
	// os.Executable() returns the path to the currently running binary.
	// On macOS/Linux this resolves symlinks to the actual binary location.
	execPath, err := os.Executable()
	if err != nil {
		return "", fmt.Errorf("cannot determine executable path: %w", err)
	}

	// filepath.Dir() strips the filename, leaving just the directory.
	// filepath.Join() then appends "pwd.json" with the correct separator.
	execDir := filepath.Dir(execPath)
	return filepath.Join(execDir, "pwd.json"), nil
}

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
// run — the testable core of the program
// =========================================================================
//
// We separate "run" from "main" so that the core logic can be tested
// without invoking os.Exit(). The run function:
//
//   1. Creates a parser from the spec file and argv.
//   2. Parses the arguments.
//   3. Type-switches on the result to handle each case.
//   4. Returns the exit code (0 for success, 1 for errors).
//
// All output goes to the provided io.Writer (stdout in production, a
// buffer in tests). Errors go to the provided stderr writer.
//
// This pattern — extracting a testable `run` function that takes writers
// and returns an exit code — is idiomatic Go for CLI tools. It lets us
// test every code path without subprocess execution.

func run(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser.
	//
	// NewParser reads and validates the JSON spec. If the spec file is
	// missing or malformed, it returns an error immediately — before we
	// try to parse any arguments. This "fail fast" behavior means we get
	// clear error messages about spec problems rather than confusing
	// parse failures later.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "pwd: %s\n", err)
		return 1
	}

	// Step 2: Parse argv against the spec.
	//
	// Parse() returns one of three result types:
	//   - *ParseResult:   normal operation — flags and arguments are ready
	//   - *HelpResult:    user passed --help; contains rendered help text
	//   - *VersionResult: user passed --version; contains version string
	//
	// Or it returns a *ParseErrors containing all validation failures.
	result, err := parser.Parse()
	if err != nil {
		// ParseErrors implements the error interface, so we can print it
		// directly. It formats all errors nicely.
		fmt.Fprintf(stderr, "%s\n", err)
		return 1
	}

	// Step 3: Handle the result.
	//
	// Go's type switch is perfect for this pattern. Each arm handles one
	// result type. The compiler warns us if we forget a case (when using
	// an interface with a sealed set of implementations).
	switch r := result.(type) {

	case *clibuilder.HelpResult:
		// The user passed --help or -h. Print the generated help text
		// and exit successfully. The help text is fully formatted by
		// cli-builder — we just print it.
		fmt.Fprintln(stdout, r.Text)
		return 0

	case *clibuilder.VersionResult:
		// The user passed --version. Print the version from the spec.
		fmt.Fprintln(stdout, r.Version)
		return 0

	case *clibuilder.ParseResult:
		// Normal operation. Check which mode the user requested.
		//
		// The "physical" flag is a boolean. If true, the user passed -P
		// or --physical. If false (the default), we use logical mode.
		//
		// Note: "logical" and "physical" are mutually exclusive (declared
		// in pwd.json), so cli-builder guarantees at most one is true.
		// We check "physical" because logical is the default behavior.
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
		// This should never happen — cli-builder always returns one of
		// the three types above. If it does, something is very wrong.
		fmt.Fprintf(stderr, "pwd: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// Main — the entry point
// =========================================================================
//
// The main function is a thin wrapper around run(). It resolves the spec
// file path and delegates everything else. This separation keeps main()
// minimal and untestable code to a bare minimum.

func main() {
	// Find pwd.json next to the binary.
	specPath, err := resolveSpecPath()
	if err != nil {
		fmt.Fprintf(os.Stderr, "pwd: %s\n", err)
		os.Exit(1)
	}

	os.Exit(run(specPath, os.Args, os.Stdout, os.Stderr))
}

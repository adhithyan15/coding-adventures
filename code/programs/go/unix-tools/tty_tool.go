// =========================================================================
// tty — Print Terminal Device Name
// =========================================================================
//
// The `tty` utility prints the file name of the terminal connected to
// standard input. This is useful for scripts that need to know whether
// they're running interactively.
//
// # Basic behavior
//
//   $ tty
//   /dev/ttys003
//
//   $ echo hello | tty
//   not a tty
//
// # Why does tty exist?
//
// Scripts often need to behave differently when running interactively
// vs. in a pipeline. For example, a script might prompt the user for
// input only if stdin is a terminal:
//
//   if tty -s; then
//     read -p "Enter password: " password
//   else
//     password=$(cat)   # read from pipe
//   fi
//
// # Exit codes
//
//   0  if stdin is a terminal (tty)
//   1  if stdin is NOT a terminal
//
// # Flags
//
//   -s / --silent    Print nothing, only return the exit status.
//                    Useful in shell conditionals: if tty -s; then ...
//
// # How terminal detection works
//
// On Unix, we check if stdin (file descriptor 0) is a terminal by
// calling os.Stdin.Fd() and checking with golang.org/x/term or
// by using os.File.Stat() to check the file mode.
//
// # Architecture
//
//   tty.json (spec)           tty_tool.go (this file)
//   ┌──────────────────┐     ┌──────────────────────────────┐
//   │ flag: -s/--silent │     │ check if stdin is a terminal  │
//   │ no arguments      │────>│ if yes: print device name     │
//   │ help, version     │     │ if no: print "not a tty"      │
//   └──────────────────┘     └──────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Terminal detection interface
// =========================================================================
//
// We define an interface for terminal detection so that tests can inject
// a mock. In production, we use the real stdin file descriptor.

// ttyChecker provides an interface for checking if stdin is a terminal
// and getting the terminal device name. This abstraction allows testing
// without needing an actual terminal.
type ttyChecker interface {
	// IsTTY returns true if stdin is connected to a terminal.
	IsTTY() bool
	// DeviceName returns the terminal device path (e.g., "/dev/ttys003").
	DeviceName() string
}

// =========================================================================
// realTTYChecker — production implementation
// =========================================================================
//
// Uses os.Stdin.Stat() to check if stdin is a character device (terminal).
// On Unix, terminals are character special files.

type realTTYChecker struct{}

func (r *realTTYChecker) IsTTY() bool {
	fi, err := os.Stdin.Stat()
	if err != nil {
		return false
	}
	// A terminal is a character device (ModeCharDevice).
	// Pipes and regular files have ModeNamedPipe or no special mode.
	return fi.Mode()&os.ModeCharDevice != 0
}

func (r *realTTYChecker) DeviceName() string {
	// On macOS/Linux, /dev/fd/0 is a symlink to the actual terminal device.
	// We can read the symlink to get the device name.
	target, err := os.Readlink("/dev/fd/0")
	if err == nil && target != "" {
		return target
	}

	// Fallback: try /proc/self/fd/0 (Linux)
	target, err = os.Readlink("/proc/self/fd/0")
	if err == nil && target != "" {
		return target
	}

	// Last resort: we know it's a tty but can't determine the name.
	return "/dev/tty"
}

// defaultTTYChecker is the production tty checker.
var defaultTTYChecker ttyChecker = &realTTYChecker{}

// =========================================================================
// ttyLogic — the testable core logic
// =========================================================================
//
// This function contains the business logic for tty, separated from
// argument parsing for testability. It takes a ttyChecker and a silent
// flag, and returns the exit code.

func ttyLogic(checker ttyChecker, silent bool, stdout io.Writer, stderr io.Writer) int {
	if checker.IsTTY() {
		if !silent {
			fmt.Fprintln(stdout, checker.DeviceName())
		}
		return 0
	}

	// Not a terminal.
	if !silent {
		fmt.Fprintln(stdout, "not a tty")
	}
	return 1
}

// =========================================================================
// runTty — the testable entry point for the tty tool
// =========================================================================
//
// The tty tool:
//   1. Parses arguments using cli-builder.
//   2. Checks if stdin is a terminal.
//   3. If yes: prints the device name (unless -s), exits 0.
//   4. If no: prints "not a tty" (unless -s), exits 1.

func runTty(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "tty: %s\n", err)
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
		silent, _ := r.Flags["silent"].(bool)
		return ttyLogic(defaultTTYChecker, silent, stdout, stderr)

	default:
		fmt.Fprintf(stderr, "tty: unexpected result type: %T\n", result)
		return 1
	}
}

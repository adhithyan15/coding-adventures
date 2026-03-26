// =========================================================================
// env — Run a Program in a Modified Environment
// =========================================================================
//
// The `env` utility prints the current environment variables or runs a
// command with a modified environment. It's essential for scripts that
// need to control the exact environment a program sees.
//
// # Basic usage
//
//   env                         Print all environment variables
//   env FOO=bar command         Run command with FOO set to bar
//   env -i command              Run command with empty environment
//   env -u PATH command         Run command without PATH
//
// # How env works
//
// Environment variables are key=value pairs inherited by child processes.
// Every process has an environment (accessible via os.Environ() in Go).
//
// env modifies this inheritance:
//   1. Start with current environment (or empty if -i)
//   2. Remove variables specified by -u
//   3. Set variables specified as NAME=VALUE arguments
//   4. If a command is given, exec it with the modified environment
//   5. If no command, print the resulting environment
//
// # Architecture
//
//   env.json (spec)              env_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -i,-u,-0  │       │ build environment map            │
//   │ -C               │──────>│ apply modifications              │
//   │ arg: NAME=VALUE  │       │ exec command or print env        │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// EnvOptions — configuration for the env operation
// =========================================================================

type EnvOptions struct {
	IgnoreEnvironment bool     // -i: start with empty environment
	Unset             []string // -u: variables to remove
	NullTerminated    bool     // -0: use NUL instead of newline
	Chdir             string   // -C: change directory before exec
}

// =========================================================================
// envBuildEnviron — construct the modified environment
// =========================================================================
//
// Starting from the base environment (os.Environ() or empty), applies
// the requested modifications:
//   1. If -i, start empty
//   2. Remove any -u variables
//   3. Add NAME=VALUE assignments
//
// The result is a []string suitable for cmd.Env.

func envBuildEnviron(assignments []string, opts EnvOptions) []string {
	var env []string

	if !opts.IgnoreEnvironment {
		env = os.Environ()
	}

	// Remove unset variables.
	// We filter out any entry whose key matches an -u name.
	for _, name := range opts.Unset {
		prefix := name + "="
		filtered := make([]string, 0, len(env))
		for _, e := range env {
			if !strings.HasPrefix(e, prefix) {
				filtered = append(filtered, e)
			}
		}
		env = filtered
	}

	// Apply NAME=VALUE assignments.
	// If a variable already exists, we replace it. Otherwise, we append.
	for _, assignment := range assignments {
		eqIdx := strings.Index(assignment, "=")
		if eqIdx < 0 {
			continue
		}
		key := assignment[:eqIdx]
		prefix := key + "="

		// Remove any existing entry for this key.
		found := false
		for i, e := range env {
			if strings.HasPrefix(e, prefix) {
				env[i] = assignment
				found = true
				break
			}
		}
		if !found {
			env = append(env, assignment)
		}
	}

	return env
}

// =========================================================================
// envExecFunc — function type for executing commands
// =========================================================================
//
// We use a function variable so tests can inject a mock executor.

type envExecFunc func(env []string, dir string, name string, args []string,
	stdout, stderr io.Writer) int

// envRealExec executes a real system command with the given environment.
func envRealExec(env []string, dir string, name string, args []string,
	stdout, stderr io.Writer) int {
	cmd := exec.Command(name, args...)
	cmd.Env = env
	cmd.Stdout = stdout
	cmd.Stderr = stderr
	if dir != "" {
		cmd.Dir = dir
	}
	err := cmd.Run()
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			return exitErr.ExitCode()
		}
		fmt.Fprintf(stderr, "env: %s: %s\n", name, err)
		return 127
	}
	return 0
}

// =========================================================================
// runEnv — the testable core of the env tool
// =========================================================================

func runEnv(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runEnvWithExec(specPath, argv, stdout, stderr, envRealExec)
}

func runEnvWithExec(specPath string, argv []string, stdout io.Writer, stderr io.Writer,
	execFn envExecFunc) int {

	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "env: %s\n", err)
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
		opts := EnvOptions{
			IgnoreEnvironment: getBool(r.Flags, "ignore_environment"),
			NullTerminated:    getBool(r.Flags, "null"),
		}

		if dir, ok := r.Flags["chdir"].(string); ok {
			opts.Chdir = dir
		}

		// Get unset variable names.
		opts.Unset = getStringSlice(r.Flags, "unset")

		// Get the positional arguments: NAME=VALUE pairs and/or command.
		allArgs := getStringSlice(r.Arguments, "assignments_and_command")

		// Separate NAME=VALUE assignments from the command.
		// The first argument that doesn't contain '=' starts the command.
		var assignments []string
		var cmdArgs []string
		cmdStarted := false

		for _, arg := range allArgs {
			if cmdStarted {
				cmdArgs = append(cmdArgs, arg)
			} else if strings.Contains(arg, "=") {
				assignments = append(assignments, arg)
			} else {
				cmdStarted = true
				cmdArgs = append(cmdArgs, arg)
			}
		}

		// Build the modified environment.
		env := envBuildEnviron(assignments, opts)

		// If no command, print the environment.
		if len(cmdArgs) == 0 {
			terminator := "\n"
			if opts.NullTerminated {
				terminator = "\x00"
			}
			for _, e := range env {
				fmt.Fprintf(stdout, "%s%s", e, terminator)
			}
			return 0
		}

		// Execute the command with the modified environment.
		return execFn(env, opts.Chdir, cmdArgs[0], cmdArgs[1:], stdout, stderr)

	default:
		fmt.Fprintf(stderr, "env: unexpected result type: %T\n", result)
		return 1
	}
}

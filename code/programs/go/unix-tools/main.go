// =========================================================================
// unix-tools — A Collection of Unix Utilities
// =========================================================================
//
// This program bundles several classic Unix utilities into a single binary.
// Each tool is implemented in its own file (pwd_tool.go, true_tool.go, etc.)
// and this file serves as the dispatcher — routing execution to the correct
// tool based on how the program is invoked.
//
// # How dispatching works
//
// Unix has a tradition of "multi-call binaries" — single executables that
// behave differently depending on the name used to invoke them. BusyBox is
// the most famous example: it contains hundreds of utilities in one binary,
// and each is accessed via a symlink with the tool's name.
//
// Our program supports two invocation styles:
//
//   1. Symlink mode (like BusyBox):
//      Create a symlink: ln -s unix-tools pwd
//      Then invoke: ./pwd -P
//      The program checks argv[0] ("pwd") and dispatches accordingly.
//
//   2. Subcommand mode:
//      Invoke directly: ./unix-tools pwd -P
//      The program checks argv[1] ("pwd") and dispatches accordingly.
//
// # Architecture
//
//   main.go (this file)          tool files
//   ┌─────────────────────┐     ┌──────────────────┐
//   │ argv[0] or argv[1]  │────>│ pwd_tool.go      │
//   │ determines which    │     │ true_tool.go     │
//   │ tool to run         │     │ false_tool.go    │
//   │                     │     │ echo_tool.go     │
//   │ resolveSpecPath()   │     │ cat_tool.go      │
//   │ finds JSON specs    │     │ wc_tool.go       │
//   └─────────────────────┘     └──────────────────┘
//
// # Adding a new tool
//
// To add a new tool:
//   1. Create a JSON spec file (e.g., head.json)
//   2. Create a Go source file with runHead() (e.g., head_tool.go)
//   3. Add "head" to the toolNames list and dispatch map below
//   4. Create tests in head_test.go

package main

import (
	"fmt"
	"os"
	"path/filepath"
)

// =========================================================================
// toolNames — the list of all supported tools
// =========================================================================
//
// This list serves two purposes:
//   1. Dispatching: we check if argv[0] or argv[1] matches a tool name
//   2. Help text: we can list all available tools when invoked incorrectly

var toolNames = []string{
	// Tier 0
	"pwd",
	// Tier 1
	"true",
	"false",
	"yes",
	"whoami",
	"logname",
	"tty",
	"nproc",
	"sleep",
	// Tier 2
	"echo",
	"cat",
	"wc",
	"head",
	"tail",
	"basename",
	"dirname",
	"seq",
	"tee",
	"rev",
	"printenv",
	// Tier 3
	"mkdir",
	"rmdir",
	"touch",
	"ln",
	"rm",
	"realpath",
	"tr",
	"uniq",
	"expand",
	"unexpand",
	"fold",
	"nl",
}

// =========================================================================
// Spec file resolution
// =========================================================================
//
// Each tool has a JSON spec file that lives alongside the compiled binary.
// We resolve spec paths relative to the executable, NOT relative to the
// current working directory. This is critical because:
//
//   1. The user might invoke the tool from any directory on the system.
//   2. If we resolved relative to cwd, we'd look for pwd.json in the
//      user's current directory — which almost certainly doesn't have it.
//   3. os.Executable() gives us the path to the running binary, and the
//      spec files are always deployed alongside it.

func resolveSpecPath(toolName string) (string, error) {
	// os.Executable() returns the path to the currently running binary.
	execPath, err := os.Executable()
	if err != nil {
		return "", fmt.Errorf("cannot determine executable path: %w", err)
	}

	// filepath.Dir() strips the filename, leaving just the directory.
	// filepath.Join() then appends "<tool>.json" with the correct separator.
	execDir := filepath.Dir(execPath)
	return filepath.Join(execDir, toolName+".json"), nil
}

// =========================================================================
// dispatch — route to the correct tool based on name
// =========================================================================
//
// Given a tool name and arguments, this function resolves the spec path
// and calls the appropriate runXxx function.

func dispatch(toolName string, argv []string) int {
	specPath, err := resolveSpecPath(toolName)
	if err != nil {
		fmt.Fprintf(os.Stderr, "%s: %s\n", toolName, err)
		return 1
	}

	switch toolName {
	case "pwd":
		return runPwd(specPath, argv, os.Stdout, os.Stderr)
	case "true":
		return runTrue(specPath, argv, os.Stdout, os.Stderr)
	case "false":
		return runFalse(specPath, argv, os.Stdout, os.Stderr)
	case "echo":
		return runEcho(specPath, argv, os.Stdout, os.Stderr)
	case "cat":
		return runCat(specPath, argv, os.Stdout, os.Stderr)
	case "wc":
		return runWc(specPath, argv, os.Stdout, os.Stderr)
	case "head":
		return runHead(specPath, argv, os.Stdout, os.Stderr)
	case "tail":
		return runTail(specPath, argv, os.Stdout, os.Stderr)
	case "basename":
		return runBasename(specPath, argv, os.Stdout, os.Stderr)
	case "dirname":
		return runDirname(specPath, argv, os.Stdout, os.Stderr)
	case "seq":
		return runSeq(specPath, argv, os.Stdout, os.Stderr)
	case "tee":
		return runTee(specPath, argv, os.Stdout, os.Stderr)
	case "rev":
		return runRev(specPath, argv, os.Stdout, os.Stderr)
	case "printenv":
		return runPrintenv(specPath, argv, os.Stdout, os.Stderr)
	// Tier 1
	case "yes":
		return runYes(specPath, argv, os.Stdout, os.Stderr)
	case "whoami":
		return runWhoami(specPath, argv, os.Stdout, os.Stderr)
	case "logname":
		return runLogname(specPath, argv, os.Stdout, os.Stderr)
	case "tty":
		return runTty(specPath, argv, os.Stdout, os.Stderr)
	case "nproc":
		return runNproc(specPath, argv, os.Stdout, os.Stderr)
	case "sleep":
		return runSleep(specPath, argv, os.Stdout, os.Stderr)
	// Tier 3
	case "mkdir":
		return runMkdir(specPath, argv, os.Stdout, os.Stderr)
	case "rmdir":
		return runRmdir(specPath, argv, os.Stdout, os.Stderr)
	case "touch":
		return runTouch(specPath, argv, os.Stdout, os.Stderr)
	case "ln":
		return runLn(specPath, argv, os.Stdout, os.Stderr)
	case "rm":
		return runRm(specPath, argv, os.Stdout, os.Stderr)
	case "realpath":
		return runRealpath(specPath, argv, os.Stdout, os.Stderr)
	case "tr":
		return runTr(specPath, argv, os.Stdout, os.Stderr)
	case "uniq":
		return runUniq(specPath, argv, os.Stdout, os.Stderr)
	case "expand":
		return runExpand(specPath, argv, os.Stdout, os.Stderr)
	case "unexpand":
		return runUnexpand(specPath, argv, os.Stdout, os.Stderr)
	case "fold":
		return runFold(specPath, argv, os.Stdout, os.Stderr)
	case "nl":
		return runNl(specPath, argv, os.Stdout, os.Stderr)
	default:
		fmt.Fprintf(os.Stderr, "unix-tools: unknown tool: %s\n", toolName)
		fmt.Fprintf(os.Stderr, "Available tools: %v\n", toolNames)
		return 1
	}
}

// =========================================================================
// isKnownTool — check if a name matches a known tool
// =========================================================================

func isKnownTool(name string) bool {
	for _, t := range toolNames {
		if name == t {
			return true
		}
	}
	return false
}

// =========================================================================
// Main — the entry point
// =========================================================================
//
// The main function determines which tool to run and delegates to dispatch().
//
// Invocation detection:
//   1. Extract the base name of argv[0] (e.g., "/usr/bin/pwd" -> "pwd")
//   2. If it matches a known tool, run that tool with the given args
//   3. Otherwise, treat argv[1] as the tool name (subcommand mode)
//   4. If neither works, print usage and exit with error

func main() {
	if len(os.Args) < 1 {
		fmt.Fprintln(os.Stderr, "unix-tools: no arguments")
		os.Exit(1)
	}

	// Check if argv[0] matches a known tool (symlink mode).
	// filepath.Base extracts just the filename: "/usr/local/bin/pwd" -> "pwd"
	baseName := filepath.Base(os.Args[0])

	if isKnownTool(baseName) {
		// Symlink mode: the binary was invoked as "pwd", "cat", etc.
		os.Exit(dispatch(baseName, os.Args))
	}

	// Subcommand mode: check if argv[1] is a tool name.
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "Usage: unix-tools <tool> [args...]")
		fmt.Fprintf(os.Stderr, "Available tools: %v\n", toolNames)
		os.Exit(1)
	}

	toolName := os.Args[1]
	if isKnownTool(toolName) {
		// Rewrite argv so the tool sees itself as argv[0].
		// For example: ["unix-tools", "pwd", "-P"] -> ["pwd", "-P"]
		toolArgv := append([]string{toolName}, os.Args[2:]...)
		os.Exit(dispatch(toolName, toolArgv))
	}

	fmt.Fprintf(os.Stderr, "unix-tools: unknown tool: %s\n", toolName)
	fmt.Fprintf(os.Stderr, "Available tools: %v\n", toolNames)
	os.Exit(1)
}

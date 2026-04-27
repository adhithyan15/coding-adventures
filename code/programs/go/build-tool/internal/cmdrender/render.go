// render.go -- Convert structured command dicts to shell strings.
//
// ============================================================================
// OVERVIEW
// ============================================================================
//
// When BUILD rules use cmd() from cmd.star, they produce structured command
// dicts like:
//
//   {"type": "cmd", "program": "python", "args": ["-m", "pytest", "--cov"]}
//
// The build tool needs to turn these into shell strings for execution:
//
//   python -m pytest --cov
//
// This package handles that conversion.  It's deliberately simple because
// all the platform-specific logic already happened in Starlark (cmd_windows,
// cmd_linux, etc.).  By the time commands reach the renderer, they're just
// program + args — no OS branching needed.
//
// ============================================================================
// QUOTING RULES
// ============================================================================
//
// Shell quoting is notoriously tricky.  We follow a conservative approach:
//
//   1. If an argument contains spaces, quotes, or shell metacharacters,
//      wrap it in double quotes with internal quotes escaped.
//   2. Empty strings become "".
//   3. Everything else is passed through as-is.
//
// This works for both sh -c (Unix) and cmd /C (Windows) because the
// executor layer (executor.go) already handles the sh vs cmd dispatch.
// We just need the arguments properly quoted within the command string.
//
// Characters that trigger quoting:
//   space, tab, ", ', $, `, \, |, &, ;, (, ), <, >, !, #, *, ?, [, ], {, }
//
// ============================================================================
// ARCHITECTURE
// ============================================================================
//
//   cmd.star (Starlark)           cmdrender (Go)           executor (Go)
//   ──────────────────           ──────────────           ──────────────
//   cmd("python", ["-m"]) ──►  RenderCommand() ──►  sh -c "python -m ..."
//   cmd_linux(...) → None       (skips None)         cmd /C "..." on Windows
//   filter_commands(...)
//
package cmdrender

import (
	"fmt"
	"strings"
)

// shellMetacharacters lists characters that require quoting in shell strings.
// If any argument contains one of these, it gets wrapped in double quotes.
const shellMetacharacters = " \t\"'$`\\|&;()<>!#*?[]{}"

// ============================================================================
// RenderCommand -- Convert a single command dict to a shell string
// ============================================================================
//
// A command dict has the shape:
//
//   {"type": "cmd", "program": "python", "args": ["-m", "pytest"]}
//
// RenderCommand extracts "program" and "args", quotes them as needed,
// and joins them into a single shell-safe string:
//
//   "python -m pytest"
//
// If the dict is missing "program", returns an error.
// If "args" is missing or empty, returns just the program name.

func RenderCommand(cmdDict map[string]interface{}) (string, error) {
	// Extract the program name.
	programRaw, ok := cmdDict["program"]
	if !ok {
		return "", fmt.Errorf("command dict missing 'program' key: %v", cmdDict)
	}
	program, ok := programRaw.(string)
	if !ok {
		return "", fmt.Errorf("command 'program' must be a string, got %T: %v", programRaw, programRaw)
	}

	// Start building the command string with the (possibly quoted) program.
	parts := []string{quoteArg(program)}

	// Extract and quote each argument.
	if argsRaw, ok := cmdDict["args"]; ok && argsRaw != nil {
		args, ok := argsRaw.([]interface{})
		if !ok {
			return "", fmt.Errorf("command 'args' must be a list, got %T: %v", argsRaw, argsRaw)
		}
		for _, argRaw := range args {
			arg := fmt.Sprintf("%v", argRaw)
			parts = append(parts, quoteArg(arg))
		}
	}

	return strings.Join(parts, " "), nil
}

// ============================================================================
// RenderCommands -- Convert a list of command dicts to shell strings
// ============================================================================
//
// Processes a list of command dicts (as returned by Starlark's _targets),
// skipping nil entries (commands filtered out by filter_commands()).
//
// Returns:
//   - A slice of shell command strings.
//   - An error if any command dict is malformed.

func RenderCommands(cmds []interface{}) ([]string, error) {
	var result []string
	for i, cmdRaw := range cmds {
		// Skip nil entries (platform-filtered commands).
		if cmdRaw == nil {
			continue
		}

		cmdDict, ok := cmdRaw.(map[string]interface{})
		if !ok {
			return nil, fmt.Errorf("command[%d] must be a dict, got %T: %v", i, cmdRaw, cmdRaw)
		}

		rendered, err := RenderCommand(cmdDict)
		if err != nil {
			return nil, fmt.Errorf("command[%d]: %w", i, err)
		}
		result = append(result, rendered)
	}
	return result, nil
}

// ============================================================================
// quoteArg -- Quote a single argument if it contains shell metacharacters
// ============================================================================
//
// Quoting strategy:
//
//   - Empty string → ""
//   - Contains metacharacters → "escaped string"
//     (internal double quotes become \", backslashes become \\)
//   - Otherwise → passed through unchanged
//
// Examples:
//   quoteArg("hello")     → hello
//   quoteArg("hello world") → "hello world"
//   quoteArg("")           → ""
//   quoteArg(".[dev]")     → ".[dev]"
//   quoteArg(`say "hi"`)   → "say \"hi\""

func quoteArg(arg string) string {
	if arg == "" {
		return `""`
	}

	if !needsQuoting(arg) {
		return arg
	}

	// Escape backslashes first (so we don't double-escape the quote escapes),
	// then escape double quotes.
	escaped := strings.ReplaceAll(arg, `\`, `\\`)
	escaped = strings.ReplaceAll(escaped, `"`, `\"`)
	return `"` + escaped + `"`
}

// needsQuoting returns true if the argument contains any shell metacharacter.
func needsQuoting(arg string) bool {
	return strings.ContainsAny(arg, shellMetacharacters)
}

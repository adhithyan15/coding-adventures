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
// Characters that trigger quoting:
//   space, tab, ", ', $, `, \, |, &, ;, (, ), <, >, !, #, *, ?, [, ], {, }
//
package cmdrender

import (
	"fmt"
	"strings"
)

// shellMetacharacters lists characters that require quoting in shell strings.
// If any argument contains one of these, it gets wrapped in double quotes.
const shellMetacharacters = " \t\"'$`\\|&;()<>!#*?[]{}"

// RenderCommand converts a single command dict to a shell string.
//
// A command dict has the shape:
//
//	{"type": "cmd", "program": "python", "args": ["-m", "pytest"]}
//
// Returns the shell-safe string "python -m pytest".
func RenderCommand(cmdDict map[string]interface{}) (string, error) {
	programRaw, ok := cmdDict["program"]
	if !ok {
		return "", fmt.Errorf("command dict missing 'program' key: %v", cmdDict)
	}
	program, ok := programRaw.(string)
	if !ok {
		return "", fmt.Errorf("command 'program' must be a string, got %T: %v", programRaw, programRaw)
	}

	parts := []string{quoteArg(program)}

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

// RenderCommands converts a list of command dicts to shell strings, skipping nil entries.
func RenderCommands(cmds []interface{}) ([]string, error) {
	var result []string
	for i, cmdRaw := range cmds {
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

// quoteArg wraps an argument in double quotes if it contains shell metacharacters.
func quoteArg(arg string) string {
	if arg == "" {
		return `""`
	}
	if !needsQuoting(arg) {
		return arg
	}
	escaped := strings.ReplaceAll(arg, `\`, `\\`)
	escaped = strings.ReplaceAll(escaped, `"`, `\"`)
	return `"` + escaped + `"`
}

// needsQuoting returns true if the argument contains any shell metacharacter.
func needsQuoting(arg string) bool {
	return strings.ContainsAny(arg, shellMetacharacters)
}

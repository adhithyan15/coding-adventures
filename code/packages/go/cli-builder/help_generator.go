package clibuilder

// =========================================================================
// Help Generator — renders help text from the spec.
// =========================================================================
//
// # What this file does
//
// When the user passes --help or -h, CLI Builder generates a help message
// from the spec rather than relying on hand-written strings. This ensures
// help is always accurate and in sync with the actual CLI definition.
//
// # Format (§9 of the spec)
//
//   USAGE
//     <name> [OPTIONS] [COMMAND] [ARGS...]
//
//   DESCRIPTION
//     <description>
//
//   COMMANDS
//     subcommand    Description of the subcommand.
//
//   OPTIONS
//     -s, --long-name <VALUE>    Description. [default: val]
//     -b, --boolean              Boolean flag description.
//
//   GLOBAL OPTIONS
//     -h, --help     Show this help message and exit.
//     --version      Show version and exit.
//
//   ARGUMENTS
//     <ARG>          Description. Required.
//     [ARG...]       Description. Optional, repeatable.
//
// # Formatting rules
//
//   - Required positional args: <NAME>
//   - Optional positional args: [NAME]
//   - Variadic required: <NAME>...
//   - Variadic optional: [NAME...]
//   - Non-boolean flags: -s, --long <VALUE>
//   - Boolean flags: -s, --long
//   - Default values: [default: X]

import (
	"fmt"
	"strings"
)

// HelpGenerator generates help text for a CLI spec at a given command path.
//
// Construct with NewHelpGenerator. The zero value is not usable.
type HelpGenerator struct {
	spec        map[string]any
	commandPath []string
	// The resolved command node (or the root spec for root-level help).
	node map[string]any
}

// NewHelpGenerator creates a HelpGenerator for the given spec and command path.
//
// commandPath should be the full path from root: ["program", "subcommand", ...].
// For root-level help, commandPath is just ["program"].
func NewHelpGenerator(spec map[string]any, commandPath []string) *HelpGenerator {
	hg := &HelpGenerator{
		spec:        spec,
		commandPath: commandPath,
	}

	if len(commandPath) <= 1 {
		hg.node = spec
	} else {
		node := findCommandNode(spec, commandPath[1:])
		if node == nil {
			hg.node = spec
		} else {
			hg.node = node
		}
	}

	return hg
}

// Generate renders and returns the help text as a string.
func (hg *HelpGenerator) Generate() string {
	var sb strings.Builder

	programName := stringField(hg.spec, "name")
	isRoot := len(hg.commandPath) <= 1

	// Determine description
	var description string
	if isRoot {
		description = stringField(hg.spec, "description")
	} else {
		description = stringField(hg.node, "description")
	}

	// --- USAGE ---
	sb.WriteString("USAGE\n")
	usage := hg.buildUsageLine(programName, isRoot)
	sb.WriteString(fmt.Sprintf("  %s\n", usage))
	sb.WriteString("\n")

	// --- DESCRIPTION ---
	if description != "" {
		sb.WriteString("DESCRIPTION\n")
		sb.WriteString(fmt.Sprintf("  %s\n", description))
		sb.WriteString("\n")
	}

	// --- COMMANDS ---
	commands := sliceOfMaps(hg.node["commands"])
	if len(commands) > 0 {
		sb.WriteString("COMMANDS\n")
		for _, cmd := range commands {
			name := stringField(cmd, "name")
			desc := stringField(cmd, "description")
			sb.WriteString(fmt.Sprintf("  %-16s%s\n", name, desc))
		}
		sb.WriteString("\n")
	}

	// --- OPTIONS (local flags) ---
	var localFlags []map[string]any
	if isRoot {
		localFlags = sliceOfMaps(hg.spec["flags"])
	} else {
		localFlags = sliceOfMaps(hg.node["flags"])
	}
	if len(localFlags) > 0 {
		sb.WriteString("OPTIONS\n")
		for _, f := range localFlags {
			hg.writeFlagLine(&sb, f)
		}
		sb.WriteString("\n")
	}

	// --- GLOBAL OPTIONS ---
	globalFlags := sliceOfMaps(hg.spec["global_flags"])

	// Add builtin --help flag
	helpEnabled := true
	versionEnabled := false
	if bi, ok := hg.spec["builtin_flags"].(map[string]any); ok {
		if v, ok := bi["help"].(bool); ok {
			helpEnabled = v
		}
		if v, ok := bi["version"].(bool); ok {
			versionEnabled = v
		}
	}
	if stringField(hg.spec, "version") != "" {
		versionEnabled = true
	}

	var builtins []map[string]any
	if helpEnabled {
		builtins = append(builtins, map[string]any{
			"id":          "help",
			"short":       "h",
			"long":        "help",
			"description": "Show this help message and exit.",
			"type":        "boolean",
		})
	}
	if versionEnabled {
		builtins = append(builtins, map[string]any{
			"id":          "version",
			"long":        "version",
			"description": "Show version and exit.",
			"type":        "boolean",
		})
	}

	allGlobal := append(globalFlags, builtins...)
	if len(allGlobal) > 0 {
		sb.WriteString("GLOBAL OPTIONS\n")
		for _, f := range allGlobal {
			hg.writeFlagLine(&sb, f)
		}
		sb.WriteString("\n")
	}

	// --- ARGUMENTS ---
	arguments := sliceOfMaps(hg.node["arguments"])
	if len(arguments) > 0 {
		sb.WriteString("ARGUMENTS\n")
		for _, a := range arguments {
			hg.writeArgLine(&sb, a)
		}
		sb.WriteString("\n")
	}

	return strings.TrimRight(sb.String(), "\n")
}

// buildUsageLine constructs the USAGE line for the help output.
func (hg *HelpGenerator) buildUsageLine(programName string, isRoot bool) string {
	// Build the command portion: "program" or "program subcommand"
	parts := make([]string, len(hg.commandPath))
	copy(parts, hg.commandPath)
	if len(parts) == 0 {
		parts = []string{programName}
	}
	cmdStr := strings.Join(parts, " ")

	var sections []string
	sections = append(sections, cmdStr)

	// Determine if there are any flags
	var hasLocalFlags bool
	var hasGlobalFlags bool
	if isRoot {
		hasLocalFlags = len(sliceOfMaps(hg.spec["flags"])) > 0
	} else {
		hasLocalFlags = len(sliceOfMaps(hg.node["flags"])) > 0
	}
	hasGlobalFlags = len(sliceOfMaps(hg.spec["global_flags"])) > 0

	if hasLocalFlags || hasGlobalFlags {
		sections = append(sections, "[OPTIONS]")
	}

	// Subcommands
	commands := sliceOfMaps(hg.node["commands"])
	if len(commands) > 0 {
		sections = append(sections, "[COMMAND]")
	}

	// Arguments
	arguments := sliceOfMaps(hg.node["arguments"])
	for _, a := range arguments {
		sections = append(sections, argUsageToken(a))
	}

	return strings.Join(sections, " ")
}

// argUsageToken returns the usage token for an argument, e.g. "<FILE>",
// "[FILE]", "<FILE>...", "[FILE...]".
func argUsageToken(a map[string]any) string {
	// Prefer display_name, fall back to name for backward compatibility.
	name := stringField(a, "display_name")
	if name == "" {
		name = stringField(a, "name")
	}
	required := boolField(a, "required", true)
	variadic := boolField(a, "variadic", false)

	if required && variadic {
		return fmt.Sprintf("<%s>...", name)
	}
	if !required && variadic {
		return fmt.Sprintf("[%s...]", name)
	}
	if required {
		return fmt.Sprintf("<%s>", name)
	}
	return fmt.Sprintf("[%s]", name)
}

// writeFlagLine writes one flag line to the string builder.
// Format: "  -s, --long-name <VALUE>    Description. [default: val]"
func (hg *HelpGenerator) writeFlagLine(sb *strings.Builder, f map[string]any) {
	short := stringField(f, "short")
	long := stringField(f, "long")
	sdl := stringField(f, "single_dash_long")
	flagType := stringField(f, "type")
	description := stringField(f, "description")
	valueName := stringField(f, "value_name")
	required := boolField(f, "required", false)

	// Build the flag signature
	var sigParts []string
	if short != "" {
		sigParts = append(sigParts, "-"+short)
	}
	if long != "" {
		sigParts = append(sigParts, "--"+long)
	}
	if sdl != "" {
		sigParts = append(sigParts, "-"+sdl)
	}
	sig := strings.Join(sigParts, ", ")

	// Append value placeholder for non-boolean flags
	if flagType != "boolean" && flagType != "" {
		vn := valueName
		if vn == "" {
			vn = strings.ToUpper(flagType)
		}
		sig = sig + " <" + vn + ">"
	}

	// Build description suffix
	desc := description
	if required {
		desc += " (required)"
	} else if f["default"] != nil {
		desc += fmt.Sprintf(" [default: %v]", f["default"])
	}

	sb.WriteString(fmt.Sprintf("  %-28s%s\n", sig, desc))
}

// writeArgLine writes one argument line to the string builder.
// Format: "  <ARG>      Description. Required."
func (hg *HelpGenerator) writeArgLine(sb *strings.Builder, a map[string]any) {
	token := argUsageToken(a)
	description := stringField(a, "description")
	required := boolField(a, "required", true)

	suffix := ""
	if required {
		suffix = " Required."
	} else if a["default"] != nil {
		suffix = fmt.Sprintf(" [default: %v]", a["default"])
	}

	sb.WriteString(fmt.Sprintf("  %-16s%s%s\n", token, description, suffix))
}

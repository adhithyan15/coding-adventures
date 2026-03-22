package clibuilder

// =========================================================================
// Spec Loader — reads, validates, and normalizes the JSON spec file.
// =========================================================================
//
// # What this file does
//
// LoadSpec reads a CLI Builder JSON specification from disk, validates it
// according to the eight rules in §6.4.3 of the spec, builds the flag
// dependency graph for every scope, and checks for cycles. If everything
// is valid, it returns the normalized spec as a map[string]any.
//
// # Why validate at load time?
//
// Spec errors are programmer errors, not user errors. Catching them at
// startup — before parsing any argv — gives developers immediate feedback
// when they ship a bad spec. The user should never see a spec error.
//
// # The eight validation rules (§6.4.3)
//
//  1. cli_builder_spec_version must be "1.0".
//  2. No duplicate flag id, command id, or argument id within any scope.
//  3. Every flag must have at least one of: short, long, single_dash_long.
//  4. All conflicts_with and requires IDs must exist in the same scope or global_flags.
//  5. All mutually_exclusive_groups flag_ids must reference valid flags in scope.
//  6. enum_values must be present and non-empty when type is "enum".
//  7. At most one argument per scope may be variadic.
//  8. Build G_flag for each scope; if HasCycle(), it is a spec error.

import (
	"encoding/json"
	"fmt"
	"os"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
)

// LoadSpec reads the JSON spec at specFilePath, validates it, and returns
// the normalized spec as a map[string]any.
//
// Returns a *SpecError if any validation rule is violated. The caller
// should treat a non-nil error as fatal — the library cannot parse argv
// until the spec is corrected.
func LoadSpec(specFilePath string) (map[string]any, error) {
	data, err := os.ReadFile(specFilePath)
	if err != nil {
		return nil, &SpecError{Message: fmt.Sprintf("cannot read spec file %q: %v", specFilePath, err)}
	}

	var spec map[string]any
	if err := json.Unmarshal(data, &spec); err != nil {
		return nil, &SpecError{Message: fmt.Sprintf("invalid JSON in spec file %q: %v", specFilePath, err)}
	}

	return validateSpec(spec)
}

// LoadSpecFromBytes parses and validates a JSON spec from a byte slice.
//
// This is useful in tests where the spec is embedded as a string constant
// rather than read from a file.
func LoadSpecFromBytes(data []byte) (map[string]any, error) {
	var spec map[string]any
	if err := json.Unmarshal(data, &spec); err != nil {
		return nil, &SpecError{Message: fmt.Sprintf("invalid JSON: %v", err)}
	}
	return validateSpec(spec)
}

// validateSpec runs all eight spec validation rules and returns either
// the (possibly normalized) spec or a *SpecError.
func validateSpec(spec map[string]any) (map[string]any, error) {
	// --- Rule 1: spec version ---
	version, _ := spec["cli_builder_spec_version"].(string)
	if version != "1.0" {
		return nil, &SpecError{Message: fmt.Sprintf(
			"unsupported cli_builder_spec_version %q; expected \"1.0\"", version)}
	}

	// --- Required top-level fields ---
	if name, _ := spec["name"].(string); name == "" {
		return nil, &SpecError{Message: "required field \"name\" is missing or empty"}
	}
	if desc, _ := spec["description"].(string); desc == "" {
		return nil, &SpecError{Message: "required field \"description\" is missing or empty"}
	}

	// --- Collect global_flags for cross-scope reference resolution ---
	globalFlags := sliceOfMaps(spec["global_flags"])

	// Build a set of global flag IDs so that conflicts_with / requires can
	// reference them from any command scope.
	globalFlagIDs := make(map[string]bool)
	for _, f := range globalFlags {
		if id, _ := f["id"].(string); id != "" {
			globalFlagIDs[id] = true
		}
	}

	// --- Validate the root scope ---
	if err := validateScope("root", spec, globalFlagIDs, globalFlags); err != nil {
		return nil, err
	}

	// --- Recursively validate all command scopes ---
	commands := sliceOfMaps(spec["commands"])
	if err := validateCommands(commands, globalFlagIDs, globalFlags); err != nil {
		return nil, err
	}

	return spec, nil
}

// validateCommands recursively validates a commands array and all nested commands.
func validateCommands(commands []map[string]any, globalFlagIDs map[string]bool, globalFlags []map[string]any) error {
	// Rule 2: command IDs must be unique among siblings
	cmdIDs := make(map[string]bool)
	cmdNames := make(map[string]bool)
	for _, cmd := range commands {
		id, _ := cmd["id"].(string)
		if id == "" {
			return &SpecError{Message: "a command is missing required field \"id\""}
		}
		if cmdIDs[id] {
			return &SpecError{Message: fmt.Sprintf("duplicate command id %q", id)}
		}
		cmdIDs[id] = true

		name, _ := cmd["name"].(string)
		if name == "" {
			return &SpecError{Message: fmt.Sprintf("command %q is missing required field \"name\"", id)}
		}
		if cmdNames[name] {
			return &SpecError{Message: fmt.Sprintf("duplicate command name %q", name)}
		}
		cmdNames[name] = true

		// aliases must also be unique among siblings
		for _, alias := range sliceOfStrings(cmd["aliases"]) {
			if cmdNames[alias] {
				return &SpecError{Message: fmt.Sprintf("duplicate command alias %q conflicts with name or alias", alias)}
			}
			cmdNames[alias] = true
		}

		if desc, _ := cmd["description"].(string); desc == "" {
			return &SpecError{Message: fmt.Sprintf("command %q is missing required field \"description\"", id)}
		}

		// Validate this command's scope (flags, arguments, exclusive groups)
		if err := validateScope(fmt.Sprintf("command %q", id), cmd, globalFlagIDs, globalFlags); err != nil {
			return err
		}

		// Recurse into nested commands
		nested := sliceOfMaps(cmd["commands"])
		if err := validateCommands(nested, globalFlagIDs, globalFlags); err != nil {
			return err
		}
	}
	return nil
}

// validateScope validates the flags, arguments, and mutually_exclusive_groups
// within a single scope (root or one command).
//
// scopeName is used only for error messages. spec is the scope's map.
// globalFlagIDs allows flags to reference global_flags in requires/conflicts_with.
// globalFlags is the actual slice of global flag definitions (for validating the
// global scope itself).
func validateScope(scopeName string, scope map[string]any, globalFlagIDs map[string]bool, globalFlags []map[string]any) error {
	flags := sliceOfMaps(scope["flags"])

	// Merge with global flags for ID lookup (but only in non-root scopes;
	// for the root scope, global_flags is already part of the scope).
	var allFlags []map[string]any
	if scopeName == "root" {
		allFlags = append(allFlags, flags...)
		allFlags = append(allFlags, globalFlags...)
	} else {
		allFlags = append(allFlags, flags...)
		allFlags = append(allFlags, globalFlags...)
	}

	// Build flag ID set for this scope (local + global)
	flagIDs := make(map[string]bool)
	for _, gf := range globalFlags {
		if id, _ := gf["id"].(string); id != "" {
			flagIDs[id] = true
		}
	}

	// --- Rule 2: no duplicate flag ids within this scope ---
	localFlagIDs := make(map[string]bool)
	for _, f := range flags {
		id, _ := f["id"].(string)
		if id == "" {
			return &SpecError{Message: fmt.Sprintf("in %s: a flag is missing required field \"id\"", scopeName)}
		}
		if localFlagIDs[id] {
			return &SpecError{Message: fmt.Sprintf("in %s: duplicate flag id %q", scopeName, id)}
		}
		localFlagIDs[id] = true
		flagIDs[id] = true
	}

	// --- Per-flag rules ---
	for _, f := range allFlags {
		id, _ := f["id"].(string)

		// Rule 3: at least one of short, long, single_dash_long
		short, _ := f["short"].(string)
		long, _ := f["long"].(string)
		sdl, _ := f["single_dash_long"].(string)
		if short == "" && long == "" && sdl == "" {
			return &SpecError{Message: fmt.Sprintf(
				"in %s: flag %q must have at least one of \"short\", \"long\", or \"single_dash_long\"", scopeName, id)}
		}

		// Description is required
		if desc, _ := f["description"].(string); desc == "" {
			return &SpecError{Message: fmt.Sprintf("in %s: flag %q is missing required field \"description\"", scopeName, id)}
		}

		// Type is required
		flagType, _ := f["type"].(string)
		if flagType == "" {
			return &SpecError{Message: fmt.Sprintf("in %s: flag %q is missing required field \"type\"", scopeName, id)}
		}

		// Rule 6: enum requires enum_values
		if flagType == "enum" {
			ev := sliceOfStrings(f["enum_values"])
			if len(ev) == 0 {
				return &SpecError{Message: fmt.Sprintf(
					"in %s: flag %q has type \"enum\" but \"enum_values\" is missing or empty", scopeName, id)}
			}
		}

		// Rule 4: conflicts_with and requires must reference known IDs
		for _, otherID := range sliceOfStrings(f["conflicts_with"]) {
			if !flagIDs[otherID] {
				return &SpecError{Message: fmt.Sprintf(
					"in %s: flag %q conflicts_with unknown flag id %q", scopeName, id, otherID)}
			}
		}
		for _, otherID := range sliceOfStrings(f["requires"]) {
			if !flagIDs[otherID] {
				return &SpecError{Message: fmt.Sprintf(
					"in %s: flag %q requires unknown flag id %q", scopeName, id, otherID)}
			}
		}
	}

	// --- Rule 7: at most one variadic argument per scope ---
	arguments := sliceOfMaps(scope["arguments"])
	argIDs := make(map[string]bool)
	variadicCount := 0
	for _, a := range arguments {
		id, _ := a["id"].(string)
		if id == "" {
			return &SpecError{Message: fmt.Sprintf("in %s: an argument is missing required field \"id\"", scopeName)}
		}
		if argIDs[id] {
			return &SpecError{Message: fmt.Sprintf("in %s: duplicate argument id %q", scopeName, id)}
		}
		argIDs[id] = true

		// Accept display_name (preferred) or name (backward compatibility).
		// Normalize to display_name for downstream consumers.
		if a["display_name"] == nil && a["name"] == nil {
			return &SpecError{Message: fmt.Sprintf("in %s: argument %q is missing required field \"display_name\"", scopeName, id)}
		}
		if a["display_name"] == nil {
			a["display_name"] = a["name"]
		}
		if _, ok := a["description"]; !ok {
			return &SpecError{Message: fmt.Sprintf("in %s: argument %q is missing required field \"description\"", scopeName, id)}
		}
		argType, _ := a["type"].(string)
		if argType == "" {
			return &SpecError{Message: fmt.Sprintf("in %s: argument %q is missing required field \"type\"", scopeName, id)}
		}
		if argType == "enum" {
			ev := sliceOfStrings(a["enum_values"])
			if len(ev) == 0 {
				return &SpecError{Message: fmt.Sprintf(
					"in %s: argument %q has type \"enum\" but \"enum_values\" is missing or empty", scopeName, id)}
			}
		}

		if variadic, _ := a["variadic"].(bool); variadic {
			variadicCount++
		}
	}
	if variadicCount > 1 {
		return &SpecError{Message: fmt.Sprintf("in %s: at most one argument may be variadic", scopeName)}
	}

	// --- Rule 5: mutually_exclusive_groups reference valid flag IDs ---
	groups := sliceOfMaps(scope["mutually_exclusive_groups"])
	groupIDs := make(map[string]bool)
	for _, g := range groups {
		gid, _ := g["id"].(string)
		if gid == "" {
			return &SpecError{Message: fmt.Sprintf("in %s: a mutually_exclusive_group is missing \"id\"", scopeName)}
		}
		if groupIDs[gid] {
			return &SpecError{Message: fmt.Sprintf("in %s: duplicate mutually_exclusive_group id %q", scopeName, gid)}
		}
		groupIDs[gid] = true
		for _, fid := range sliceOfStrings(g["flag_ids"]) {
			if !flagIDs[fid] {
				return &SpecError{Message: fmt.Sprintf(
					"in %s: mutually_exclusive_group %q references unknown flag id %q", scopeName, gid, fid)}
			}
		}
	}

	// --- Rule 8: build G_flag and check for cycles ---
	gFlag := directedgraph.New()
	for _, f := range allFlags {
		id, _ := f["id"].(string)
		if id != "" {
			gFlag.AddNode(id)
		}
	}
	for _, f := range allFlags {
		id, _ := f["id"].(string)
		for _, req := range sliceOfStrings(f["requires"]) {
			// Edge A → B means "A requires B"
			gFlag.AddEdge(id, req)
		}
	}
	if gFlag.HasCycle() {
		return &SpecError{Message: fmt.Sprintf(
			"in %s: circular requires dependency detected in flag dependency graph", scopeName)}
	}

	return nil
}

// =========================================================================
// Spec navigation helpers
// =========================================================================
//
// These helpers safely extract typed values from the map[string]any that
// json.Unmarshal produces. They return zero values on type mismatches,
// which is much safer than direct casts in validation code.

// sliceOfMaps extracts []map[string]any from a raw any value.
// Returns nil if v is nil or the wrong type.
func sliceOfMaps(v any) []map[string]any {
	if v == nil {
		return nil
	}
	raw, ok := v.([]any)
	if !ok {
		return nil
	}
	result := make([]map[string]any, 0, len(raw))
	for _, item := range raw {
		if m, ok := item.(map[string]any); ok {
			result = append(result, m)
		}
	}
	return result
}

// sliceOfStrings extracts []string from a raw any value.
// Returns nil if v is nil or the wrong type. Skips non-string elements.
func sliceOfStrings(v any) []string {
	if v == nil {
		return nil
	}
	raw, ok := v.([]any)
	if !ok {
		return nil
	}
	result := make([]string, 0, len(raw))
	for _, item := range raw {
		if s, ok := item.(string); ok {
			result = append(result, s)
		}
	}
	return result
}

// boolField extracts a bool from a map[string]any, returning defaultVal
// if the key is absent or the value is not a bool.
func boolField(m map[string]any, key string, defaultVal bool) bool {
	v, ok := m[key]
	if !ok {
		return defaultVal
	}
	b, ok := v.(bool)
	if !ok {
		return defaultVal
	}
	return b
}

// stringField extracts a string from a map[string]any, returning "" if absent.
func stringField(m map[string]any, key string) string {
	s, _ := m[key].(string)
	return s
}

// displayNameFallback returns the display_name of an argument map,
// falling back to name for backward compatibility.
func displayNameFallback(m map[string]any) string {
	if dn := stringField(m, "display_name"); dn != "" {
		return dn
	}
	return stringField(m, "name")
}

// getAllFlagsForScope returns the combined list of flags visible at a given
// command node: local flags + global_flags (if inherit_global_flags is true).
//
// At root level, both flags and global_flags are returned together.
func getAllFlagsForScope(spec map[string]any, commandPath []string) []map[string]any {
	globalFlags := sliceOfMaps(spec["global_flags"])

	if len(commandPath) <= 1 {
		// Root scope: root flags + global flags
		rootFlags := sliceOfMaps(spec["flags"])
		combined := make([]map[string]any, 0, len(rootFlags)+len(globalFlags))
		combined = append(combined, rootFlags...)
		combined = append(combined, globalFlags...)
		return combined
	}

	// Walk the command path to find the leaf command node
	node := findCommandNode(spec, commandPath[1:])
	if node == nil {
		// Fallback to root flags
		rootFlags := sliceOfMaps(spec["flags"])
		combined := make([]map[string]any, 0, len(rootFlags)+len(globalFlags))
		combined = append(combined, rootFlags...)
		combined = append(combined, globalFlags...)
		return combined
	}

	localFlags := sliceOfMaps(node["flags"])
	inherit := boolField(node, "inherit_global_flags", true)

	var combined []map[string]any
	combined = append(combined, localFlags...)
	if inherit {
		combined = append(combined, globalFlags...)
	}
	return combined
}

// findCommandNode walks the commands tree following the given path segments
// (which do NOT include the program name, just subcommand names).
// Returns the command map at the end of the path, or nil if not found.
func findCommandNode(spec map[string]any, path []string) map[string]any {
	if len(path) == 0 {
		return nil
	}
	commands := sliceOfMaps(spec["commands"])
	return findCommandNodeInList(commands, path)
}

// findCommandNodeInList searches a commands list for a matching path.
func findCommandNodeInList(commands []map[string]any, path []string) map[string]any {
	if len(path) == 0 {
		return nil
	}
	target := path[0]
	for _, cmd := range commands {
		name, _ := cmd["name"].(string)
		matched := name == target
		if !matched {
			for _, alias := range sliceOfStrings(cmd["aliases"]) {
				if alias == target {
					matched = true
					break
				}
			}
		}
		if matched {
			if len(path) == 1 {
				return cmd
			}
			nested := sliceOfMaps(cmd["commands"])
			return findCommandNodeInList(nested, path[1:])
		}
	}
	return nil
}

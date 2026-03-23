package clibuilder

// =========================================================================
// Flag Validator — validates parsed flags against spec constraints.
// =========================================================================
//
// # What this file does
//
// Phase 3 of parsing (§6.4.2) checks that the parsed flags satisfy all
// declared constraints:
//
//  1. conflicts_with: two flags that conflict are not both present.
//  2. requires (via G_flag transitive closure): if flag A is present,
//     all flags it transitively requires must also be present.
//  3. required flags: flags with `required: true` must be present unless
//     exempted by `required_unless`.
//  4. mutually_exclusive_groups: at most one flag from each group
//     (or exactly one if the group is required).
//
// # G_flag and transitive closure
//
// The flag dependency graph G_flag has one node per flag in scope. An edge
// A → B means "A requires B". TransitiveClosure(A) gives all flags that A
// transitively requires.
//
// The FlagValidator builds G_flag from the active flag set and uses it for
// the requires constraint check.

import (
	"fmt"
	"strings"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
)

// FlagValidator validates parsed flags against the spec constraints.
//
// Construct with NewFlagValidator. The zero value is not usable.
type FlagValidator struct {
	// Active flag definitions (local + global + builtins).
	activeFlags []map[string]any

	// Flag dependency graph: edge A→B means "A requires B".
	gFlag *directedgraph.Graph

	// Exclusive groups for this scope.
	exclusiveGroups []map[string]any

	// Index from flag ID to definition for O(1) lookup.
	flagByID map[string]map[string]any
}

// NewFlagValidator constructs a FlagValidator.
//
// activeFlags is the complete set of flag definitions in scope (local +
// global + builtins). exclusiveGroups is the `mutually_exclusive_groups`
// array for the resolved command node.
func NewFlagValidator(activeFlags []map[string]any, exclusiveGroups []map[string]any) *FlagValidator {
	fv := &FlagValidator{
		activeFlags:     activeFlags,
		exclusiveGroups: exclusiveGroups,
		flagByID:        make(map[string]map[string]any),
	}

	// Build the flag dependency graph.
	fv.gFlag = directedgraph.New()
	for _, f := range activeFlags {
		id := stringField(f, "id")
		if id != "" {
			fv.gFlag.AddNode(id)
			fv.flagByID[id] = f
		}
	}
	for _, f := range activeFlags {
		id := stringField(f, "id")
		for _, req := range sliceOfStrings(f["requires"]) {
			if fv.gFlag.HasNode(req) {
				fv.gFlag.AddEdge(id, req)
			}
		}
	}

	return fv
}

// Validate checks all flag constraints and returns any errors found.
//
// parsedFlags maps flag ID to its parsed value (from Phase 2 scanning).
// Errors are collected (not fail-fast) so the user sees all problems at once.
func (fv *FlagValidator) Validate(parsedFlags map[string]any) []ParseError {
	var errs []ParseError

	// 1. conflicts_with and requires checks for each present flag
	for flagID := range parsedFlags {
		def, ok := fv.flagByID[flagID]
		if !ok {
			continue
		}

		// Skip flags that are false (boolean absent)
		if b, isBool := parsedFlags[flagID].(bool); isBool && !b {
			continue
		}
		// Skip nil values (optional flag absent)
		if parsedFlags[flagID] == nil {
			continue
		}

		// --- conflicts_with ---
		for _, otherID := range sliceOfStrings(def["conflicts_with"]) {
			if otherVal, present := parsedFlags[otherID]; present {
				// Only conflict if the other flag is actually set (not false/nil)
				if isPresent(otherVal) {
					errs = append(errs, ParseError{
						ErrorType: ErrConflictingFlags,
						Message:   fmt.Sprintf("%s and %s cannot be used together", flagLabel(def), flagLabel(fv.flagByID[otherID])),
					})
				}
			}
		}

		// --- requires (transitive via G_flag) ---
		if fv.gFlag.HasNode(flagID) {
			closure, err := fv.gFlag.TransitiveClosure(flagID)
			if err == nil {
				for reqID := range closure {
					reqVal, present := parsedFlags[reqID]
					if !present || !isPresent(reqVal) {
						reqDef := fv.flagByID[reqID]
						errs = append(errs, ParseError{
							ErrorType: ErrMissingDependencyFlag,
							Message:   fmt.Sprintf("%s requires %s", flagLabel(def), flagLabel(reqDef)),
						})
					}
				}
			}
		}
	}

	// 2. required flags
	for _, def := range fv.activeFlags {
		id := stringField(def, "id")
		if !boolField(def, "required", false) {
			continue
		}

		val, present := parsedFlags[id]
		if present && isPresent(val) {
			continue // present and set
		}

		// Check required_unless exemption
		exempted := false
		for _, unlessID := range sliceOfStrings(def["required_unless"]) {
			if unlessVal, ok := parsedFlags[unlessID]; ok && isPresent(unlessVal) {
				exempted = true
				break
			}
		}
		if !exempted {
			errs = append(errs, ParseError{
				ErrorType: ErrMissingRequiredFlag,
				Message:   fmt.Sprintf("%s is required", flagLabel(def)),
			})
		}
	}

	// 3. mutually_exclusive_groups
	for _, group := range fv.exclusiveGroups {
		flagIDs := sliceOfStrings(group["flag_ids"])
		groupRequired := boolField(group, "required", false)

		present := make([]string, 0)
		for _, fid := range flagIDs {
			if val, ok := parsedFlags[fid]; ok && isPresent(val) {
				present = append(present, fid)
			}
		}

		if len(present) > 1 {
			// Build labels for error message
			labels := make([]string, len(present))
			for i, fid := range present {
				labels[i] = flagLabel(fv.flagByID[fid])
			}
			errs = append(errs, ParseError{
				ErrorType: ErrExclusiveGroupViolation,
				Message:   fmt.Sprintf("only one of %s may be used", strings.Join(labels, ", ")),
			})
		} else if groupRequired && len(present) == 0 {
			labels := make([]string, len(flagIDs))
			for i, fid := range flagIDs {
				labels[i] = flagLabel(fv.flagByID[fid])
			}
			errs = append(errs, ParseError{
				ErrorType: ErrMissingExclusiveGroup,
				Message:   fmt.Sprintf("one of %s is required", strings.Join(labels, ", ")),
			})
		}
	}

	return errs
}

// isPresent returns true if a parsed flag value is "set" — i.e., not
// a boolean false, not nil, and not a count of 0. This distinguishes
// "flag was seen in argv" from "flag is absent with a zero default".
//
// For boolean flags, false means absent. For count flags, int64(0) means
// absent. For all other types, nil means absent.
func isPresent(v any) bool {
	if v == nil {
		return false
	}
	if b, ok := v.(bool); ok {
		return b
	}
	// Count flags default to int64(0) when absent; treat 0 as not present
	// for constraint checking purposes (conflicts_with, requires, etc.).
	if n, ok := v.(int64); ok {
		return n != 0
	}
	return true
}

// flagLabel constructs a human-readable label for a flag, like
// "-l/--long-listing" or "--verbose" or "-v".
//
// If the flag definition is nil (e.g., the ID is unknown), returns the raw ID.
func flagLabel(def map[string]any) string {
	if def == nil {
		return "(unknown)"
	}
	short := stringField(def, "short")
	long := stringField(def, "long")
	sdl := stringField(def, "single_dash_long")

	var parts []string
	if short != "" {
		parts = append(parts, "-"+short)
	}
	if long != "" {
		parts = append(parts, "--"+long)
	}
	if sdl != "" {
		parts = append(parts, "-"+sdl)
	}
	if len(parts) == 0 {
		return stringField(def, "id")
	}
	result := strings.Join(parts, "/")
	return result
}

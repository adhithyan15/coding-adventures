package clibuilder

// =========================================================================
// Type coercion — converts raw string token values to Go native types.
// =========================================================================
//
// # Why coerce at parse time?
//
// The spec (§3) requires that parsed results contain native typed values —
// integers as int64, floats as float64, booleans as bool — not raw strings.
// Coercing eagerly gives callers clean data without any further conversion.
//
// # Supported types
//
//   string    → string (validated non-empty)
//   integer   → int64
//   float     → float64
//   boolean   → bool (flag presence, not a string "true"/"false")
//   path      → string (syntactic validity only; existence not checked)
//   file      → string (must exist and be readable at parse time)
//   directory → string (must be an existing directory at parse time)
//   enum      → string (must be in enum_values; case-sensitive)

import (
	"fmt"
	"os"
	"strconv"
)

// coerceValue converts a raw string token value to the appropriate Go type
// based on the flag or argument's `type` field.
//
// For "enum" types, the flag/arg definition's enum_values is consulted.
// For "file" and "directory", the filesystem is accessed.
//
// Returns the coerced value and nil on success. Returns nil and an error
// on failure.
func coerceValue(raw string, typeName string, def map[string]any) (any, error) {
	switch typeName {
	case "string":
		if raw == "" {
			return nil, fmt.Errorf("value must be non-empty")
		}
		return raw, nil

	case "integer":
		n, err := strconv.ParseInt(raw, 10, 64)
		if err != nil {
			return nil, fmt.Errorf("not a valid integer: %q", raw)
		}
		return n, nil

	case "float":
		f, err := strconv.ParseFloat(raw, 64)
		if err != nil {
			return nil, fmt.Errorf("not a valid float: %q", raw)
		}
		return f, nil

	case "boolean":
		// Boolean flags are set by presence, not value. If we somehow
		// end up coercing a boolean value string, treat "true"/"1" as
		// true and everything else as false.
		switch raw {
		case "true", "1", "yes":
			return true, nil
		case "false", "0", "no":
			return false, nil
		}
		return nil, fmt.Errorf("not a valid boolean: %q", raw)

	case "path":
		// Path: syntactically valid. We accept any non-empty string —
		// the OS will report errors when the path is actually used.
		if raw == "" {
			return nil, fmt.Errorf("path must be non-empty")
		}
		return raw, nil

	case "file":
		// File: must exist and be readable at parse time.
		info, err := os.Stat(raw)
		if err != nil {
			if os.IsPermission(err) {
				return nil, fmt.Errorf("permission denied: %q", raw)
			}
			return nil, fmt.Errorf("file not found: %q", raw)
		}
		if info.IsDir() {
			return nil, fmt.Errorf("%q is a directory, not a file", raw)
		}
		return raw, nil

	case "directory":
		// Directory: must exist at parse time.
		info, err := os.Stat(raw)
		if err != nil {
			if os.IsPermission(err) {
				return nil, fmt.Errorf("permission denied: %q", raw)
			}
			return nil, fmt.Errorf("directory not found: %q", raw)
		}
		if !info.IsDir() {
			return nil, fmt.Errorf("%q is not a directory", raw)
		}
		return raw, nil

	case "enum":
		// Enum: must exactly match one of enum_values (case-sensitive).
		allowed := sliceOfStrings(def["enum_values"])
		for _, v := range allowed {
			if v == raw {
				return raw, nil
			}
		}
		return nil, fmt.Errorf("invalid enum value %q; must be one of: %v", raw, allowed)

	default:
		// Unknown type: pass through as string and let the caller decide.
		return raw, nil
	}
}

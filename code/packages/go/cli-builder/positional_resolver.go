package clibuilder

// =========================================================================
// Positional Resolver — assigns positional tokens to argument slots.
// =========================================================================
//
// # The problem
//
// After Phase 2 scanning, we have a flat list of positional_tokens and
// a list of argument definitions (from the spec's `arguments` array).
// We need to assign each token to the correct argument slot.
//
// # Simple case (no variadic argument)
//
// When no argument is variadic, it is a one-to-one mapping in order:
//
//	definitions: [src, dest]
//	tokens:      ["a.txt", "/tmp/b.txt"]
//	result:      {src: "a.txt", dest: "/tmp/b.txt"}
//
// # The variadic case
//
// When one argument is variadic (like `cp`'s source files), the algorithm
// uses a "last-wins" strategy:
//
//   - Leading non-variadic arguments consume from the front.
//   - Trailing non-variadic arguments consume from the back.
//   - The variadic argument gets everything in between.
//
// Example: `cp a.txt b.txt c.txt /dest/`
//
//	definitions: [source (variadic), dest (required, after variadic)]
//	tokens:      ["a.txt", "b.txt", "c.txt", "/dest/"]
//	trailing_start = 4 - 1 = 3
//	dest     = tokens[3] = "/dest/"
//	variadic = tokens[0..2] = ["a.txt", "b.txt", "c.txt"]
//
// This is called the "last-wins" algorithm because trailing (later) required
// arguments win their slots from the end of the token list.

import "fmt"

// PositionalResolver assigns positional tokens to argument slots.
//
// Construct with NewPositionalResolver. The zero value is not usable.
type PositionalResolver struct {
	argDefs []map[string]any
}

// NewPositionalResolver constructs a PositionalResolver from the argument
// definitions for the resolved command scope.
//
// argDefs should be the `arguments` array from the command node, already
// in order (the spec guarantees argument order matters).
func NewPositionalResolver(argDefs []map[string]any) *PositionalResolver {
	return &PositionalResolver{argDefs: argDefs}
}

// Resolve assigns positional tokens to argument slots, returning a map of
// argument id → coerced value, plus any errors encountered.
//
// parsedFlags is passed so that `required_unless_flag` exemptions can be
// checked: if an argument is not required because a certain flag is present,
// we do not report it as missing.
//
// Errors are collected (not fail-fast) so the user sees all problems at once.
func (pr *PositionalResolver) Resolve(tokens []string, parsedFlags map[string]any) (map[string]any, []ParseError) {
	result := make(map[string]any)
	var errs []ParseError

	if len(pr.argDefs) == 0 {
		if len(tokens) > 0 {
			errs = append(errs, ParseError{
				ErrorType: ErrTooManyArguments,
				Message:   fmt.Sprintf("unexpected positional argument(s): %v", tokens),
			})
		}
		return result, errs
	}

	// Find the variadic argument, if any.
	variadicIdx := -1
	for i, def := range pr.argDefs {
		if boolField(def, "variadic", false) {
			variadicIdx = i
			break
		}
	}

	if variadicIdx == -1 {
		// ---- No variadic: simple one-to-one assignment ----
		errs = append(errs, pr.resolveSimple(tokens, result, parsedFlags)...)
	} else {
		// ---- Variadic present: leading + variadic + trailing ----
		errs = append(errs, pr.resolveVariadic(tokens, variadicIdx, result, parsedFlags)...)
	}

	return result, errs
}

// resolveSimple handles the no-variadic case.
func (pr *PositionalResolver) resolveSimple(tokens []string, result map[string]any, parsedFlags map[string]any) []ParseError {
	var errs []ParseError

	for i, def := range pr.argDefs {
		id := stringField(def, "id")
		argType := stringField(def, "type")
		required := boolField(def, "required", true) // default is true per spec
		displayName := displayNameFallback(def)

		// Check required_unless_flag exemption
		if required && isExemptedByFlag(def, parsedFlags) {
			required = false
		}

		if i < len(tokens) {
			val, err := coerceValue(tokens[i], argType, def)
			if err != nil {
				errs = append(errs, ParseError{
					ErrorType: ErrInvalidValue,
					Message:   fmt.Sprintf("invalid %s for argument <%s>: %q", argType, displayName, tokens[i]),
				})
			} else {
				result[id] = val
			}
		} else {
			// Token not provided
			if required {
				errs = append(errs, ParseError{
					ErrorType: ErrMissingRequiredArgument,
					Message:   fmt.Sprintf("missing required argument: <%s>", displayName),
				})
			} else {
				// Use default value if provided
				if def["default"] != nil {
					result[id] = def["default"]
				} else {
					result[id] = nil
				}
			}
		}
	}

	if len(tokens) > len(pr.argDefs) {
		extra := tokens[len(pr.argDefs):]
		errs = append(errs, ParseError{
			ErrorType: ErrTooManyArguments,
			Message:   fmt.Sprintf("unexpected argument(s): %v (expected at most %d positional argument(s))", extra, len(pr.argDefs)),
		})
	}

	return errs
}

// resolveVariadic handles the case where one argument is variadic.
func (pr *PositionalResolver) resolveVariadic(tokens []string, variadicIdx int, result map[string]any, parsedFlags map[string]any) []ParseError {
	var errs []ParseError

	leadingDefs := pr.argDefs[:variadicIdx]
	variadicDef := pr.argDefs[variadicIdx]
	trailingDefs := pr.argDefs[variadicIdx+1:]

	varID := stringField(variadicDef, "id")
	varType := stringField(variadicDef, "type")
	varDisplayName := displayNameFallback(variadicDef)

	// Read variadic_min (default 1 if required, else 0)
	varRequired := boolField(variadicDef, "required", true)
	variadicMin := 1
	if !varRequired {
		variadicMin = 0
	}
	if v, ok := variadicDef["variadic_min"].(float64); ok {
		variadicMin = int(v)
	}

	variadicMax := -1 // -1 means unlimited
	if v, ok := variadicDef["variadic_max"].(float64); ok {
		variadicMax = int(v)
	}

	// Assign leading (before variadic) — consume from the front
	nLeading := len(leadingDefs)
	nTrailing := len(trailingDefs)

	for i, def := range leadingDefs {
		id := stringField(def, "id")
		argType := stringField(def, "type")
		required := boolField(def, "required", true)
		displayName := displayNameFallback(def)

		if required && isExemptedByFlag(def, parsedFlags) {
			required = false
		}

		if i < len(tokens) {
			val, err := coerceValue(tokens[i], argType, def)
			if err != nil {
				errs = append(errs, ParseError{
					ErrorType: ErrInvalidValue,
					Message:   fmt.Sprintf("invalid %s for argument <%s>: %q", argType, displayName, tokens[i]),
				})
			} else {
				result[id] = val
			}
		} else if required {
			errs = append(errs, ParseError{
				ErrorType: ErrMissingRequiredArgument,
				Message:   fmt.Sprintf("missing required argument: <%s>", displayName),
			})
		} else {
			if def["default"] != nil {
				result[id] = def["default"]
			} else {
				result[id] = nil
			}
		}
	}

	// Assign trailing (after variadic) — consume from the end
	trailingStart := len(tokens) - nTrailing
	if trailingStart < nLeading {
		trailingStart = nLeading
	}

	for i, def := range trailingDefs {
		id := stringField(def, "id")
		argType := stringField(def, "type")
		required := boolField(def, "required", true)
		displayName := displayNameFallback(def)

		if required && isExemptedByFlag(def, parsedFlags) {
			required = false
		}

		tokenIdx := trailingStart + i
		if tokenIdx < len(tokens) {
			val, err := coerceValue(tokens[tokenIdx], argType, def)
			if err != nil {
				errs = append(errs, ParseError{
					ErrorType: ErrInvalidValue,
					Message:   fmt.Sprintf("invalid %s for argument <%s>: %q", argType, displayName, tokens[tokenIdx]),
				})
			} else {
				result[id] = val
			}
		} else if required {
			errs = append(errs, ParseError{
				ErrorType: ErrMissingRequiredArgument,
				Message:   fmt.Sprintf("missing required argument: <%s>", displayName),
			})
		} else {
			if def["default"] != nil {
				result[id] = def["default"]
			} else {
				result[id] = nil
			}
		}
	}

	// Variadic gets everything in between: tokens[nLeading .. trailingStart]
	variadicTokens := tokens[nLeading:trailingStart]
	if trailingStart > len(tokens) {
		variadicTokens = tokens[nLeading:]
		if nLeading > len(tokens) {
			variadicTokens = []string{}
		}
	}

	count := len(variadicTokens)
	if count < variadicMin {
		if varRequired || variadicMin > 0 {
			errs = append(errs, ParseError{
				ErrorType: ErrTooFewArguments,
				Message:   fmt.Sprintf("expected at least %d <%s>, got %d", variadicMin, varDisplayName, count),
			})
		}
	}
	if variadicMax >= 0 && count > variadicMax {
		errs = append(errs, ParseError{
			ErrorType: ErrTooManyArguments,
			Message:   fmt.Sprintf("expected at most %d <%s>, got %d", variadicMax, varDisplayName, count),
		})
	}

	// Coerce each variadic token
	varValues := make([]any, 0, len(variadicTokens))
	for _, tok := range variadicTokens {
		val, err := coerceValue(tok, varType, variadicDef)
		if err != nil {
			errs = append(errs, ParseError{
				ErrorType: ErrInvalidValue,
				Message:   fmt.Sprintf("invalid %s for variadic argument <%s>: %q", varType, varDisplayName, tok),
			})
		} else {
			varValues = append(varValues, val)
		}
	}
	result[varID] = varValues

	return errs
}

// isExemptedByFlag checks the `required_unless_flag` field of an argument
// definition. Returns true if any of the listed flag IDs is present in
// parsedFlags, meaning the argument is not required in this invocation.
func isExemptedByFlag(argDef map[string]any, parsedFlags map[string]any) bool {
	exemptions := sliceOfStrings(argDef["required_unless_flag"])
	for _, flagID := range exemptions {
		if v, ok := parsedFlags[flagID]; ok {
			// A boolean flag is exempting only if it is true
			if b, isBool := v.(bool); isBool {
				if b {
					return true
				}
			} else if v != nil {
				return true
			}
		}
	}
	return false
}

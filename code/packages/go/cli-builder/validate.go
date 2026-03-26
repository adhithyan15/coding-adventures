package clibuilder

// =========================================================================
// Standalone Spec Validation — non-panicking, result-oriented API.
// =========================================================================
//
// # Why a separate validation API?
//
// The existing LoadSpec / LoadSpecFromBytes functions are designed for the
// parser's startup path: they return a single error and expect the caller
// to treat it as fatal. That's the right design for a parser — if the spec
// is broken, you can't parse argv.
//
// But there are other use cases where you want to validate a spec *without*
// needing the parsed result:
//
//   - CI linters that check spec files in a pre-commit hook
//   - Editor plugins that show inline validation errors
//   - CLI tools that offer a `validate` subcommand
//   - Test harnesses that assert spec correctness
//
// For these use cases, callers want:
//   1. A simple Valid/Invalid boolean — no need to inspect the parsed map.
//   2. ALL errors at once — not just the first one.
//   3. No panics — errors are data, not control flow.
//
// # How it works
//
// Both ValidateSpec and ValidateSpecBytes delegate to the same internal
// validateSpec function that LoadSpec uses. The difference is in the
// return type: instead of (map[string]any, error), they return a
// ValidationResult struct that collects the error message(s).
//
// Currently, the internal validateSpec returns on the first error it
// finds (fail-fast). So ValidationResult.Errors will contain at most
// one entry. If validateSpec is ever extended to collect multiple errors,
// this API will automatically surface them all.

import (
	"encoding/json"
	"fmt"
)

// =========================================================================
// ValidationResult — the outcome of spec validation.
// =========================================================================
//
// This is a value type, not a pointer type. Callers can check the Valid
// field directly without worrying about nil.
//
// Example:
//
//	result := clibuilder.ValidateSpec("my-cli.json")
//	if !result.Valid {
//	    for _, errMsg := range result.Errors {
//	        fmt.Fprintln(os.Stderr, errMsg)
//	    }
//	    os.Exit(1)
//	}
//	fmt.Println("Spec is valid!")
type ValidationResult struct {
	// Valid is true when the spec passes all validation rules.
	// When Valid is true, Errors is guaranteed to be empty.
	Valid bool

	// Errors contains human-readable descriptions of each validation
	// failure. When Valid is false, this slice has at least one entry.
	// When Valid is true, this slice is empty (nil).
	Errors []string
}

// =========================================================================
// ValidateSpec — validate a spec file on disk.
// =========================================================================
//
// ValidateSpec reads the JSON spec at specFilePath and runs all eight
// validation rules from §6.4.3 of the CLI Builder specification.
//
// It never panics. All errors — including file-not-found and JSON syntax
// errors — are captured in the returned ValidationResult.
//
// # Difference from LoadSpec
//
// LoadSpec returns the parsed spec map on success, because the parser
// needs it. ValidateSpec discards the parsed map and only reports
// whether validation passed or failed. This makes the API simpler for
// callers who only care about correctness.
func ValidateSpec(specFilePath string) ValidationResult {
	result, _ := StartNew[ValidationResult]("clibuilder.ValidateSpec", ValidationResult{},
		func(op *Operation[ValidationResult], rf *ResultFactory[ValidationResult]) *OperationResult[ValidationResult] {
			// --- Step 1: Read the file ---
			//
			// If the file doesn't exist or can't be read, that's a validation
			// failure, not a panic. We wrap the OS error in a friendly message.
			data, err := op.File.ReadFile(specFilePath)
			if err != nil {
				return rf.Generate(true, false, ValidationResult{
					Valid:  false,
					Errors: []string{fmt.Sprintf("cannot read spec file %q: %v", specFilePath, err)},
				})
			}

			// --- Step 2: Delegate to the byte-slice validator ---
			return rf.Generate(true, false, ValidateSpecBytes(data))
		}).GetResult()
	return result
}

// =========================================================================
// ValidateSpecBytes — validate a spec from raw bytes.
// =========================================================================
//
// ValidateSpecBytes parses the JSON from data and runs all eight
// validation rules. This is useful when the spec is embedded in a test
// or received over a network — anywhere you have bytes but not a file.
//
// Like ValidateSpec, it never panics.
func ValidateSpecBytes(data []byte) ValidationResult {
	// --- Step 1: Parse JSON ---
	//
	// Invalid JSON is a validation error, not a panic. The error message
	// from encoding/json is already quite descriptive (it includes the
	// byte offset), so we just wrap it.
	var spec map[string]any
	if err := json.Unmarshal(data, &spec); err != nil {
		return ValidationResult{
			Valid:  false,
			Errors: []string{fmt.Sprintf("invalid JSON: %v", err)},
		}
	}

	// --- Step 2: Run validation rules ---
	//
	// validateSpec is the same internal function used by LoadSpec. It
	// returns a *SpecError on failure. We extract the message and put
	// it in the Errors slice.
	_, err := validateSpec(spec)
	if err != nil {
		return ValidationResult{
			Valid:  false,
			Errors: []string{err.(*SpecError).Message},
		}
	}

	// --- Step 3: All rules passed ---
	return ValidationResult{
		Valid:  true,
		Errors: nil,
	}
}

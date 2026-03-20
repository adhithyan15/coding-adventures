package analyzer

// Manifest loading and capability comparison.
//
// This module loads a package's `required_capabilities.json` manifest and
// compares it against the capabilities detected by the analyzer. The
// comparison answers the question: "Does this package use only the
// capabilities it declared?"
//
// # Comparison logic
//
// The comparison is asymmetric:
//
//   - Undeclared capabilities (detected but not in manifest) are ERRORS.
//     The code uses something it didn't declare.
//
//   - Unused declarations (in manifest but not detected) are WARNINGS.
//     The manifest declares a capability the code doesn't use. This isn't
//     a security issue — it's just a stale declaration.
//
// # Default deny
//
// If no `required_capabilities.json` exists, the package is treated as
// having zero declared capabilities. Any detected capability is an error.
// This is the "no manifest = block everything" principle.
//
// # Target matching
//
// When comparing detected targets against declared targets, we use glob-
// style matching via filepath.Match:
//
//   - "../../grammars/*.tokens" matches "../../grammars/python.tokens"
//   - "*" matches anything
//   - Exact strings match exactly
//
// This mirrors OpenBSD's unveil() path matching.

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// Manifest represents a parsed required_capabilities.json file.
//
// Every package in the capability system has (or should have) a manifest
// that declares what OS capabilities it needs. The manifest is the
// "contract" between the package and the security system.
type Manifest struct {
	// Package is the qualified package name (e.g., "go/logic-gates")
	Package string `json:"package"`

	// Capabilities is the list of declared capability requirements.
	// Each entry has "category", "action", and "target" fields.
	Capabilities []DeclaredCapability `json:"capabilities"`

	// Justification is a human-readable explanation of why these
	// capabilities are needed.
	Justification string `json:"justification"`

	// BannedConstructExceptions lists banned constructs that this
	// package is explicitly allowed to use, with justifications.
	BannedConstructExceptions []BannedException `json:"banned_construct_exceptions"`

	// Path is the filesystem path to the manifest file (empty if
	// created programmatically via DefaultManifest).
	Path string `json:"-"`
}

// DeclaredCapability is a single capability declaration in a manifest.
//
// The triple (Category, Action, Target) uses the same format as
// DetectedCapability. Wildcards are allowed:
//
//   - category: "fs", "net", "proc", "env", "ffi"
//   - action: specific action or "*" for any
//   - target: specific path/name, glob pattern, or "*" for any
type DeclaredCapability struct {
	Category string `json:"category"`
	Action   string `json:"action"`
	Target   string `json:"target"`
}

// BannedException is a declared exception for a banned construct.
type BannedException struct {
	Construct     string `json:"construct"`
	Justification string `json:"justification"`
}

// IsEmpty returns true if the manifest declares zero capabilities.
func (m *Manifest) IsEmpty() bool {
	return len(m.Capabilities) == 0
}

// LoadManifest loads a manifest from a JSON file.
//
// The expected JSON structure is:
//
//	{
//	  "package": "go/my-package",
//	  "capabilities": [
//	    {"category": "fs", "action": "read", "target": "*.txt"}
//	  ],
//	  "justification": "Reads config files",
//	  "banned_construct_exceptions": []
//	}
func LoadManifest(path string) (*Manifest, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("loading manifest: %w", err)
	}

	var manifest Manifest
	if err := json.Unmarshal(data, &manifest); err != nil {
		return nil, fmt.Errorf("parsing manifest %s: %w", path, err)
	}

	manifest.Path = path
	return &manifest, nil
}

// DefaultManifest creates a default (empty) manifest for a package without one.
//
// This represents the "no manifest = default deny" policy. A package
// without a required_capabilities.json is treated as declaring zero
// capabilities. Any detected capability will be flagged as an error.
func DefaultManifest(packageName string) *Manifest {
	return &Manifest{
		Package:       packageName,
		Capabilities:  nil,
		Justification: "No manifest file — default deny (zero capabilities).",
	}
}

// ComparisonResult holds the result of comparing detected capabilities
// against a manifest.
type ComparisonResult struct {
	// Passed is true if all detected capabilities are declared in the manifest.
	Passed bool

	// Errors are detected capabilities not covered by any declaration.
	// These are security violations — the code uses something it didn't declare.
	Errors []DetectedCapability

	// Warnings are declared capabilities that weren't detected in the code.
	// These are stale declarations — not security issues, but maintenance debt.
	Warnings []DeclaredCapability

	// Matched are detected capabilities that matched a declaration.
	Matched []DetectedCapability
}

// Summary returns a human-readable summary of the comparison result.
func (cr *ComparisonResult) Summary() string {
	var sb strings.Builder

	if cr.Passed {
		sb.WriteString("PASS — all detected capabilities are declared.\n")
	} else {
		sb.WriteString(fmt.Sprintf("FAIL — %d undeclared capability(ies) detected.\n", len(cr.Errors)))
	}

	if len(cr.Errors) > 0 {
		sb.WriteString("\nUndeclared capabilities (ERRORS):\n")
		for _, cap := range cr.Errors {
			sb.WriteString(fmt.Sprintf("  %s:%d: %s (%s)\n", cap.File, cap.Line, cap.String(), cap.Evidence))
		}
	}

	if len(cr.Warnings) > 0 {
		sb.WriteString("\nUnused declarations (WARNINGS):\n")
		for _, decl := range cr.Warnings {
			sb.WriteString(fmt.Sprintf("  %s:%s:%s\n", decl.Category, decl.Action, decl.Target))
		}
	}

	if len(cr.Matched) > 0 {
		sb.WriteString(fmt.Sprintf("\nMatched: %d capability(ies).\n", len(cr.Matched)))
	}

	return sb.String()
}

// targetMatches checks if a detected target matches a declared target pattern.
//
// Uses filepath.Match for glob-style matching:
//
//   - "*" matches anything (fast path, no need for glob matching)
//   - "../../grammars/*.tokens" matches "../../grammars/python.tokens"
//   - "file.txt" matches "file.txt" exactly
//
// If the detected target is "*" (meaning we couldn't determine the actual
// target statically), we accept any declared pattern. This is conservative:
// we accept it rather than producing a false positive. The reasoning is that
// if the code uses `os.Open(someVariable)`, we don't know what file it opens,
// so any file declaration should cover it.
func targetMatches(pattern string, actual string) bool {
	// Fast path: wildcard matches everything
	if pattern == "*" {
		return true
	}

	// If the detected target is a wildcard, accept any declared pattern.
	// We can't know what the code will actually access at runtime.
	if actual == "*" {
		return true
	}

	// Use filepath.Match for glob matching
	matched, err := filepath.Match(pattern, actual)
	if err != nil {
		// filepath.Match returns an error for malformed patterns.
		// Fall back to exact string comparison.
		return pattern == actual
	}
	return matched
}

// capabilityMatches checks if a detected capability matches a declared one.
//
// A match requires:
//  1. Same category (fs, net, proc, etc.)
//  2. Compatible action (exact match, or declared is "*")
//  3. Compatible target (glob match via targetMatches)
func capabilityMatches(declared DeclaredCapability, detected DetectedCapability) bool {
	// Category must match exactly
	if declared.Category != detected.Category {
		return false
	}

	// Action must match exactly or declared must be wildcard
	if declared.Action != "*" && declared.Action != detected.Action {
		return false
	}

	// Target must match via glob pattern
	return targetMatches(declared.Target, detected.Target)
}

// CompareCapabilities compares detected capabilities against a manifest.
//
// This is the core comparison logic used by the CI gate. It determines
// whether a package's source code uses only the capabilities it declared.
//
// The algorithm:
//  1. For each detected capability, check if any declaration covers it
//  2. If no declaration matches, it's an error (undeclared usage)
//  3. For each declaration, check if any detected capability uses it
//  4. If no detected capability matches, it's a warning (stale declaration)
func CompareCapabilities(detected []DetectedCapability, manifest *Manifest) *ComparisonResult {
	var errors []DetectedCapability
	var matched []DetectedCapability

	// Step 1: Check each detected capability against all declarations
	for _, cap := range detected {
		foundMatch := false
		for _, decl := range manifest.Capabilities {
			if capabilityMatches(decl, cap) {
				foundMatch = true
				break
			}
		}
		if foundMatch {
			matched = append(matched, cap)
		} else {
			errors = append(errors, cap)
		}
	}

	// Step 2: Find unused declarations (warnings)
	//
	// A declaration is "used" if at least one detected capability matches it.
	// We track which declarations were used by index.
	usedDeclarations := make(map[int]bool)
	for _, cap := range detected {
		for i, decl := range manifest.Capabilities {
			if capabilityMatches(decl, cap) {
				usedDeclarations[i] = true
				break
			}
		}
	}

	var warnings []DeclaredCapability
	for i, decl := range manifest.Capabilities {
		if !usedDeclarations[i] {
			warnings = append(warnings, decl)
		}
	}

	return &ComparisonResult{
		Passed:   len(errors) == 0,
		Errors:   errors,
		Warnings: warnings,
		Matched:  matched,
	}
}

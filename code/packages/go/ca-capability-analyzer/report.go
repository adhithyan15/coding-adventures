// Package main implements the ca-capability-analyzer: a static analysis tool
// that walks the AST of a Go package and reports any raw OS-level capability
// usage that is not declared in the package's required_capabilities.json manifest.
//
// # The Problem
//
// Every Go package in this monorepo declares its OS-level capabilities in a
// required_capabilities.json file. The capability-cage-generator then bakes
// those declarations into gen_capabilities.go, which provides the Operations
// system: op.File.ReadFile, op.Time.Now, op.Net.Connect, etc.
//
// But nothing prevents a developer from writing os.ReadFile(...) directly
// without updating the manifest. When that happens, the capability system is
// silently bypassed — the package uses more OS access than it declared.
//
// # What This Tool Does
//
// 1. Walk every .go file in a directory (skipping gen_capabilities.go).
// 2. Detect raw stdlib calls that map to capability categories.
// 3. Read required_capabilities.json to get what is declared.
// 4. Report CAP001 violations for any detected capability not in the manifest.
// 5. Report CAP002 violations for any disallowed restricted construct.
//
// # Exemptions
//
// Three categories of code are always exempt:
//   - gen_capabilities.go: auto-generated, raw OS calls are intentional.
//   - Lines ending in //nolint:cap: explicitly suppressed by the developer.
//   - Calls via op.File.*, op.Net.*, op.Time.*, cage.ReadFile, etc.: already
//     routed through the Operations system.
//
// # Usage
//
//	ca-capability-analyzer --dir /path/to/package [--verbose]
//	# Exits 0 if clean, 1 if violations, 2 if tool error.
package main

import "fmt"

// CapabilityString is the canonical string form of a detected capability,
// e.g., "fs:read:*" or "net:*:*". The format matches entries in
// required_capabilities.json, making comparison straightforward.
//
// Using a named type instead of plain string prevents accidentally passing
// arbitrary strings where capability strings are expected. The compiler
// will catch the mismatch.
type CapabilityString string

// DetectedCapability records a single raw OS call and the capability it implies.
//
// Every field is populated so violation messages can be actionable:
// a developer should be able to open the file, navigate to the line, and
// immediately understand what to add to required_capabilities.json.
type DetectedCapability struct {
	// File is the path to the source file, relative to the analyzed directory.
	File string

	// Line is the 1-based line number of the raw OS call.
	Line int

	// Capability is the inferred capability string, e.g., "fs:read:*".
	Capability CapabilityString

	// Evidence is a human-readable description of what was found,
	// e.g., "os.ReadFile call". Included in violation messages.
	Evidence string
}

// BannedConstruct records a detected restricted code pattern. The Go analyzer
// only allows a tiny subset of these through an explicit manifest opt-in;
// everything else remains a CAP002 violation because it defeats static
// capability analysis (dynamic dispatch, native code interop, linker tricks).
type BannedConstruct struct {
	// File is the path to the source file, relative to the analyzed directory.
	File string

	// Line is the 1-based line number.
	Line int

	// Construct is the canonical manifest-facing identifier, e.g. `import "C"`,
	// "plugin.Open", "reflect.Value.Call", or "//go:linkname".
	Construct string

	// Kind identifies the banned construct, e.g., "unsafe.Pointer",
	// `import "C"`, "reflect.Value.Call", "go:linkname".
	Kind string
}

// Violation is a single reportable issue. The CLI iterates violations and
// prints each one. Two error codes are defined:
//
//   - CAP001: an undeclared capability was detected. Fix: add the capability
//     to required_capabilities.json and regenerate gen_capabilities.go.
//   - CAP002: a restricted construct was detected and not explicitly authorized.
//     Fix: remove it, or for supported FFI-style constructs add the matching
//     banned_construct_exceptions entry plus the required ffi capability.
type Violation struct {
	// Code is the short error code, "CAP001" or "CAP002".
	Code string

	// Message is the full human-readable line, e.g.:
	//   "lexer.go:42: [CAP001] undeclared capability: fs:read:* (os.ReadFile call)"
	Message string
}

// Format returns the violation as a printable string. Identical to Message
// but provided so callers can treat Violation uniformly with other types
// that implement a Format() method.
func (v Violation) Format() string {
	return v.Message
}

// AnalysisResult is the complete output of analyzing one package directory.
// It is returned by AnalyzeDir and is also useful for programmatic consumers
// that want to inspect individual fields rather than just the violation list.
type AnalysisResult struct {
	// Dir is the absolute path to the analyzed directory.
	Dir string

	// Detected is every raw OS call found, after applying exemptions.
	// This list is populated even when all calls are declared in the manifest.
	// With --verbose, the CLI prints this list even on a passing run.
	Detected []DetectedCapability

	// Banned is every restricted construct that remains disallowed after
	// manifest exceptions are applied.
	Banned []BannedConstruct

	// Declared is the capability set loaded from required_capabilities.json.
	// Empty if no manifest exists (treated as zero declared capabilities).
	Declared []CapabilityString

	// Violations is the final verdict: undeclared capabilities + disallowed
	// restricted constructs.
	// If len(Violations) == 0, the analysis passes.
	Violations []Violation

	// ParseErrors records files that could not be parsed. These are reported as
	// warnings (written to stderr by the CLI) rather than hard errors so that
	// build-tag-restricted files or files with syntax errors don't abort the
	// entire analysis. However they should be visible: a file that fails to
	// parse is not analyzed, so violations within it go undetected.
	ParseErrors []string
}

// Passed reports whether the analysis found no violations. Use this as the
// exit condition: if result.Passed() { os.Exit(0) } else { os.Exit(1) }.
func (r *AnalysisResult) Passed() bool {
	return len(r.Violations) == 0
}

// newViolationCAP001 constructs a CAP001 (undeclared capability) violation
// from a DetectedCapability. The message format is:
//
//	<file>:<line>: [CAP001] undeclared capability: <cap> (<evidence>)
//
// The fix instruction is embedded so developers can act immediately.
func newViolationCAP001(d DetectedCapability) Violation {
	return Violation{
		Code: "CAP001",
		Message: fmt.Sprintf(
			"%s:%d: [CAP001] undeclared capability: %s (%s) — add to required_capabilities.json and regenerate gen_capabilities.go",
			d.File, d.Line, d.Capability, d.Evidence,
		),
	}
}

// newViolationCAP002 constructs a CAP002 (banned construct) violation from a
// BannedConstruct. The message format is:
//
//	<file>:<line>: [CAP002] banned construct: <kind>
//
// No fix instruction because the only valid fix is removing the construct.
func newViolationCAP002(b BannedConstruct) Violation {
	return newViolationCAP002Hint(b, "")
}

// newViolationCAP002Hint constructs a CAP002 violation with extra guidance.
func newViolationCAP002Hint(b BannedConstruct, hint string) Violation {
	message := fmt.Sprintf("%s:%d: [CAP002] banned construct: %s", b.File, b.Line, b.Kind)
	if hint != "" {
		message += " — " + hint
	}
	return Violation{
		Code:    "CAP002",
		Message: message,
	}
}

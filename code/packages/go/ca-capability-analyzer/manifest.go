package main

// manifest.go loads and parses the required_capabilities.json file for a
// given Go package directory. It returns the declared capability set as a
// map for O(1) lookup during the analysis phase.
//
// # The Manifest Format
//
// required_capabilities.json is a JSON file with this structure:
//
//	{
//	  "version": 1,
//	  "package": "go/json-lexer",
//	  "capabilities": [
//	    {
//	      "category": "fs",
//	      "action": "read",
//	      "target": "../../grammars/json.tokens",
//	      "justification": "Reads grammar file at init time."
//	    }
//	  ],
//	  "justification": "Reads one grammar file."
//	}
//
// The canonical form of a capability string is "category:action:target",
// e.g., "fs:read:*" or "fs:read:../../grammars/json.tokens".
//
// # Absence of Manifest
//
// If no required_capabilities.json exists in the directory, LoadManifest
// returns an empty map (not an error). Absence means zero declared
// capabilities — which is the correct baseline for pure computation packages.
//
// # Detecting Undeclared Capabilities
//
// The analyzer detects capabilities like "fs:read:*" (category:action:wildcard).
// A manifest that declares "fs:read:../../grammars/json.tokens" does NOT
// cover "fs:read:*" because the targets differ. To avoid false positives for
// packages that declare specific targets but still use os.ReadFile (routed
// through the Operations system), the analyzer treats detected capabilities
// as category:action:* wildcards and compares against all entries sharing the
// same category and action prefix.

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// manifestJSON mirrors the on-disk JSON structure of required_capabilities.json.
// The "JSON" suffix signals "raw parse target, not an abstraction." Callers
// receive the higher-level map[CapabilityString]bool.
type manifestJSON struct {
	Schema                    string                         `json:"$schema"`
	Version                   int                            `json:"version"`
	Package                   string                         `json:"package"`
	Capabilities              []capabilityJSON               `json:"capabilities"`
	Justification             string                         `json:"justification"`
	BannedConstructExceptions []bannedConstructExceptionJSON `json:"banned_construct_exceptions"`
}

// capabilityJSON is one capability entry in the raw JSON.
type capabilityJSON struct {
	Category      string `json:"category"`
	Action        string `json:"action"`
	Target        string `json:"target"`
	Justification string `json:"justification"`
}

// bannedConstructExceptionJSON is one banned_construct_exceptions entry in the
// raw JSON.
type bannedConstructExceptionJSON struct {
	Construct     string `json:"construct"`
	Language      string `json:"language"`
	Justification string `json:"justification"`
}

// ManifestData is the parsed manifest information the analyzer needs.
//
// Declared contains the capability declaration set, including the category:action:*
// wildcard entries added by LoadManifest.
//
// BannedConstructExceptions contains explicit per-language exception keys in the
// form "<language>:<construct>". Only a small subset of constructs are currently
// honorably exemptible in the Go analyzer.
type ManifestData struct {
	Declared                  map[CapabilityString]bool
	BannedConstructExceptions map[string]bool
}

// canonicalCapabilityString converts a (category, action, target) triple to
// the standard "category:action:target" form used throughout the analyzer.
//
// Example: ("fs", "read", "*") → "fs:read:*"
// Example: ("net", "*", "*")  → "net:*:*"
func canonicalCapabilityString(category, action, target string) CapabilityString {
	return CapabilityString(category + ":" + action + ":" + target)
}

// canonicalBannedConstructExceptionKey converts a (language, construct) pair
// into the standard "<language>:<construct>" form used for O(1) exemption checks.
func canonicalBannedConstructExceptionKey(language, construct string) string {
	return strings.ToLower(strings.TrimSpace(language)) + ":" + normalizeBannedConstructName(construct)
}

// LoadManifestData reads required_capabilities.json from dir and returns the
// declared capability set plus any explicit banned-construct exceptions.
//
// If no manifest file exists, both maps are empty but non-nil — this represents
// a package with zero declared capabilities and zero construct exemptions.
//
// The function also adds "wildcard" entries for each (category:action) pair
// present in the manifest. This lets the analyzer treat a manifest that
// declares "fs:read:../../grammars/json.tokens" as covering "fs:read:*" —
// because if the package has declared any fs:read target, it has gone through
// the manifest process, and the specific path enforcement is handled at
// runtime by the Operations system, not by this static analyzer.
func LoadManifestData(dir string) (*ManifestData, error) {
	manifestPath := filepath.Join(dir, "required_capabilities.json")

	// Use op.File.ReadFile via the Operations system (this package declares
	// fs:read:* in its own manifest).
	result := StartNew[[]byte]("ca-capability-analyzer.LoadManifest", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			data, err := op.File.ReadFile(manifestPath)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, data)
		})

	data, err := result.GetResult()
	if err != nil {
		// If the file doesn't exist, return an empty map (not an error).
		// Absence of required_capabilities.json is the correct baseline for
		// pure-computation packages that make no OS calls.
		//
		// The Operations system (gen_capabilities.go) passes the original
		// os.ReadFile error through unchanged via rf.Fail(nil, err) and
		// GetResult() returns result.Err as-is (see GetResult, line 164).
		// This means the error is an *os.PathError wrapping os.ErrNotExist,
		// so errors.Is(err, os.ErrNotExist) resolves correctly through the
		// error chain regardless of locale or OS.
		if errors.Is(err, os.ErrNotExist) {
			return &ManifestData{
				Declared:                  make(map[CapabilityString]bool),
				BannedConstructExceptions: make(map[string]bool),
			}, nil
		}
		return nil, fmt.Errorf("LoadManifest: reading %s: %w", manifestPath, err)
	}

	var mf manifestJSON
	if err := json.Unmarshal(data, &mf); err != nil {
		return nil, fmt.Errorf("LoadManifest: parsing %s: %w", manifestPath, err)
	}

	manifest := &ManifestData{
		Declared:                  make(map[CapabilityString]bool),
		BannedConstructExceptions: make(map[string]bool),
	}
	for _, cap := range mf.Capabilities {
		// Add the exact capability as declared.
		manifest.Declared[canonicalCapabilityString(cap.Category, cap.Action, cap.Target)] = true

		// Also add a wildcard-target entry for this (category, action) pair.
		// This allows the analyzer to treat any declared target as covering the
		// wildcard form. Rationale: the analyzer detects capability usage as
		// "fs:read:*" (it cannot know which specific path will be accessed);
		// if the manifest declares any fs:read entry, the package has accepted
		// the manifest discipline and path enforcement is handled at runtime.
		manifest.Declared[canonicalCapabilityString(cap.Category, cap.Action, "*")] = true
	}

	for _, exception := range mf.BannedConstructExceptions {
		key := canonicalBannedConstructExceptionKey(exception.Language, exception.Construct)
		manifest.BannedConstructExceptions[key] = true
	}

	return manifest, nil
}

// LoadManifest reads required_capabilities.json from dir and returns just the
// declared capability set. This compatibility helper keeps the old API surface
// while the analyzer uses LoadManifestData for the richer manifest shape.
func LoadManifest(dir string) (map[CapabilityString]bool, error) {
	manifest, err := LoadManifestData(dir)
	if err != nil {
		return nil, err
	}
	return manifest.Declared, nil
}

// DeclaredCapabilityList returns the canonical capability strings from a manifest
// directory, as a flat slice. Used to populate AnalysisResult.Declared.
func DeclaredCapabilityList(dir string) ([]CapabilityString, error) {
	m, err := LoadManifest(dir)
	if err != nil {
		return nil, err
	}
	result := make([]CapabilityString, 0, len(m))
	for k := range m {
		result = append(result, k)
	}
	return result, nil
}

package main

// manifest_test.go tests the LoadManifest function.
//
// Each test creates a temporary directory (via t.TempDir()), writes a
// required_capabilities.json file using os.WriteFile, then calls LoadManifest.
// The temp directory is automatically cleaned up after each test.

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

type taxonomyFixture struct {
	Categories                    map[string][]string `json:"categories"`
	AllActions                    []string            `json:"all_actions"`
	ExpectedValidPairCount        int                 `json:"expected_valid_pair_count"`
	ExpectedInvalidCrossPairCount int                 `json:"expected_invalid_cross_pair_count"`
}

func loadTaxonomyFixture(t *testing.T) taxonomyFixture {
	t.Helper()
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "capability-security-v1", "taxonomy.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read shared taxonomy fixture: %v", err)
	}
	var fixture taxonomyFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("parse shared taxonomy fixture: %v", err)
	}
	return fixture
}

func containsAction(actions []string, wanted string) bool {
	for _, action := range actions {
		if action == wanted {
			return true
		}
	}
	return false
}

func TestCapabilityTaxonomyMatchesSharedFixture(t *testing.T) {
	fixture := loadTaxonomyFixture(t)
	validCount := 0
	invalidCount := 0
	for category, allowed := range fixture.Categories {
		for _, action := range fixture.AllActions {
			want := containsAction(allowed, action)
			if got := validCapabilityPair(category, action); got != want {
				t.Errorf("validCapabilityPair(%q, %q) = %v, want %v", category, action, got, want)
			}
			if want {
				validCount++
			} else {
				invalidCount++
			}
		}
	}
	if validCount != fixture.ExpectedValidPairCount {
		t.Errorf("valid pair count = %d, want %d", validCount, fixture.ExpectedValidPairCount)
	}
	if invalidCount != fixture.ExpectedInvalidCrossPairCount {
		t.Errorf("invalid cross-pair count = %d, want %d", invalidCount, fixture.ExpectedInvalidCrossPairCount)
	}
	if validCapabilityPair("filesystem", "read") {
		t.Error("unknown category must fail closed")
	}
	if validCapabilityPair("fs", "destroy") {
		t.Error("unknown action must fail closed")
	}
}

// writeManifest is a test helper that writes content to
// required_capabilities.json in dir.
func writeManifest(t *testing.T, dir, content string) {
	t.Helper()
	path := filepath.Join(dir, "required_capabilities.json")
	if err := os.WriteFile(path, []byte(content), 0o600); err != nil {
		t.Fatalf("writeManifest: %v", err)
	}
}

// TestLoadManifest_NoFile verifies that a missing manifest returns an empty
// map with no error. Absence of a manifest means zero declared capabilities —
// the correct baseline for pure computation packages.
func TestLoadManifest_NoFile(t *testing.T) {
	dir := t.TempDir()
	m, err := LoadManifest(dir)
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}
	if len(m) != 0 {
		t.Errorf("expected empty map, got %v", m)
	}
}

// TestLoadManifestData_NoFile verifies that the richer manifest loader returns
// empty maps for both declared capabilities and banned-construct exceptions.
func TestLoadManifestData_NoFile(t *testing.T) {
	dir := t.TempDir()
	m, err := LoadManifestData(dir)
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}
	if len(m.Declared) != 0 {
		t.Errorf("expected empty declared map, got %v", m.Declared)
	}
	if len(m.BannedConstructExceptions) != 0 {
		t.Errorf("expected empty exception map, got %v", m.BannedConstructExceptions)
	}
}

// TestLoadManifest_EmptyCapabilities verifies that a manifest with an empty
// capabilities array returns an empty map.
func TestLoadManifest_EmptyCapabilities(t *testing.T) {
	dir := t.TempDir()
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [],
		"justification": "Pure computation."
	}`)

	m, err := LoadManifest(dir)
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}
	if len(m) != 0 {
		t.Errorf("expected empty map for empty capabilities, got %v", m)
	}
}

// TestLoadManifest_SingleCapability verifies that a single capability entry
// is correctly loaded and both the exact form and wildcard form are present.
func TestLoadManifest_SingleCapability(t *testing.T) {
	dir := t.TempDir()
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{
				"category": "fs",
				"action": "read",
				"target": "../../grammars/json/json.tokens",
				"justification": "Reads grammar file."
			}
		],
		"justification": "Reads one file."
	}`)

	m, err := LoadManifest(dir)
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	// The exact capability as declared.
	exactKey := CapabilityString("fs:read:../../grammars/json/json.tokens")
	if !m[exactKey] {
		t.Errorf("expected %q in declared set, got %v", exactKey, m)
	}

	// The wildcard form should also be present, so the analyzer recognizes
	// that this package covers the "fs:read:*" pattern.
	wildcardKey := CapabilityString("fs:read:*")
	if !m[wildcardKey] {
		t.Errorf("expected wildcard %q in declared set, got %v", wildcardKey, m)
	}
}

// TestLoadManifest_WildcardTarget verifies that a manifest declaring "*" as
// the target is loaded correctly.
func TestLoadManifest_WildcardTarget(t *testing.T) {
	dir := t.TempDir()
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{
				"category": "fs",
				"action": "read",
				"target": "*",
				"justification": "Reads various files."
			}
		],
		"justification": "Reads files."
	}`)

	m, err := LoadManifest(dir)
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	wildcardKey := CapabilityString("fs:read:*")
	if !m[wildcardKey] {
		t.Errorf("expected %q in declared set, got %v", wildcardKey, m)
	}
}

// TestLoadManifest_MultipleCapabilities verifies that multiple capability
// entries are all loaded.
func TestLoadManifest_MultipleCapabilities(t *testing.T) {
	dir := t.TempDir()
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{
				"category": "fs",
				"action": "read",
				"target": "*",
				"justification": "Reads files."
			},
			{
				"category": "time",
				"action": "read",
				"target": "*",
				"justification": "Reads current time."
			},
			{
				"category": "env",
				"action": "read",
				"target": "*",
				"justification": "Reads env vars."
			}
		],
		"justification": "Needs multiple OS capabilities."
	}`)

	m, err := LoadManifest(dir)
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	for _, expected := range []CapabilityString{"fs:read:*", "time:read:*", "env:read:*"} {
		if !m[expected] {
			t.Errorf("expected %q in declared set, got %v", expected, m)
		}
	}
}

// TestLoadManifestData_BannedConstructExceptions verifies that manifest
// exceptions are parsed and canonicalized for language-specific lookup.
func TestLoadManifestData_BannedConstructExceptions(t *testing.T) {
	dir := t.TempDir()
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [],
		"banned_construct_exceptions": [
			{
				"construct": "import \"C\" (CGo)",
				"language": "go",
				"justification": "Reviewed C bridge."
			},
			{
				"construct": "plugin.Open()",
				"language": "go",
				"justification": "Reviewed plugin loader."
			}
		],
		"justification": "Explicit FFI package."
	}`)

	m, err := LoadManifestData(dir)
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	if !m.BannedConstructExceptions[`go:import "C"`] {
		t.Errorf("expected import C exception, got %v", m.BannedConstructExceptions)
	}
	if !m.BannedConstructExceptions["go:plugin.Open"] {
		t.Errorf("expected plugin.Open exception, got %v", m.BannedConstructExceptions)
	}
}

// TestLoadManifest_InvalidJSON verifies that malformed JSON returns an error.
func TestLoadManifest_InvalidJSON(t *testing.T) {
	dir := t.TempDir()
	writeManifest(t, dir, `{ this is not valid JSON`)

	_, err := LoadManifest(dir)
	if err == nil {
		t.Error("expected error for invalid JSON, got nil")
	}
}

func TestLoadManifest_InvalidCategoryActionPairRejected(t *testing.T) {
	dir := t.TempDir()
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{
				"category": "net",
				"action": "read",
				"target": "*",
				"justification": "Invalid cross-pair fixture."
			}
		],
		"justification": "Must fail before declarations are recorded."
	}`)

	m, err := LoadManifestData(dir)
	if err == nil {
		t.Fatalf("expected invalid pair error, got manifest: %#v", m)
	}
}

// TestLoadManifest_CanonicalForm verifies that canonical capability strings
// use the "category:action:target" colon-separated format.
func TestLoadManifest_CanonicalForm(t *testing.T) {
	dir := t.TempDir()
	writeManifest(t, dir, `{
		"version": 1,
		"package": "go/test-pkg",
		"capabilities": [
			{
				"category": "net",
				"action": "connect",
				"target": "*",
				"justification": "Network access."
			}
		],
		"justification": "Needs network."
	}`)

	m, err := LoadManifest(dir)
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	// The canonical form must use colons, not any other separator.
	expected := CapabilityString("net:connect:*")
	if !m[expected] {
		t.Errorf("expected canonical %q in declared set, got %v", expected, m)
	}
}

// TestCanonicalCapabilityString tests the helper function that builds
// canonical capability strings.
func TestCanonicalCapabilityString(t *testing.T) {
	cases := []struct {
		category, action, target string
		want                     CapabilityString
	}{
		{"fs", "read", "*", "fs:read:*"},
		{"fs", "write", "../../out/file.txt", "fs:write:../../out/file.txt"},
		{"net", "*", "*", "net:*:*"},
		{"time", "read", "*", "time:read:*"},
	}
	for _, tc := range cases {
		got := canonicalCapabilityString(tc.category, tc.action, tc.target)
		if got != tc.want {
			t.Errorf("canonicalCapabilityString(%q, %q, %q) = %q, want %q",
				tc.category, tc.action, tc.target, got, tc.want)
		}
	}
}

func TestCanonicalBannedConstructExceptionKey(t *testing.T) {
	cases := []struct {
		language  string
		construct string
		want      string
	}{
		{"go", `import "C" (CGo)`, `go:import "C"`},
		{"Go", "plugin.Open()", "go:plugin.Open"},
		{"go", "reflect.Value.Call()", "go:reflect.Value.Call"},
	}

	for _, tc := range cases {
		got := canonicalBannedConstructExceptionKey(tc.language, tc.construct)
		if got != tc.want {
			t.Errorf("canonicalBannedConstructExceptionKey(%q, %q) = %q, want %q",
				tc.language, tc.construct, got, tc.want)
		}
	}
}

package analyzer

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// ── Helper: create a temporary manifest file ────────────────────────

func writeTempManifest(t *testing.T, data map[string]interface{}) string {
	t.Helper()
	b, err := json.Marshal(data)
	if err != nil {
		t.Fatalf("failed to marshal manifest: %v", err)
	}
	dir := t.TempDir()
	path := filepath.Join(dir, "required_capabilities.json")
	if err := os.WriteFile(path, b, 0644); err != nil {
		t.Fatalf("failed to write manifest: %v", err)
	}
	return path
}

// ── LoadManifest tests ──────────────────────────────────────────────

func TestLoadManifestBasic(t *testing.T) {
	path := writeTempManifest(t, map[string]interface{}{
		"package":       "go/my-package",
		"justification": "Needs file access",
		"capabilities": []map[string]string{
			{"category": "fs", "action": "read", "target": "*.txt"},
		},
	})

	manifest, err := LoadManifest(path)
	if err != nil {
		t.Fatalf("failed to load manifest: %v", err)
	}

	if manifest.Package != "go/my-package" {
		t.Errorf("expected package 'go/my-package', got %q", manifest.Package)
	}
	if manifest.Justification != "Needs file access" {
		t.Errorf("expected justification 'Needs file access', got %q", manifest.Justification)
	}
	if len(manifest.Capabilities) != 1 {
		t.Fatalf("expected 1 capability, got %d", len(manifest.Capabilities))
	}
	if manifest.Capabilities[0].Category != "fs" {
		t.Errorf("expected category 'fs', got %q", manifest.Capabilities[0].Category)
	}
}

func TestLoadManifestWithExceptions(t *testing.T) {
	path := writeTempManifest(t, map[string]interface{}{
		"package":       "go/special-package",
		"justification": "Needs reflect for serialization",
		"capabilities":  []map[string]string{},
		"banned_construct_exceptions": []map[string]string{
			{"construct": "reflect.Value.Call", "justification": "Serialization library"},
		},
	})

	manifest, err := LoadManifest(path)
	if err != nil {
		t.Fatalf("failed to load manifest: %v", err)
	}

	if len(manifest.BannedConstructExceptions) != 1 {
		t.Fatalf("expected 1 exception, got %d", len(manifest.BannedConstructExceptions))
	}
	if manifest.BannedConstructExceptions[0].Construct != "reflect.Value.Call" {
		t.Errorf("expected construct 'reflect.Value.Call', got %q",
			manifest.BannedConstructExceptions[0].Construct)
	}
}

func TestLoadManifestFileNotFound(t *testing.T) {
	_, err := LoadManifest("/nonexistent/path/manifest.json")
	if err == nil {
		t.Error("expected error for nonexistent manifest file")
	}
}

func TestLoadManifestInvalidJSON(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "bad.json")
	if err := os.WriteFile(path, []byte("not json {{{"), 0644); err != nil {
		t.Fatal(err)
	}
	_, err := LoadManifest(path)
	if err == nil {
		t.Error("expected error for invalid JSON")
	}
}

func TestLoadManifestEmptyCapabilities(t *testing.T) {
	path := writeTempManifest(t, map[string]interface{}{
		"package":       "go/pure-package",
		"justification": "Pure computation",
		"capabilities":  []map[string]string{},
	})

	manifest, err := LoadManifest(path)
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	if !manifest.IsEmpty() {
		t.Error("expected empty manifest")
	}
}

// ── DefaultManifest tests ───────────────────────────────────────────

func TestDefaultManifest(t *testing.T) {
	manifest := DefaultManifest("go/unknown-package")
	if manifest.Package != "go/unknown-package" {
		t.Errorf("expected package 'go/unknown-package', got %q", manifest.Package)
	}
	if !manifest.IsEmpty() {
		t.Error("expected default manifest to be empty")
	}
	if manifest.Path != "" {
		t.Error("expected default manifest to have empty path")
	}
}

// ── targetMatches tests ─────────────────────────────────────────────
//
// Target matching is the glob-based comparison between declared patterns
// and detected targets.

func TestTargetMatchesWildcard(t *testing.T) {
	// Declared "*" matches anything
	if !targetMatches("*", "anything") {
		t.Error("wildcard pattern should match anything")
	}
}

func TestTargetMatchesDetectedWildcard(t *testing.T) {
	// Detected "*" (unknown target) matches any declared pattern
	if !targetMatches("specific.txt", "*") {
		t.Error("detected wildcard should match any declared pattern")
	}
}

func TestTargetMatchesExact(t *testing.T) {
	if !targetMatches("file.txt", "file.txt") {
		t.Error("exact match should succeed")
	}
}

func TestTargetMatchesExactMismatch(t *testing.T) {
	if targetMatches("file.txt", "other.txt") {
		t.Error("different filenames should not match")
	}
}

func TestTargetMatchesGlob(t *testing.T) {
	if !targetMatches("*.txt", "readme.txt") {
		t.Error("glob *.txt should match readme.txt")
	}
}

func TestTargetMatchesGlobNoMatch(t *testing.T) {
	if targetMatches("*.txt", "readme.md") {
		t.Error("glob *.txt should not match readme.md")
	}
}

func TestTargetMatchesPathGlob(t *testing.T) {
	// Note: filepath.Match does NOT match path separators with *.
	// "grammars/*.tokens" matches "grammars/python.tokens" but not
	// "grammars/sub/python.tokens".
	if !targetMatches("grammars/*.tokens", "grammars/python.tokens") {
		t.Error("path glob should match")
	}
}

// ── capabilityMatches tests ─────────────────────────────────────────

func TestCapabilityMatchesExact(t *testing.T) {
	decl := DeclaredCapability{Category: "fs", Action: "read", Target: "file.txt"}
	det := DetectedCapability{Category: "fs", Action: "read", Target: "file.txt"}
	if !capabilityMatches(decl, det) {
		t.Error("exact match should succeed")
	}
}

func TestCapabilityMatchesCategoryMismatch(t *testing.T) {
	decl := DeclaredCapability{Category: "fs", Action: "read", Target: "*"}
	det := DetectedCapability{Category: "net", Action: "read", Target: "*"}
	if capabilityMatches(decl, det) {
		t.Error("different categories should not match")
	}
}

func TestCapabilityMatchesActionWildcard(t *testing.T) {
	decl := DeclaredCapability{Category: "fs", Action: "*", Target: "*"}
	det := DetectedCapability{Category: "fs", Action: "read", Target: "file.txt"}
	if !capabilityMatches(decl, det) {
		t.Error("wildcard action should match any action")
	}
}

func TestCapabilityMatchesActionMismatch(t *testing.T) {
	decl := DeclaredCapability{Category: "fs", Action: "read", Target: "*"}
	det := DetectedCapability{Category: "fs", Action: "write", Target: "*"}
	if capabilityMatches(decl, det) {
		t.Error("different actions should not match (unless declared is *)")
	}
}

func TestCapabilityMatchesTargetGlob(t *testing.T) {
	decl := DeclaredCapability{Category: "fs", Action: "read", Target: "*.txt"}
	det := DetectedCapability{Category: "fs", Action: "read", Target: "config.txt"}
	if !capabilityMatches(decl, det) {
		t.Error("glob target should match")
	}
}

// ── CompareCapabilities tests ───────────────────────────────────────
//
// These test the core comparison logic that the CI gate uses.

func TestCompareAllDeclared(t *testing.T) {
	// All detected capabilities are covered by declarations → PASS
	detected := []DetectedCapability{
		{Category: "fs", Action: "read", Target: "config.txt"},
		{Category: "fs", Action: "write", Target: "output.txt"},
	}
	manifest := &Manifest{
		Package: "go/test-pkg",
		Capabilities: []DeclaredCapability{
			{Category: "fs", Action: "*", Target: "*"},
		},
	}

	result := CompareCapabilities(detected, manifest)
	if !result.Passed {
		t.Error("expected PASS when all capabilities are declared")
	}
	if len(result.Errors) != 0 {
		t.Errorf("expected 0 errors, got %d", len(result.Errors))
	}
	if len(result.Matched) != 2 {
		t.Errorf("expected 2 matched, got %d", len(result.Matched))
	}
}

func TestCompareUndeclared(t *testing.T) {
	// Detected net capability not in manifest → FAIL
	detected := []DetectedCapability{
		{Category: "fs", Action: "read", Target: "config.txt"},
		{Category: "net", Action: "connect", Target: "*"},
	}
	manifest := &Manifest{
		Package: "go/test-pkg",
		Capabilities: []DeclaredCapability{
			{Category: "fs", Action: "read", Target: "*.txt"},
		},
	}

	result := CompareCapabilities(detected, manifest)
	if result.Passed {
		t.Error("expected FAIL when undeclared capabilities exist")
	}
	if len(result.Errors) != 1 {
		t.Errorf("expected 1 error, got %d", len(result.Errors))
	}
	if result.Errors[0].Category != "net" {
		t.Errorf("expected error for 'net' capability, got %q", result.Errors[0].Category)
	}
}

func TestCompareUnusedDeclaration(t *testing.T) {
	// Manifest declares net capability but code doesn't use it → WARNING
	detected := []DetectedCapability{
		{Category: "fs", Action: "read", Target: "config.txt"},
	}
	manifest := &Manifest{
		Package: "go/test-pkg",
		Capabilities: []DeclaredCapability{
			{Category: "fs", Action: "read", Target: "*"},
			{Category: "net", Action: "connect", Target: "*"},
		},
	}

	result := CompareCapabilities(detected, manifest)
	if !result.Passed {
		t.Error("expected PASS (unused declarations are warnings, not errors)")
	}
	if len(result.Warnings) != 1 {
		t.Errorf("expected 1 warning, got %d", len(result.Warnings))
	}
	if result.Warnings[0].Category != "net" {
		t.Errorf("expected warning for 'net' declaration, got %q", result.Warnings[0].Category)
	}
}

func TestCompareEmptyManifest(t *testing.T) {
	// Empty manifest + any detected capability → FAIL (default deny)
	detected := []DetectedCapability{
		{Category: "fs", Action: "read", Target: "*"},
	}
	manifest := DefaultManifest("go/test-pkg")

	result := CompareCapabilities(detected, manifest)
	if result.Passed {
		t.Error("expected FAIL with empty manifest (default deny)")
	}
	if len(result.Errors) != 1 {
		t.Errorf("expected 1 error, got %d", len(result.Errors))
	}
}

func TestComparePureCode(t *testing.T) {
	// No detected capabilities + empty manifest → PASS
	detected := []DetectedCapability{}
	manifest := DefaultManifest("go/pure-pkg")

	result := CompareCapabilities(detected, manifest)
	if !result.Passed {
		t.Error("expected PASS for pure code with empty manifest")
	}
}

func TestCompareMultipleDeclarationsCoverOne(t *testing.T) {
	// Multiple declarations, one detected capability matches one of them
	detected := []DetectedCapability{
		{Category: "fs", Action: "read", Target: "config.txt"},
	}
	manifest := &Manifest{
		Package: "go/test-pkg",
		Capabilities: []DeclaredCapability{
			{Category: "net", Action: "connect", Target: "*"},
			{Category: "fs", Action: "read", Target: "*.txt"},
			{Category: "proc", Action: "exec", Target: "*"},
		},
	}

	result := CompareCapabilities(detected, manifest)
	if !result.Passed {
		t.Error("expected PASS")
	}
	// Two declarations are unused
	if len(result.Warnings) != 2 {
		t.Errorf("expected 2 warnings, got %d", len(result.Warnings))
	}
}

// ── ComparisonResult.Summary tests ──────────────────────────────────

func TestSummaryPass(t *testing.T) {
	result := &ComparisonResult{
		Passed:  true,
		Matched: []DetectedCapability{{Category: "fs", Action: "read", Target: "*"}},
	}
	summary := result.Summary()
	if !strings.Contains(summary, "PASS") {
		t.Errorf("expected summary to contain 'PASS', got %q", summary)
	}
}

func TestSummaryFail(t *testing.T) {
	result := &ComparisonResult{
		Passed: false,
		Errors: []DetectedCapability{
			{Category: "net", Action: "connect", Target: "*", File: "main.go", Line: 5},
		},
	}
	summary := result.Summary()
	if !strings.Contains(summary, "FAIL") {
		t.Errorf("expected summary to contain 'FAIL', got %q", summary)
	}
	if !strings.Contains(summary, "ERRORS") {
		t.Errorf("expected summary to list errors, got %q", summary)
	}
}

func TestSummaryWithWarnings(t *testing.T) {
	result := &ComparisonResult{
		Passed:   true,
		Warnings: []DeclaredCapability{{Category: "net", Action: "connect", Target: "*"}},
	}
	summary := result.Summary()
	if !strings.Contains(summary, "WARNINGS") {
		t.Errorf("expected summary to contain 'WARNINGS', got %q", summary)
	}
}

// ── Manifest.IsEmpty tests ──────────────────────────────────────────

func TestManifestIsEmpty(t *testing.T) {
	m := &Manifest{Package: "test", Capabilities: nil}
	if !m.IsEmpty() {
		t.Error("nil capabilities should be empty")
	}

	m2 := &Manifest{Package: "test", Capabilities: []DeclaredCapability{}}
	if !m2.IsEmpty() {
		t.Error("empty slice should be empty")
	}

	m3 := &Manifest{Package: "test", Capabilities: []DeclaredCapability{
		{Category: "fs", Action: "read", Target: "*"},
	}}
	if m3.IsEmpty() {
		t.Error("non-empty capabilities should not be empty")
	}
}

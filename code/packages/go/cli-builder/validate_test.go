package clibuilder

// =========================================================================
// Tests for ValidateSpec and ValidateSpecBytes
// =========================================================================
//
// # Test strategy
//
// These tests verify the standalone validation API — the one that returns
// a ValidationResult instead of (map[string]any, error). We cover:
//
//   1. Valid spec → Valid=true, empty Errors
//   2. Missing cli_builder_spec_version → Valid=false
//   3. Unsupported version (e.g., "2.0") → Valid=false
//   4. Missing required fields (name, description) → Valid=false
//   5. Invalid JSON → Valid=false
//   6. Nonexistent file → Valid=false (file-based API only)
//   7. Flag with no short/long/single_dash_long → Valid=false
//   8. Circular requires → Valid=false
//
// Each test follows the same pattern:
//   - Build a JSON spec string (valid or broken)
//   - Call ValidateSpecBytes (or ValidateSpec for file tests)
//   - Assert Valid and check that Errors contains the expected substring
//
// We use table-driven tests where the cases are similar, and standalone
// tests where the setup is unique.

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// =========================================================================
// Test 1: A valid spec should pass validation.
// =========================================================================
//
// This is the happy path. If this test fails, everything else is suspect.

func TestValidateSpecBytes_ValidSpec(t *testing.T) {
	// minimalValidSpec is defined in spec_loader_test.go — it has just
	// the three required fields: cli_builder_spec_version, name, description.
	result := ValidateSpecBytes([]byte(minimalValidSpec))

	if !result.Valid {
		t.Errorf("expected Valid=true for minimal valid spec, got Valid=false with errors: %v", result.Errors)
	}
	if len(result.Errors) != 0 {
		t.Errorf("expected empty Errors for valid spec, got %v", result.Errors)
	}
}

// =========================================================================
// Test 2: Missing cli_builder_spec_version should fail.
// =========================================================================
//
// The spec version field is the very first thing validateSpec checks
// (Rule 1 from §6.4.3). Without it, the library doesn't know which
// validation rules to apply.

func TestValidateSpecBytes_MissingVersion(t *testing.T) {
	spec := `{
		"name": "myapp",
		"description": "A test application"
	}`

	result := ValidateSpecBytes([]byte(spec))

	if result.Valid {
		t.Fatal("expected Valid=false when cli_builder_spec_version is missing")
	}
	if len(result.Errors) == 0 {
		t.Fatal("expected at least one error")
	}
	// The error message should mention the version field.
	if !strings.Contains(result.Errors[0], "cli_builder_spec_version") {
		t.Errorf("error should mention cli_builder_spec_version, got: %s", result.Errors[0])
	}
}

// =========================================================================
// Test 3: Unsupported version (e.g., "2.0") should fail.
// =========================================================================
//
// This is different from "missing" — the field exists but has a value
// the library doesn't support. Today only "1.0" is valid. If we ever
// add "2.0" support, this test will need updating.

func TestValidateSpecBytes_UnsupportedVersion(t *testing.T) {
	spec := `{
		"cli_builder_spec_version": "2.0",
		"name": "myapp",
		"description": "A test application"
	}`

	result := ValidateSpecBytes([]byte(spec))

	if result.Valid {
		t.Fatal("expected Valid=false for unsupported version \"2.0\"")
	}
	if len(result.Errors) == 0 {
		t.Fatal("expected at least one error")
	}
	// The error should mention the unsupported version value.
	if !strings.Contains(result.Errors[0], "2.0") {
		t.Errorf("error should mention the bad version \"2.0\", got: %s", result.Errors[0])
	}
}

// =========================================================================
// Test 4: Missing required fields should fail.
// =========================================================================
//
// The spec requires both "name" and "description" at the top level.
// We test each independently using a table-driven approach.

func TestValidateSpecBytes_MissingRequiredFields(t *testing.T) {
	// Table of specs, each missing one required field.
	//
	// Why table-driven? The test logic is identical for each case —
	// only the input and expected error substring differ.
	cases := []struct {
		name          string
		spec          string
		errorContains string
	}{
		{
			name: "missing name",
			spec: `{
				"cli_builder_spec_version": "1.0",
				"description": "A test application"
			}`,
			errorContains: "name",
		},
		{
			name: "missing description",
			spec: `{
				"cli_builder_spec_version": "1.0",
				"name": "myapp"
			}`,
			errorContains: "description",
		},
		{
			name: "empty name",
			spec: `{
				"cli_builder_spec_version": "1.0",
				"name": "",
				"description": "A test application"
			}`,
			errorContains: "name",
		},
		{
			name: "empty description",
			spec: `{
				"cli_builder_spec_version": "1.0",
				"name": "myapp",
				"description": ""
			}`,
			errorContains: "description",
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			result := ValidateSpecBytes([]byte(tc.spec))

			if result.Valid {
				t.Fatal("expected Valid=false for spec with missing required field")
			}
			if len(result.Errors) == 0 {
				t.Fatal("expected at least one error")
			}
			if !strings.Contains(result.Errors[0], tc.errorContains) {
				t.Errorf("error should mention %q, got: %s", tc.errorContains, result.Errors[0])
			}
		})
	}
}

// =========================================================================
// Test 5: Invalid JSON should fail gracefully.
// =========================================================================
//
// The validation API must never panic, even on garbage input. Invalid
// JSON should produce a clear error message.

func TestValidateSpecBytes_InvalidJSON(t *testing.T) {
	// A few flavors of invalid JSON to make sure we're robust.
	cases := []struct {
		name  string
		input string
	}{
		{"truncated object", `{"name": "oops"`},
		{"not JSON at all", `this is not json`},
		{"empty string", ``},
		{"just a number", `42`},
		{"array instead of object", `[1, 2, 3]`},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			result := ValidateSpecBytes([]byte(tc.input))

			if result.Valid {
				t.Fatal("expected Valid=false for invalid JSON")
			}
			if len(result.Errors) == 0 {
				t.Fatal("expected at least one error")
			}
			// The error should mention "JSON" to help the user understand
			// that the problem is syntax, not semantics.
			if !strings.Contains(strings.ToLower(result.Errors[0]), "json") {
				t.Errorf("error should mention JSON, got: %s", result.Errors[0])
			}
		})
	}
}

// =========================================================================
// Test 6: Nonexistent file should fail gracefully.
// =========================================================================
//
// ValidateSpec (the file-based API) must handle missing files without
// panicking. This test uses a path that definitely doesn't exist.

func TestValidateSpec_NonexistentFile(t *testing.T) {
	result := ValidateSpec("/tmp/this-file-definitely-does-not-exist-cli-builder-test.json")

	if result.Valid {
		t.Fatal("expected Valid=false for nonexistent file")
	}
	if len(result.Errors) == 0 {
		t.Fatal("expected at least one error")
	}
	// The error should mention the file path so the user knows which
	// file couldn't be found.
	if !strings.Contains(result.Errors[0], "this-file-definitely-does-not-exist") {
		t.Errorf("error should mention the file path, got: %s", result.Errors[0])
	}
}

// =========================================================================
// Test 6b: ValidateSpec with a valid file on disk.
// =========================================================================
//
// To round-trip the file-based API, we write a valid spec to a temp file
// and validate it.

func TestValidateSpec_ValidFile(t *testing.T) {
	// Write the minimal valid spec to a temporary file.
	dir := t.TempDir()
	path := filepath.Join(dir, "test-spec.json")
	if err := os.WriteFile(path, []byte(minimalValidSpec), 0644); err != nil {
		t.Fatalf("failed to write temp spec file: %v", err)
	}

	result := ValidateSpec(path)

	if !result.Valid {
		t.Errorf("expected Valid=true for valid spec file, got errors: %v", result.Errors)
	}
}

// =========================================================================
// Test 7: Flag with no short/long/single_dash_long should fail.
// =========================================================================
//
// Rule 3 from §6.4.3: every flag must have at least one name form.
// A flag with only an "id" and "type" but no short/long/single_dash_long
// is useless — the parser wouldn't know how to match it in argv.

func TestValidateSpecBytes_FlagNoShortOrLong(t *testing.T) {
	spec := `{
		"cli_builder_spec_version": "1.0",
		"name": "myapp",
		"description": "A test application",
		"flags": [
			{
				"id": "orphan",
				"description": "A flag with no name form",
				"type": "boolean"
			}
		]
	}`

	result := ValidateSpecBytes([]byte(spec))

	if result.Valid {
		t.Fatal("expected Valid=false for flag with no short/long/single_dash_long")
	}
	if len(result.Errors) == 0 {
		t.Fatal("expected at least one error")
	}
	// The error should identify the problematic flag by its ID.
	if !strings.Contains(result.Errors[0], "orphan") {
		t.Errorf("error should mention flag id \"orphan\", got: %s", result.Errors[0])
	}
	// And it should explain what's missing.
	if !strings.Contains(result.Errors[0], "short") || !strings.Contains(result.Errors[0], "long") {
		t.Errorf("error should mention short/long requirement, got: %s", result.Errors[0])
	}
}

// =========================================================================
// Test 8: Circular requires should fail.
// =========================================================================
//
// Rule 8 from §6.4.3: the flag dependency graph G_flag must be acyclic.
// If flag A requires B and flag B requires A, that's a cycle — neither
// flag can be used without the other already being present, which is
// a logical impossibility.
//
// The cycle detection uses the directed-graph package's HasCycle method,
// which performs a DFS-based topological sort check.

func TestValidateSpecBytes_CircularRequires(t *testing.T) {
	spec := `{
		"cli_builder_spec_version": "1.0",
		"name": "myapp",
		"description": "A test application",
		"flags": [
			{
				"id": "alpha",
				"long": "alpha",
				"description": "First flag",
				"type": "boolean",
				"requires": ["beta"]
			},
			{
				"id": "beta",
				"long": "beta",
				"description": "Second flag",
				"type": "boolean",
				"requires": ["alpha"]
			}
		]
	}`

	result := ValidateSpecBytes([]byte(spec))

	if result.Valid {
		t.Fatal("expected Valid=false for circular requires dependency")
	}
	if len(result.Errors) == 0 {
		t.Fatal("expected at least one error")
	}
	// The error should mention "circular" or "cycle".
	errLower := strings.ToLower(result.Errors[0])
	if !strings.Contains(errLower, "circular") && !strings.Contains(errLower, "cycle") {
		t.Errorf("error should mention circular/cycle, got: %s", result.Errors[0])
	}
}

// =========================================================================
// Additional edge cases — making sure the API is robust.
// =========================================================================

// TestValidateSpecBytes_FullSpec validates the full spec from the existing
// test constants to make sure a realistic spec passes.
func TestValidateSpecBytes_FullSpec(t *testing.T) {
	result := ValidateSpecBytes([]byte(fullValidSpec))

	if !result.Valid {
		t.Errorf("expected Valid=true for full valid spec, got errors: %v", result.Errors)
	}
}

// TestValidateSpec_InvalidFileContent writes invalid JSON to a file and
// validates it — testing the interaction between file reading and JSON parsing.
func TestValidateSpec_InvalidFileContent(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "bad-spec.json")
	if err := os.WriteFile(path, []byte(`not valid json`), 0644); err != nil {
		t.Fatalf("failed to write temp file: %v", err)
	}

	result := ValidateSpec(path)

	if result.Valid {
		t.Fatal("expected Valid=false for file with invalid JSON")
	}
	if len(result.Errors) == 0 {
		t.Fatal("expected at least one error")
	}
}

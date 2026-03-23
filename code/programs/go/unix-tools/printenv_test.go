// =========================================================================
// printenv — Tests
// =========================================================================
//
// These tests verify the printenv tool's behavior, covering:
//
//   1. Spec loading
//   2. Printing a specific variable
//   3. Printing all environment variables
//   4. Missing variable (exit code 1)
//   5. Null-terminated output (-0)
//   6. Multiple variables
//   7. Help and version flags
//   8. Error handling (bad spec)

package main

import (
	"bytes"
	"os"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

// TestPrintenvSpecLoads verifies that printenv.json is a valid spec.
func TestPrintenvSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "printenv"), []string{"printenv"})
	if err != nil {
		t.Fatalf("failed to load printenv.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Specific variable tests
// =========================================================================

// TestPrintenvSpecificVar verifies printing a specific environment variable.
func TestPrintenvSpecificVar(t *testing.T) {
	// Set a test variable.
	os.Setenv("TEST_PRINTENV_VAR", "hello_printenv")
	defer os.Unsetenv("TEST_PRINTENV_VAR")

	var stdout, stderr bytes.Buffer
	code := runPrintenv(toolSpecPath(t, "printenv"), []string{"printenv", "TEST_PRINTENV_VAR"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runPrintenv(TEST_PRINTENV_VAR) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	expected := "hello_printenv\n"
	if stdout.String() != expected {
		t.Errorf("runPrintenv() output = %q, want %q", stdout.String(), expected)
	}
}

// TestPrintenvMissingVar verifies that a missing variable returns exit code 1.
func TestPrintenvMissingVar(t *testing.T) {
	// Ensure the variable doesn't exist.
	os.Unsetenv("NONEXISTENT_TEST_VAR_12345")

	var stdout, stderr bytes.Buffer
	code := runPrintenv(toolSpecPath(t, "printenv"), []string{"printenv", "NONEXISTENT_TEST_VAR_12345"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runPrintenv(missing) returned exit code %d, want 1", code)
	}

	if stdout.Len() != 0 {
		t.Errorf("runPrintenv(missing) should produce no stdout, got: %q", stdout.String())
	}
}

// TestPrintenvMultipleVars verifies printing multiple variables.
func TestPrintenvMultipleVars(t *testing.T) {
	os.Setenv("TEST_PE_A", "alpha")
	os.Setenv("TEST_PE_B", "beta")
	defer os.Unsetenv("TEST_PE_A")
	defer os.Unsetenv("TEST_PE_B")

	var stdout, stderr bytes.Buffer
	code := runPrintenv(toolSpecPath(t, "printenv"), []string{"printenv", "TEST_PE_A", "TEST_PE_B"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runPrintenv(multi) returned exit code %d, want 0", code)
	}

	output := stdout.String()
	if !strings.Contains(output, "alpha") || !strings.Contains(output, "beta") {
		t.Errorf("runPrintenv(multi) output = %q, should contain alpha and beta", output)
	}
}

// =========================================================================
// All variables tests
// =========================================================================

// TestPrintenvAllVars verifies that printing all variables includes
// a known variable.
func TestPrintenvAllVars(t *testing.T) {
	os.Setenv("TEST_PRINTENV_ALL", "found_me")
	defer os.Unsetenv("TEST_PRINTENV_ALL")

	var stdout, stderr bytes.Buffer
	code := runPrintenv(toolSpecPath(t, "printenv"), []string{"printenv"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runPrintenv(all) returned exit code %d, want 0", code)
	}

	output := stdout.String()
	if !strings.Contains(output, "TEST_PRINTENV_ALL=found_me") {
		t.Errorf("runPrintenv(all) should contain TEST_PRINTENV_ALL=found_me, got length %d", len(output))
	}
}

// =========================================================================
// Null-terminated output tests
// =========================================================================

// TestPrintenvNullTerminated verifies -0 uses NUL terminator.
func TestPrintenvNullTerminated(t *testing.T) {
	os.Setenv("TEST_PE_NULL", "value")
	defer os.Unsetenv("TEST_PE_NULL")

	var stdout, stderr bytes.Buffer
	code := runPrintenv(toolSpecPath(t, "printenv"), []string{"printenv", "-0", "TEST_PE_NULL"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runPrintenv(-0) returned exit code %d, want 0", code)
	}

	if stdout.String() != "value\x00" {
		t.Errorf("runPrintenv(-0) output = %q, want %q", stdout.String(), "value\x00")
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestPrintenvHelpFlag verifies --help.
func TestPrintenvHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runPrintenv(toolSpecPath(t, "printenv"), []string{"printenv", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runPrintenv(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runPrintenv(--help) produced no stdout output")
	}
}

// TestPrintenvVersionFlag verifies --version.
func TestPrintenvVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runPrintenv(toolSpecPath(t, "printenv"), []string{"printenv", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runPrintenv(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runPrintenv(--version) = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestPrintenvInvalidSpec verifies bad spec path returns exit code 1.
func TestPrintenvInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runPrintenv("/nonexistent/printenv.json", []string{"printenv"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runPrintenv(bad spec) returned exit code %d, want 1", code)
	}
}

// TestPrintenvMixedExistence verifies partial success with some missing vars.
func TestPrintenvMixedExistence(t *testing.T) {
	os.Setenv("TEST_PE_EXISTS", "yes")
	os.Unsetenv("TEST_PE_NOEXIST")
	defer os.Unsetenv("TEST_PE_EXISTS")

	var stdout, stderr bytes.Buffer
	code := runPrintenv(toolSpecPath(t, "printenv"), []string{"printenv", "TEST_PE_EXISTS", "TEST_PE_NOEXIST"}, &stdout, &stderr)

	// Should return 1 because one var is missing.
	if code != 1 {
		t.Errorf("runPrintenv(mixed) returned exit code %d, want 1", code)
	}

	// But the existing var should still be printed.
	if !strings.Contains(stdout.String(), "yes") {
		t.Errorf("runPrintenv(mixed) should still print existing var, got: %q", stdout.String())
	}
}

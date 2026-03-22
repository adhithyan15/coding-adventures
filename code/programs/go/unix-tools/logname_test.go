// =========================================================================
// logname — Tests
// =========================================================================
//
// These tests verify the logname tool's behavior:
//
//   1. Default: prints the login name from $LOGNAME or $USER
//   2. Missing login name: prints error and exits 1
//   3. $LOGNAME takes precedence over $USER
//   4. --help and --version: standard meta-flags
//   5. Error handling: invalid spec path returns exit code 1

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

// TestLognameSpecLoads verifies that logname.json is a valid cli-builder spec.
func TestLognameSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "logname"), []string{"logname"})
	if err != nil {
		t.Fatalf("failed to load logname.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Business logic tests
// =========================================================================

// TestGetLoginNameReturnsLogname verifies that getLoginName prefers $LOGNAME.
func TestGetLoginNameReturnsLogname(t *testing.T) {
	// Save and restore environment.
	origLogname := os.Getenv("LOGNAME")
	origUser := os.Getenv("USER")
	defer func() {
		os.Setenv("LOGNAME", origLogname)
		os.Setenv("USER", origUser)
	}()

	os.Setenv("LOGNAME", "testlogin")
	os.Setenv("USER", "testuser")

	result := getLoginName()
	if result != "testlogin" {
		t.Errorf("getLoginName() = %q, want %q (should prefer $LOGNAME)", result, "testlogin")
	}
}

// TestGetLoginNameFallsBackToUser verifies that getLoginName uses $USER
// when $LOGNAME is empty.
func TestGetLoginNameFallsBackToUser(t *testing.T) {
	origLogname := os.Getenv("LOGNAME")
	origUser := os.Getenv("USER")
	defer func() {
		os.Setenv("LOGNAME", origLogname)
		os.Setenv("USER", origUser)
	}()

	os.Unsetenv("LOGNAME")
	os.Setenv("USER", "fallbackuser")

	result := getLoginName()
	if result != "fallbackuser" {
		t.Errorf("getLoginName() = %q, want %q (should fallback to $USER)", result, "fallbackuser")
	}
}

// TestGetLoginNameReturnsEmptyWhenBothMissing verifies that getLoginName
// returns empty when both $LOGNAME and $USER are unset.
func TestGetLoginNameReturnsEmptyWhenBothMissing(t *testing.T) {
	origLogname := os.Getenv("LOGNAME")
	origUser := os.Getenv("USER")
	defer func() {
		os.Setenv("LOGNAME", origLogname)
		os.Setenv("USER", origUser)
	}()

	os.Unsetenv("LOGNAME")
	os.Unsetenv("USER")

	result := getLoginName()
	if result != "" {
		t.Errorf("getLoginName() = %q, want empty string", result)
	}
}

// =========================================================================
// runLogname integration tests
// =========================================================================

// TestRunLognameDefault verifies that logname prints a non-empty login name.
func TestRunLognameDefault(t *testing.T) {
	// Make sure at least $USER is set for this test.
	if os.Getenv("LOGNAME") == "" && os.Getenv("USER") == "" {
		t.Skip("neither $LOGNAME nor $USER is set")
	}

	var stdout, stderr bytes.Buffer
	code := runLogname(toolSpecPath(t, "logname"), []string{"logname"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLogname() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if output == "" {
		t.Error("runLogname() produced no output")
	}
}

// TestRunLognameNoLoginName verifies that logname exits 1 when no login
// name is available.
func TestRunLognameNoLoginName(t *testing.T) {
	origLogname := os.Getenv("LOGNAME")
	origUser := os.Getenv("USER")
	defer func() {
		os.Setenv("LOGNAME", origLogname)
		os.Setenv("USER", origUser)
	}()

	os.Unsetenv("LOGNAME")
	os.Unsetenv("USER")

	var stdout, stderr bytes.Buffer
	code := runLogname(toolSpecPath(t, "logname"), []string{"logname"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runLogname() with no login name returned exit code %d, want 1", code)
	}

	if !strings.Contains(stderr.String(), "no login name") {
		t.Errorf("stderr = %q, want it to contain 'no login name'", stderr.String())
	}
}

// TestRunLognameOutputMatchesGetLoginName verifies that runLogname output
// matches getLoginName().
func TestRunLognameOutputMatchesGetLoginName(t *testing.T) {
	expected := getLoginName()
	if expected == "" {
		t.Skip("no login name available")
	}

	var stdout, stderr bytes.Buffer
	code := runLogname(toolSpecPath(t, "logname"), []string{"logname"}, &stdout, &stderr)

	if code != 0 {
		t.Fatalf("runLogname() failed: %s", stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if output != expected {
		t.Errorf("runLogname() = %q, want %q", output, expected)
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestLognameHelpFlag verifies that --help prints help text and returns 0.
func TestLognameHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLogname(toolSpecPath(t, "logname"), []string{"logname", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLogname(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runLogname(--help) produced no stdout output")
	}
}

// TestLognameVersionFlag verifies that --version prints the version.
func TestLognameVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLogname(toolSpecPath(t, "logname"), []string{"logname", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLogname(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runLogname(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestLognameInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestLognameInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLogname("/nonexistent/logname.json", []string{"logname"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runLogname(bad spec) returned exit code %d, want 1", code)
	}
}

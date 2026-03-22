// =========================================================================
// whoami — Tests
// =========================================================================
//
// These tests verify the whoami tool's behavior:
//
//   1. Default: prints the current effective username
//   2. --help: prints help text
//   3. --version: prints version string
//   4. Error handling: invalid spec path returns exit code 1
//   5. getEffectiveUsername: returns a non-empty username

package main

import (
	"bytes"
	"os"
	"os/user"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

// TestWhoamiSpecLoads verifies that whoami.json is a valid cli-builder spec.
func TestWhoamiSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "whoami"), []string{"whoami"})
	if err != nil {
		t.Fatalf("failed to load whoami.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Business logic tests
// =========================================================================

// TestGetEffectiveUsername verifies that getEffectiveUsername returns a
// non-empty string matching the current user.
func TestGetEffectiveUsername(t *testing.T) {
	username, err := getEffectiveUsername()
	if err != nil {
		t.Fatalf("getEffectiveUsername() failed: %v", err)
	}

	if username == "" {
		t.Error("getEffectiveUsername() returned empty string")
	}
}

// TestGetEffectiveUsernameMatchesOsUser verifies that getEffectiveUsername
// returns the same value as os/user.Current().
func TestGetEffectiveUsernameMatchesOsUser(t *testing.T) {
	expected, err := user.Current()
	if err != nil {
		t.Skipf("user.Current() failed: %v — skipping comparison test", err)
	}

	username, err := getEffectiveUsername()
	if err != nil {
		t.Fatalf("getEffectiveUsername() failed: %v", err)
	}

	if username != expected.Username {
		t.Errorf("getEffectiveUsername() = %q, want %q", username, expected.Username)
	}
}

// TestGetEffectiveUsernameFallsBackToEnv verifies that when user.Current()
// would normally succeed, the result matches $USER.
func TestGetEffectiveUsernameMatchesEnv(t *testing.T) {
	envUser := os.Getenv("USER")
	if envUser == "" {
		t.Skip("$USER not set — skipping env comparison test")
	}

	username, err := getEffectiveUsername()
	if err != nil {
		t.Fatalf("getEffectiveUsername() failed: %v", err)
	}

	// On most systems, user.Current().Username == $USER.
	// If they differ, that's a valid system configuration, so we just log it.
	if username != envUser {
		t.Logf("Note: getEffectiveUsername() = %q, $USER = %q (may differ on some systems)", username, envUser)
	}
}

// =========================================================================
// runWhoami integration tests
// =========================================================================

// TestRunWhoamiDefault verifies that whoami prints a non-empty username.
func TestRunWhoamiDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runWhoami(toolSpecPath(t, "whoami"), []string{"whoami"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runWhoami() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if output == "" {
		t.Error("runWhoami() produced no output")
	}
}

// TestRunWhoamiMatchesUser verifies that the output matches the actual user.
func TestRunWhoamiMatchesUser(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runWhoami(toolSpecPath(t, "whoami"), []string{"whoami"}, &stdout, &stderr)

	if code != 0 {
		t.Fatalf("runWhoami() failed: %s", stderr.String())
	}

	expected, err := getEffectiveUsername()
	if err != nil {
		t.Fatalf("getEffectiveUsername() failed: %v", err)
	}

	output := strings.TrimSpace(stdout.String())
	if output != expected {
		t.Errorf("runWhoami() = %q, want %q", output, expected)
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestWhoamiHelpFlag verifies that --help prints help text and returns 0.
func TestWhoamiHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runWhoami(toolSpecPath(t, "whoami"), []string{"whoami", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runWhoami(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runWhoami(--help) produced no stdout output")
	}
}

// TestWhoamiVersionFlag verifies that --version prints the version.
func TestWhoamiVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runWhoami(toolSpecPath(t, "whoami"), []string{"whoami", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runWhoami(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runWhoami(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestWhoamiInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestWhoamiInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runWhoami("/nonexistent/whoami.json", []string{"whoami"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runWhoami(bad spec) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// groups — Tests
// =========================================================================

package main

import (
	"bytes"
	"strings"
	"testing"
)

// =========================================================================
// Business logic tests
// =========================================================================

// TestGetGroupsCurrent verifies that getGroups returns groups for current user.
func TestGetGroupsCurrent(t *testing.T) {
	groups, err := getGroups("")
	if err != nil {
		t.Fatalf("getGroups(\"\") failed: %v", err)
	}

	if len(groups) == 0 {
		t.Error("getGroups(\"\") returned no groups")
	}

	// Each group name should be non-empty.
	for i, g := range groups {
		if g == "" {
			t.Errorf("groups[%d] is empty", i)
		}
	}
}

// TestGetGroupsInvalidUser verifies error for nonexistent user.
func TestGetGroupsInvalidUser(t *testing.T) {
	_, err := getGroups("nonexistent_user_xyzzy_99999")
	if err == nil {
		t.Error("getGroups(invalid) should have returned an error")
	}
}

// =========================================================================
// runGroups integration tests
// =========================================================================

// TestRunGroupsDefault verifies default output (current user's groups).
func TestRunGroupsDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runGroups(toolSpecPath(t, "groups"), []string{"groups"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGroups() returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if output == "" {
		t.Error("runGroups() produced no output")
	}
}

// TestRunGroupsHelp verifies --help flag.
func TestRunGroupsHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runGroups(toolSpecPath(t, "groups"), []string{"groups", "--help"}, &stdout, &stderr)
	if code != 0 {
		t.Errorf("runGroups(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runGroups(--help) produced no output")
	}
}

// TestRunGroupsVersion verifies --version flag.
func TestRunGroupsVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runGroups(toolSpecPath(t, "groups"), []string{"groups", "--version"}, &stdout, &stderr)
	if code != 0 {
		t.Errorf("runGroups(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runGroups(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunGroupsInvalidSpec verifies error handling for bad spec path.
func TestRunGroupsInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runGroups("/nonexistent/groups.json", []string{"groups"}, &stdout, &stderr)
	if code != 1 {
		t.Errorf("runGroups(bad spec) returned %d, want 1", code)
	}
}

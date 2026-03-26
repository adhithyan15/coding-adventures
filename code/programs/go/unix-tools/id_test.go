// =========================================================================
// id — Tests
// =========================================================================
//
// These tests verify the id tool's behavior:
//   1. Business logic: getUserInfo(), formatId()
//   2. Integration: runId() with various flags
//   3. Help/version flags, error handling

package main

import (
	"bytes"
	"os/user"
	"strings"
	"testing"
)

// =========================================================================
// Business logic tests
// =========================================================================

// TestGetUserInfoCurrent verifies that getUserInfo returns info for current user.
func TestGetUserInfoCurrent(t *testing.T) {
	info, err := getUserInfo("")
	if err != nil {
		t.Fatalf("getUserInfo(\"\") failed: %v", err)
	}

	if info.Uid == "" {
		t.Error("Uid is empty")
	}
	if info.Username == "" {
		t.Error("Username is empty")
	}
	if info.Gid == "" {
		t.Error("Gid is empty")
	}
	if len(info.Groups) == 0 {
		t.Error("Groups is empty")
	}
}

// TestGetUserInfoMatchesOsUser checks that getUserInfo matches os/user.Current().
func TestGetUserInfoMatchesOsUser(t *testing.T) {
	u, err := user.Current()
	if err != nil {
		t.Skipf("user.Current() failed: %v", err)
	}

	info, err := getUserInfo("")
	if err != nil {
		t.Fatalf("getUserInfo(\"\") failed: %v", err)
	}

	if info.Uid != u.Uid {
		t.Errorf("Uid = %q, want %q", info.Uid, u.Uid)
	}
	if info.Username != u.Username {
		t.Errorf("Username = %q, want %q", info.Username, u.Username)
	}
}

// TestGetUserInfoInvalidUser verifies error for nonexistent user.
func TestGetUserInfoInvalidUser(t *testing.T) {
	_, err := getUserInfo("nonexistent_user_xyzzy_12345")
	if err == nil {
		t.Error("getUserInfo(invalid) should have returned an error")
	}
}

// TestFormatIdDefault verifies the default (full) output format.
func TestFormatIdDefault(t *testing.T) {
	info := &IdInfo{
		Uid:      "501",
		Username: "alice",
		Gid:      "20",
		Gname:    "staff",
		Groups:   []string{"20", "501"},
		Gnames:   []string{"staff", "access_bpf"},
	}

	got := formatId(info, false, false, false, false)
	want := "uid=501(alice) gid=20(staff) groups=20(staff),501(access_bpf)"
	if got != want {
		t.Errorf("formatId(default) = %q, want %q", got, want)
	}
}

// TestFormatIdUserOnly verifies -u flag (numeric UID).
func TestFormatIdUserOnly(t *testing.T) {
	info := &IdInfo{Uid: "501", Username: "alice"}

	got := formatId(info, true, false, false, false)
	if got != "501" {
		t.Errorf("formatId(-u) = %q, want %q", got, "501")
	}
}

// TestFormatIdUserName verifies -un flags (username).
func TestFormatIdUserName(t *testing.T) {
	info := &IdInfo{Uid: "501", Username: "alice"}

	got := formatId(info, true, false, false, true)
	if got != "alice" {
		t.Errorf("formatId(-un) = %q, want %q", got, "alice")
	}
}

// TestFormatIdGroupOnly verifies -g flag (numeric GID).
func TestFormatIdGroupOnly(t *testing.T) {
	info := &IdInfo{Gid: "20", Gname: "staff"}

	got := formatId(info, false, true, false, false)
	if got != "20" {
		t.Errorf("formatId(-g) = %q, want %q", got, "20")
	}
}

// TestFormatIdGroupName verifies -gn flags (group name).
func TestFormatIdGroupName(t *testing.T) {
	info := &IdInfo{Gid: "20", Gname: "staff"}

	got := formatId(info, false, true, false, true)
	if got != "staff" {
		t.Errorf("formatId(-gn) = %q, want %q", got, "staff")
	}
}

// TestFormatIdGroups verifies -G flag (all group IDs).
func TestFormatIdGroups(t *testing.T) {
	info := &IdInfo{Groups: []string{"20", "501", "12"}, Gnames: []string{"staff", "bpf", "everyone"}}

	got := formatId(info, false, false, true, false)
	if got != "20 501 12" {
		t.Errorf("formatId(-G) = %q, want %q", got, "20 501 12")
	}
}

// TestFormatIdGroupNames verifies -Gn flags (all group names).
func TestFormatIdGroupNames(t *testing.T) {
	info := &IdInfo{Groups: []string{"20", "501"}, Gnames: []string{"staff", "bpf"}}

	got := formatId(info, false, false, true, true)
	if got != "staff bpf" {
		t.Errorf("formatId(-Gn) = %q, want %q", got, "staff bpf")
	}
}

// =========================================================================
// runId integration tests
// =========================================================================

// TestRunIdDefault verifies default output contains uid= and gid=.
func TestRunIdDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runId(toolSpecPath(t, "id"), []string{"id"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runId() returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	if !strings.Contains(output, "uid=") || !strings.Contains(output, "gid=") {
		t.Errorf("runId() output missing uid=/gid=: %q", output)
	}
}

// TestRunIdHelp verifies --help flag.
func TestRunIdHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runId(toolSpecPath(t, "id"), []string{"id", "--help"}, &stdout, &stderr)
	if code != 0 {
		t.Errorf("runId(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runId(--help) produced no output")
	}
}

// TestRunIdVersion verifies --version flag.
func TestRunIdVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runId(toolSpecPath(t, "id"), []string{"id", "--version"}, &stdout, &stderr)
	if code != 0 {
		t.Errorf("runId(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runId(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunIdInvalidSpec verifies error handling for bad spec path.
func TestRunIdInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runId("/nonexistent/id.json", []string{"id"}, &stdout, &stderr)
	if code != 1 {
		t.Errorf("runId(bad spec) returned %d, want 1", code)
	}
}

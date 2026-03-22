// =========================================================================
// chown — Tests
// =========================================================================
//
// These tests verify the chown tool's behavior, covering:
//
//   1. Spec loading
//   2. Ownership spec parsing (OWNER, OWNER:GROUP, :GROUP, etc.)
//   3. UID/GID resolution
//   4. Applying ownership changes (may fail on non-root systems)
//   5. Verbose and changes-only output
//   6. Error handling (invalid users/groups, missing files)
//
// IMPORTANT NOTE: On most systems, only root can change file ownership.
// These tests are designed to handle EPERM gracefully — they test the
// parsing and logic without requiring root privileges.

package main

import (
	"bytes"
	"os"
	"os/user"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading test
// =========================================================================

func TestChownSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "chown"), []string{"chown", "root", "file"})
	if err != nil {
		t.Fatalf("failed to load chown.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// parseChownSpec tests
// =========================================================================

func TestParseChownSpecCurrentUser(t *testing.T) {
	// Get current user — this should always resolve.
	u, err := user.Current()
	if err != nil {
		t.Skip("cannot get current user")
	}

	spec, err := parseChownSpec(u.Username)
	if err != nil {
		t.Fatalf("parseChownSpec(%q) error: %v", u.Username, err)
	}

	expectedUID, _ := strconv.Atoi(u.Uid)
	if spec.UID != expectedUID {
		t.Errorf("UID = %d, want %d", spec.UID, expectedUID)
	}
	if spec.GID != -1 {
		t.Errorf("GID = %d, want -1 (not specified)", spec.GID)
	}
}

func TestParseChownSpecNumericUID(t *testing.T) {
	spec, err := parseChownSpec("1000")
	if err != nil {
		t.Fatalf("parseChownSpec(\"1000\") error: %v", err)
	}
	if spec.UID != 1000 {
		t.Errorf("UID = %d, want 1000", spec.UID)
	}
}

func TestParseChownSpecNumericUIDAndGID(t *testing.T) {
	spec, err := parseChownSpec("1000:100")
	if err != nil {
		t.Fatalf("parseChownSpec(\"1000:100\") error: %v", err)
	}
	if spec.UID != 1000 {
		t.Errorf("UID = %d, want 1000", spec.UID)
	}
	if spec.GID != 100 {
		t.Errorf("GID = %d, want 100", spec.GID)
	}
}

func TestParseChownSpecGroupOnly(t *testing.T) {
	spec, err := parseChownSpec(":100")
	if err != nil {
		t.Fatalf("parseChownSpec(\":100\") error: %v", err)
	}
	if spec.UID != -1 {
		t.Errorf("UID = %d, want -1 (not specified)", spec.UID)
	}
	if spec.GID != 100 {
		t.Errorf("GID = %d, want 100", spec.GID)
	}
}

func TestParseChownSpecOwnerColonOnly(t *testing.T) {
	spec, err := parseChownSpec("1000:")
	if err != nil {
		t.Fatalf("parseChownSpec(\"1000:\") error: %v", err)
	}
	if spec.UID != 1000 {
		t.Errorf("UID = %d, want 1000", spec.UID)
	}
	if spec.GID != -1 {
		t.Errorf("GID = %d, want -1 (group not specified)", spec.GID)
	}
}

func TestParseChownSpecDotSeparator(t *testing.T) {
	spec, err := parseChownSpec("1000.100")
	if err != nil {
		t.Fatalf("parseChownSpec(\"1000.100\") error: %v", err)
	}
	if spec.UID != 1000 || spec.GID != 100 {
		t.Errorf("UID=%d GID=%d, want 1000:100", spec.UID, spec.GID)
	}
}

func TestParseChownSpecInvalidUser(t *testing.T) {
	_, err := parseChownSpec("nonexistent_user_xyz_12345")
	if err == nil {
		t.Error("parseChownSpec with invalid user should return error")
	}
}

func TestParseChownSpecInvalidGroup(t *testing.T) {
	_, err := parseChownSpec("1000:nonexistent_group_xyz_12345")
	if err == nil {
		t.Error("parseChownSpec with invalid group should return error")
	}
}

// =========================================================================
// formatChownOwner tests
// =========================================================================

func TestFormatChownOwnerName(t *testing.T) {
	result := formatChownOwner(ChownSpec{UID: 1000, GID: -1, OwnerName: "alice"})
	if result != "alice" {
		t.Errorf("formatChownOwner = %q, want \"alice\"", result)
	}
}

func TestFormatChownOwnerAndGroup(t *testing.T) {
	result := formatChownOwner(ChownSpec{UID: 1000, GID: 100, OwnerName: "alice", GroupName: "staff"})
	if result != "alice:staff" {
		t.Errorf("formatChownOwner = %q, want \"alice:staff\"", result)
	}
}

func TestFormatChownGroupOnly(t *testing.T) {
	result := formatChownOwner(ChownSpec{UID: -1, GID: 100, GroupName: "staff"})
	if result != ":staff" {
		t.Errorf("formatChownOwner = %q, want \":staff\"", result)
	}
}

// =========================================================================
// Integration tests — chown on actual files
// =========================================================================

func TestChownCurrentUserOnFile(t *testing.T) {
	// Try to chown a file to the current user — this should succeed
	// even without root privileges.
	u, err := user.Current()
	if err != nil {
		t.Skip("cannot get current user")
	}

	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runChown(toolSpecPath(t, "chown"),
		[]string{"chown", u.Uid, file},
		&stdout, &stderr)

	// May succeed or fail depending on OS — just check it doesn't panic.
	if rc != 0 && rc != 1 {
		t.Errorf("exit code = %d, want 0 or 1", rc)
	}
}

func TestChownVerbose(t *testing.T) {
	u, err := user.Current()
	if err != nil {
		t.Skip("cannot get current user")
	}

	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runChown(toolSpecPath(t, "chown"),
		[]string{"chown", "-v", u.Uid, file},
		&stdout, &stderr)

	// If the chown succeeded, check verbose output.
	if rc == 0 {
		if !strings.Contains(stdout.String(), "ownership") {
			t.Errorf("verbose output should mention ownership, got %q", stdout.String())
		}
	}
}

// =========================================================================
// Error handling
// =========================================================================

func TestChownMissingFile(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runChown(toolSpecPath(t, "chown"),
		[]string{"chown", "1000", "/nonexistent/file"},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}
}

func TestChownInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runChown("/nonexistent/chown.json", []string{"chown", "root", "file"}, &stdout, &stderr)
	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}
}

func TestChownInvalidUser(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runChown(toolSpecPath(t, "chown"),
		[]string{"chown", "nonexistent_user_xyz_12345", file},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1 for invalid user", rc)
	}
}

// =========================================================================
// resolveChownUser / resolveChownGroup tests
// =========================================================================

func TestResolveChownUserNumeric(t *testing.T) {
	uid, err := resolveChownUser("1000")
	if err != nil {
		t.Fatalf("resolveChownUser(\"1000\") error: %v", err)
	}
	if uid != 1000 {
		t.Errorf("uid = %d, want 1000", uid)
	}
}

func TestResolveChownGroupNumeric(t *testing.T) {
	gid, err := resolveChownGroup("100")
	if err != nil {
		t.Fatalf("resolveChownGroup(\"100\") error: %v", err)
	}
	if gid != 100 {
		t.Errorf("gid = %d, want 100", gid)
	}
}

func TestResolveChownGroupByName(t *testing.T) {
	// Try resolving the current user's primary group.
	u, err := user.Current()
	if err != nil {
		t.Skip("cannot get current user")
	}

	// The user's GID should be resolvable.
	gid, err := resolveChownGroup(u.Gid)
	if err != nil {
		t.Fatalf("resolveChownGroup(%q) error: %v", u.Gid, err)
	}

	expectedGID, _ := strconv.Atoi(u.Gid)
	if gid != expectedGID {
		t.Errorf("gid = %d, want %d", gid, expectedGID)
	}
}

func TestChownRecursive(t *testing.T) {
	u, err := user.Current()
	if err != nil {
		t.Skip("cannot get current user")
	}

	dir := t.TempDir()
	subdir := filepath.Join(dir, "sub")
	os.MkdirAll(subdir, 0755)
	os.WriteFile(filepath.Join(subdir, "file.txt"), []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runChown(toolSpecPath(t, "chown"),
		[]string{"chown", "-R", u.Uid, dir},
		&stdout, &stderr)

	// May succeed or fail depending on OS permissions.
	if rc != 0 && rc != 1 {
		t.Errorf("exit code = %d, want 0 or 1", rc)
	}
}

func TestResolveChownUserByName(t *testing.T) {
	u, err := user.Current()
	if err != nil {
		t.Skip("cannot get current user")
	}

	uid, err := resolveChownUser(u.Username)
	if err != nil {
		t.Fatalf("resolveChownUser(%q) error: %v", u.Username, err)
	}

	expectedUID, _ := strconv.Atoi(u.Uid)
	if uid != expectedUID {
		t.Errorf("uid = %d, want %d", uid, expectedUID)
	}
}

func TestFormatChownOwnerNumericUID(t *testing.T) {
	result := formatChownOwner(ChownSpec{UID: 1000, GID: -1})
	if result != "1000" {
		t.Errorf("formatChownOwner = %q, want \"1000\"", result)
	}
}

func TestFormatChownOwnerNumericUIDAndGID(t *testing.T) {
	result := formatChownOwner(ChownSpec{UID: 1000, GID: 100})
	if result != "1000:100" {
		t.Errorf("formatChownOwner = %q, want \"1000:100\"", result)
	}
}

func TestFormatChownGroupOnlyNumeric(t *testing.T) {
	result := formatChownOwner(ChownSpec{UID: -1, GID: 100})
	if result != ":100" {
		t.Errorf("formatChownOwner = %q, want \":100\"", result)
	}
}

func TestFormatChownEmpty(t *testing.T) {
	result := formatChownOwner(ChownSpec{UID: -1, GID: -1})
	if result != "" {
		t.Errorf("formatChownOwner = %q, want empty", result)
	}
}

func TestParseChownSpecColonSeparator(t *testing.T) {
	// Test owner:group with colon.
	spec, err := parseChownSpec("500:200")
	if err != nil {
		t.Fatalf("parseChownSpec error: %v", err)
	}
	if spec.UID != 500 || spec.GID != 200 {
		t.Errorf("spec = UID:%d GID:%d, want 500:200", spec.UID, spec.GID)
	}
}

func TestChownSilentMode(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runChown(toolSpecPath(t, "chown"),
		[]string{"chown", "-f", "1000", "/nonexistent/file"},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}
	if stderr.Len() != 0 {
		t.Errorf("silent mode should suppress errors, got %q", stderr.String())
	}
}

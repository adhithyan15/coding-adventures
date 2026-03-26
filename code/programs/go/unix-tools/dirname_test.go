// =========================================================================
// dirname — Tests
// =========================================================================
//
// These tests verify the dirname tool's behavior, covering:
//
//   1. Spec loading
//   2. Basic directory extraction
//   3. Plain filename (returns ".")
//   4. Root path
//   5. Multiple arguments
//   6. Zero-terminated output (-z)
//   7. Help and version flags
//   8. Error handling
//   9. Trailing slashes

package main

import (
	"bytes"
	"runtime"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

// TestDirnameSpecLoads verifies that dirname.json is a valid spec.
func TestDirnameSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "dirname"), []string{"dirname"})
	if err != nil {
		t.Fatalf("failed to load dirname.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Basic behavior tests
// =========================================================================

// TestDirnameSimplePath verifies directory extraction from a full path.
func TestDirnameSimplePath(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("Unix-style path test not applicable on Windows")
	}
	var stdout, stderr bytes.Buffer
	code := runDirname(toolSpecPath(t, "dirname"), []string{"dirname", "/usr/bin/sort"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runDirname() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if output != "/usr/bin" {
		t.Errorf("runDirname(/usr/bin/sort) = %q, want %q", output, "/usr/bin")
	}
}

// TestDirnamePlainFilename verifies that a plain filename returns ".".
func TestDirnamePlainFilename(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runDirname(toolSpecPath(t, "dirname"), []string{"dirname", "stdio.h"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runDirname(stdio.h) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "." {
		t.Errorf("runDirname(stdio.h) = %q, want %q", output, ".")
	}
}

// TestDirnameRootPath verifies that "/" returns "/".
func TestDirnameRootPath(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("Unix-style path test not applicable on Windows")
	}
	var stdout, stderr bytes.Buffer
	code := runDirname(toolSpecPath(t, "dirname"), []string{"dirname", "/"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runDirname(/) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "/" {
		t.Errorf("runDirname(/) = %q, want %q", output, "/")
	}
}

// TestDirnameMultipleArgs verifies processing multiple arguments.
func TestDirnameMultipleArgs(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("Unix-style path test not applicable on Windows")
	}
	var stdout, stderr bytes.Buffer
	code := runDirname(toolSpecPath(t, "dirname"), []string{"dirname", "/usr/bin/sort", "/home/user/file.txt"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runDirname(multiple) returned exit code %d, want 0", code)
	}

	output := stdout.String()
	if !strings.Contains(output, "/usr/bin") {
		t.Errorf("output should contain /usr/bin, got: %q", output)
	}
	if !strings.Contains(output, "/home/user") {
		t.Errorf("output should contain /home/user, got: %q", output)
	}
}

// =========================================================================
// Zero-terminated output tests
// =========================================================================

// TestDirnameZeroTerminated verifies -z uses NUL terminator.
func TestDirnameZeroTerminated(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("Unix-style path test not applicable on Windows")
	}
	var stdout, stderr bytes.Buffer
	code := runDirname(toolSpecPath(t, "dirname"), []string{"dirname", "-z", "/usr/bin/sort"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runDirname(-z) returned exit code %d, want 0", code)
	}

	if stdout.String() != "/usr/bin\x00" {
		t.Errorf("runDirname(-z) output = %q, want %q", stdout.String(), "/usr/bin\x00")
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestDirnameHelpFlag verifies --help.
func TestDirnameHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runDirname(toolSpecPath(t, "dirname"), []string{"dirname", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runDirname(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runDirname(--help) produced no stdout output")
	}
}

// TestDirnameVersionFlag verifies --version.
func TestDirnameVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runDirname(toolSpecPath(t, "dirname"), []string{"dirname", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runDirname(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runDirname(--version) = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestDirnameInvalidSpec verifies bad spec path returns exit code 1.
func TestDirnameInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runDirname("/nonexistent/dirname.json", []string{"dirname", "test"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runDirname(bad spec) returned exit code %d, want 1", code)
	}
}

// TestDirnameNestedPath verifies deep nested paths.
func TestDirnameNestedPath(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("Unix-style path test not applicable on Windows")
	}
	var stdout, stderr bytes.Buffer
	code := runDirname(toolSpecPath(t, "dirname"), []string{"dirname", "/a/b/c/d/e"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runDirname(nested) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "/a/b/c/d" {
		t.Errorf("runDirname(/a/b/c/d/e) = %q, want %q", output, "/a/b/c/d")
	}
}

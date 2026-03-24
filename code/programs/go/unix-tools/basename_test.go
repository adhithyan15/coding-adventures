// =========================================================================
// basename — Tests
// =========================================================================
//
// These tests verify the basename tool's behavior, covering:
//
//   1. Spec loading
//   2. Basic path stripping
//   3. Suffix removal
//   4. Multiple mode (-a)
//   5. Suffix flag (-s) with multiple names
//   6. Zero-terminated output (-z)
//   7. Help and version flags
//   8. Error handling (missing operand)
//   9. Edge cases (root path, trailing slashes)

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

// TestBasenameSpecLoads verifies that basename.json is a valid spec.
func TestBasenameSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "basename"), []string{"basename"})
	if err != nil {
		t.Fatalf("failed to load basename.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Basic behavior tests
// =========================================================================

// TestBasenameSimplePath verifies stripping a directory from a path.
func TestBasenameSimplePath(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runBasename(toolSpecPath(t, "basename"), []string{"basename", "/usr/bin/sort"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runBasename() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if output != "sort" {
		t.Errorf("runBasename(/usr/bin/sort) = %q, want %q", output, "sort")
	}
}

// TestBasenameWithSuffix verifies suffix removal using positional argument.
func TestBasenameWithSuffix(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runBasename(toolSpecPath(t, "basename"), []string{"basename", "/home/user/file.txt", ".txt"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runBasename(suffix) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "file" {
		t.Errorf("runBasename(file.txt, .txt) = %q, want %q", output, "file")
	}
}

// TestBasenameNoDirectory verifies behavior with a plain filename.
func TestBasenameNoDirectory(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runBasename(toolSpecPath(t, "basename"), []string{"basename", "stdio.h"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runBasename(stdio.h) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "stdio.h" {
		t.Errorf("runBasename(stdio.h) = %q, want %q", output, "stdio.h")
	}
}

// TestBasenameRootPath verifies behavior with the root path.
func TestBasenameRootPath(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("root path behavior differs on Windows")
	}
	var stdout, stderr bytes.Buffer
	code := runBasename(toolSpecPath(t, "basename"), []string{"basename", "/"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runBasename(/) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "/" {
		t.Errorf("runBasename(/) = %q, want %q", output, "/")
	}
}

// =========================================================================
// Multiple mode tests
// =========================================================================

// TestBasenameSuffixFlag verifies -s flag with multiple names.
func TestBasenameSuffixFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runBasename(toolSpecPath(t, "basename"), []string{"basename", "-s", ".go", "main.go", "test.go"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runBasename(-s .go) returned exit code %d, want 0", code)
	}

	output := stdout.String()
	if !strings.Contains(output, "main") || !strings.Contains(output, "test") {
		t.Errorf("runBasename(-s .go) output = %q, should contain 'main' and 'test'", output)
	}
}

// TestBasenameMultipleFlag verifies -a flag processes all args as names.
func TestBasenameMultipleFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runBasename(toolSpecPath(t, "basename"), []string{"basename", "-a", "/usr/bin/sort", "/usr/bin/head"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runBasename(-a) returned exit code %d, want 0", code)
	}

	output := stdout.String()
	if !strings.Contains(output, "sort") || !strings.Contains(output, "head") {
		t.Errorf("runBasename(-a) output = %q, should contain 'sort' and 'head'", output)
	}
}

// =========================================================================
// Zero-terminated output tests
// =========================================================================

// TestBasenameZeroTerminated verifies -z flag uses NUL terminator.
func TestBasenameZeroTerminated(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runBasename(toolSpecPath(t, "basename"), []string{"basename", "-z", "/usr/bin/sort"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runBasename(-z) returned exit code %d, want 0", code)
	}

	if stdout.String() != "sort\x00" {
		t.Errorf("runBasename(-z) output = %q, want %q", stdout.String(), "sort\x00")
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestBasenameHelpFlag verifies --help.
func TestBasenameHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runBasename(toolSpecPath(t, "basename"), []string{"basename", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runBasename(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runBasename(--help) produced no stdout output")
	}
}

// TestBasenameVersionFlag verifies --version.
func TestBasenameVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runBasename(toolSpecPath(t, "basename"), []string{"basename", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runBasename(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runBasename(--version) = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestBasenameInvalidSpec verifies bad spec path returns exit code 1.
func TestBasenameInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runBasename("/nonexistent/basename.json", []string{"basename", "test"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runBasename(bad spec) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// stripBasename unit tests
// =========================================================================

// TestStripBasenameUnit verifies the core stripping function.
func TestStripBasenameUnit(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("path separator behavior differs on Windows")
	}
	tests := []struct {
		name   string
		suffix string
		want   string
	}{
		{"/usr/bin/sort", "", "sort"},
		{"/home/user/file.txt", ".txt", "file"},
		{"stdio.h", "", "stdio.h"},
		{"/", "", "/"},
		{".", "", "."},
	}

	for _, tt := range tests {
		got := stripBasename(tt.name, tt.suffix)
		if got != tt.want {
			t.Errorf("stripBasename(%q, %q) = %q, want %q", tt.name, tt.suffix, got, tt.want)
		}
	}
}

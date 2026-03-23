// =========================================================================
// head — Tests
// =========================================================================
//
// These tests verify the head tool's behavior, covering:
//
//   1. Spec loading
//   2. Default behavior (first 10 lines)
//   3. Line count flag (-n)
//   4. Byte count flag (-c)
//   5. Quiet and verbose headers (-q, -v)
//   6. Multiple files with headers
//   7. Help and version flags
//   8. Error handling for nonexistent files
//   9. Reading from stdin
//  10. Zero-terminated mode (-z)

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

// TestHeadSpecLoads verifies that head.json is a valid cli-builder spec.
func TestHeadSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "head"), []string{"head"})
	if err != nil {
		t.Fatalf("failed to load head.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Default behavior tests
// =========================================================================

// TestHeadDefaultTenLines verifies that head prints the first 10 lines
// by default.
func TestHeadDefaultTenLines(t *testing.T) {
	// Create a file with 15 lines.
	var content strings.Builder
	for i := 1; i <= 15; i++ {
		content.WriteString(strings.Repeat("x", i) + "\n")
	}
	dir := t.TempDir()
	path := filepath.Join(dir, "test.txt")
	os.WriteFile(path, []byte(content.String()), 0644)

	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runHead() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	lines := strings.Split(strings.TrimSuffix(stdout.String(), "\n"), "\n")
	if len(lines) != 10 {
		t.Errorf("runHead() output %d lines, want 10", len(lines))
	}
}

// TestHeadStdinDefault verifies that head reads from stdin by default.
func TestHeadStdinDefault(t *testing.T) {
	input := "line1\nline2\nline3\n"
	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head"}, &stdout, &stderr, strings.NewReader(input))

	if code != 0 {
		t.Errorf("runHead(stdin) returned exit code %d, want 0", code)
	}

	expected := "line1\nline2\nline3\n"
	if stdout.String() != expected {
		t.Errorf("runHead(stdin) output = %q, want %q", stdout.String(), expected)
	}
}

// =========================================================================
// Line count flag tests
// =========================================================================

// TestHeadLineCount verifies that -n NUM prints exactly NUM lines.
func TestHeadLineCount(t *testing.T) {
	input := "a\nb\nc\nd\ne\n"
	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head", "-n", "3"}, &stdout, &stderr, strings.NewReader(input))

	if code != 0 {
		t.Errorf("runHead(-n 3) returned exit code %d, want 0", code)
	}

	expected := "a\nb\nc\n"
	if stdout.String() != expected {
		t.Errorf("runHead(-n 3) output = %q, want %q", stdout.String(), expected)
	}
}

// TestHeadLineCountExceedsFile verifies that -n NUM with more lines
// than the file just prints the entire file.
func TestHeadLineCountExceedsFile(t *testing.T) {
	input := "a\nb\n"
	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head", "-n", "100"}, &stdout, &stderr, strings.NewReader(input))

	if code != 0 {
		t.Errorf("runHead(-n 100) returned exit code %d, want 0", code)
	}

	if stdout.String() != "a\nb\n" {
		t.Errorf("runHead(-n 100) output = %q, want %q", stdout.String(), "a\nb\n")
	}
}

// =========================================================================
// Byte count flag tests
// =========================================================================

// TestHeadByteCount verifies that -c NUM prints exactly NUM bytes.
func TestHeadByteCount(t *testing.T) {
	input := "hello world\n"
	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head", "-c", "5"}, &stdout, &stderr, strings.NewReader(input))

	if code != 0 {
		t.Errorf("runHead(-c 5) returned exit code %d, want 0", code)
	}

	if stdout.String() != "hello" {
		t.Errorf("runHead(-c 5) output = %q, want %q", stdout.String(), "hello")
	}
}

// =========================================================================
// Header tests
// =========================================================================

// TestHeadMultipleFilesHeaders verifies that headers are shown for
// multiple files.
func TestHeadMultipleFilesHeaders(t *testing.T) {
	dir := t.TempDir()
	path1 := filepath.Join(dir, "file1.txt")
	path2 := filepath.Join(dir, "file2.txt")
	os.WriteFile(path1, []byte("hello\n"), 0644)
	os.WriteFile(path2, []byte("world\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head", path1, path2}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runHead(two files) returned exit code %d, want 0", code)
	}

	output := stdout.String()
	if !strings.Contains(output, "==> "+path1+" <==") {
		t.Errorf("output should contain header for file1, got: %q", output)
	}
	if !strings.Contains(output, "==> "+path2+" <==") {
		t.Errorf("output should contain header for file2, got: %q", output)
	}
}

// TestHeadVerboseFlag verifies that -v shows headers even for a single file.
func TestHeadVerboseFlag(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "file.txt")
	os.WriteFile(path, []byte("hello\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head", "-v", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runHead(-v) returned exit code %d, want 0", code)
	}

	if !strings.Contains(stdout.String(), "==> "+path+" <==") {
		t.Errorf("verbose mode should show header, got: %q", stdout.String())
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestHeadHelpFlag verifies that --help prints help text and returns 0.
func TestHeadHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runHead(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runHead(--help) produced no stdout output")
	}
}

// TestHeadVersionFlag verifies that --version prints the version.
func TestHeadVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runHead(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runHead(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestHeadInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestHeadInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin("/nonexistent/head.json", []string{"head"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runHead(bad spec) returned exit code %d, want 1", code)
	}
}

// TestHeadNonexistentFile verifies that a nonexistent file produces an error.
func TestHeadNonexistentFile(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head", "/nonexistent/file.txt"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runHead(nonexistent) returned exit code %d, want 1", code)
	}

	if stderr.Len() == 0 {
		t.Error("runHead(nonexistent) should produce stderr output")
	}
}

// =========================================================================
// splitOnNUL unit tests
// =========================================================================

// TestSplitOnNULBasic verifies the NUL-based line splitting function.
func TestSplitOnNULBasic(t *testing.T) {
	input := "hello\x00world\x00"
	var stdout, stderr bytes.Buffer
	code := runHeadWithStdin(toolSpecPath(t, "head"), []string{"head", "-z", "-n", "1"}, &stdout, &stderr, strings.NewReader(input))

	if code != 0 {
		t.Errorf("runHead(-z) returned exit code %d, want 0", code)
	}

	if stdout.String() != "hello\x00" {
		t.Errorf("runHead(-z) output = %q, want %q", stdout.String(), "hello\x00")
	}
}

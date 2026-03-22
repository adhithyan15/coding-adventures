// =========================================================================
// tee — Tests
// =========================================================================
//
// These tests verify the tee tool's behavior, covering:
//
//   1. Spec loading
//   2. Basic stdout passthrough
//   3. Writing to a file
//   4. Writing to multiple files
//   5. Append mode (-a)
//   6. Help and version flags
//   7. Error handling (bad file path)
//   8. Empty input

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

// TestTeeSpecLoads verifies that tee.json is a valid spec.
func TestTeeSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "tee"), []string{"tee"})
	if err != nil {
		t.Fatalf("failed to load tee.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Default behavior tests
// =========================================================================

// TestTeeStdoutPassthrough verifies that tee copies stdin to stdout.
func TestTeeStdoutPassthrough(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello world\n")
	code := runTeeWithStdin(toolSpecPath(t, "tee"), []string{"tee"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runTee() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	if stdout.String() != "hello world\n" {
		t.Errorf("runTee() stdout = %q, want %q", stdout.String(), "hello world\n")
	}
}

// TestTeeWriteToFile verifies that tee writes to both stdout and a file.
func TestTeeWriteToFile(t *testing.T) {
	dir := t.TempDir()
	filePath := filepath.Join(dir, "output.txt")

	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello tee\n")
	code := runTeeWithStdin(toolSpecPath(t, "tee"), []string{"tee", filePath}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runTee(file) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	// Check stdout.
	if stdout.String() != "hello tee\n" {
		t.Errorf("stdout = %q, want %q", stdout.String(), "hello tee\n")
	}

	// Check file contents.
	data, err := os.ReadFile(filePath)
	if err != nil {
		t.Fatalf("failed to read output file: %v", err)
	}
	if string(data) != "hello tee\n" {
		t.Errorf("file content = %q, want %q", string(data), "hello tee\n")
	}
}

// TestTeeMultipleFiles verifies writing to multiple files.
func TestTeeMultipleFiles(t *testing.T) {
	dir := t.TempDir()
	path1 := filepath.Join(dir, "out1.txt")
	path2 := filepath.Join(dir, "out2.txt")

	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("multi\n")
	code := runTeeWithStdin(toolSpecPath(t, "tee"), []string{"tee", path1, path2}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runTee(multi) returned exit code %d, want 0", code)
	}

	for _, p := range []string{path1, path2} {
		data, err := os.ReadFile(p)
		if err != nil {
			t.Fatalf("failed to read %s: %v", p, err)
		}
		if string(data) != "multi\n" {
			t.Errorf("file %s content = %q, want %q", p, string(data), "multi\n")
		}
	}
}

// =========================================================================
// Append mode tests
// =========================================================================

// TestTeeAppendMode verifies that -a appends instead of overwriting.
func TestTeeAppendMode(t *testing.T) {
	dir := t.TempDir()
	filePath := filepath.Join(dir, "append.txt")

	// Write initial content.
	os.WriteFile(filePath, []byte("first\n"), 0644)

	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("second\n")
	code := runTeeWithStdin(toolSpecPath(t, "tee"), []string{"tee", "-a", filePath}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runTee(-a) returned exit code %d, want 0", code)
	}

	data, err := os.ReadFile(filePath)
	if err != nil {
		t.Fatalf("failed to read file: %v", err)
	}

	expected := "first\nsecond\n"
	if string(data) != expected {
		t.Errorf("append mode file content = %q, want %q", string(data), expected)
	}
}

// =========================================================================
// Empty input tests
// =========================================================================

// TestTeeEmptyInput verifies that tee handles empty stdin gracefully.
func TestTeeEmptyInput(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("")
	code := runTeeWithStdin(toolSpecPath(t, "tee"), []string{"tee"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runTee(empty) returned exit code %d, want 0", code)
	}

	if stdout.Len() != 0 {
		t.Errorf("runTee(empty) should produce no output, got: %q", stdout.String())
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestTeeHelpFlag verifies --help.
func TestTeeHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTeeWithStdin(toolSpecPath(t, "tee"), []string{"tee", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runTee(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runTee(--help) produced no stdout output")
	}
}

// TestTeeVersionFlag verifies --version.
func TestTeeVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTeeWithStdin(toolSpecPath(t, "tee"), []string{"tee", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runTee(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runTee(--version) = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestTeeInvalidSpec verifies bad spec path returns exit code 1.
func TestTeeInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTeeWithStdin("/nonexistent/tee.json", []string{"tee"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runTee(bad spec) returned exit code %d, want 1", code)
	}
}

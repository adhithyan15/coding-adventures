// =========================================================================
// tail — Tests
// =========================================================================
//
// These tests verify the tail tool's behavior, covering:
//
//   1. Spec loading
//   2. Default behavior (last 10 lines)
//   3. Line count flag (-n)
//   4. From-start mode (-n +NUM)
//   5. Byte count flag (-c)
//   6. Headers for multiple files (-q, -v)
//   7. Help and version flags
//   8. Error handling for nonexistent files
//   9. parseTailNum() unit tests
//  10. Reading from stdin

package main

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

// TestTailSpecLoads verifies that tail.json is a valid cli-builder spec.
func TestTailSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "tail"), []string{"tail"})
	if err != nil {
		t.Fatalf("failed to load tail.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Default behavior tests
// =========================================================================

// TestTailDefaultLastTenLines verifies that tail prints the last 10 lines.
func TestTailDefaultLastTenLines(t *testing.T) {
	var content strings.Builder
	for i := 1; i <= 15; i++ {
		fmt.Fprintf(&content, "line%d\n", i)
	}
	dir := t.TempDir()
	path := filepath.Join(dir, "test.txt")
	os.WriteFile(path, []byte(content.String()), 0644)

	var stdout, stderr bytes.Buffer
	code := runTailWithStdin(toolSpecPath(t, "tail"), []string{"tail", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runTail() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	lines := strings.Split(strings.TrimSuffix(stdout.String(), "\n"), "\n")
	if len(lines) != 10 {
		t.Errorf("runTail() output %d lines, want 10", len(lines))
	}
	// First line should be "line6" (lines 6-15 = last 10).
	if lines[0] != "line6" {
		t.Errorf("first output line = %q, want %q", lines[0], "line6")
	}
}

// TestTailStdinDefault verifies that tail reads from stdin by default.
func TestTailStdinDefault(t *testing.T) {
	input := "a\nb\nc\n"
	var stdout, stderr bytes.Buffer
	code := runTailWithStdin(toolSpecPath(t, "tail"), []string{"tail"}, &stdout, &stderr, strings.NewReader(input))

	if code != 0 {
		t.Errorf("runTail(stdin) returned exit code %d, want 0", code)
	}

	if stdout.String() != "a\nb\nc\n" {
		t.Errorf("runTail(stdin) output = %q, want %q", stdout.String(), "a\nb\nc\n")
	}
}

// =========================================================================
// Line count flag tests
// =========================================================================

// TestTailLineCount verifies that -n NUM prints the last NUM lines.
func TestTailLineCount(t *testing.T) {
	input := "a\nb\nc\nd\ne\n"
	var stdout, stderr bytes.Buffer
	code := runTailWithStdin(toolSpecPath(t, "tail"), []string{"tail", "-n", "2"}, &stdout, &stderr, strings.NewReader(input))

	if code != 0 {
		t.Errorf("runTail(-n 2) returned exit code %d, want 0", code)
	}

	if stdout.String() != "d\ne\n" {
		t.Errorf("runTail(-n 2) output = %q, want %q", stdout.String(), "d\ne\n")
	}
}

// TestTailFromStart verifies that -n +NUM prints from line NUM onward.
func TestTailFromStart(t *testing.T) {
	input := "a\nb\nc\nd\ne\n"
	var stdout, stderr bytes.Buffer
	code := runTailWithStdin(toolSpecPath(t, "tail"), []string{"tail", "-n", "+3"}, &stdout, &stderr, strings.NewReader(input))

	if code != 0 {
		t.Errorf("runTail(-n +3) returned exit code %d, want 0", code)
	}

	if stdout.String() != "c\nd\ne\n" {
		t.Errorf("runTail(-n +3) output = %q, want %q", stdout.String(), "c\nd\ne\n")
	}
}

// =========================================================================
// Byte count flag tests
// =========================================================================

// TestTailByteCount verifies that -c NUM prints the last NUM bytes.
func TestTailByteCount(t *testing.T) {
	input := "hello world\n"
	var stdout, stderr bytes.Buffer
	code := runTailWithStdin(toolSpecPath(t, "tail"), []string{"tail", "-c", "6"}, &stdout, &stderr, strings.NewReader(input))

	if code != 0 {
		t.Errorf("runTail(-c 6) returned exit code %d, want 0", code)
	}

	// "hello world\n" is 12 bytes. Last 6 bytes = "world\n".
	if stdout.String() != "world\n" {
		t.Errorf("runTail(-c 6) output = %q, want %q", stdout.String(), "world\n")
	}
}

// =========================================================================
// Header tests
// =========================================================================

// TestTailVerboseHeader verifies that -v shows a header for single files.
func TestTailVerboseHeader(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "file.txt")
	os.WriteFile(path, []byte("hello\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runTailWithStdin(toolSpecPath(t, "tail"), []string{"tail", "-v", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runTail(-v) returned exit code %d, want 0", code)
	}

	if !strings.Contains(stdout.String(), "==> "+path+" <==") {
		t.Errorf("verbose mode should show header, got: %q", stdout.String())
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestTailHelpFlag verifies that --help prints help text and returns 0.
func TestTailHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTailWithStdin(toolSpecPath(t, "tail"), []string{"tail", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runTail(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runTail(--help) produced no stdout output")
	}
}

// TestTailVersionFlag verifies that --version prints the version.
func TestTailVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTailWithStdin(toolSpecPath(t, "tail"), []string{"tail", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runTail(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runTail(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestTailInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestTailInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTailWithStdin("/nonexistent/tail.json", []string{"tail"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runTail(bad spec) returned exit code %d, want 1", code)
	}
}

// TestTailNonexistentFile verifies that a nonexistent file produces an error.
func TestTailNonexistentFile(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTailWithStdin(toolSpecPath(t, "tail"), []string{"tail", "/nonexistent/file.txt"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runTail(nonexistent) returned exit code %d, want 1", code)
	}

	if stderr.Len() == 0 {
		t.Error("runTail(nonexistent) should produce stderr output")
	}
}

// =========================================================================
// parseTailNum unit tests
// =========================================================================

// TestParseTailNumDefault verifies default parsing.
func TestParseTailNumDefault(t *testing.T) {
	n, fromStart, err := parseTailNum("10")
	if err != nil {
		t.Fatalf("parseTailNum(10) failed: %v", err)
	}
	if n != 10 || fromStart {
		t.Errorf("parseTailNum(10) = (%d, %v), want (10, false)", n, fromStart)
	}
}

// TestParseTailNumPlus verifies +NUM parsing.
func TestParseTailNumPlus(t *testing.T) {
	n, fromStart, err := parseTailNum("+5")
	if err != nil {
		t.Fatalf("parseTailNum(+5) failed: %v", err)
	}
	if n != 5 || !fromStart {
		t.Errorf("parseTailNum(+5) = (%d, %v), want (5, true)", n, fromStart)
	}
}

// TestParseTailNumMinus verifies -NUM parsing.
func TestParseTailNumMinus(t *testing.T) {
	n, fromStart, err := parseTailNum("-3")
	if err != nil {
		t.Fatalf("parseTailNum(-3) failed: %v", err)
	}
	if n != 3 || fromStart {
		t.Errorf("parseTailNum(-3) = (%d, %v), want (3, false)", n, fromStart)
	}
}

// TestParseTailNumInvalid verifies error for non-numeric input.
func TestParseTailNumInvalid(t *testing.T) {
	_, _, err := parseTailNum("abc")
	if err == nil {
		t.Error("parseTailNum(abc) should return error")
	}
}

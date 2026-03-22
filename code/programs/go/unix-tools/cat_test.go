// =========================================================================
// cat — Tests
// =========================================================================
//
// These tests verify the cat tool's behavior, covering:
//
//   1. Spec loading
//   2. Reading from stdin (via mock reader)
//   3. Reading from files
//   4. Line numbering (-n and -b)
//   5. Squeeze blank lines (-s)
//   6. Show tabs (-T) and show ends (-E)
//   7. Show all (-A)
//   8. Help and version flags
//   9. Error handling for nonexistent files
//
// For tests involving files, we create temporary files in t.TempDir().
// For tests involving stdin, we use runCatWithStdin() which accepts a
// custom io.Reader.

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
// Helper: create a temporary file with given content
// =========================================================================

func createTempFile(t *testing.T, content string) string {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, "test.txt")
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("failed to create temp file: %v", err)
	}
	return path
}

// =========================================================================
// Spec loading tests
// =========================================================================

// TestCatSpecLoads verifies that cat.json is a valid cli-builder spec.
func TestCatSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "cat"), []string{"cat"})
	if err != nil {
		t.Fatalf("failed to load cat.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Default behavior tests
// =========================================================================

// TestCatStdinDefault verifies that cat reads from stdin when no files
// are specified.
func TestCatStdinDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello from stdin\n")
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runCat() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	expected := "hello from stdin\n"
	if stdout.String() != expected {
		t.Errorf("runCat() output = %q, want %q", stdout.String(), expected)
	}
}

// TestCatFileDefault verifies that cat reads a file and prints its contents.
func TestCatFileDefault(t *testing.T) {
	path := createTempFile(t, "line one\nline two\n")

	var stdout, stderr bytes.Buffer
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runCat(file) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	expected := "line one\nline two\n"
	if stdout.String() != expected {
		t.Errorf("runCat(file) output = %q, want %q", stdout.String(), expected)
	}
}

// =========================================================================
// Line numbering tests
// =========================================================================

// TestCatNumberAllLines verifies that -n numbers all lines.
func TestCatNumberAllLines(t *testing.T) {
	path := createTempFile(t, "alpha\n\nbeta\n")

	var stdout, stderr bytes.Buffer
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat", "-n", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runCat(-n) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	// All three lines should be numbered (including the blank one).
	if !strings.Contains(output, "1\talpha") {
		t.Errorf("line 1 should contain 'alpha', got: %q", output)
	}
	if !strings.Contains(output, "2\t") {
		t.Errorf("line 2 (blank) should be numbered, got: %q", output)
	}
	if !strings.Contains(output, "3\tbeta") {
		t.Errorf("line 3 should contain 'beta', got: %q", output)
	}
}

// TestCatNumberNonblank verifies that -b numbers only non-blank lines.
func TestCatNumberNonblank(t *testing.T) {
	path := createTempFile(t, "alpha\n\nbeta\n")

	var stdout, stderr bytes.Buffer
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat", "-b", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runCat(-b) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	lines := strings.Split(output, "\n")

	// The blank line should NOT have a number prefix.
	// Line 1: "     1\talpha"
	// Line 2: ""  (blank, unnumbered)
	// Line 3: "     2\tbeta"
	foundNumberedBlank := false
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		// A numbered blank line would be just a number followed by a tab.
		if trimmed != "" && !strings.ContainsAny(trimmed, "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ") {
			// This line has only digits and tabs — it's a numbered blank line.
			if strings.Contains(trimmed, "\t") && !strings.ContainsAny(strings.Split(trimmed, "\t")[1], "abcdefghijklmnopqrstuvwxyz") {
				foundNumberedBlank = true
			}
		}
	}
	if foundNumberedBlank {
		t.Errorf("blank line should not be numbered with -b, got: %q", output)
	}
}

// =========================================================================
// Squeeze blank lines tests
// =========================================================================

// TestCatSqueezeBlank verifies that -s collapses consecutive blank lines.
func TestCatSqueezeBlank(t *testing.T) {
	path := createTempFile(t, "hello\n\n\n\nworld\n")

	var stdout, stderr bytes.Buffer
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat", "-s", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runCat(-s) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	// Should collapse 3 blank lines into 1.
	expected := "hello\n\nworld\n"
	if stdout.String() != expected {
		t.Errorf("runCat(-s) output = %q, want %q", stdout.String(), expected)
	}
}

// =========================================================================
// Show tabs and show ends tests
// =========================================================================

// TestCatShowTabs verifies that -T displays tabs as ^I.
func TestCatShowTabs(t *testing.T) {
	path := createTempFile(t, "hello\tworld\n")

	var stdout, stderr bytes.Buffer
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat", "-T", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runCat(-T) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	expected := "hello^Iworld\n"
	if stdout.String() != expected {
		t.Errorf("runCat(-T) output = %q, want %q", stdout.String(), expected)
	}
}

// TestCatShowEnds verifies that -E displays $ at end of each line.
func TestCatShowEnds(t *testing.T) {
	path := createTempFile(t, "hello\nworld\n")

	var stdout, stderr bytes.Buffer
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat", "-E", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runCat(-E) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	expected := "hello$\nworld$\n"
	if stdout.String() != expected {
		t.Errorf("runCat(-E) output = %q, want %q", stdout.String(), expected)
	}
}

// =========================================================================
// Show all tests
// =========================================================================

// TestCatShowAll verifies that -A is equivalent to -vET.
func TestCatShowAll(t *testing.T) {
	path := createTempFile(t, "hello\tworld\n")

	var stdout, stderr bytes.Buffer
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat", "-A", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runCat(-A) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	// -A = -vET: tabs as ^I and $ at end.
	expected := "hello^Iworld$\n"
	if stdout.String() != expected {
		t.Errorf("runCat(-A) output = %q, want %q", stdout.String(), expected)
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestCatHelpFlag verifies that --help prints help text and returns 0.
func TestCatHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runCat(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runCat(--help) produced no stdout output")
	}
}

// TestCatVersionFlag verifies that --version prints the version.
func TestCatVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runCat(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runCat(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestCatInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestCatInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCatWithStdin("/nonexistent/cat.json", []string{"cat"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runCat(bad spec) returned exit code %d, want 1", code)
	}
}

// TestCatNonexistentFile verifies that a nonexistent file produces an
// error on stderr and returns exit code 1.
func TestCatNonexistentFile(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCatWithStdin(toolSpecPath(t, "cat"), []string{"cat", "/nonexistent/file.txt"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runCat(nonexistent) returned exit code %d, want 1", code)
	}

	if stderr.Len() == 0 {
		t.Error("runCat(nonexistent) should produce stderr output")
	}
}

// =========================================================================
// processLine() unit tests
// =========================================================================

// TestProcessLineNoOpts verifies passthrough when no options are set.
func TestProcessLineNoOpts(t *testing.T) {
	opts := catOptions{}
	result := processLine("hello world", opts)
	if result != "hello world" {
		t.Errorf("processLine(no opts) = %q, want %q", result, "hello world")
	}
}

// TestProcessLineShowEnds verifies that $ is appended.
func TestProcessLineShowEnds(t *testing.T) {
	opts := catOptions{showEnds: true}
	result := processLine("hello", opts)
	if result != "hello$" {
		t.Errorf("processLine(showEnds) = %q, want %q", result, "hello$")
	}
}

// TestProcessLineShowTabs verifies that tabs become ^I.
func TestProcessLineShowTabs(t *testing.T) {
	opts := catOptions{showTabs: true}
	result := processLine("hello\tworld", opts)
	if result != "hello^Iworld" {
		t.Errorf("processLine(showTabs) = %q, want %q", result, "hello^Iworld")
	}
}

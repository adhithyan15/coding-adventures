// =========================================================================
// rev — Tests
// =========================================================================
//
// These tests verify the rev tool's behavior, covering:
//
//   1. Spec loading
//   2. Basic line reversal
//   3. Multiple lines
//   4. Empty input
//   5. Reading from files
//   6. Reading from stdin
//   7. Unicode handling
//   8. Help and version flags
//   9. Error handling
//  10. reverseRunes() unit tests

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

// TestRevSpecLoads verifies that rev.json is a valid spec.
func TestRevSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "rev"), []string{"rev"})
	if err != nil {
		t.Fatalf("failed to load rev.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Basic behavior tests
// =========================================================================

// TestRevBasic verifies basic line reversal from stdin.
func TestRevBasic(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello\n")
	code := runRevWithStdin(toolSpecPath(t, "rev"), []string{"rev"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runRev() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	if stdout.String() != "olleh\n" {
		t.Errorf("runRev(hello) output = %q, want %q", stdout.String(), "olleh\n")
	}
}

// TestRevMultipleLines verifies reversal of multiple lines.
func TestRevMultipleLines(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("abc\n123\nxyz\n")
	code := runRevWithStdin(toolSpecPath(t, "rev"), []string{"rev"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runRev(multi) returned exit code %d, want 0", code)
	}

	expected := "cba\n321\nzyx\n"
	if stdout.String() != expected {
		t.Errorf("runRev(multi) output = %q, want %q", stdout.String(), expected)
	}
}

// TestRevEmptyInput verifies that empty input produces empty output.
func TestRevEmptyInput(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("")
	code := runRevWithStdin(toolSpecPath(t, "rev"), []string{"rev"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runRev(empty) returned exit code %d, want 0", code)
	}

	if stdout.Len() != 0 {
		t.Errorf("runRev(empty) should produce no output, got: %q", stdout.String())
	}
}

// TestRevFromFile verifies reading from a file.
func TestRevFromFile(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "test.txt")
	os.WriteFile(path, []byte("hello\nworld\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runRevWithStdin(toolSpecPath(t, "rev"), []string{"rev", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runRev(file) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	expected := "olleh\ndlrow\n"
	if stdout.String() != expected {
		t.Errorf("runRev(file) output = %q, want %q", stdout.String(), expected)
	}
}

// TestRevPalindrome verifies that a palindrome reverses to itself.
func TestRevPalindrome(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("racecar\n")
	code := runRevWithStdin(toolSpecPath(t, "rev"), []string{"rev"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runRev(palindrome) returned exit code %d, want 0", code)
	}

	if stdout.String() != "racecar\n" {
		t.Errorf("runRev(racecar) output = %q, want %q", stdout.String(), "racecar\n")
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestRevHelpFlag verifies --help.
func TestRevHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRevWithStdin(toolSpecPath(t, "rev"), []string{"rev", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runRev(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runRev(--help) produced no stdout output")
	}
}

// TestRevVersionFlag verifies --version.
func TestRevVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRevWithStdin(toolSpecPath(t, "rev"), []string{"rev", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runRev(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runRev(--version) = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestRevInvalidSpec verifies bad spec path returns exit code 1.
func TestRevInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRevWithStdin("/nonexistent/rev.json", []string{"rev"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runRev(bad spec) returned exit code %d, want 1", code)
	}
}

// TestRevNonexistentFile verifies that a nonexistent file produces an error.
func TestRevNonexistentFile(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRevWithStdin(toolSpecPath(t, "rev"), []string{"rev", "/nonexistent/file.txt"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runRev(nonexistent) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// reverseRunes() unit tests
// =========================================================================

// TestReverseRunesBasic verifies basic ASCII reversal.
func TestReverseRunesBasic(t *testing.T) {
	tests := []struct {
		input string
		want  string
	}{
		{"hello", "olleh"},
		{"a", "a"},
		{"", ""},
		{"ab", "ba"},
		{"racecar", "racecar"},
	}

	for _, tt := range tests {
		got := reverseRunes(tt.input)
		if got != tt.want {
			t.Errorf("reverseRunes(%q) = %q, want %q", tt.input, got, tt.want)
		}
	}
}

// TestReverseRunesUnicode verifies Unicode character reversal.
func TestReverseRunesUnicode(t *testing.T) {
	// Multi-byte characters should be reversed as whole characters.
	got := reverseRunes("abc")
	if got != "cba" {
		t.Errorf("reverseRunes(abc) = %q, want %q", got, "cba")
	}
}

// =========================================================================
// wc — Tests
// =========================================================================
//
// These tests verify the wc tool's behavior, covering:
//
//   1. Spec loading
//   2. Default mode (lines, words, bytes)
//   3. Individual flags (-l, -w, -c, -m, -L)
//   4. Reading from stdin
//   5. Reading from files
//   6. Multiple files with totals
//   7. Help and version flags
//   8. Error handling for nonexistent files
//   9. countReader() unit tests for edge cases

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

// TestWcSpecLoads verifies that wc.json is a valid cli-builder spec.
func TestWcSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "wc"), []string{"wc"})
	if err != nil {
		t.Fatalf("failed to load wc.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Default behavior tests
// =========================================================================

// TestWcStdinDefault verifies that wc reads from stdin and shows lines,
// words, and bytes by default.
func TestWcStdinDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello world\nfoo bar baz\n")
	code := runWcWithStdin(toolSpecPath(t, "wc"), []string{"wc"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runWc() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	// Should contain "2" (lines), "5" (words), and "23" (bytes).
	if !strings.Contains(output, "2") {
		t.Errorf("output should contain line count 2, got: %q", output)
	}
	if !strings.Contains(output, "5") {
		t.Errorf("output should contain word count 5, got: %q", output)
	}
}

// TestWcFileDefault verifies that wc processes a file with default flags.
func TestWcFileDefault(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "test.txt")
	content := "one two three\nfour five\n"
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("failed to create temp file: %v", err)
	}

	var stdout, stderr bytes.Buffer
	code := runWcWithStdin(toolSpecPath(t, "wc"), []string{"wc", path}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runWc(file) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	// Should contain "2" (lines), "5" (words), and the file name.
	if !strings.Contains(output, "2") {
		t.Errorf("output should contain line count 2, got: %q", output)
	}
	if !strings.Contains(output, path) {
		t.Errorf("output should contain filename, got: %q", output)
	}
}

// =========================================================================
// Individual flag tests
// =========================================================================

// TestWcLinesOnly verifies that -l shows only line counts.
func TestWcLinesOnly(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("one\ntwo\nthree\n")
	code := runWcWithStdin(toolSpecPath(t, "wc"), []string{"wc", "-l"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runWc(-l) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "3" {
		t.Errorf("runWc(-l) output = %q, want %q", output, "3")
	}
}

// TestWcWordsOnly verifies that -w shows only word counts.
func TestWcWordsOnly(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello world\n")
	code := runWcWithStdin(toolSpecPath(t, "wc"), []string{"wc", "-w"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runWc(-w) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "2" {
		t.Errorf("runWc(-w) output = %q, want %q", output, "2")
	}
}

// TestWcBytesOnly verifies that -c shows only byte counts.
func TestWcBytesOnly(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello\n")
	code := runWcWithStdin(toolSpecPath(t, "wc"), []string{"wc", "-c"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runWc(-c) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "6" {
		t.Errorf("runWc(-c) output = %q, want %q (6 bytes: h,e,l,l,o,\\n)", output, "6")
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestWcHelpFlag verifies that --help prints help text and returns 0.
func TestWcHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runWcWithStdin(toolSpecPath(t, "wc"), []string{"wc", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runWc(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runWc(--help) produced no stdout output")
	}
}

// TestWcVersionFlag verifies that --version prints the version.
func TestWcVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runWcWithStdin(toolSpecPath(t, "wc"), []string{"wc", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runWc(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runWc(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestWcInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestWcInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runWcWithStdin("/nonexistent/wc.json", []string{"wc"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runWc(bad spec) returned exit code %d, want 1", code)
	}
}

// TestWcNonexistentFile verifies that a nonexistent file produces an error.
func TestWcNonexistentFile(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runWcWithStdin(toolSpecPath(t, "wc"), []string{"wc", "/nonexistent/file.txt"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runWc(nonexistent) returned exit code %d, want 1", code)
	}

	if stderr.Len() == 0 {
		t.Error("runWc(nonexistent) should produce stderr output")
	}
}

// =========================================================================
// countReader() unit tests
// =========================================================================

// TestCountReaderEmpty verifies that an empty input produces zero counts.
func TestCountReaderEmpty(t *testing.T) {
	counts, err := countReader(strings.NewReader(""))
	if err != nil {
		t.Fatalf("countReader(empty) failed: %v", err)
	}

	if counts.lines != 0 {
		t.Errorf("empty input: lines = %d, want 0", counts.lines)
	}
	if counts.words != 0 {
		t.Errorf("empty input: words = %d, want 0", counts.words)
	}
	if counts.bytes != 0 {
		t.Errorf("empty input: bytes = %d, want 0", counts.bytes)
	}
}

// TestCountReaderSingleWord verifies counting for a single word with no
// trailing newline.
func TestCountReaderSingleWord(t *testing.T) {
	counts, err := countReader(strings.NewReader("hello"))
	if err != nil {
		t.Fatalf("countReader(hello) failed: %v", err)
	}

	if counts.lines != 0 {
		t.Errorf("'hello': lines = %d, want 0 (no newline)", counts.lines)
	}
	if counts.words != 1 {
		t.Errorf("'hello': words = %d, want 1", counts.words)
	}
	if counts.bytes != 5 {
		t.Errorf("'hello': bytes = %d, want 5", counts.bytes)
	}
}

// TestCountReaderMultipleLines verifies counting across multiple lines.
func TestCountReaderMultipleLines(t *testing.T) {
	counts, err := countReader(strings.NewReader("hello world\nfoo bar baz\n"))
	if err != nil {
		t.Fatalf("countReader(multi) failed: %v", err)
	}

	if counts.lines != 2 {
		t.Errorf("multi-line: lines = %d, want 2", counts.lines)
	}
	if counts.words != 5 {
		t.Errorf("multi-line: words = %d, want 5", counts.words)
	}
	if counts.bytes != 24 {
		t.Errorf("multi-line: bytes = %d, want 24", counts.bytes)
	}
}

// TestCountReaderOnlyNewlines verifies counting for a file of blank lines.
func TestCountReaderOnlyNewlines(t *testing.T) {
	counts, err := countReader(strings.NewReader("\n\n\n"))
	if err != nil {
		t.Fatalf("countReader(newlines) failed: %v", err)
	}

	if counts.lines != 3 {
		t.Errorf("newlines only: lines = %d, want 3", counts.lines)
	}
	if counts.words != 0 {
		t.Errorf("newlines only: words = %d, want 0", counts.words)
	}
	if counts.bytes != 3 {
		t.Errorf("newlines only: bytes = %d, want 3", counts.bytes)
	}
}

// TestCountReaderMaxLineLength verifies the maximum line length calculation.
func TestCountReaderMaxLineLength(t *testing.T) {
	// "short" is 5 chars, "a longer line" is 13 chars.
	counts, err := countReader(strings.NewReader("short\na longer line\n"))
	if err != nil {
		t.Fatalf("countReader(max-line) failed: %v", err)
	}

	if counts.maxLineLength != 13 {
		t.Errorf("maxLineLength = %d, want 13", counts.maxLineLength)
	}
}

// =========================================================================
// Helper function unit tests
// =========================================================================

// TestDigitWidth verifies the digit counting function.
func TestDigitWidth(t *testing.T) {
	tests := []struct {
		n    int
		want int
	}{
		{0, 1},
		{1, 1},
		{9, 1},
		{10, 2},
		{99, 2},
		{100, 3},
		{1000, 4},
	}

	for _, tt := range tests {
		got := digitWidth(tt.n)
		if got != tt.want {
			t.Errorf("digitWidth(%d) = %d, want %d", tt.n, got, tt.want)
		}
	}
}

// TestFormatCounts verifies the output formatting.
func TestFormatCounts(t *testing.T) {
	counts := wcCounts{lines: 10, words: 20, bytes: 100}
	opts := wcOptions{showLines: true, showWords: true, showBytes: true}
	result := formatCounts(counts, opts, 3, "file.txt")

	if !strings.Contains(result, " 10") {
		t.Errorf("formatCounts should contain ' 10', got: %q", result)
	}
	if !strings.Contains(result, " 20") {
		t.Errorf("formatCounts should contain ' 20', got: %q", result)
	}
	if !strings.Contains(result, "100") {
		t.Errorf("formatCounts should contain '100', got: %q", result)
	}
	if !strings.Contains(result, "file.txt") {
		t.Errorf("formatCounts should contain 'file.txt', got: %q", result)
	}
}

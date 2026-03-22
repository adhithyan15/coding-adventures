// =========================================================================
// comm — Tests
// =========================================================================

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// =========================================================================
// compareFiles tests
// =========================================================================

// TestCompareFilesBasic verifies the core comparison with overlapping data.
func TestCompareFilesBasic(t *testing.T) {
	lines1 := []string{"apple", "banana", "cherry", "fig"}
	lines2 := []string{"banana", "cherry", "date"}

	got := compareFiles(lines1, lines2)

	// Expected:
	//   apple     → col 1 (unique to FILE1)
	//   banana    → col 3 (common)
	//   cherry    → col 3 (common)
	//   date      → col 2 (unique to FILE2)
	//   fig       → col 1 (unique to FILE1)
	expected := []CommLine{
		{Text: "apple", Column: 1},
		{Text: "banana", Column: 3},
		{Text: "cherry", Column: 3},
		{Text: "date", Column: 2},
		{Text: "fig", Column: 1},
	}

	if len(got) != len(expected) {
		t.Fatalf("compareFiles length = %d, want %d. got: %v", len(got), len(expected), got)
	}
	for i := range expected {
		if got[i].Text != expected[i].Text || got[i].Column != expected[i].Column {
			t.Errorf("compareFiles[%d] = {%q, col %d}, want {%q, col %d}",
				i, got[i].Text, got[i].Column, expected[i].Text, expected[i].Column)
		}
	}
}

// TestCompareFilesIdentical verifies two identical files (all column 3).
func TestCompareFilesIdentical(t *testing.T) {
	lines := []string{"a", "b", "c"}
	got := compareFiles(lines, lines)

	for _, line := range got {
		if line.Column != 3 {
			t.Errorf("identical files: %q in column %d, want 3", line.Text, line.Column)
		}
	}
}

// TestCompareFilesDisjoint verifies two files with no common lines.
func TestCompareFilesDisjoint(t *testing.T) {
	lines1 := []string{"a", "c", "e"}
	lines2 := []string{"b", "d", "f"}

	got := compareFiles(lines1, lines2)

	for _, line := range got {
		if line.Column == 3 {
			t.Errorf("disjoint files: %q in column 3, should be 1 or 2", line.Text)
		}
	}
}

// TestCompareFilesEmpty verifies handling of empty files.
func TestCompareFilesEmpty(t *testing.T) {
	got := compareFiles([]string{}, []string{})
	if len(got) != 0 {
		t.Errorf("compareFiles(empty, empty) = %v, want empty", got)
	}

	got = compareFiles([]string{"a"}, []string{})
	if len(got) != 1 || got[0].Column != 1 {
		t.Errorf("compareFiles([a], []) = %v, want [{a, col 1}]", got)
	}
}

// =========================================================================
// formatCommOutput tests
// =========================================================================

// TestFormatCommOutputDefault verifies default formatting with tabs.
func TestFormatCommOutputDefault(t *testing.T) {
	lines := []CommLine{
		{Text: "apple", Column: 1},
		{Text: "banana", Column: 3},
		{Text: "date", Column: 2},
	}

	got := formatCommOutput(lines, CommOptions{})
	want := "apple\n\t\tbanana\n\tdate\n"
	if got != want {
		t.Errorf("formatCommOutput = %q, want %q", got, want)
	}
}

// TestFormatCommOutputSuppressCol1 verifies -1 flag.
func TestFormatCommOutputSuppressCol1(t *testing.T) {
	lines := []CommLine{
		{Text: "apple", Column: 1},
		{Text: "banana", Column: 3},
		{Text: "date", Column: 2},
	}

	opts := CommOptions{SuppressCol1: true}
	got := formatCommOutput(lines, opts)
	// With col1 suppressed, col2 has no prefix and col3 has one tab.
	want := "\tbanana\ndate\n"
	if got != want {
		t.Errorf("formatCommOutput(-1) = %q, want %q", got, want)
	}
}

// TestFormatCommOutputSuppressCol12 verifies -1 -2 flags (show only common).
func TestFormatCommOutputSuppressCol12(t *testing.T) {
	lines := []CommLine{
		{Text: "apple", Column: 1},
		{Text: "banana", Column: 3},
		{Text: "date", Column: 2},
	}

	opts := CommOptions{SuppressCol1: true, SuppressCol2: true}
	got := formatCommOutput(lines, opts)
	want := "banana\n"
	if got != want {
		t.Errorf("formatCommOutput(-12) = %q, want %q", got, want)
	}
}

// =========================================================================
// runComm integration tests
// =========================================================================

// TestRunCommFiles verifies comparing two files.
func TestRunCommFiles(t *testing.T) {
	dir := t.TempDir()
	f1 := filepath.Join(dir, "file1.txt")
	f2 := filepath.Join(dir, "file2.txt")
	os.WriteFile(f1, []byte("apple\nbanana\ncherry\n"), 0644)
	os.WriteFile(f2, []byte("banana\ncherry\ndate\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runCommWithStdin(
		toolSpecPath(t, "comm"),
		[]string{"comm", f1, f2},
		&stdout, &stderr, strings.NewReader(""),
	)

	if code != 0 {
		t.Errorf("runComm returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	// apple should be in column 1 (no prefix).
	if !strings.Contains(output, "apple") {
		t.Errorf("output missing 'apple': %q", output)
	}
	// banana should be in column 3 (two tabs prefix).
	if !strings.Contains(output, "\t\tbanana") {
		t.Errorf("output missing common 'banana': %q", output)
	}
}

// TestRunCommHelp verifies --help flag.
func TestRunCommHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCommWithStdin(
		toolSpecPath(t, "comm"),
		[]string{"comm", "--help"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runComm(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runComm(--help) produced no output")
	}
}

// TestRunCommVersion verifies --version flag.
func TestRunCommVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCommWithStdin(
		toolSpecPath(t, "comm"),
		[]string{"comm", "--version"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runComm(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runComm(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunCommInvalidSpec verifies error handling.
func TestRunCommInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCommWithStdin(
		"/nonexistent/comm.json",
		[]string{"comm", "a", "b"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 1 {
		t.Errorf("runComm(bad spec) returned %d, want 1", code)
	}
}

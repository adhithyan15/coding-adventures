// =========================================================================
// sort — Tests
// =========================================================================
//
// These tests verify the sort tool's business logic and integration:
//   1. sortLines: lexicographic, numeric, reverse, unique, case-insensitive
//   2. extractSortKey: field extraction with -k and -t
//   3. parseLeadingNumber: numeric parsing
//   4. runSort: integration tests with files and stdin

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// =========================================================================
// sortLines tests
// =========================================================================

// TestSortLinesLexicographic verifies default lexicographic sort.
func TestSortLinesLexicographic(t *testing.T) {
	lines := []string{"banana", "apple", "cherry", "date"}
	opts := SortOptions{}

	got := sortLines(lines, opts)
	want := []string{"apple", "banana", "cherry", "date"}

	if len(got) != len(want) {
		t.Fatalf("sortLines length = %d, want %d", len(got), len(want))
	}
	for i := range want {
		if got[i] != want[i] {
			t.Errorf("sortLines[%d] = %q, want %q", i, got[i], want[i])
		}
	}
}

// TestSortLinesReverse verifies -r (reverse sort).
func TestSortLinesReverse(t *testing.T) {
	lines := []string{"banana", "apple", "cherry"}
	opts := SortOptions{Reverse: true}

	got := sortLines(lines, opts)
	want := []string{"cherry", "banana", "apple"}

	for i := range want {
		if got[i] != want[i] {
			t.Errorf("sortLines(-r)[%d] = %q, want %q", i, got[i], want[i])
		}
	}
}

// TestSortLinesNumeric verifies -n (numeric sort).
func TestSortLinesNumeric(t *testing.T) {
	lines := []string{"10", "2", "1", "20", "3"}
	opts := SortOptions{NumericSort: true}

	got := sortLines(lines, opts)
	want := []string{"1", "2", "3", "10", "20"}

	for i := range want {
		if got[i] != want[i] {
			t.Errorf("sortLines(-n)[%d] = %q, want %q", i, got[i], want[i])
		}
	}
}

// TestSortLinesIgnoreCase verifies -f (case-insensitive sort).
func TestSortLinesIgnoreCase(t *testing.T) {
	lines := []string{"Cherry", "apple", "Banana"}
	opts := SortOptions{IgnoreCase: true}

	got := sortLines(lines, opts)
	want := []string{"apple", "Banana", "Cherry"}

	for i := range want {
		if got[i] != want[i] {
			t.Errorf("sortLines(-f)[%d] = %q, want %q", i, got[i], want[i])
		}
	}
}

// TestSortLinesUnique verifies -u (unique, remove duplicates).
func TestSortLinesUnique(t *testing.T) {
	lines := []string{"banana", "apple", "banana", "cherry", "apple"}
	opts := SortOptions{Unique: true}

	got := sortLines(lines, opts)
	want := []string{"apple", "banana", "cherry"}

	if len(got) != len(want) {
		t.Fatalf("sortLines(-u) length = %d, want %d. got: %v", len(got), len(want), got)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Errorf("sortLines(-u)[%d] = %q, want %q", i, got[i], want[i])
		}
	}
}

// TestSortLinesEmpty verifies sorting an empty slice.
func TestSortLinesEmpty(t *testing.T) {
	got := sortLines([]string{}, SortOptions{})
	if len(got) != 0 {
		t.Errorf("sortLines(empty) = %v, want empty", got)
	}
}

// TestSortLinesSingleLine verifies sorting a single-element slice.
func TestSortLinesSingleLine(t *testing.T) {
	got := sortLines([]string{"only"}, SortOptions{})
	if len(got) != 1 || got[0] != "only" {
		t.Errorf("sortLines(single) = %v, want [only]", got)
	}
}

// =========================================================================
// extractSortKey tests
// =========================================================================

// TestExtractSortKeyDefault verifies that without -k, the entire line is the key.
func TestExtractSortKeyDefault(t *testing.T) {
	got := extractSortKey("hello world", SortOptions{})
	if got != "hello world" {
		t.Errorf("extractSortKey(default) = %q, want %q", got, "hello world")
	}
}

// TestExtractSortKeyField verifies -k2 extracts the second field onwards.
func TestExtractSortKeyField(t *testing.T) {
	opts := SortOptions{KeySpecs: []string{"2"}}
	got := extractSortKey("Alice 25 Engineering", opts)
	if got != "25 Engineering" {
		t.Errorf("extractSortKey(-k2) = %q, want %q", got, "25 Engineering")
	}
}

// TestExtractSortKeyFieldRange verifies -k2,2 extracts only the second field.
func TestExtractSortKeyFieldRange(t *testing.T) {
	opts := SortOptions{KeySpecs: []string{"2,2"}}
	got := extractSortKey("Alice 25 Engineering", opts)
	if got != "25" {
		t.Errorf("extractSortKey(-k2,2) = %q, want %q", got, "25")
	}
}

// =========================================================================
// parseLeadingNumber tests
// =========================================================================

// TestParseLeadingNumber verifies various numeric formats.
func TestParseLeadingNumber(t *testing.T) {
	tests := []struct {
		input string
		want  float64
	}{
		{"42", 42},
		{"-7", -7},
		{"3.14", 3.14},
		{"  10", 10},
		{"abc", 0},
		{"", 0},
		{"12abc", 12},
	}

	for _, tt := range tests {
		got := parseLeadingNumber(tt.input)
		if got != tt.want {
			t.Errorf("parseLeadingNumber(%q) = %v, want %v", tt.input, got, tt.want)
		}
	}
}

// =========================================================================
// runSort integration tests
// =========================================================================

// TestRunSortFile verifies sorting a file.
func TestRunSortFile(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "test.txt")
	content := "cherry\napple\nbanana\n"
	if err := os.WriteFile(testFile, []byte(content), 0644); err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}

	var stdout, stderr bytes.Buffer
	code := runSortWithStdin(
		toolSpecPath(t, "sort"),
		[]string{"sort", testFile},
		&stdout, &stderr, strings.NewReader(""),
	)

	if code != 0 {
		t.Errorf("runSort returned %d, want 0. stderr: %s", code, stderr.String())
	}

	got := strings.TrimSpace(stdout.String())
	want := "apple\nbanana\ncherry"
	if got != want {
		t.Errorf("runSort output = %q, want %q", got, want)
	}
}

// TestRunSortStdin verifies sorting from stdin.
func TestRunSortStdin(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSortWithStdin(
		toolSpecPath(t, "sort"),
		[]string{"sort"},
		&stdout, &stderr, strings.NewReader("cherry\napple\nbanana\n"),
	)

	if code != 0 {
		t.Errorf("runSort(stdin) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	got := strings.TrimSpace(stdout.String())
	want := "apple\nbanana\ncherry"
	if got != want {
		t.Errorf("runSort(stdin) = %q, want %q", got, want)
	}
}

// TestRunSortHelp verifies --help flag.
func TestRunSortHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSortWithStdin(
		toolSpecPath(t, "sort"),
		[]string{"sort", "--help"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runSort(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runSort(--help) produced no output")
	}
}

// TestRunSortVersion verifies --version flag.
func TestRunSortVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSortWithStdin(
		toolSpecPath(t, "sort"),
		[]string{"sort", "--version"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runSort(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runSort(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunSortInvalidSpec verifies error handling.
func TestRunSortInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSortWithStdin(
		"/nonexistent/sort.json",
		[]string{"sort"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 1 {
		t.Errorf("runSort(bad spec) returned %d, want 1", code)
	}
}

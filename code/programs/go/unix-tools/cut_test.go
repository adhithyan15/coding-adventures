// =========================================================================
// cut — Tests
// =========================================================================
//
// These tests verify the cut tool:
//   1. parseList: LIST format parsing
//   2. cutLine: character, byte, and field extraction
//   3. runCut: integration tests

package main

import (
	"bytes"
	"math"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// =========================================================================
// parseList tests
// =========================================================================

// TestParseListSingle verifies parsing a single position.
func TestParseListSingle(t *testing.T) {
	ranges, err := parseList("3")
	if err != nil {
		t.Fatalf("parseList(\"3\") failed: %v", err)
	}
	if len(ranges) != 1 || ranges[0].Start != 3 || ranges[0].End != 3 {
		t.Errorf("parseList(\"3\") = %v, want [{3,3}]", ranges)
	}
}

// TestParseListMultiple verifies parsing comma-separated positions.
func TestParseListMultiple(t *testing.T) {
	ranges, err := parseList("1,3,5")
	if err != nil {
		t.Fatalf("parseList(\"1,3,5\") failed: %v", err)
	}
	if len(ranges) != 3 {
		t.Fatalf("expected 3 ranges, got %d", len(ranges))
	}
	if ranges[0].Start != 1 || ranges[1].Start != 3 || ranges[2].Start != 5 {
		t.Errorf("parseList(\"1,3,5\") = %v", ranges)
	}
}

// TestParseListRange verifies parsing a range like "2-5".
func TestParseListRange(t *testing.T) {
	ranges, err := parseList("2-5")
	if err != nil {
		t.Fatalf("parseList(\"2-5\") failed: %v", err)
	}
	if len(ranges) != 1 || ranges[0].Start != 2 || ranges[0].End != 5 {
		t.Errorf("parseList(\"2-5\") = %v, want [{2,5}]", ranges)
	}
}

// TestParseListOpenEnd verifies parsing "5-" (5 to end).
func TestParseListOpenEnd(t *testing.T) {
	ranges, err := parseList("5-")
	if err != nil {
		t.Fatalf("parseList(\"5-\") failed: %v", err)
	}
	if len(ranges) != 1 || ranges[0].Start != 5 || ranges[0].End != math.MaxInt {
		t.Errorf("parseList(\"5-\") = %v", ranges)
	}
}

// TestParseListOpenStart verifies parsing "-3" (1 to 3).
func TestParseListOpenStart(t *testing.T) {
	ranges, err := parseList("-3")
	if err != nil {
		t.Fatalf("parseList(\"-3\") failed: %v", err)
	}
	if len(ranges) != 1 || ranges[0].Start != 1 || ranges[0].End != 3 {
		t.Errorf("parseList(\"-3\") = %v, want [{1,3}]", ranges)
	}
}

// TestParseListInvalid verifies error handling for invalid input.
func TestParseListInvalid(t *testing.T) {
	_, err := parseList("abc")
	if err == nil {
		t.Error("parseList(\"abc\") should have failed")
	}
}

// =========================================================================
// cutLine tests
// =========================================================================

// TestCutLineByChars verifies character extraction.
func TestCutLineByChars(t *testing.T) {
	opts := CutOptions{
		Mode:   "characters",
		Ranges: []CutRange{{Start: 1, End: 3}},
	}

	got, _ := cutLine("hello world", opts)
	if got != "hel" {
		t.Errorf("cutLine(chars 1-3) = %q, want %q", got, "hel")
	}
}

// TestCutLineByCharsSingle verifies selecting a single character.
func TestCutLineByCharsSingle(t *testing.T) {
	opts := CutOptions{
		Mode:   "characters",
		Ranges: []CutRange{{Start: 5, End: 5}},
	}

	got, _ := cutLine("hello", opts)
	if got != "o" {
		t.Errorf("cutLine(char 5) = %q, want %q", got, "o")
	}
}

// TestCutLineByFields verifies field extraction with custom delimiter.
func TestCutLineByFields(t *testing.T) {
	opts := CutOptions{
		Mode:      "fields",
		Ranges:    []CutRange{{Start: 2, End: 2}},
		Delimiter: ",",
	}

	got, ok := cutLine("name,age,city", opts)
	if !ok {
		t.Error("cutLine returned shouldPrint=false unexpectedly")
	}
	if got != "age" {
		t.Errorf("cutLine(field 2, delim=',') = %q, want %q", got, "age")
	}
}

// TestCutLineByFieldsMultiple verifies selecting multiple fields.
func TestCutLineByFieldsMultiple(t *testing.T) {
	opts := CutOptions{
		Mode:      "fields",
		Ranges:    []CutRange{{Start: 1, End: 1}, {Start: 3, End: 3}},
		Delimiter: ",",
	}

	got, _ := cutLine("name,age,city", opts)
	if got != "name,city" {
		t.Errorf("cutLine(fields 1,3) = %q, want %q", got, "name,city")
	}
}

// TestCutLineByFieldsOnlyDelimited verifies -s flag.
func TestCutLineByFieldsOnlyDelimited(t *testing.T) {
	opts := CutOptions{
		Mode:          "fields",
		Ranges:        []CutRange{{Start: 1, End: 1}},
		Delimiter:     "\t",
		OnlyDelimited: true,
	}

	// Line without delimiter should be skipped.
	_, ok := cutLine("no tabs here", opts)
	if ok {
		t.Error("cutLine(-s, no delimiter) should return false")
	}
}

// TestCutLineByBytes verifies byte extraction.
func TestCutLineByBytes(t *testing.T) {
	opts := CutOptions{
		Mode:   "bytes",
		Ranges: []CutRange{{Start: 1, End: 4}},
	}

	got, _ := cutLine("hello", opts)
	if got != "hell" {
		t.Errorf("cutLine(bytes 1-4) = %q, want %q", got, "hell")
	}
}

// =========================================================================
// runCut integration tests
// =========================================================================

// TestRunCutFieldsFromFile verifies cutting fields from a file.
func TestRunCutFieldsFromFile(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "data.csv")
	content := "alice,25,nyc\nbob,30,sf\n"
	if err := os.WriteFile(testFile, []byte(content), 0644); err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}

	var stdout, stderr bytes.Buffer
	code := runCutWithStdin(
		toolSpecPath(t, "cut"),
		[]string{"cut", "-d", ",", "-f", "2", testFile},
		&stdout, &stderr, strings.NewReader(""),
	)

	if code != 0 {
		t.Errorf("runCut returned %d, want 0. stderr: %s", code, stderr.String())
	}

	got := strings.TrimSpace(stdout.String())
	if got != "25\n30" {
		t.Errorf("runCut output = %q, want %q", got, "25\n30")
	}
}

// TestRunCutCharsFromStdin verifies cutting characters from stdin.
func TestRunCutCharsFromStdin(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCutWithStdin(
		toolSpecPath(t, "cut"),
		[]string{"cut", "-c", "1-3"},
		&stdout, &stderr, strings.NewReader("hello\nworld\n"),
	)

	if code != 0 {
		t.Errorf("runCut(stdin) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	got := strings.TrimSpace(stdout.String())
	if got != "hel\nwor" {
		t.Errorf("runCut(chars) = %q, want %q", got, "hel\nwor")
	}
}

// TestRunCutHelp verifies --help flag.
func TestRunCutHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCutWithStdin(
		toolSpecPath(t, "cut"),
		[]string{"cut", "--help"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runCut(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runCut(--help) produced no output")
	}
}

// TestRunCutVersion verifies --version flag.
func TestRunCutVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCutWithStdin(
		toolSpecPath(t, "cut"),
		[]string{"cut", "--version"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runCut(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runCut(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunCutInvalidSpec verifies error handling.
func TestRunCutInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCutWithStdin(
		"/nonexistent/cut.json",
		[]string{"cut", "-c", "1"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 1 {
		t.Errorf("runCut(bad spec) returned %d, want 1", code)
	}
}

// =========================================================================
// join — Tests
// =========================================================================
//
// These tests verify the join tool's behavior, covering:
//
//   1. Spec loading
//   2. Basic join on first field
//   3. Join on different fields (-1, -2)
//   4. Custom separator (-t)
//   5. Unpaired lines (-a)
//   6. Case-insensitive join (-i)
//   7. Header mode (--header)
//   8. Help and version flags
//   9. Error handling

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

func TestJoinSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "join"), []string{"join", "a", "b"})
	if err != nil {
		t.Fatalf("failed to load join.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Business logic tests — splitJoinFields
// =========================================================================

func TestSplitJoinFieldsWhitespace(t *testing.T) {
	fields := splitJoinFields("hello world test", "")
	if len(fields) != 3 {
		t.Errorf("expected 3 fields, got %d: %v", len(fields), fields)
	}
	if fields[0] != "hello" || fields[1] != "world" || fields[2] != "test" {
		t.Errorf("unexpected fields: %v", fields)
	}
}

func TestSplitJoinFieldsCustomSep(t *testing.T) {
	fields := splitJoinFields("a:b:c", ":")
	if len(fields) != 3 {
		t.Errorf("expected 3 fields, got %d: %v", len(fields), fields)
	}
	if fields[0] != "a" || fields[1] != "b" || fields[2] != "c" {
		t.Errorf("unexpected fields: %v", fields)
	}
}

// =========================================================================
// Business logic tests — getJoinField
// =========================================================================

func TestGetJoinFieldValid(t *testing.T) {
	fields := []string{"alpha", "beta", "gamma"}
	val, ok := getJoinField(fields, 2)
	if !ok {
		t.Error("getJoinField should succeed for valid index")
	}
	if val != "beta" {
		t.Errorf("getJoinField(2) = %q, want %q", val, "beta")
	}
}

func TestGetJoinFieldOutOfRange(t *testing.T) {
	fields := []string{"alpha", "beta"}
	_, ok := getJoinField(fields, 5)
	if ok {
		t.Error("getJoinField should fail for out-of-range index")
	}
}

func TestGetJoinFieldZero(t *testing.T) {
	fields := []string{"alpha"}
	_, ok := getJoinField(fields, 0)
	if ok {
		t.Error("getJoinField should fail for index 0 (1-based)")
	}
}

// =========================================================================
// Business logic tests — compareFields
// =========================================================================

func TestCompareFieldsEqual(t *testing.T) {
	if compareFields("abc", "abc", false) != 0 {
		t.Error("'abc' should equal 'abc'")
	}
}

func TestCompareFieldsLess(t *testing.T) {
	if compareFields("abc", "def", false) >= 0 {
		t.Error("'abc' should be less than 'def'")
	}
}

func TestCompareFieldsIgnoreCase(t *testing.T) {
	if compareFields("ABC", "abc", true) != 0 {
		t.Error("'ABC' should equal 'abc' with ignore-case")
	}
}

// =========================================================================
// Business logic tests — joinFiles
// =========================================================================

func TestJoinFilesBasic(t *testing.T) {
	lines1 := []string{"1 Alice", "2 Bob", "3 Carol"}
	lines2 := []string{"1 Engineering", "2 Marketing", "3 Sales"}

	result := joinFiles(lines1, lines2, JoinOptions{Field1: 1, Field2: 1})

	if len(result) != 3 {
		t.Fatalf("expected 3 output lines, got %d: %v", len(result), result)
	}

	if result[0] != "1 Alice Engineering" {
		t.Errorf("line 0 = %q, want %q", result[0], "1 Alice Engineering")
	}
	if result[1] != "2 Bob Marketing" {
		t.Errorf("line 1 = %q, want %q", result[1], "2 Bob Marketing")
	}
}

func TestJoinFilesUnmatched(t *testing.T) {
	lines1 := []string{"1 Alice", "2 Bob", "4 Dave"}
	lines2 := []string{"1 Engineering", "3 Sales"}

	// Without -a: only matched lines.
	result := joinFiles(lines1, lines2, JoinOptions{Field1: 1, Field2: 1})
	if len(result) != 1 {
		t.Errorf("without -a: expected 1 matched line, got %d: %v", len(result), result)
	}

	// With -a 1: include unmatched from file 1.
	result = joinFiles(lines1, lines2, JoinOptions{Field1: 1, Field2: 1, Unpaired1: true})
	found := false
	for _, line := range result {
		if strings.Contains(line, "Bob") {
			found = true
		}
	}
	if !found {
		t.Errorf("with -a 1: should include unmatched line 'Bob': %v", result)
	}
}

func TestJoinFilesCustomField(t *testing.T) {
	lines1 := []string{"Alice 1", "Bob 2"}
	lines2 := []string{"1 Engineering", "2 Marketing"}

	result := joinFiles(lines1, lines2, JoinOptions{Field1: 2, Field2: 1})

	if len(result) != 2 {
		t.Fatalf("expected 2 output lines, got %d: %v", len(result), result)
	}
}

func TestJoinFilesCustomSeparator(t *testing.T) {
	lines1 := []string{"1:Alice", "2:Bob"}
	lines2 := []string{"1:Engineering", "2:Marketing"}

	result := joinFiles(lines1, lines2, JoinOptions{Field1: 1, Field2: 1, Separator: ":"})

	if len(result) != 2 {
		t.Fatalf("expected 2 output lines, got %d: %v", len(result), result)
	}
	if result[0] != "1:Alice:Engineering" {
		t.Errorf("line 0 = %q, want %q", result[0], "1:Alice:Engineering")
	}
}

func TestJoinFilesIgnoreCase(t *testing.T) {
	lines1 := []string{"alpha data1", "BETA data2"}
	lines2 := []string{"Alpha info1", "beta info2"}

	result := joinFiles(lines1, lines2, JoinOptions{Field1: 1, Field2: 1, IgnoreCase: true})

	if len(result) != 2 {
		t.Errorf("with -i: expected 2 output lines, got %d: %v", len(result), result)
	}
}

func TestJoinFilesEmpty(t *testing.T) {
	result := joinFiles(nil, nil, JoinOptions{Field1: 1, Field2: 1})
	if len(result) != 0 {
		t.Errorf("expected 0 output lines for empty input, got %d", len(result))
	}
}

// =========================================================================
// runJoin integration tests
// =========================================================================

func TestRunJoinBasic(t *testing.T) {
	dir := t.TempDir()
	file1 := filepath.Join(dir, "file1.txt")
	file2 := filepath.Join(dir, "file2.txt")

	os.WriteFile(file1, []byte("1 Alice\n2 Bob\n3 Carol\n"), 0644)
	os.WriteFile(file2, []byte("1 Engineering\n2 Marketing\n3 Sales\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runJoin(toolSpecPath(t, "join"), []string{"join", file1, file2}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runJoin() returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	lines := strings.Split(output, "\n")
	if len(lines) != 3 {
		t.Errorf("expected 3 output lines, got %d: %q", len(lines), output)
	}
}

func TestRunJoinCustomField(t *testing.T) {
	dir := t.TempDir()
	file1 := filepath.Join(dir, "file1.txt")
	file2 := filepath.Join(dir, "file2.txt")

	os.WriteFile(file1, []byte("Alice 1\nBob 2\n"), 0644)
	os.WriteFile(file2, []byte("1 Engineering\n2 Marketing\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runJoin(toolSpecPath(t, "join"), []string{"join", "-1", "2", file1, file2}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runJoin(-1 2) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if !strings.Contains(output, "Alice") {
		t.Errorf("output should contain 'Alice': %q", output)
	}
}

func TestRunJoinNonexistentFile(t *testing.T) {
	dir := t.TempDir()
	file1 := filepath.Join(dir, "exists.txt")
	os.WriteFile(file1, []byte("1 data\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runJoin(toolSpecPath(t, "join"), []string{"join", file1, "/nonexistent/file.txt"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runJoin(nonexistent) returned %d, want 1", code)
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

func TestJoinHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runJoin(toolSpecPath(t, "join"), []string{"join", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runJoin(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runJoin(--help) produced no output")
	}
}

func TestJoinVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runJoin(toolSpecPath(t, "join"), []string{"join", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runJoin(--version) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runJoin(--version) = %q, want %q", output, "1.0.0")
	}
}

func TestJoinInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runJoin("/nonexistent/join.json", []string{"join", "a", "b"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runJoin(bad spec) returned %d, want 1", code)
	}
}

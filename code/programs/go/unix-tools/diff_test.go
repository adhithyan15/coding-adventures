// =========================================================================
// diff — Tests
// =========================================================================
//
// These tests verify the diff tool's behavior, covering:
//
//   1. Spec loading
//   2. Identical files (exit code 0)
//   3. Different files — normal format
//   4. Unified diff format (-u)
//   5. Context diff format (-c)
//   6. Brief mode (-q)
//   7. Ignore case (-i)
//   8. Ignore whitespace (-b, -w)
//   9. Ignore blank lines (-B)
//  10. Recursive directory comparison (-r)
//  11. Error handling (missing files)
//  12. Help and version flags

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
// Spec loading test
// =========================================================================

func TestDiffSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "diff"), []string{"diff", "a", "b"})
	if err != nil {
		t.Fatalf("failed to load diff.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Core algorithm tests — computeDiffEdits
// =========================================================================

func TestDiffIdenticalFiles(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "line1\nline2\nline3\n")
	writeTestFile(t, dir, "b.txt", "line1\nline2\nline3\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", filepath.Join(dir, "a.txt"), filepath.Join(dir, "b.txt")},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0 for identical files", rc)
	}
	if stdout.Len() != 0 {
		t.Errorf("stdout should be empty for identical files, got %q", stdout.String())
	}
}

func TestDiffNormalFormat(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "apple\nbanana\ncherry\n")
	writeTestFile(t, dir, "b.txt", "apple\ncherry\ndate\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", filepath.Join(dir, "a.txt"), filepath.Join(dir, "b.txt")},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1 for different files", rc)
	}

	output := stdout.String()
	// Should contain delete and add markers.
	if !strings.Contains(output, "< banana") {
		t.Errorf("normal diff should show deleted line 'banana', got:\n%s", output)
	}
	if !strings.Contains(output, "> date") {
		t.Errorf("normal diff should show added line 'date', got:\n%s", output)
	}
}

func TestDiffUnifiedFormat(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "line1\nline2\nline3\nline4\n")
	writeTestFile(t, dir, "b.txt", "line1\nmodified\nline3\nline4\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", "--unified=3",
			filepath.Join(dir, "a.txt"), filepath.Join(dir, "b.txt")},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}

	output := stdout.String()
	if !strings.Contains(output, "---") || !strings.Contains(output, "+++") {
		t.Errorf("unified diff should contain --- and +++ headers, got:\n%s", output)
	}
	if !strings.Contains(output, "@@") {
		t.Errorf("unified diff should contain @@ hunk header, got:\n%s", output)
	}
	if !strings.Contains(output, "-line2") {
		t.Errorf("unified diff should show -line2, got:\n%s", output)
	}
	if !strings.Contains(output, "+modified") {
		t.Errorf("unified diff should show +modified, got:\n%s", output)
	}
}

func TestDiffContextFormat(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "alpha\nbeta\ngamma\n")
	writeTestFile(t, dir, "b.txt", "alpha\ndelta\ngamma\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", "--context=3",
			filepath.Join(dir, "a.txt"), filepath.Join(dir, "b.txt")},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}

	output := stdout.String()
	if !strings.Contains(output, "***") {
		t.Errorf("context diff should contain *** markers, got:\n%s", output)
	}
}

func TestDiffBriefMode(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "hello\n")
	writeTestFile(t, dir, "b.txt", "world\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", "-q",
			filepath.Join(dir, "a.txt"), filepath.Join(dir, "b.txt")},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}

	output := stdout.String()
	if !strings.Contains(output, "differ") {
		t.Errorf("brief mode should report files differ, got %q", output)
	}
}

func TestDiffBriefModeIdentical(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "same\n")
	writeTestFile(t, dir, "b.txt", "same\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", "-q",
			filepath.Join(dir, "a.txt"), filepath.Join(dir, "b.txt")},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0 for identical files in brief mode", rc)
	}
}

// =========================================================================
// Flag behavior tests
// =========================================================================

func TestDiffIgnoreCase(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "Hello\nWorld\n")
	writeTestFile(t, dir, "b.txt", "hello\nworld\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", "-i",
			filepath.Join(dir, "a.txt"), filepath.Join(dir, "b.txt")},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0 (case differences ignored)", rc)
	}
}

func TestDiffIgnoreAllSpace(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "hello world\n")
	writeTestFile(t, dir, "b.txt", "helloworld\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", "-w",
			filepath.Join(dir, "a.txt"), filepath.Join(dir, "b.txt")},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0 (whitespace ignored)", rc)
	}
}

func TestDiffIgnoreSpaceChange(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "hello   world\n")
	writeTestFile(t, dir, "b.txt", "hello world\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", "-b",
			filepath.Join(dir, "a.txt"), filepath.Join(dir, "b.txt")},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0 (space changes ignored)", rc)
	}
}

func TestDiffIgnoreBlankLines(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "hello\n\nworld\n")
	writeTestFile(t, dir, "b.txt", "hello\nworld\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", "-B",
			filepath.Join(dir, "a.txt"), filepath.Join(dir, "b.txt")},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0 (blank line changes ignored)", rc)
	}
}

// =========================================================================
// Recursive directory comparison
// =========================================================================

func TestDiffRecursiveDirectories(t *testing.T) {
	dir := t.TempDir()
	dirA := filepath.Join(dir, "a")
	dirB := filepath.Join(dir, "b")
	os.MkdirAll(dirA, 0755)
	os.MkdirAll(dirB, 0755)

	writeTestFile(t, dirA, "same.txt", "same content\n")
	writeTestFile(t, dirB, "same.txt", "same content\n")
	writeTestFile(t, dirA, "diff.txt", "original\n")
	writeTestFile(t, dirB, "diff.txt", "modified\n")
	writeTestFile(t, dirA, "only_a.txt", "only in a\n")
	writeTestFile(t, dirB, "only_b.txt", "only in b\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", "-r", dirA, dirB},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}

	output := stdout.String()
	if !strings.Contains(output, "Only in") {
		t.Errorf("recursive diff should report files only in one dir, got:\n%s", output)
	}
}

// =========================================================================
// Error handling
// =========================================================================

func TestDiffMissingFile(t *testing.T) {
	dir := t.TempDir()
	writeTestFile(t, dir, "a.txt", "hello\n")

	var stdout, stderr bytes.Buffer
	rc := runDiff(toolSpecPath(t, "diff"),
		[]string{"diff", filepath.Join(dir, "a.txt"), filepath.Join(dir, "nonexistent.txt")},
		&stdout, &stderr)

	if rc != 2 {
		t.Errorf("exit code = %d, want 2 for error", rc)
	}
}

func TestDiffInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runDiff("/nonexistent/diff.json", []string{"diff", "a", "b"}, &stdout, &stderr)
	if rc != 2 {
		t.Errorf("exit code = %d, want 2 for invalid spec", rc)
	}
}

// =========================================================================
// Helper — normalizeDiffLine
// =========================================================================

func TestNormalizeDiffLine(t *testing.T) {
	tests := []struct {
		name     string
		line     string
		opts     DiffOptions
		expected string
	}{
		{"no options", "Hello World", DiffOptions{}, "Hello World"},
		{"ignore case", "Hello World", DiffOptions{IgnoreCase: true}, "hello world"},
		{"ignore all space", "hello world", DiffOptions{IgnoreAllSpace: true}, "helloworld"},
		{"ignore space change", "hello   world", DiffOptions{IgnoreSpaceChg: true}, "hello world"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := normalizeDiffLine(tt.line, tt.opts)
			if result != tt.expected {
				t.Errorf("normalizeDiffLine(%q) = %q, want %q", tt.line, result, tt.expected)
			}
		})
	}
}

// =========================================================================
// Helper — computeDiffEdits
// =========================================================================

func TestComputeDiffEditsEmpty(t *testing.T) {
	edits := computeDiffEdits([]string{}, []string{}, DiffOptions{})
	if len(edits) != 0 {
		t.Errorf("edits for empty files should be empty, got %d edits", len(edits))
	}
}

func TestComputeDiffEditsAllNew(t *testing.T) {
	edits := computeDiffEdits([]string{}, []string{"a", "b"}, DiffOptions{})
	insertCount := 0
	for _, e := range edits {
		if e.Op == '+' {
			insertCount++
		}
	}
	if insertCount != 2 {
		t.Errorf("expected 2 inserts, got %d", insertCount)
	}
}

func TestComputeDiffEditsAllDeleted(t *testing.T) {
	edits := computeDiffEdits([]string{"a", "b"}, []string{}, DiffOptions{})
	deleteCount := 0
	for _, e := range edits {
		if e.Op == '-' {
			deleteCount++
		}
	}
	if deleteCount != 2 {
		t.Errorf("expected 2 deletes, got %d", deleteCount)
	}
}

// =========================================================================
// Helper — diffRange
// =========================================================================

func TestDiffRange(t *testing.T) {
	if diffRange(3, 3) != "3" {
		t.Errorf("diffRange(3,3) = %q, want \"3\"", diffRange(3, 3))
	}
	if diffRange(3, 5) != "3,5" {
		t.Errorf("diffRange(3,5) = %q, want \"3,5\"", diffRange(3, 5))
	}
}

// =========================================================================
// Helper — hasDiffChanges
// =========================================================================

func TestHasDiffChangesNoChanges(t *testing.T) {
	edits := []diffEdit{{Op: '=', Line: "same"}}
	if hasDiffChanges(edits) {
		t.Error("hasDiffChanges should return false when all edits are equal")
	}
}

func TestHasDiffChangesWithChanges(t *testing.T) {
	edits := []diffEdit{{Op: '=', Line: "same"}, {Op: '-', Line: "del"}}
	if !hasDiffChanges(edits) {
		t.Error("hasDiffChanges should return true when there are changes")
	}
}

// =========================================================================
// Helper function for writing test files
// =========================================================================

func writeTestFile(t *testing.T, dir, name, content string) {
	t.Helper()
	err := os.WriteFile(filepath.Join(dir, name), []byte(content), 0644)
	if err != nil {
		t.Fatalf("failed to write test file %s: %v", name, err)
	}
}

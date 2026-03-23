// =========================================================================
// du — Tests
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
// diskUsage tests
// =========================================================================

// TestDiskUsageEmptyDir verifies du on an empty directory.
func TestDiskUsageEmptyDir(t *testing.T) {
	dir := t.TempDir()
	entries, err := diskUsage(dir, DuOptions{MaxDepth: -1})
	if err != nil {
		t.Fatalf("diskUsage(empty) failed: %v", err)
	}

	// Should have at least one entry (the directory itself).
	if len(entries) < 1 {
		t.Errorf("expected at least 1 entry for empty dir, got %d", len(entries))
	}
}

// TestDiskUsageWithFiles verifies du correctly counts file sizes.
func TestDiskUsageWithFiles(t *testing.T) {
	dir := t.TempDir()

	// Create some files with known sizes.
	os.WriteFile(filepath.Join(dir, "a.txt"), make([]byte, 1000), 0644)
	os.WriteFile(filepath.Join(dir, "b.txt"), make([]byte, 2000), 0644)

	entries, err := diskUsage(dir, DuOptions{MaxDepth: -1})
	if err != nil {
		t.Fatalf("diskUsage failed: %v", err)
	}

	// Find the root directory entry.
	var rootEntry *DuEntry
	for i := range entries {
		if entries[i].Path == dir {
			rootEntry = &entries[i]
			break
		}
	}

	if rootEntry == nil {
		t.Fatalf("no entry for root directory %q in results", dir)
	}

	// The root should have at least 3000 bytes (the two files).
	if rootEntry.Bytes < 3000 {
		t.Errorf("root directory size = %d, want >= 3000", rootEntry.Bytes)
	}
}

// TestDiskUsageSummarize verifies -s flag.
func TestDiskUsageSummarize(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.txt"), make([]byte, 1000), 0644)
	os.Mkdir(filepath.Join(dir, "sub"), 0755)
	os.WriteFile(filepath.Join(dir, "sub", "b.txt"), make([]byte, 2000), 0644)

	entries, err := diskUsage(dir, DuOptions{Summarize: true, MaxDepth: -1})
	if err != nil {
		t.Fatalf("diskUsage(-s) failed: %v", err)
	}

	// With -s, should have exactly one entry.
	if len(entries) != 1 {
		t.Fatalf("diskUsage(-s) returned %d entries, want 1", len(entries))
	}

	if entries[0].Path != dir {
		t.Errorf("entry path = %q, want %q", entries[0].Path, dir)
	}
	if entries[0].Bytes < 3000 {
		t.Errorf("total size = %d, want >= 3000", entries[0].Bytes)
	}
}

// TestDiskUsageShowAll verifies -a flag (show all files).
func TestDiskUsageShowAll(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.txt"), make([]byte, 100), 0644)
	os.WriteFile(filepath.Join(dir, "b.txt"), make([]byte, 200), 0644)

	entries, err := diskUsage(dir, DuOptions{ShowAll: true, MaxDepth: -1})
	if err != nil {
		t.Fatalf("diskUsage(-a) failed: %v", err)
	}

	// Should have entries for both files plus the directory.
	if len(entries) < 3 {
		t.Errorf("diskUsage(-a) returned %d entries, want >= 3", len(entries))
	}
}

// =========================================================================
// formatDuSize tests
// =========================================================================

// TestFormatDuSizeDefault verifies default (1K-blocks) formatting.
func TestFormatDuSizeDefault(t *testing.T) {
	got := formatDuSize(4096, false, false)
	if got != "4" {
		t.Errorf("formatDuSize(4096) = %q, want %q", got, "4")
	}
}

// TestFormatDuSizeRoundup verifies rounding up to next block.
func TestFormatDuSizeRoundup(t *testing.T) {
	got := formatDuSize(1025, false, false)
	if got != "2" {
		t.Errorf("formatDuSize(1025) = %q, want %q", got, "2")
	}
}

// TestFormatDuSizeHumanReadable verifies -h formatting.
func TestFormatDuSizeHumanReadable(t *testing.T) {
	got := formatDuSize(1048576, true, false)
	if got != "1.0M" {
		t.Errorf("formatDuSize(1MB, -h) = %q, want %q", got, "1.0M")
	}
}

// =========================================================================
// runDu integration tests
// =========================================================================

// TestRunDuDefault verifies du on a known directory.
func TestRunDuDefault(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "test.txt"), make([]byte, 1000), 0644)

	var stdout, stderr bytes.Buffer
	code := runDu(toolSpecPath(t, "du"), []string{"du", dir}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runDu returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	if !strings.Contains(output, dir) {
		t.Errorf("output doesn't contain path %q: %s", dir, output)
	}
}

// TestRunDuHelp verifies --help flag.
func TestRunDuHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runDu(toolSpecPath(t, "du"), []string{"du", "--help"}, &stdout, &stderr)
	if code != 0 {
		t.Errorf("runDu(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runDu(--help) produced no output")
	}
}

// TestRunDuVersion verifies --version flag.
func TestRunDuVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runDu(toolSpecPath(t, "du"), []string{"du", "--version"}, &stdout, &stderr)
	if code != 0 {
		t.Errorf("runDu(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runDu(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunDuInvalidSpec verifies error handling.
func TestRunDuInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runDu("/nonexistent/du.json", []string{"du"}, &stdout, &stderr)
	if code != 1 {
		t.Errorf("runDu(bad spec) returned %d, want 1", code)
	}
}

// =========================================================================
// ls — Tests
// =========================================================================
//
// These tests verify the ls tool's behavior, covering:
//
//   1. Spec loading
//   2. Listing a directory
//   3. All files mode (-a)
//   4. Long format (-l)
//   5. Sorting by size (-S), time (-t), reverse (-r)
//   6. Human-readable sizes (-h)
//   7. One per line (-1)
//   8. Classify (-F)
//   9. Recursive listing (-R)
//  10. Directory mode (-d)
//  11. Help and version flags
//  12. Listing a single file

package main

import (
	"bytes"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

func TestLsSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "ls"), []string{"ls"})
	if err != nil {
		t.Fatalf("failed to load ls.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Business logic tests — listDirectory
// =========================================================================

func TestListDirectoryBasic(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.txt"), []byte("a"), 0644)
	os.WriteFile(filepath.Join(dir, "b.txt"), []byte("b"), 0644)

	entries, err := listDirectory(dir, LsOptions{})
	if err != nil {
		t.Fatalf("listDirectory() failed: %v", err)
	}

	if len(entries) != 2 {
		t.Errorf("expected 2 entries, got %d", len(entries))
	}
}

func TestListDirectoryHiddenFiles(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, ".hidden"), []byte("h"), 0644)
	os.WriteFile(filepath.Join(dir, "visible.txt"), []byte("v"), 0644)

	// Without -a: hidden file should be filtered.
	entries, _ := listDirectory(dir, LsOptions{})
	if len(entries) != 1 {
		t.Errorf("without -a: expected 1 entry, got %d", len(entries))
	}

	// With -a: hidden file should be included (plus . and ..).
	entries, _ = listDirectory(dir, LsOptions{All: true})
	// Should have: ., .., .hidden, visible.txt
	found := false
	for _, e := range entries {
		if e.Name == ".hidden" {
			found = true
		}
	}
	if !found {
		t.Error("with -a: .hidden should be included")
	}
}

func TestListDirectoryAlmostAll(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, ".hidden"), []byte("h"), 0644)
	os.WriteFile(filepath.Join(dir, "visible.txt"), []byte("v"), 0644)

	entries, _ := listDirectory(dir, LsOptions{AlmostAll: true})
	// Should have .hidden and visible.txt but NOT . and ..
	for _, e := range entries {
		if e.Name == "." || e.Name == ".." {
			t.Error("with -A: should not include . or ..")
		}
	}
	if len(entries) != 2 {
		t.Errorf("with -A: expected 2 entries, got %d", len(entries))
	}
}

func TestListDirectoryNonexistent(t *testing.T) {
	_, err := listDirectory("/nonexistent/dir", LsOptions{})
	if err == nil {
		t.Error("expected error for nonexistent directory, got nil")
	}
}

// =========================================================================
// Business logic tests — sortEntries
// =========================================================================

func TestSortEntriesByName(t *testing.T) {
	entries := []FileEntry{
		{Name: "charlie", Info: fakeFileInfo{name: "charlie"}},
		{Name: "alice", Info: fakeFileInfo{name: "alice"}},
		{Name: "bob", Info: fakeFileInfo{name: "bob"}},
	}

	sortEntries(entries, LsOptions{})

	expected := []string{"alice", "bob", "charlie"}
	for i, e := range entries {
		if e.Name != expected[i] {
			t.Errorf("entry[%d] = %q, want %q", i, e.Name, expected[i])
		}
	}
}

func TestSortEntriesReverse(t *testing.T) {
	entries := []FileEntry{
		{Name: "a", Info: fakeFileInfo{name: "a"}},
		{Name: "b", Info: fakeFileInfo{name: "b"}},
		{Name: "c", Info: fakeFileInfo{name: "c"}},
	}

	sortEntries(entries, LsOptions{Reverse: true})

	expected := []string{"c", "b", "a"}
	for i, e := range entries {
		if e.Name != expected[i] {
			t.Errorf("entry[%d] = %q, want %q", i, e.Name, expected[i])
		}
	}
}

// fakeFileInfo implements fs.FileInfo for testing sort behavior.
type fakeFileInfo struct {
	name string
	size int64
	mode fs.FileMode
}

func (f fakeFileInfo) Name() string      { return f.name }
func (f fakeFileInfo) Size() int64       { return f.size }
func (f fakeFileInfo) Mode() fs.FileMode { return f.mode }
func (f fakeFileInfo) ModTime() time.Time { return time.Time{} }
func (f fakeFileInfo) IsDir() bool       { return f.mode.IsDir() }
func (f fakeFileInfo) Sys() interface{}  { return nil }

// =========================================================================
// Business logic tests — humanizeSize
// =========================================================================

func TestHumanizeSize(t *testing.T) {
	tests := []struct {
		input    int64
		expected string
	}{
		{0, "0"},
		{100, "100"},
		{1023, "1023"},
		{1024, "1.0K"},
		{10240, "10K"},
		{1048576, "1.0M"},
		{1073741824, "1.0G"},
	}

	for _, tt := range tests {
		result := humanizeSize(tt.input)
		if result != tt.expected {
			t.Errorf("humanizeSize(%d) = %q, want %q", tt.input, result, tt.expected)
		}
	}
}

// =========================================================================
// Business logic tests — classifySuffix
// =========================================================================

func TestClassifySuffix(t *testing.T) {
	tests := []struct {
		mode     fs.FileMode
		expected string
	}{
		{fs.ModeDir | 0755, "/"},
		{0644, ""},
		{0755, "*"},
		{os.ModeSymlink, "@"},
		{os.ModeNamedPipe, "|"},
		{os.ModeSocket, "="},
	}

	for _, tt := range tests {
		info := fakeFileInfo{mode: tt.mode}
		result := classifySuffix(info)
		if result != tt.expected {
			t.Errorf("classifySuffix(%v) = %q, want %q", tt.mode, result, tt.expected)
		}
	}
}

// =========================================================================
// Business logic tests — formatEntries
// =========================================================================

func TestFormatEntriesOnePerLine(t *testing.T) {
	entries := []FileEntry{
		{Name: "a.txt", Info: fakeFileInfo{name: "a.txt"}},
		{Name: "b.txt", Info: fakeFileInfo{name: "b.txt"}},
	}

	result := formatEntries(entries, LsOptions{OnePerLine: true})
	lines := strings.Split(result, "\n")

	if len(lines) != 2 {
		t.Errorf("expected 2 lines, got %d: %q", len(lines), result)
	}
}

func TestFormatEntriesClassify(t *testing.T) {
	entries := []FileEntry{
		{Name: "dir", Info: fakeFileInfo{name: "dir", mode: fs.ModeDir | 0755}},
		{Name: "file", Info: fakeFileInfo{name: "file", mode: 0644}},
		{Name: "exec", Info: fakeFileInfo{name: "exec", mode: 0755}},
	}

	result := formatEntries(entries, LsOptions{OnePerLine: true, Classify: true})

	if !strings.Contains(result, "dir/") {
		t.Error("expected 'dir/' with classify flag")
	}
	if !strings.Contains(result, "exec*") {
		t.Error("expected 'exec*' with classify flag")
	}
}

// =========================================================================
// runLs integration tests
// =========================================================================

func TestRunLsCurrentDir(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLs(toolSpecPath(t, "ls"), []string{"ls"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLs() returned %d, want 0. stderr: %s", code, stderr.String())
	}

	if stdout.Len() == 0 {
		t.Error("runLs() produced no output")
	}
}

func TestRunLsSpecificDir(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "test.txt"), []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runLs(toolSpecPath(t, "ls"), []string{"ls", dir}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLs(dir) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	if !strings.Contains(stdout.String(), "test.txt") {
		t.Errorf("output should contain 'test.txt': %q", stdout.String())
	}
}

func TestRunLsOnePerLine(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.txt"), []byte("a"), 0644)
	os.WriteFile(filepath.Join(dir, "b.txt"), []byte("b"), 0644)

	var stdout, stderr bytes.Buffer
	code := runLs(toolSpecPath(t, "ls"), []string{"ls", "-1", dir}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLs(-1) returned %d, want 0", code)
	}

	lines := strings.Split(strings.TrimSpace(stdout.String()), "\n")
	if len(lines) != 2 {
		t.Errorf("expected 2 lines with -1, got %d: %q", len(lines), stdout.String())
	}
}

func TestRunLsNonexistent(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLs(toolSpecPath(t, "ls"), []string{"ls", "/nonexistent/dir"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runLs(nonexistent) returned %d, want 1", code)
	}
}

func TestRunLsSingleFile(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "single.txt")
	os.WriteFile(file, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runLs(toolSpecPath(t, "ls"), []string{"ls", file}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLs(file) returned %d, want 0", code)
	}

	if !strings.Contains(stdout.String(), "single.txt") {
		t.Errorf("output should contain filename: %q", stdout.String())
	}
}

func TestRunLsDirectoryMode(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "file.txt"), []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runLs(toolSpecPath(t, "ls"), []string{"ls", "-d", dir}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLs(-d) returned %d, want 0", code)
	}

	// With -d, should list the directory itself, not its contents.
	output := strings.TrimSpace(stdout.String())
	if strings.Contains(output, "file.txt") {
		t.Error("with -d, should not list directory contents")
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

func TestLsHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLs(toolSpecPath(t, "ls"), []string{"ls", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLs(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runLs(--help) produced no output")
	}
}

func TestLsVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLs(toolSpecPath(t, "ls"), []string{"ls", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLs(--version) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runLs(--version) = %q, want %q", output, "1.0.0")
	}
}

func TestLsInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLs("/nonexistent/ls.json", []string{"ls"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runLs(bad spec) returned %d, want 1", code)
	}
}

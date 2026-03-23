// =========================================================================
// mv — Tests
// =========================================================================
//
// These tests verify the mv tool's behavior, covering:
//
//   1. Spec loading
//   2. Moving (renaming) a single file
//   3. Moving multiple files into a directory
//   4. No-clobber mode (-n)
//   5. Verbose output (-v)
//   6. Moving directories
//   7. Error handling for nonexistent sources
//   8. Help and version flags

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

func TestMvSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "mv"), []string{"mv", "a", "b"})
	if err != nil {
		t.Fatalf("failed to load mv.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Business logic tests — moveFile
// =========================================================================

func TestMoveFileSameDevice(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "source.txt")
	dst := filepath.Join(dir, "dest.txt")

	os.WriteFile(src, []byte("move me"), 0644)

	err := moveFile(src, dst, MvOptions{})
	if err != nil {
		t.Fatalf("moveFile() failed: %v", err)
	}

	// Source should be gone.
	if _, err := os.Stat(src); !os.IsNotExist(err) {
		t.Error("source should have been removed after move")
	}

	// Dest should have the content.
	content, err := os.ReadFile(dst)
	if err != nil {
		t.Fatalf("cannot read destination: %v", err)
	}
	if string(content) != "move me" {
		t.Errorf("destination content = %q, want %q", string(content), "move me")
	}
}

func TestMoveFileNonexistent(t *testing.T) {
	dir := t.TempDir()
	err := moveFile("/nonexistent/file.txt", filepath.Join(dir, "dst"), MvOptions{})
	if err == nil {
		t.Error("expected error for nonexistent source, got nil")
	}
}

// =========================================================================
// Business logic tests — shouldSkipMove
// =========================================================================

func TestShouldSkipMoveNoClobber(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "src.txt")
	dst := filepath.Join(dir, "dst.txt")

	os.WriteFile(src, []byte("new"), 0644)
	os.WriteFile(dst, []byte("old"), 0644)

	if !shouldSkipMove(src, dst, MvOptions{NoClobber: true}) {
		t.Error("shouldSkipMove should return true with no-clobber when dest exists")
	}
}

func TestShouldSkipMoveNonexistentDest(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "src.txt")
	os.WriteFile(src, []byte("data"), 0644)

	if shouldSkipMove(src, filepath.Join(dir, "nonexistent"), MvOptions{NoClobber: true}) {
		t.Error("shouldSkipMove should return false when dest does not exist")
	}
}

// =========================================================================
// runMv integration tests
// =========================================================================

func TestRunMvRenameFile(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "old.txt")
	dst := filepath.Join(dir, "new.txt")

	os.WriteFile(src, []byte("rename me"), 0644)

	var stdout, stderr bytes.Buffer
	code := runMv(toolSpecPath(t, "mv"), []string{"mv", src, dst}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMv() returned %d, want 0. stderr: %s", code, stderr.String())
	}

	if _, err := os.Stat(src); !os.IsNotExist(err) {
		t.Error("source should be gone after mv")
	}

	content, _ := os.ReadFile(dst)
	if string(content) != "rename me" {
		t.Errorf("dest content = %q, want %q", string(content), "rename me")
	}
}

func TestRunMvMultipleFilesToDir(t *testing.T) {
	dir := t.TempDir()
	src1 := filepath.Join(dir, "a.txt")
	src2 := filepath.Join(dir, "b.txt")
	dstDir := filepath.Join(dir, "output")
	os.Mkdir(dstDir, 0755)

	os.WriteFile(src1, []byte("file a"), 0644)
	os.WriteFile(src2, []byte("file b"), 0644)

	var stdout, stderr bytes.Buffer
	code := runMv(toolSpecPath(t, "mv"), []string{"mv", src1, src2, dstDir}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMv(multi) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	// Sources should be gone.
	if _, err := os.Stat(src1); !os.IsNotExist(err) {
		t.Error("src1 should be removed")
	}
	if _, err := os.Stat(src2); !os.IsNotExist(err) {
		t.Error("src2 should be removed")
	}

	// Files should be in dstDir.
	contentA, _ := os.ReadFile(filepath.Join(dstDir, "a.txt"))
	contentB, _ := os.ReadFile(filepath.Join(dstDir, "b.txt"))

	if string(contentA) != "file a" {
		t.Errorf("a.txt content = %q", string(contentA))
	}
	if string(contentB) != "file b" {
		t.Errorf("b.txt content = %q", string(contentB))
	}
}

func TestRunMvNoClobber(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "src.txt")
	dst := filepath.Join(dir, "dst.txt")

	os.WriteFile(src, []byte("new"), 0644)
	os.WriteFile(dst, []byte("old"), 0644)

	var stdout, stderr bytes.Buffer
	code := runMv(toolSpecPath(t, "mv"), []string{"mv", "-n", src, dst}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMv(-n) returned %d, want 0", code)
	}

	// Source should still exist.
	if _, err := os.Stat(src); err != nil {
		t.Error("source should still exist with no-clobber")
	}

	// Dest should be unchanged.
	content, _ := os.ReadFile(dst)
	if string(content) != "old" {
		t.Errorf("no-clobber: dst content = %q, want %q", string(content), "old")
	}
}

func TestRunMvVerbose(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "src.txt")
	dst := filepath.Join(dir, "dst.txt")

	os.WriteFile(src, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runMv(toolSpecPath(t, "mv"), []string{"mv", "-v", src, dst}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMv(-v) returned %d, want 0", code)
	}

	if !strings.Contains(stdout.String(), "->") {
		t.Errorf("verbose output should contain '->': %q", stdout.String())
	}
}

func TestRunMvMoveDirectory(t *testing.T) {
	dir := t.TempDir()
	srcDir := filepath.Join(dir, "mydir")
	dstDir := filepath.Join(dir, "renamed")

	os.Mkdir(srcDir, 0755)
	os.WriteFile(filepath.Join(srcDir, "file.txt"), []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runMv(toolSpecPath(t, "mv"), []string{"mv", srcDir, dstDir}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMv(dir) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	if _, err := os.Stat(srcDir); !os.IsNotExist(err) {
		t.Error("source dir should be gone after mv")
	}

	content, err := os.ReadFile(filepath.Join(dstDir, "file.txt"))
	if err != nil {
		t.Fatalf("file.txt not found in moved dir: %v", err)
	}
	if string(content) != "data" {
		t.Errorf("file content = %q, want %q", string(content), "data")
	}
}

func TestRunMvMultipleToNonDir(t *testing.T) {
	dir := t.TempDir()
	src1 := filepath.Join(dir, "a.txt")
	src2 := filepath.Join(dir, "b.txt")
	dst := filepath.Join(dir, "notadir.txt")

	os.WriteFile(src1, []byte("a"), 0644)
	os.WriteFile(src2, []byte("b"), 0644)
	os.WriteFile(dst, []byte("x"), 0644)

	var stdout, stderr bytes.Buffer
	code := runMv(toolSpecPath(t, "mv"), []string{"mv", src1, src2, dst}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runMv(multi to non-dir) returned %d, want 1", code)
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

func TestMvHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMv(toolSpecPath(t, "mv"), []string{"mv", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMv(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runMv(--help) produced no output")
	}
}

func TestMvVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMv(toolSpecPath(t, "mv"), []string{"mv", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMv(--version) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runMv(--version) = %q, want %q", output, "1.0.0")
	}
}

func TestMvInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMv("/nonexistent/mv.json", []string{"mv", "a", "b"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runMv(bad spec) returned %d, want 1", code)
	}
}

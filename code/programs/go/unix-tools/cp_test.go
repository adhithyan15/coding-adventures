// =========================================================================
// cp — Tests
// =========================================================================
//
// These tests verify the cp tool's behavior, covering:
//
//   1. Spec loading
//   2. Copying a single file
//   3. Copying multiple files into a directory
//   4. Recursive directory copying (-R)
//   5. No-clobber mode (-n)
//   6. Update mode (-u)
//   7. Verbose output (-v)
//   8. Hard link mode (-l)
//   9. Symbolic link mode (-s)
//  10. Force mode (-f)
//  11. Refusing to copy directories without -R
//  12. Help and version flags

package main

import (
	"bytes"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

func TestCpSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "cp"), []string{"cp", "a", "b"})
	if err != nil {
		t.Fatalf("failed to load cp.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Business logic tests — copyFile
// =========================================================================

func TestCopyFileSingleFile(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "source.txt")
	dst := filepath.Join(dir, "dest.txt")

	os.WriteFile(src, []byte("hello world"), 0644)

	err := copyFile(src, dst, CpOptions{})
	if err != nil {
		t.Fatalf("copyFile() failed: %v", err)
	}

	content, err := os.ReadFile(dst)
	if err != nil {
		t.Fatalf("cannot read destination: %v", err)
	}

	if string(content) != "hello world" {
		t.Errorf("destination content = %q, want %q", string(content), "hello world")
	}
}

func TestCopyFilePreservesPermissions(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "source.txt")
	dst := filepath.Join(dir, "dest.txt")

	os.WriteFile(src, []byte("data"), 0755)

	err := copyFile(src, dst, CpOptions{})
	if err != nil {
		t.Fatalf("copyFile() failed: %v", err)
	}

	srcInfo, _ := os.Stat(src)
	dstInfo, _ := os.Stat(dst)

	if srcInfo.Mode() != dstInfo.Mode() {
		t.Errorf("permissions differ: src=%v, dst=%v", srcInfo.Mode(), dstInfo.Mode())
	}
}

func TestCopyFileHardLink(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "source.txt")
	dst := filepath.Join(dir, "dest.txt")

	os.WriteFile(src, []byte("linked"), 0644)

	err := copyFile(src, dst, CpOptions{Link: true})
	if err != nil {
		t.Fatalf("copyFile(link) failed: %v", err)
	}

	// Both files should have the same content.
	content, _ := os.ReadFile(dst)
	if string(content) != "linked" {
		t.Errorf("hard link content = %q, want %q", string(content), "linked")
	}

	// Verify it's actually a hard link by checking inode numbers.
	srcInfo, _ := os.Stat(src)
	dstInfo, _ := os.Stat(dst)
	if !os.SameFile(srcInfo, dstInfo) {
		t.Error("expected hard link (same inode), got different files")
	}
}

func TestCopyFileSymLink(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "source.txt")
	dst := filepath.Join(dir, "dest.txt")

	os.WriteFile(src, []byte("symlinked"), 0644)

	err := copyFile(src, dst, CpOptions{SymLink: true})
	if err != nil {
		t.Fatalf("copyFile(symlink) failed: %v", err)
	}

	// Destination should be a symlink.
	linkTarget, err := os.Readlink(dst)
	if err != nil {
		t.Fatalf("dest is not a symlink: %v", err)
	}
	if linkTarget != src {
		t.Errorf("symlink target = %q, want %q", linkTarget, src)
	}
}

func TestCopyFileNonexistentSource(t *testing.T) {
	dir := t.TempDir()
	err := copyFile("/nonexistent/file.txt", filepath.Join(dir, "dst"), CpOptions{})
	if err == nil {
		t.Error("expected error for nonexistent source, got nil")
	}
}

// =========================================================================
// Business logic tests — copyDir
// =========================================================================

func TestCopyDirRecursive(t *testing.T) {
	dir := t.TempDir()
	srcDir := filepath.Join(dir, "src")
	dstDir := filepath.Join(dir, "dst")

	// Create source directory structure.
	os.MkdirAll(filepath.Join(srcDir, "sub"), 0755)
	os.WriteFile(filepath.Join(srcDir, "file1.txt"), []byte("file1"), 0644)
	os.WriteFile(filepath.Join(srcDir, "sub", "file2.txt"), []byte("file2"), 0644)

	err := copyDir(srcDir, dstDir, CpOptions{}, io.Discard)
	if err != nil {
		t.Fatalf("copyDir() failed: %v", err)
	}

	// Verify destination structure.
	content1, err := os.ReadFile(filepath.Join(dstDir, "file1.txt"))
	if err != nil {
		t.Fatalf("file1.txt not copied: %v", err)
	}
	if string(content1) != "file1" {
		t.Errorf("file1.txt content = %q, want %q", string(content1), "file1")
	}

	content2, err := os.ReadFile(filepath.Join(dstDir, "sub", "file2.txt"))
	if err != nil {
		t.Fatalf("sub/file2.txt not copied: %v", err)
	}
	if string(content2) != "file2" {
		t.Errorf("sub/file2.txt content = %q, want %q", string(content2), "file2")
	}
}

// =========================================================================
// Business logic tests — shouldSkipCopy
// =========================================================================

func TestShouldSkipCopyNoClobber(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "src.txt")
	dst := filepath.Join(dir, "dst.txt")

	os.WriteFile(src, []byte("new"), 0644)
	os.WriteFile(dst, []byte("old"), 0644)

	opts := CpOptions{NoClobber: true}
	if !shouldSkipCopy(src, dst, opts) {
		t.Error("shouldSkipCopy should return true with no-clobber when dest exists")
	}
}

func TestShouldSkipCopyNonexistentDest(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "src.txt")
	dst := filepath.Join(dir, "nonexistent.txt")

	os.WriteFile(src, []byte("data"), 0644)

	opts := CpOptions{NoClobber: true}
	if shouldSkipCopy(src, dst, opts) {
		t.Error("shouldSkipCopy should return false when dest does not exist")
	}
}

func TestShouldSkipCopyUpdateNewerSource(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "src.txt")
	dst := filepath.Join(dir, "dst.txt")

	// Create dest first, then source (source is newer).
	os.WriteFile(dst, []byte("old"), 0644)
	os.WriteFile(src, []byte("new"), 0644)

	opts := CpOptions{Update: true}
	if shouldSkipCopy(src, dst, opts) {
		t.Error("shouldSkipCopy should return false when source is newer")
	}
}

// =========================================================================
// runCp integration tests
// =========================================================================

func TestRunCpSingleFile(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "source.txt")
	dst := filepath.Join(dir, "dest.txt")

	os.WriteFile(src, []byte("test data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runCp(toolSpecPath(t, "cp"), []string{"cp", src, dst}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runCp() returned %d, want 0. stderr: %s", code, stderr.String())
	}

	content, _ := os.ReadFile(dst)
	if string(content) != "test data" {
		t.Errorf("dest content = %q, want %q", string(content), "test data")
	}
}

func TestRunCpMultipleFilesToDir(t *testing.T) {
	dir := t.TempDir()
	src1 := filepath.Join(dir, "a.txt")
	src2 := filepath.Join(dir, "b.txt")
	dstDir := filepath.Join(dir, "output")
	os.Mkdir(dstDir, 0755)

	os.WriteFile(src1, []byte("file a"), 0644)
	os.WriteFile(src2, []byte("file b"), 0644)

	var stdout, stderr bytes.Buffer
	code := runCp(toolSpecPath(t, "cp"), []string{"cp", src1, src2, dstDir}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runCp(multi) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	contentA, _ := os.ReadFile(filepath.Join(dstDir, "a.txt"))
	contentB, _ := os.ReadFile(filepath.Join(dstDir, "b.txt"))

	if string(contentA) != "file a" {
		t.Errorf("a.txt content = %q, want %q", string(contentA), "file a")
	}
	if string(contentB) != "file b" {
		t.Errorf("b.txt content = %q, want %q", string(contentB), "file b")
	}
}

func TestRunCpRecursiveDir(t *testing.T) {
	dir := t.TempDir()
	srcDir := filepath.Join(dir, "mydir")
	dstDir := filepath.Join(dir, "copy")

	os.MkdirAll(filepath.Join(srcDir, "sub"), 0755)
	os.WriteFile(filepath.Join(srcDir, "file.txt"), []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runCp(toolSpecPath(t, "cp"), []string{"cp", "-R", srcDir, dstDir}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runCp(-R) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	content, err := os.ReadFile(filepath.Join(dstDir, "file.txt"))
	if err != nil {
		t.Fatalf("file not copied recursively: %v", err)
	}
	if string(content) != "data" {
		t.Errorf("file content = %q, want %q", string(content), "data")
	}
}

func TestRunCpDirectoryWithoutRecursive(t *testing.T) {
	dir := t.TempDir()
	srcDir := filepath.Join(dir, "mydir")
	os.Mkdir(srcDir, 0755)

	var stdout, stderr bytes.Buffer
	code := runCp(toolSpecPath(t, "cp"), []string{"cp", srcDir, filepath.Join(dir, "copy")}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runCp(dir without -R) returned %d, want 1", code)
	}
}

func TestRunCpNoClobber(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "src.txt")
	dst := filepath.Join(dir, "dst.txt")

	os.WriteFile(src, []byte("new"), 0644)
	os.WriteFile(dst, []byte("old"), 0644)

	var stdout, stderr bytes.Buffer
	code := runCp(toolSpecPath(t, "cp"), []string{"cp", "-n", src, dst}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runCp(-n) returned %d, want 0", code)
	}

	// Destination should be unchanged.
	content, _ := os.ReadFile(dst)
	if string(content) != "old" {
		t.Errorf("no-clobber: dst content = %q, want %q", string(content), "old")
	}
}

func TestRunCpVerbose(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "src.txt")
	dst := filepath.Join(dir, "dst.txt")

	os.WriteFile(src, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runCp(toolSpecPath(t, "cp"), []string{"cp", "-v", src, dst}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runCp(-v) returned %d, want 0", code)
	}

	if !strings.Contains(stdout.String(), "->") {
		t.Errorf("verbose output should contain '->': %q", stdout.String())
	}
}

func TestRunCpNonexistentSource(t *testing.T) {
	dir := t.TempDir()
	var stdout, stderr bytes.Buffer
	code := runCp(toolSpecPath(t, "cp"), []string{"cp", "/nonexistent/file.txt", filepath.Join(dir, "dst")}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runCp(nonexistent) returned %d, want 1", code)
	}
}

func TestRunCpMultipleToNonDir(t *testing.T) {
	dir := t.TempDir()
	src1 := filepath.Join(dir, "a.txt")
	src2 := filepath.Join(dir, "b.txt")
	dst := filepath.Join(dir, "notadir.txt")

	os.WriteFile(src1, []byte("a"), 0644)
	os.WriteFile(src2, []byte("b"), 0644)
	os.WriteFile(dst, []byte("x"), 0644) // Regular file, not a dir.

	var stdout, stderr bytes.Buffer
	code := runCp(toolSpecPath(t, "cp"), []string{"cp", src1, src2, dst}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runCp(multi to non-dir) returned %d, want 1", code)
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

func TestCpHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCp(toolSpecPath(t, "cp"), []string{"cp", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runCp(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runCp(--help) produced no output")
	}
}

func TestCpVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCp(toolSpecPath(t, "cp"), []string{"cp", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runCp(--version) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runCp(--version) = %q, want %q", output, "1.0.0")
	}
}

func TestCpInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runCp("/nonexistent/cp.json", []string{"cp", "a", "b"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runCp(bad spec) returned %d, want 1", code)
	}
}

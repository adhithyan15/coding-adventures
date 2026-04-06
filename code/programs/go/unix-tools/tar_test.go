// =========================================================================
// tar — Tests
// =========================================================================
//
// These tests verify the tar tool's behavior, covering:
//
//   1. Spec loading
//   2. Creating a tar archive from files
//   3. Listing archive contents (-t)
//   4. Extracting files from an archive (-x)
//   5. Verbose mode (-v)
//   6. Archiving directories
//   7. Keep old files (-k)
//   8. Strip components (--strip-components)
//   9. Change directory (-C)
//  10. Compression flag stubs (-z, -j, -J)
//  11. Path traversal security
//  12. Error handling

package main

import (
	"archive/tar"
	"bytes"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading test
// =========================================================================

func TestTarSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "tar"), []string{"tar", "-cf", "out.tar", "file"})
	if err != nil {
		t.Fatalf("failed to load tar.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Create and list round-trip test
// =========================================================================

func TestTarCreateAndList(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "hello.txt"), []byte("hello world"), 0644)
	os.WriteFile(filepath.Join(dir, "bye.txt"), []byte("goodbye"), 0644)

	archivePath := filepath.Join(dir, "test.tar")

	// Create archive.
	var stdout, stderr bytes.Buffer
	rc := runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-c", "-f", archivePath,
			"-C", dir, "hello.txt", "bye.txt"},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("create: exit code = %d, stderr: %s", rc, stderr.String())
	}

	// List archive.
	stdout.Reset()
	stderr.Reset()
	rc = runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-t", "-f", archivePath},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("list: exit code = %d, stderr: %s", rc, stderr.String())
	}

	output := stdout.String()
	if !strings.Contains(output, "hello.txt") {
		t.Errorf("listing should contain hello.txt, got:\n%s", output)
	}
	if !strings.Contains(output, "bye.txt") {
		t.Errorf("listing should contain bye.txt, got:\n%s", output)
	}
}

// =========================================================================
// Create and extract round-trip test
// =========================================================================

func TestTarCreateAndExtract(t *testing.T) {
	srcDir := t.TempDir()
	dstDir := t.TempDir()

	os.WriteFile(filepath.Join(srcDir, "data.txt"), []byte("some data"), 0644)
	archivePath := filepath.Join(srcDir, "test.tar")

	// Create archive.
	var stdout, stderr bytes.Buffer
	rc := runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-c", "-f", archivePath,
			"-C", srcDir, "data.txt"},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("create: exit code = %d, stderr: %s", rc, stderr.String())
	}

	// Extract archive.
	stdout.Reset()
	stderr.Reset()
	rc = runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-x", "-f", archivePath,
			"-C", dstDir},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("extract: exit code = %d, stderr: %s", rc, stderr.String())
	}

	// Verify extracted file.
	content, err := os.ReadFile(filepath.Join(dstDir, "data.txt"))
	if err != nil {
		t.Fatalf("cannot read extracted file: %v", err)
	}
	if string(content) != "some data" {
		t.Errorf("extracted content = %q, want %q", string(content), "some data")
	}
}

// =========================================================================
// Verbose mode
// =========================================================================

func TestTarCreateVerbose(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "file.txt"), []byte("data"), 0644)
	archivePath := filepath.Join(dir, "test.tar")

	var stdout, stderr bytes.Buffer
	rc := runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-c", "-v", "-f", archivePath,
			"-C", dir, "file.txt"},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("exit code = %d, stderr: %s", rc, stderr.String())
	}

	if !strings.Contains(stdout.String(), "file.txt") {
		t.Errorf("verbose create should list file.txt, got %q", stdout.String())
	}
}

func TestTarListVerbose(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "file.txt"), []byte("data"), 0644)
	archivePath := filepath.Join(dir, "test.tar")

	// Create archive first.
	var stdout, stderr bytes.Buffer
	runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-c", "-f", archivePath,
			"-C", dir, "file.txt"},
		&stdout, &stderr)

	// List with verbose.
	stdout.Reset()
	stderr.Reset()
	rc := runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-t", "-v", "-f", archivePath},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("exit code = %d, stderr: %s", rc, stderr.String())
	}

	output := stdout.String()
	// Verbose listing should include permissions and size.
	if !strings.Contains(output, "file.txt") {
		t.Errorf("verbose listing should contain file.txt, got:\n%s", output)
	}
}

// =========================================================================
// Archive directories
// =========================================================================

func TestTarDirectory(t *testing.T) {
	dir := t.TempDir()
	subdir := filepath.Join(dir, "mydir")
	os.MkdirAll(subdir, 0755)
	os.WriteFile(filepath.Join(subdir, "a.txt"), []byte("aaa"), 0644)
	os.WriteFile(filepath.Join(subdir, "b.txt"), []byte("bbb"), 0644)

	archivePath := filepath.Join(dir, "test.tar")
	extractDir := filepath.Join(dir, "extract")
	os.MkdirAll(extractDir, 0755)

	// Create archive of the directory.
	var stdout, stderr bytes.Buffer
	rc := runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-c", "-f", archivePath,
			"-C", dir, "mydir"},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("create: exit code = %d, stderr: %s", rc, stderr.String())
	}

	// Extract.
	stdout.Reset()
	stderr.Reset()
	rc = runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-x", "-f", archivePath,
			"-C", extractDir},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("extract: exit code = %d, stderr: %s", rc, stderr.String())
	}

	// Verify extracted files.
	content, err := os.ReadFile(filepath.Join(extractDir, "mydir", "a.txt"))
	if err != nil {
		t.Fatalf("cannot read extracted file: %v", err)
	}
	if string(content) != "aaa" {
		t.Errorf("content = %q, want %q", string(content), "aaa")
	}
}

// =========================================================================
// Keep old files (-k)
// =========================================================================

func TestTarKeepOldFiles(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "file.txt"), []byte("original"), 0644)
	archivePath := filepath.Join(dir, "test.tar")

	// Create archive.
	var stdout, stderr bytes.Buffer
	runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-c", "-f", archivePath,
			"-C", dir, "file.txt"},
		&stdout, &stderr)

	// Modify the file on disk.
	os.WriteFile(filepath.Join(dir, "file.txt"), []byte("modified"), 0644)

	// Extract with -k — should NOT overwrite.
	stdout.Reset()
	stderr.Reset()
	rc := runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-x", "-k", "-f", archivePath,
			"-C", dir},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("exit code = %d, stderr: %s", rc, stderr.String())
	}

	// File should still contain "modified".
	content, _ := os.ReadFile(filepath.Join(dir, "file.txt"))
	if string(content) != "modified" {
		t.Errorf("file should not be overwritten with -k, got %q", string(content))
	}
}

// =========================================================================
// Strip components
// =========================================================================

func TestTarStripComponents(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		n        int
		expected string
	}{
		{"strip 1", "dir/file.txt", 1, "file.txt"},
		{"strip 2", "a/b/c.txt", 2, "c.txt"},
		{"strip too many", "dir/file.txt", 3, ""},
		{"strip 0", "dir/file.txt", 0, "dir/file.txt"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := tarStripComponents(tt.input, tt.n)
			if result != tt.expected {
				t.Errorf("tarStripComponents(%q, %d) = %q, want %q",
					tt.input, tt.n, result, tt.expected)
			}
		})
	}
}

// =========================================================================
// Compression stubs
// =========================================================================

func TestTarCompressionNotSupported(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "file.txt"), []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-c", "-z", "-f", filepath.Join(dir, "test.tar.gz"),
			"-C", dir, "file.txt"},
		&stdout, &stderr)

	if rc != 2 {
		t.Errorf("exit code = %d, want 2 for unsupported compression", rc)
	}
	if !strings.Contains(stderr.String(), "not supported") {
		t.Errorf("should report compression not supported, got %q", stderr.String())
	}
}

// =========================================================================
// Error handling
// =========================================================================

func TestTarMissingArchive(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-t", "-f", "/nonexistent/archive.tar"},
		&stdout, &stderr)

	if rc != 2 {
		t.Errorf("exit code = %d, want 2", rc)
	}
}

func TestTarInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runTar("/nonexistent/tar.json", []string{"tar", "-c"}, &stdout, &stderr)
	if rc != 2 {
		t.Errorf("exit code = %d, want 2", rc)
	}
}

func TestTarEmptyCreate(t *testing.T) {
	dir := t.TempDir()

	var stdout, stderr bytes.Buffer
	rc := runTar(toolSpecPath(t, "tar"),
		[]string{"tar", "-c", "-f", filepath.Join(dir, "test.tar")},
		&stdout, &stderr)

	if rc != 2 {
		t.Errorf("exit code = %d, want 2 for empty archive", rc)
	}
}

// =========================================================================
// tarCreate and tarList via Go's archive/tar (direct)
// =========================================================================

func TestTarCreateWritesValidArchive(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "content.txt"), []byte("test content"), 0644)
	archivePath := filepath.Join(dir, "out.tar")

	// Create via our tool.
	var stdout, stderr bytes.Buffer
	rc := tarCreate([]string{"content.txt"},
		TarOptions{File: archivePath, Directory: dir},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("tarCreate failed: rc=%d, stderr=%s", rc, stderr.String())
	}

	// Verify with Go's tar reader.
	f, err := os.Open(archivePath)
	if err != nil {
		t.Fatalf("cannot open archive: %v", err)
	}
	defer f.Close()

	tr := tar.NewReader(f)
	header, err := tr.Next()
	if err != nil {
		t.Fatalf("cannot read tar header: %v", err)
	}

	if header.Name != "content.txt" {
		t.Errorf("header name = %q, want content.txt", header.Name)
	}

	data, _ := io.ReadAll(tr)
	if string(data) != "test content" {
		t.Errorf("file content = %q, want 'test content'", string(data))
	}
}

func TestTarExtractVerbose(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "v.txt"), []byte("verbose"), 0644)
	archivePath := filepath.Join(dir, "test.tar")

	// Create archive.
	var stdout, stderr bytes.Buffer
	tarCreate([]string{"v.txt"},
		TarOptions{File: archivePath, Directory: dir},
		&stdout, &stderr)

	// Extract with verbose.
	extractDir := filepath.Join(dir, "out")
	os.MkdirAll(extractDir, 0755)

	stdout.Reset()
	stderr.Reset()
	rc := tarExtract(nil,
		TarOptions{File: archivePath, Directory: extractDir, Verbose: true},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("extract failed: rc=%d, stderr=%s", rc, stderr.String())
	}

	if !strings.Contains(stdout.String(), "v.txt") {
		t.Errorf("verbose extract should list v.txt, got %q", stdout.String())
	}
}

func TestTarExtractRejectsSymlinkPathEscapeThroughExistingSymlink(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("symlink extraction test requires Unix-style symlink semantics")
	}

	dir := t.TempDir()
	archivePath := filepath.Join(dir, "escape.tar")
	archiveFile, err := os.Create(archivePath)
	if err != nil {
		t.Fatalf("cannot create archive: %v", err)
	}

	tw := tar.NewWriter(archiveFile)
	err = tw.WriteHeader(&tar.Header{
		Name:     "subdir/parent/escape-link",
		Typeflag: tar.TypeSymlink,
		Linkname: "../inside.txt",
		Mode:     0644,
	})
	if err != nil {
		t.Fatalf("cannot write tar header: %v", err)
	}
	if err := tw.Close(); err != nil {
		t.Fatalf("cannot close tar writer: %v", err)
	}
	if err := archiveFile.Close(); err != nil {
		t.Fatalf("cannot close archive file: %v", err)
	}

	extractDir := filepath.Join(dir, "out")
	if err := os.MkdirAll(filepath.Join(extractDir, "subdir"), 0755); err != nil {
		t.Fatalf("cannot create extract dir: %v", err)
	}
	outsideDir := filepath.Join(dir, "outside")
	if err := os.MkdirAll(outsideDir, 0755); err != nil {
		t.Fatalf("cannot create outside dir: %v", err)
	}
	if err := os.Symlink(filepath.Join("..", "..", "outside"), filepath.Join(extractDir, "subdir", "parent")); err != nil {
		t.Fatalf("cannot create escape symlink: %v", err)
	}

	var stdout, stderr bytes.Buffer
	rc := tarExtract(nil,
		TarOptions{File: archivePath, Directory: extractDir},
		&stdout, &stderr)

	if rc != 0 {
		t.Fatalf("extract failed: rc=%d, stderr=%s", rc, stderr.String())
	}
	if !strings.Contains(stderr.String(), "path escapes target directory") {
		t.Fatalf("expected path escape warning, got %q", stderr.String())
	}
	if _, err := os.Lstat(filepath.Join(outsideDir, "escape-link")); !os.IsNotExist(err) {
		t.Fatalf("expected escaped symlink not to be created, got err=%v", err)
	}
}

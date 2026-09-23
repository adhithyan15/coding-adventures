package executor

import (
	"os"
	"path/filepath"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// TestManifestReadRejectsFIFO: a FIFO package.json blocked os.ReadFile
// forever before the Lstat guard. Because key derivation runs inside the
// build goroutine after a semaphore slot is taken, that hung the whole build.
func TestManifestReadRejectsFIFO(t *testing.T) {
	dir := t.TempDir()
	if err := syscall.Mkfifo(filepath.Join(dir, "package.json"), 0o644); err != nil {
		t.Skipf("mkfifo unavailable: %v", err)
	}
	done := make(chan []string, 1)
	go func() { done <- fileDependencyDirs(dir) }()
	select {
	case got := <-done:
		if len(got) != 0 {
			t.Errorf("FIFO manifest should yield nothing, got %v", got)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("fileDependencyDirs blocked on a FIFO package.json")
	}
}

// TestManifestReadRejectsSymlink: package.json symlinked at /dev/zero read
// without EOF and grew unboundedly. Lstat must reject it unfollowed.
func TestManifestReadRejectsSymlink(t *testing.T) {
	dir := t.TempDir()
	if err := os.Symlink("/dev/zero", filepath.Join(dir, "package.json")); err != nil {
		t.Skipf("symlink unavailable: %v", err)
	}
	done := make(chan []string, 1)
	go func() { done <- fileDependencyDirs(dir) }()
	select {
	case got := <-done:
		if len(got) != 0 {
			t.Errorf("symlinked manifest should yield nothing, got %v", got)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("fileDependencyDirs blocked reading a symlinked package.json")
	}
}

// TestWalkStaysInsideKnownPackages: a file: specifier escaping the checkout
// must not be traversed or read.
func TestWalkStaysInsideKnownPackages(t *testing.T) {
	root := t.TempDir()
	pkgDir := filepath.Join(root, "pkg")
	outside := filepath.Join(root, "outside")
	writeTestPackageJSON(t, pkgDir, map[string]string{"@ca/evil": "../../outside"})
	writeTestPackageJSON(t, outside, map[string]string{"@ca/deeper": "../deeper"})

	pkg := discovery.Package{
		Name:          "unknown/pkg",
		Path:          pkgDir,
		BuildCommands: []string{"npm ci"},
	}
	// pathToPkg deliberately does NOT contain `outside`.
	if keys := buildReadResourceKeys(pkg, map[string]string{}); len(keys) != 0 {
		t.Errorf("escaping file: specifier should yield no keys, got %v", keys)
	}
}

//go:build aix || darwin || dragonfly || freebsd || linux || netbsd || openbsd || solaris

package executor

import (
	"path/filepath"
	"syscall"
	"testing"
	"time"
)

// TestManifestReadRejectsFIFO: a FIFO package.json blocked os.ReadFile
// forever before the Lstat guard. Because key derivation runs inside the
// build goroutine after a semaphore slot is taken, that hung the whole build.
// Mkfifo is unavailable in Go's syscall package on Windows, so keep this one
// Unix-specific regression separate from the portable B07 probe tests.
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

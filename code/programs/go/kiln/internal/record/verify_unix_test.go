//go:build unix

package record

import (
	"os"
	"path/filepath"
	"syscall"
	"testing"
	"time"
)

// verifyWithin runs VerifyTree and fails the test if it does not return soon:
// a FIFO that was opened blocking would hang here forever.
func verifyWithin(t *testing.T, dir string) error {
	t.Helper()
	done := make(chan error, 1)
	go func() {
		_, err := VerifyTree(dir, "")
		done <- err
	}()
	select {
	case err := <-done:
		return err
	case <-time.After(20 * time.Second):
		t.Fatal("VerifyTree blocked")
		return nil
	}
}

func TestVerifyTreeFifoInPlaceOfFileDoesNotHang(t *testing.T) {
	dir := newRepo(t, baseFiles)
	remove(t, dir, "README.md")
	if err := syscall.Mkfifo(filepath.Join(dir, "README.md"), 0o644); err != nil {
		t.Skipf("cannot create a FIFO: %v", err)
	}
	wantTreeError(t, verifyWithin(t, dir), "README.md", "a FIFO", "non-regular")
}

func TestVerifyTreeExecBit(t *testing.T) {
	dir := newRepo(t, baseFiles)
	// A file committed 100755 is intact...
	if err := os.Chmod(filepath.Join(dir, "src/a.go"), 0o755); err != nil {
		t.Fatal(err)
	}
	runGit(t, dir, "add", "src/a.go")
	runGit(t, dir, "commit", "-q", "-m", "exec")
	if err := verifyWithin(t, dir); err != nil {
		t.Fatalf("an intact executable: %v", err)
	}

	// ...flipping its bit off is a difference, content unchanged.
	if err := os.Chmod(filepath.Join(dir, "src/a.go"), 0o644); err != nil {
		t.Fatal(err)
	}
	wantTreeError(t, verifyWithin(t, dir), "src/a.go", "committed executable")

	// And the other direction: a plain file that became executable.
	if err := os.Chmod(filepath.Join(dir, "src/a.go"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.Chmod(filepath.Join(dir, "README.md"), 0o755); err != nil {
		t.Fatal(err)
	}
	wantTreeError(t, verifyWithin(t, dir), "README.md", "committed non-executable")
}

func TestVerifyTreeUnreadableFile(t *testing.T) {
	dir := newRepo(t, baseFiles)
	if os.Getuid() != 0 {
		if err := os.Chmod(filepath.Join(dir, "README.md"), 0); err != nil {
			t.Fatal(err)
		}
		wantTreeError(t, verifyWithin(t, dir), "README.md")
	}
}

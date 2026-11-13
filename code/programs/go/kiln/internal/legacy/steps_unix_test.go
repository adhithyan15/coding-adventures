//go:build unix

package legacy

import (
	"os"
	"path/filepath"
	"syscall"
	"testing"
	"time"
)

// A FIFO or a symlink at the steps path is refused, and a FIFO does not hang
// the reader.
func TestReadStepsFileRefusesFifoAndSymlink(t *testing.T) {
	dir := t.TempDir()
	real := filepath.Join(dir, "real.json")
	if err := os.WriteFile(real, []byte(okSteps), 0o644); err != nil {
		t.Fatal(err)
	}
	link := filepath.Join(dir, "link.json")
	if err := os.Symlink(real, link); err != nil {
		t.Skip(err)
	}
	fifo := filepath.Join(dir, "fifo.json")
	if err := syscall.Mkfifo(fifo, 0o644); err != nil {
		t.Skip(err)
	}
	for name, p := range map[string]string{"symlink": link, "fifo": fifo} {
		done := make(chan error, 1)
		go func() {
			_, err := ReadStepsFile(p)
			done <- err
		}()
		select {
		case err := <-done:
			if err == nil {
				t.Errorf("%s: accepted", name)
			}
		case <-time.After(10 * time.Second):
			t.Fatalf("%s: ReadStepsFile blocked", name)
		}
	}
}

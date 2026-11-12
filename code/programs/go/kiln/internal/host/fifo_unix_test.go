//go:build unix

package host

import (
	"os"
	"path/filepath"
	"syscall"
	"testing"
	"time"
)

func TestEventPayloadFIFOAndSymlinkRefused(t *testing.T) {
	dir := t.TempDir()
	fifo := filepath.Join(dir, "event.fifo")
	if err := syscall.Mkfifo(fifo, 0o600); err != nil {
		t.Skipf("cannot create a FIFO: %v", err)
	}
	real := eventFile(t, "main")
	link := filepath.Join(dir, "link.json")
	if err := os.Symlink(real, link); err != nil {
		t.Skipf("cannot create symlinks: %v", err)
	}
	for name, p := range map[string]string{"fifo": fifo, "symlink": link} {
		done := make(chan bool, 1)
		go func() {
			g := NewGitHub(env(map[string]string{"GITHUB_ACTIONS": "true", "GITHUB_EVENT_PATH": p,
				"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push"}), nil)
			done <- g.Trusted()
		}()
		select {
		case trusted := <-done:
			if trusted {
				t.Errorf("%s: trusted", name)
			}
		case <-time.After(5 * time.Second):
			t.Fatalf("%s: Trusted() blocked", name)
		}
	}
}

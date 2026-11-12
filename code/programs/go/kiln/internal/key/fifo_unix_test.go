//go:build unix

package key

import (
	"strings"
	"syscall"
	"testing"
)

func TestReadVectorFileRefusesFIFO(t *testing.T) {
	p := t.TempDir() + "/fifo"
	if err := syscall.Mkfifo(p, 0o600); err != nil {
		t.Skipf("mkfifo unavailable: %v", err)
	}
	// Without O_NONBLOCK this open would block forever waiting for a writer.
	if _, err := ReadVectorFile(p); err == nil || !strings.Contains(err.Error(), "not a regular file") {
		t.Errorf("FIFO accepted: %v", err)
	}
}

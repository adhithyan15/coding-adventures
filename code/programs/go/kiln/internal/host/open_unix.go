//go:build unix

package host

import (
	"os"
	"syscall"
)

// openEvent opens path read-only without blocking and without following a
// symlink in the final component: O_NOFOLLOW refuses a symlink, and
// O_NONBLOCK makes opening a FIFO return at once instead of waiting for a
// writer.  The caller's fstat regular-file check then refuses a FIFO.
func openEvent(path string) (*os.File, error) {
	return os.OpenFile(path, os.O_RDONLY|syscall.O_NONBLOCK|syscall.O_NOFOLLOW, 0)
}

//go:build unix

package ledger

import (
	"os"
	"syscall"
)

// openRead opens path read-only without blocking and without following a
// final symlink.  O_NONBLOCK stops a FIFO from hanging the open until a writer
// appears (the regular-file check then refuses it); O_NOFOLLOW makes a
// symlink swapped in after the directory listing fail instead of being read.
func openRead(path string) (*os.File, error) {
	return os.OpenFile(path, os.O_RDONLY|syscall.O_NONBLOCK|syscall.O_NOFOLLOW, 0)
}

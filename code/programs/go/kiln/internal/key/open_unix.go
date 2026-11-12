//go:build unix

package key

import (
	"os"
	"syscall"
)

// openNoBlock opens path read-only with O_NONBLOCK.  Opening a FIFO for
// reading normally blocks until a writer appears, which would hang the
// loader before it could look at the file type; with O_NONBLOCK the open
// returns at once and the caller's regular-file check then refuses it.
// O_NONBLOCK has no effect on reads from regular files.
func openNoBlock(path string) (*os.File, error) {
	return os.OpenFile(path, os.O_RDONLY|syscall.O_NONBLOCK, 0)
}

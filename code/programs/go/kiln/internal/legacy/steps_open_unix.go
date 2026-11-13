//go:build unix

package legacy

import (
	"os"
	"syscall"
)

// openNoFollow opens path read-only so that nothing can hang or redirect the
// open: O_NOFOLLOW refuses a symlink in the final component, O_NONBLOCK makes
// opening a FIFO return at once (the caller's fstat then refuses it), and
// O_NOCTTY keeps a terminal from becoming the controlling one.
func openNoFollow(path string) (*os.File, error) {
	return os.OpenFile(path, os.O_RDONLY|syscall.O_NOFOLLOW|syscall.O_NONBLOCK|syscall.O_NOCTTY, 0)
}

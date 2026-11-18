//go:build unix

package main

import (
	"os"
	"syscall"
)

// openTraceNoFollow opens a trace for the diagnostic with one open(2): a symlink
// in the last component is refused (O_NOFOLLOW), a FIFO does not make the open
// wait for a writer (O_NONBLOCK), and a terminal cannot become the controlling
// terminal (O_NOCTTY).  The caller checks the handle with Stat.  (The same flags
// as openNoFollow in internal/record.)
func openTraceNoFollow(path string) (*os.File, error) {
	return os.OpenFile(path, os.O_RDONLY|syscall.O_NOFOLLOW|syscall.O_NONBLOCK|syscall.O_NOCTTY, 0)
}

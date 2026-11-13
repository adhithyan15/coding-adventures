//go:build unix

package record

import (
	"fmt"
	"os"
	"syscall"
)

// openNoFollow opens path read-only such that nothing can make the open hang
// or escape:
//
//	O_NOFOLLOW  a symlink in the final component is refused, not followed
//	O_NONBLOCK  opening a FIFO returns at once instead of waiting for a writer
//	O_NOCTTY    a terminal device cannot become our controlling terminal
//
// The caller's fstat on the handle then refuses anything that is not a
// regular file.
func openNoFollow(path string) (*os.File, error) {
	return os.OpenFile(path, os.O_RDONLY|syscall.O_NOFOLLOW|syscall.O_NONBLOCK|syscall.O_NOCTTY, 0)
}

// linkTargetText is the symlink target as Git stores it. On POSIX that is
// exactly what readlink returns, byte for byte (a backslash is an ordinary
// character there and must keep its meaning).
func linkTargetText(target string) string { return target }

// execMismatch compares the executable bit with what the tree records. Git
// itself only looks at the owner's execute bit, so that is what counts.
func execMismatch(m os.FileMode, gitMode string) string {
	isExec := m&0o100 != 0
	switch {
	case gitMode == modeExec && !isExec:
		return fmt.Sprintf("committed executable (%s) but the file is not", gitMode)
	case gitMode == modeFile && isExec:
		return fmt.Sprintf("committed non-executable (%s) but the file is executable", gitMode)
	}
	return ""
}

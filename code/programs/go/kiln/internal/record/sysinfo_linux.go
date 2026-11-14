//go:build linux

package record

import (
	"os"
	"strconv"
	"syscall"
	"time"
)

// fdPath is the kernel's name for an open file: the target of
// /proc/self/fd/N, which is the RESOLVED path (and gains a " (deleted)" suffix
// for an unlinked file, so that too is a mismatch with the path asked for).
// SyscallConn, not Fd(): Fd() would switch the descriptor to blocking mode.
func fdPath(f *os.File) (string, error) {
	rc, err := f.SyscallConn()
	if err != nil {
		return "", err
	}
	var n int
	if err := rc.Control(func(fd uintptr) { n = int(fd) }); err != nil {
		return "", err
	}
	return os.Readlink("/proc/self/fd/" + strconv.Itoa(n))
}

// ctimeOf is the inode change time.
func ctimeOf(fi os.FileInfo) time.Time {
	if st, ok := fi.Sys().(*syscall.Stat_t); ok {
		return time.Unix(int64(st.Ctim.Sec), int64(st.Ctim.Nsec))
	}
	return time.Time{}
}

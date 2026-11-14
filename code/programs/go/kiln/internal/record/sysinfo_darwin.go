//go:build darwin

package record

import (
	"fmt"
	"os"
	"syscall"
	"time"
	"unsafe"
)

// fcntlGetPath is F_GETPATH: the kernel's resolved path of an open descriptor,
// macOS's counterpart of /proc/self/fd (for the developers' machines; the
// recorder itself runs on Linux).
const fcntlGetPath = 50

func fdPath(f *os.File) (string, error) {
	rc, err := f.SyscallConn()
	if err != nil {
		return "", err
	}
	var buf [1024]byte
	var errno syscall.Errno
	if err := rc.Control(func(fd uintptr) {
		_, _, errno = syscall.Syscall(syscall.SYS_FCNTL, fd, fcntlGetPath, uintptr(unsafe.Pointer(&buf[0])))
	}); err != nil {
		return "", err
	}
	if errno != 0 {
		return "", fmt.Errorf("fcntl(F_GETPATH): %v", errno)
	}
	n := 0
	for n < len(buf) && buf[n] != 0 {
		n++
	}
	return string(buf[:n]), nil
}

func ctimeOf(fi os.FileInfo) time.Time {
	if st, ok := fi.Sys().(*syscall.Stat_t); ok {
		return time.Unix(int64(st.Ctimespec.Sec), int64(st.Ctimespec.Nsec))
	}
	return time.Time{}
}

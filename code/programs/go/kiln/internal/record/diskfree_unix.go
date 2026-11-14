//go:build linux || darwin

package record

import "syscall"

// freeBytes is the space a non-privileged writer can still use on the file
// system holding path (statfs's f_bavail times f_bsize).
func freeBytes(path string) (uint64, error) {
	var st syscall.Statfs_t
	if err := syscall.Statfs(path, &st); err != nil {
		return 0, err
	}
	return uint64(st.Bavail) * uint64(st.Bsize), nil
}

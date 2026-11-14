//go:build unix

package record

import (
	"os"
	"syscall"
)

// linkCount is st_nlink of a file, and false where the platform does not say.
// The conversion is needed because the field's width differs by OS (uint16 on
// macOS, uint64 on Linux).
func linkCount(fi os.FileInfo) (uint64, bool) {
	st, ok := fi.Sys().(*syscall.Stat_t)
	if !ok {
		return 0, false
	}
	return uint64(st.Nlink), true
}

// deviceOf is st_dev of a file: two paths with different devices are on
// different mounts. The field is int32 on macOS and uint64 on Linux.
func deviceOf(fi os.FileInfo) (uint64, bool) {
	st, ok := fi.Sys().(*syscall.Stat_t)
	if !ok {
		return 0, false
	}
	return uint64(st.Dev), true
}

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

// searchable reports whether this process may traverse (search) dir. A
// directory with --x and no r cannot be listed but its entries can be reached by
// name, so "cannot list it" does not mean "a step cannot reach what is in it".
func searchable(dir string) bool { return syscall.Access(dir, 1) == nil } // X_OK

// ownerOf is st_uid of a file.
func ownerOf(fi os.FileInfo) (uint32, bool) {
	st, ok := fi.Sys().(*syscall.Stat_t)
	if !ok {
		return 0, false
	}
	return st.Uid, true
}

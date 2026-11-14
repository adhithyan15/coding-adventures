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
//
// It is judged by the EFFECTIVE identity (access(2) uses the real one): the
// owner, group or other search bit of the directory's mode, by whichever class
// the effective uid, gid and supplementary groups put us in. Anything we cannot
// tell (no stat, no ids) counts as searchable, so the scan fails closed; an ACL
// that grants search is not visible in the mode bits, so a "no" from the mode is
// confirmed by access(2) before it is believed.
func searchable(dir string) bool {
	st, err := os.Stat(dir)
	if err != nil {
		return true
	}
	sys, ok := st.Sys().(*syscall.Stat_t)
	if !ok {
		return true
	}
	groups, err := os.Getgroups()
	if err != nil {
		return true
	}
	if searchByMode(st.Mode().Perm(), sys.Uid, sys.Gid, uint32(os.Geteuid()), uint32(os.Getegid()), groups) {
		return true
	}
	return syscall.Access(dir, 1) == nil // X_OK, real ids: a second opinion
}

// searchByMode is the mode-bit decision of searchable. The superuser may search
// any directory.
func searchByMode(perm os.FileMode, uid, gid, euid, egid uint32, groups []int) bool {
	switch {
	case euid == 0:
		return true
	case uid == euid:
		return perm&0o100 != 0
	case gid == egid || inGroups(gid, groups):
		return perm&0o010 != 0
	}
	return perm&0o001 != 0
}

func inGroups(gid uint32, groups []int) bool {
	for _, g := range groups {
		if uint32(g) == gid {
			return true
		}
	}
	return false
}

// ownerOf is st_uid of a file.
func ownerOf(fi os.FileInfo) (uint32, bool) {
	st, ok := fi.Sys().(*syscall.Stat_t)
	if !ok {
		return 0, false
	}
	return st.Uid, true
}

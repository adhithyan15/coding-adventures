// =========================================================================
// chown — Unix-specific ownership helpers
// =========================================================================
//
// On Unix systems, every file has a numeric UID (user ID) and GID (group ID)
// stored in the inode. We access these via syscall.Stat_t, which is the Go
// binding for the C struct stat.
//
// These functions are used by chown_tool.go for:
//   - Reading current ownership to detect changes (-c flag)
//   - Copying ownership from a reference file (--reference flag)
//
// On Windows, these functions are stubbed out (see chown_windows.go)
// because Windows uses a completely different security model (ACLs/SIDs).

//go:build !windows

package main

import (
	"os"
	"syscall"
)

// getFileOwnership extracts the numeric UID and GID from a file's metadata.
//
// On Unix, os.FileInfo.Sys() returns a *syscall.Stat_t, which contains
// the Uid and Gid fields. If the type assertion fails (shouldn't happen
// on Unix, but defensive coding), we return -1 for both.
func getFileOwnership(info os.FileInfo) (uid, gid int) {
	stat, ok := info.Sys().(*syscall.Stat_t)
	if !ok {
		return -1, -1
	}
	return int(stat.Uid), int(stat.Gid)
}

// getRefFileOwnership extracts a ChownSpec from a reference file's metadata.
//
// Used by the --reference flag: instead of specifying OWNER:GROUP on the
// command line, you say "make this file's ownership match that file's."
// Returns false if the platform info is unavailable.
func getRefFileOwnership(info os.FileInfo) (ChownSpec, bool) {
	stat, ok := info.Sys().(*syscall.Stat_t)
	if !ok {
		return ChownSpec{}, false
	}
	return ChownSpec{UID: int(stat.Uid), GID: int(stat.Gid)}, true
}

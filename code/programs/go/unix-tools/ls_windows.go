// =========================================================================
// ls — Windows stubs for file metadata helpers
// =========================================================================
//
// Windows does not have Unix-style inodes, hard link counts exposed via
// stat, or numeric UID/GID ownership. These stubs provide sensible
// defaults so that `ls` can still produce output on Windows, just
// without the Unix-specific metadata.
//
// On Windows:
//   - Inode numbers don't exist, so we return 0
//   - Hard link count defaults to 1
//   - Owner/group show as "?" (a full implementation could use
//     Windows security APIs to look up the file's owner SID)

//go:build windows

package main

import "os"

// getInode returns 0 on Windows — there are no Unix-style inodes.
func getInode(info os.FileInfo) uint64 {
	return 0
}

// getNlink returns 1 on Windows — hard link counts are not exposed
// through Go's os.FileInfo on this platform.
func getNlink(info os.FileInfo) uint64 {
	return 1
}

// getOwnerGroup returns "?" for both owner and group on Windows,
// since Windows uses SIDs rather than Unix-style UID/GID.
func getOwnerGroup(info os.FileInfo, numericUID bool) (owner, group string) {
	return "?", "?"
}

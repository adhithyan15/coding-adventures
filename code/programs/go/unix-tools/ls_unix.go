// =========================================================================
// ls — Unix-specific file metadata helpers
// =========================================================================
//
// Unix filesystems store rich metadata in each file's inode:
//
//   Field   Meaning
//   ──────  ──────────────────────────────────────
//   Ino     Inode number (unique ID within filesystem)
//   Nlink   Number of hard links pointing to this inode
//   Uid     Numeric user ID of the file's owner
//   Gid     Numeric group ID of the file's group
//
// We access these via syscall.Stat_t, which Go exposes through
// os.FileInfo.Sys(). These helpers keep the platform-specific code
// isolated so that ls_tool.go remains portable.

//go:build !windows

package main

import (
	"os"
	"os/user"
	"strconv"
	"syscall"
)

// getInode returns the inode number for a file. On Unix, every file has
// a unique inode number within its filesystem. The `ls -i` flag displays
// this value, which is useful for identifying hard links (files sharing
// the same inode).
func getInode(info os.FileInfo) uint64 {
	if sys, ok := info.Sys().(*syscall.Stat_t); ok {
		return sys.Ino
	}
	return 0
}

// getNlink returns the number of hard links to a file. A regular file
// starts with nlink=1. Each additional hard link increments this count.
// Directories have nlink=2+N where N is the number of subdirectories
// (because each subdirectory's ".." entry is a hard link to the parent).
func getNlink(info os.FileInfo) uint64 {
	if sys, ok := info.Sys().(*syscall.Stat_t); ok {
		return uint64(sys.Nlink)
	}
	return 1
}

// getOwnerGroup returns the owner and group names for a file.
//
// If numericUID is true, returns the raw numeric UID/GID as strings.
// Otherwise, looks up the username and group name from the system's
// user database (/etc/passwd and /etc/group on traditional Unix).
//
// Falls back to "?" if the lookup fails (e.g., deleted user accounts).
func getOwnerGroup(info os.FileInfo, numericUID bool) (owner, group string) {
	owner = "?"
	group = "?"
	sys, ok := info.Sys().(*syscall.Stat_t)
	if !ok {
		return
	}
	if numericUID {
		owner = strconv.FormatUint(uint64(sys.Uid), 10)
		group = strconv.FormatUint(uint64(sys.Gid), 10)
	} else {
		if u, err := user.LookupId(strconv.FormatUint(uint64(sys.Uid), 10)); err == nil {
			owner = u.Username
		}
		if g, err := user.LookupGroupId(strconv.FormatUint(uint64(sys.Gid), 10)); err == nil {
			group = g.Name
		}
	}
	return
}

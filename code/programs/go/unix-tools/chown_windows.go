// =========================================================================
// chown — Windows stubs for ownership helpers
// =========================================================================
//
// Windows does not use Unix-style numeric UIDs and GIDs. Instead, it uses
// Security Identifiers (SIDs) and Access Control Lists (ACLs). The chown
// command is fundamentally a Unix concept that has no direct equivalent
// on Windows.
//
// These stubs allow the code to compile on Windows. The actual chown
// operation will fail at the os.Chown/os.Lchown level (which return
// "not supported" on Windows), so these stubs just need to provide
// reasonable defaults.

//go:build windows

package main

import "os"

// getFileOwnership always returns -1, -1 on Windows because the platform
// does not expose Unix-style UID/GID ownership.
func getFileOwnership(info os.FileInfo) (uid, gid int) {
	return -1, -1
}

// getRefFileOwnership always returns false on Windows because Unix-style
// ownership information is not available.
func getRefFileOwnership(info os.FileInfo) (ChownSpec, bool) {
	return ChownSpec{}, false
}

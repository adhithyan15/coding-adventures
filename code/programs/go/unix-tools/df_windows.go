// =========================================================================
// df — Windows stub for filesystem statistics
// =========================================================================
//
// On Windows, filesystem statistics are obtained through the
// GetDiskFreeSpaceEx Win32 API, not through the Unix statfs(2) syscall.
//
// This stub returns an error indicating the tool is not supported on
// Windows. A full implementation would use golang.org/x/sys/windows
// to call GetDiskFreeSpaceEx.

//go:build windows

package main

import "fmt"

// getFilesystemInfo is not supported on Windows. Returns an error
// directing the user to use platform-native tools instead.
func getFilesystemInfo(path string) (*FsInfo, error) {
	return nil, fmt.Errorf("df is not supported on Windows")
}

// =========================================================================
// df — Unix-specific filesystem statistics
// =========================================================================
//
// On Unix systems, the statfs(2) system call returns filesystem statistics
// including total blocks, free blocks, and block size. This is how df
// computes disk usage.
//
// The key fields from syscall.Statfs_t:
//
//   Field    Meaning
//   ──────   ──────────────────────────────────────
//   Bsize    Fundamental filesystem block size
//   Blocks   Total number of blocks on filesystem
//   Bfree    Free blocks (includes reserved-for-root blocks)
//   Bavail   Free blocks available to non-root users
//
// From these we compute:
//   total = Blocks * Bsize
//   free  = Bfree  * Bsize
//   used  = total - free
//   avail = Bavail * Bsize

//go:build !windows

package main

import (
	"fmt"
	"syscall"
)

// getFilesystemInfo queries the kernel for filesystem statistics at the
// given path using the statfs(2) system call.
func getFilesystemInfo(path string) (*FsInfo, error) {
	var stat syscall.Statfs_t
	if err := syscall.Statfs(path, &stat); err != nil {
		return nil, fmt.Errorf("cannot stat filesystem at %s: %w", path, err)
	}

	// Calculate sizes in bytes.
	blockSize := uint64(stat.Bsize)
	totalBytes := stat.Blocks * blockSize
	availBytes := stat.Bavail * blockSize
	freeBytes := stat.Bfree * blockSize
	usedBytes := totalBytes - freeBytes

	// Calculate usage percentage. Guard against divide-by-zero.
	usePercent := 0
	if totalBytes > 0 {
		// The "used" for percentage calculation is: total - free-for-root.
		// Denominator is: used + available-to-user.
		denominator := usedBytes + availBytes
		if denominator > 0 {
			usePercent = int((usedBytes * 100) / denominator)
		}
	}

	return &FsInfo{
		Filesystem: "unknown", // Real implementation would read /proc/mounts
		MountPoint: path,
		TotalBytes: totalBytes,
		UsedBytes:  usedBytes,
		AvailBytes: availBytes,
		UsePercent: usePercent,
	}, nil
}

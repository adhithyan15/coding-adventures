// =========================================================================
// df — Report File System Disk Space Usage
// =========================================================================
//
// The `df` utility reports how much disk space is used and available on
// mounted filesystems. It's essential for monitoring disk capacity.
//
// # Default output
//
//   $ df
//   Filesystem     1K-blocks      Used Available Use% Mounted on
//   /dev/disk1s1   488245288 234567890 253677398  49% /
//
// # Human-readable mode (-h)
//
//   $ df -h
//   Filesystem      Size  Used Avail Use% Mounted on
//   /dev/disk1s1    466G  224G  242G  49% /
//
// # How does df get its data?
//
// On Unix systems, df uses the statfs(2) system call, which returns:
//   - Total blocks on the filesystem
//   - Free blocks
//   - Block size
//
// We can compute:
//   total = blocks * block_size
//   free  = free_blocks * block_size
//   used  = total - free
//
// # Architecture
//
//   df.json (spec)               df_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -h,-H,-i  │       │ call statfs on paths             │
//   │ -B,-l,-P,-T,-t,-x│──────>│ compute used/avail/percent       │
//   │ arg: FILES...     │       │ format table output              │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"syscall"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// FsInfo — filesystem information for one mount point
// =========================================================================

type FsInfo struct {
	Filesystem string // device name or "unknown"
	MountPoint string // where it's mounted
	TotalBytes uint64 // total capacity in bytes
	UsedBytes  uint64 // bytes in use
	AvailBytes uint64 // bytes available to non-root users
	UsePercent int    // usage percentage (0-100)
}

// =========================================================================
// getFilesystemInfo — get filesystem stats for a given path
// =========================================================================
//
// Uses syscall.Statfs to query the kernel for filesystem statistics.
// The Statfs struct contains:
//   - Bsize:  fundamental filesystem block size
//   - Blocks: total number of blocks
//   - Bfree:  free blocks (total, including reserved for root)
//   - Bavail: free blocks available to non-root users

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

// =========================================================================
// formatSize — format a byte count for display
// =========================================================================
//
// Three modes:
//   - Default: show in 1K-blocks (bytes / 1024)
//   - Human-readable (-h): use powers of 1024 (K, M, G, T)
//   - SI (-H): use powers of 1000 (kB, MB, GB, TB)

func formatSize(bytes uint64, humanReadable, si bool) string {
	if humanReadable {
		return formatHumanReadable(bytes, 1024)
	}
	if si {
		return formatHumanReadable(bytes, 1000)
	}
	// Default: 1K-blocks.
	return fmt.Sprintf("%d", bytes/1024)
}

// =========================================================================
// formatHumanReadable — format bytes with unit suffixes
// =========================================================================
//
// Uses the given base (1024 for -h, 1000 for -H).
//
//   1024-based:  K, M, G, T, P
//   1000-based:  kB, MB, GB, TB, PB

func formatHumanReadable(bytes uint64, base uint64) string {
	units := []string{"B", "K", "M", "G", "T", "P"}
	if base == 1000 {
		units = []string{"B", "kB", "MB", "GB", "TB", "PB"}
	}

	size := float64(bytes)
	unitIdx := 0

	for size >= float64(base) && unitIdx < len(units)-1 {
		size /= float64(base)
		unitIdx++
	}

	if unitIdx == 0 {
		return fmt.Sprintf("%d%s", bytes, units[0])
	}
	if size >= 10 {
		return fmt.Sprintf("%.0f%s", size, units[unitIdx])
	}
	return fmt.Sprintf("%.1f%s", size, units[unitIdx])
}

// =========================================================================
// runDf — the testable core of the df tool
// =========================================================================

func runDf(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "df: %s\n", err)
		return 1
	}

	// Step 2: Parse the arguments.
	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 1
	}

	// Step 3: Handle the result.
	switch r := result.(type) {

	case *clibuilder.HelpResult:
		fmt.Fprintln(stdout, r.Text)
		return 0

	case *clibuilder.VersionResult:
		fmt.Fprintln(stdout, r.Version)
		return 0

	case *clibuilder.ParseResult:
		humanReadable := getBool(r.Flags, "human_readable")
		si := getBool(r.Flags, "si")

		// Get paths to report on.
		paths := getStringSlice(r.Arguments, "files")
		if len(paths) == 0 {
			paths = []string{"/"}
		}

		// Print header.
		if humanReadable || si {
			fmt.Fprintf(stdout, "%-20s %10s %10s %10s %5s %s\n",
				"Filesystem", "Size", "Used", "Avail", "Use%", "Mounted on")
		} else {
			fmt.Fprintf(stdout, "%-20s %12s %12s %12s %5s %s\n",
				"Filesystem", "1K-blocks", "Used", "Available", "Use%", "Mounted on")
		}

		exitCode := 0
		for _, path := range paths {
			info, err := getFilesystemInfo(path)
			if err != nil {
				fmt.Fprintf(stderr, "df: %s\n", err)
				exitCode = 1
				continue
			}

			totalStr := formatSize(info.TotalBytes, humanReadable, si)
			usedStr := formatSize(info.UsedBytes, humanReadable, si)
			availStr := formatSize(info.AvailBytes, humanReadable, si)

			if humanReadable || si {
				fmt.Fprintf(stdout, "%-20s %10s %10s %10s %4d%% %s\n",
					info.Filesystem, totalStr, usedStr, availStr,
					info.UsePercent, info.MountPoint)
			} else {
				fmt.Fprintf(stdout, "%-20s %12s %12s %12s %4d%% %s\n",
					info.Filesystem, totalStr, usedStr, availStr,
					info.UsePercent, info.MountPoint)
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "df: unexpected result type: %T\n", result)
		return 1
	}
}

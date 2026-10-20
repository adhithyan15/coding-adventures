//go:build windows

package irtojvmclassfile

import (
	"os"
	"syscall"
)

// ── Windows: path checks ───────────────────────────────────────────────────
//
// Windows has no openat(2), and golang.org/x/sys/unix does not build here at
// all, so the descriptor walk in backend_unix.go is not an option.  The
// writeStrategyAuto walk on Windows is the path-check walk from
// backend_pathwalk.go.  Its one known gap — a directory swapped for a junction
// between its check and the write — is described there.

// writeClassFilePlatform is the writeStrategyAuto walk on Windows.
func writeClassFilePlatform(artifact *JVMClassArtifact, root string, relativePath string) (string, error) {
	return writeClassFileViaPathChecks(artifact, root, relativePath)
}

// fileAttributeReparsePoint is FILE_ATTRIBUTE_REPARSE_POINT from winnt.h.
// NTFS sets it on every entry whose open is redirected by a filter: symbolic
// links, junctions (mount points), and a few rarer kinds.
const fileAttributeReparsePoint = 0x0400

// isReparsePoint reports whether an Lstat result is a Windows reparse point.
//
// Why not just os.ModeSymlink?  Since Go 1.23 a junction is reported as
// os.ModeIrregular rather than os.ModeSymlink (and the winsymlink GODEBUG can
// flip that back).  The attribute bit is the one answer that does not depend
// on how the Go runtime chooses to classify the entry.
func isReparsePoint(info os.FileInfo) bool {
	data, ok := info.Sys().(*syscall.Win32FileAttributeData)
	if !ok || data == nil {
		return false
	}
	return data.FileAttributes&fileAttributeReparsePoint != 0
}

//go:build unix

package irtojvmclassfile

import (
	"os"
	"strings"
	"testing"

	"golang.org/x/sys/unix"
)

// ── Direct tests of the Unix temp-and-rename step ──────────────────────────
//
// WriteClassFile only reaches these error paths when the filesystem itself
// misbehaves, so the tests open a directory descriptor and drive
// replaceFileAtomicallyAt by hand.

func openTestDirectory(t *testing.T, dir string) int {
	t.Helper()
	fd, err := unix.Open(dir, unix.O_RDONLY|unix.O_DIRECTORY|unix.O_CLOEXEC, 0)
	if err != nil {
		t.Fatalf("open dir failed: %v", err)
	}
	t.Cleanup(func() { _ = unix.Close(fd) })
	return fd
}

func TestReplaceFileAtomicallyAtReportsUninspectableTarget(t *testing.T) {
	dirFD := openTestDirectory(t, t.TempDir())
	// A name longer than NAME_MAX makes fstatat fail with ENAMETOOLONG
	// rather than ENOENT, which must be reported, not treated as "missing".
	err := replaceFileAtomicallyAt(dirFD, strings.Repeat("x", 300)+".class", []byte("class-bytes"))
	requireErrorContaining(t, err, "failed to inspect output file")
}

func TestReplaceFileAtomicallyAtReportsUnwritableDirectory(t *testing.T) {
	if os.Geteuid() == 0 {
		t.Skip("root ignores directory permission bits")
	}
	dir := t.TempDir()
	dirFD := openTestDirectory(t, dir)
	if err := os.Chmod(dir, 0o555); err != nil {
		t.Fatalf("chmod failed: %v", err)
	}
	t.Cleanup(func() { _ = os.Chmod(dir, 0o755) })
	err := replaceFileAtomicallyAt(dirFD, "Example.class", []byte("class-bytes"))
	requireErrorContaining(t, err, "failed to create temporary class file")
	requireNoTempFiles(t, dir)
}

//go:build linux && amd64

package sandbox

import (
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
)

// TestInspectOutputFD runs the real fstat/fcntl decision on real descriptors.
func TestInspectOutputFD(t *testing.T) {
	pr, pw, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	defer pr.Close()
	defer pw.Close()
	if err := inspectOutputFD("--stdout-fd", int(pw.Fd())); err != nil {
		t.Errorf("a pipe's write end: %v", err)
	}
	file, err := os.Create(filepath.Join(t.TempDir(), "f"))
	if err != nil {
		t.Fatal(err)
	}
	defer file.Close()
	dir, err := os.Open(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	defer dir.Close()
	null, err := os.OpenFile(os.DevNull, os.O_WRONLY, 0)
	if err != nil {
		t.Fatal(err)
	}
	defer null.Close()
	closed, err := os.CreateTemp(t.TempDir(), "c")
	if err != nil {
		t.Fatal(err)
	}
	closedFD := int(closed.Fd())
	closed.Close()
	for name, tc := range map[string]struct {
		fd   int
		want string
	}{
		"read end":     {int(pr.Fd()), "not open for writing"},
		"regular file": {int(file.Fd()), "a regular file"},
		"directory":    {int(dir.Fd()), "a directory"},
		"/dev/null":    {int(null.Fd()), "character device"},
		"closed":       {closedFD, "not an open descriptor"},
		"never opened": {1 << 19, "not an open descriptor"},
	} {
		err := inspectOutputFD("--stderr-fd", tc.fd)
		if err == nil || !strings.Contains(err.Error(), tc.want) {
			t.Errorf("%s: %v, want an error containing %q", name, err, tc.want)
		}
	}
	// A pipe opened non-blocking and a socketpair end are told apart by type.
	sv, err := syscall.Socketpair(syscall.AF_UNIX, syscall.SOCK_STREAM, 0)
	if err != nil {
		t.Fatal(err)
	}
	defer syscall.Close(sv[0])
	defer syscall.Close(sv[1])
	if err := inspectOutputFD("--stdout-fd", sv[0]); err == nil || !strings.Contains(err.Error(), "a socket") {
		t.Errorf("a socket: %v", err)
	}
}

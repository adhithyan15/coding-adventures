//go:build !windows

package executor

import (
	"syscall"
	"testing"
)

func createFIFOForTest(_ *testing.T, path string) error {
	return syscall.Mkfifo(path, 0o644)
}

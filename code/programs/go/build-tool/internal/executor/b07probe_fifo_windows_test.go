//go:build windows

package executor

import "testing"

func createFIFOForTest(t *testing.T, _ string) error {
	t.Helper()
	t.Skip("Windows does not provide POSIX FIFOs")
	return nil
}

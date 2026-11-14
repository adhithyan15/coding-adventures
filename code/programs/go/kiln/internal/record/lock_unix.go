//go:build unix

package record

import (
	"fmt"
	"os"
	"syscall"
)

// LockDir takes an flock(2) on the record directory without waiting: shared
// for `kiln record step` (any number of steps may run together), exclusive for
// `collect` (KILN04 4.8, P12; the collect side is stage 4 PR 2b-2b). The two
// exclude each other, so `collect` cannot start while any step is still
// running, an orphaned supervisor included: the lock belongs to the open file
// description, which only that supervisor holds (Go opens it close-on-exec, so
// strace and the step never inherit it), and it goes with the process.
//
// The returned function releases the lock.
func LockDir(dir string, exclusive bool) (func(), error) {
	f, err := os.Open(dir)
	if err != nil {
		return nil, fmt.Errorf("the record directory: %v", err)
	}
	how, what := syscall.LOCK_SH, "shared"
	if exclusive {
		how, what = syscall.LOCK_EX, "exclusive"
	}
	if err := syscall.Flock(int(f.Fd()), how|syscall.LOCK_NB); err != nil {
		f.Close()
		return nil, fmt.Errorf("the record directory cannot be locked %s (a step or `collect` holds it): %v", what, err)
	}
	return func() { f.Close() }, nil // closing the descriptor releases the lock
}

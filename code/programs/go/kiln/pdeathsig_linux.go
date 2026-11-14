//go:build linux

package main

import (
	"errors"
	"os"
	"syscall"
)

// prSetPdeathsig is PR_SET_PDEATHSIG (prctl(2)).
const prSetPdeathsig = 1

// armParentDeath asks the kernel to kill this process when its parent (the
// build tool) dies, so that a supervisor orphaned by a killed build cannot go
// on writing into the record directory (KILN04 4.8, P12). It is checked against
// a race: if the parent is already gone when the request is made, the signal
// will never come, so that is an error here. The parent is the build tool's
// executor, which stays alive for as long as it waits for this process; a
// legitimate run is never affected.
func armParentDeath() error {
	ppid := os.Getppid()
	if _, _, e := syscall.RawSyscall(syscall.SYS_PRCTL, prSetPdeathsig, uintptr(syscall.SIGKILL), 0); e != 0 {
		return e
	}
	if os.Getppid() != ppid {
		return errors.New("the build tool exited while the supervisor was starting")
	}
	return nil
}

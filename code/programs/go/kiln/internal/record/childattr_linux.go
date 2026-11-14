//go:build linux

package record

import (
	"syscall"
)

// setDataLimit sets RLIMIT_DATA, the hard bound on the child's writable memory
// (classify_child.go explains why not RLIMIT_AS).
func setDataLimit(n uint64) error {
	return syscall.Setrlimit(syscall.RLIMIT_DATA, &syscall.Rlimit{Cur: n, Max: n})
}

// childAttr makes strace die with the supervisor: the kernel sends SIGKILL to
// the child when the thread that forked it exits, which for a killed process is
// every thread (see the note on Pdeathsig in step.go).
func childAttr() *syscall.SysProcAttr {
	return &syscall.SysProcAttr{Pdeathsig: syscall.SIGKILL}
}

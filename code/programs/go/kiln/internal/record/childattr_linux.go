//go:build linux

package record

import "syscall"

// childAttr makes strace die with the supervisor: the kernel sends SIGKILL to
// the child when the thread that forked it exits, which for a killed process is
// every thread (see the note on Pdeathsig in step.go).
func childAttr() *syscall.SysProcAttr {
	return &syscall.SysProcAttr{Pdeathsig: syscall.SIGKILL}
}

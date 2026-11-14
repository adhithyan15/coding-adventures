//go:build linux

package record

import (
	"os"
	"syscall"
)

// adjustOOM makes the kernel's OOM killer prefer this process (best effort).
func adjustOOM() { os.WriteFile("/proc/self/oom_score_adj", []byte("1000"), 0) }

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

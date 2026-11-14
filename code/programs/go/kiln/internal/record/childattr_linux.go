//go:build linux

package record

import (
	"os"
	"os/exec"
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

// isolateProbe makes a probe's sandbox root lead its own process group, die
// with the thread that started it, and take its whole group down when the
// deadline kills it.
func isolateProbe(cmd *exec.Cmd) {
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true, Pdeathsig: syscall.SIGKILL}
	cmd.Cancel = func() error {
		if cmd.Process == nil {
			return nil
		}
		return syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
	}
}

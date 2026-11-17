//go:build !linux

package record

import (
	"os/exec"
	"syscall"
)

// childAttr: there is no Pdeathsig off Linux, and the recorder does not run
// there (kiln record step refuses before it gets this far).
func childAttr() *syscall.SysProcAttr { return nil }

// setDataLimit: no RLIMIT_DATA that means the same thing off Linux; the soft
// GOMEMLIMIT still applies, and the recorder does not run here.
func setDataLimit(uint64) error { return nil }

// adjustOOM: no oom_score_adj off Linux.
func adjustOOM() {}

// isolateProbe: no process groups or Pdeathsig to set off Linux; the deadline
// still kills the process.
func isolateProbe(*exec.Cmd) {}

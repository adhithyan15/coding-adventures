//go:build !linux

package record

import "syscall"

// childAttr: there is no Pdeathsig off Linux, and the recorder does not run
// there (kiln record step refuses before it gets this far).
func childAttr() *syscall.SysProcAttr { return nil }

// setDataLimit: no RLIMIT_DATA that means the same thing off Linux; the soft
// GOMEMLIMIT still applies, and the recorder does not run here.
func setDataLimit(uint64) error { return nil }

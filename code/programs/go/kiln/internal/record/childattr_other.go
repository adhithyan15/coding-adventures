//go:build !linux

package record

import "syscall"

// childAttr: there is no Pdeathsig off Linux, and the recorder does not run
// there (kiln record step refuses before it gets this far).
func childAttr() *syscall.SysProcAttr { return nil }

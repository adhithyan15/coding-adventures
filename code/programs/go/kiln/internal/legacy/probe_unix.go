//go:build !windows

package legacy

import (
	"os/exec"
	"syscall"
)

// isolate makes the probe lead its own process group and makes a timeout kill
// the whole group, so no grandchild outlives the probe.
func isolate(cmd *exec.Cmd) {
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	cmd.Cancel = func() error {
		if cmd.Process == nil {
			return nil
		}
		return syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
	}
}

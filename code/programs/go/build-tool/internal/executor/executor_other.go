//go:build !windows

package executor

import (
	"os/exec"
	"syscall"
)

// setRawWindowsCmdLine is a no-op on non-Windows platforms. See
// executor_windows.go for why this exists: shellCommandForOS can be called
// with goos="windows" from any host (to test Windows behavior without
// running on Windows), but the actual raw-command-line override only makes
// sense -- and only compiles, since syscall.SysProcAttr's fields are
// platform-specific -- when truly built for Windows.
func setRawWindowsCmdLine(cmd *exec.Cmd, command string) {}

// prepareProcessTree puts the command in a process group of its own.
//
// A BUILD line runs as `sh -c <line>`, and the shell forks whatever the line
// names (`cargo`, which forks `rustc`, which forks a linker...). Killing only
// the shell would leave all of those running, and they would keep the output
// pipes open. With Setpgid the shell becomes the leader of a new group whose
// id equals its pid, so one signal sent to the NEGATIVE pid reaches every
// member that has not deliberately moved to another group or session.
func prepareProcessTree(cmd *exec.Cmd) {
	if cmd.SysProcAttr == nil {
		cmd.SysProcAttr = &syscall.SysProcAttr{}
	}
	cmd.SysProcAttr.Setpgid = true
}

// killProcessTree is the Cancel hook for a timed-out command: SIGKILL to the
// whole process group (-pid). It falls back to killing just the shell if the
// group signal fails, and treats "already gone" as success.
func killProcessTree(cmd *exec.Cmd) error {
	if cmd.Process == nil {
		return nil
	}
	if err := syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL); err != nil && err != syscall.ESRCH {
		return cmd.Process.Kill()
	}
	return nil
}

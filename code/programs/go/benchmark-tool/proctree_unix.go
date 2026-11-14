//go:build !windows

package main

import (
	"errors"
	"os/exec"
	"syscall"
)

// processTree lets a service subject be stopped together with everything it started.
//
// A service command is run through a shell (`sh -c "go run server.go ..."`), so killing
// only the shell leaves the real server running as an orphan. On Unix the service is
// started as the leader of its own process group, and the whole group is killed.
type processTree struct {
	cmd *exec.Cmd
}

// configureProcessTree must be called before cmd.Start.
func configureProcessTree(cmd *exec.Cmd) {
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
}

// attachProcessTree must be called right after cmd.Start succeeds.
func attachProcessTree(cmd *exec.Cmd) *processTree {
	return &processTree{cmd: cmd}
}

// Kill kills the service and every process in its group.
func (t *processTree) Kill() error {
	if t == nil || t.cmd == nil || t.cmd.Process == nil {
		return nil
	}
	// A negative pid signals the whole process group.
	err := syscall.Kill(-t.cmd.Process.Pid, syscall.SIGKILL)
	if err == nil || errors.Is(err, syscall.ESRCH) {
		return nil
	}
	return t.cmd.Process.Kill()
}

// Close releases anything held for the tree. Nothing is held on Unix.
func (t *processTree) Close() {}

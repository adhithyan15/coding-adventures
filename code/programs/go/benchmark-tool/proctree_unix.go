//go:build !windows

package main

import (
	"os"
	"os/exec"
	"os/signal"
	"sync"
	"syscall"
)

// processTree lets a service subject be stopped together with everything it started.
//
// A service command is run through a shell (`sh -c "go run server.go ..."`), so killing
// only the shell leaves the real server running as an orphan. On Unix the service is
// started as the leader of its own process group, and the whole group is killed.
//
// Known limits, accepted: a command that deliberately leaves the group (setsid, setpgid)
// escapes cleanup; and Stop's final sweep runs after the shell has been reaped, so in
// theory the shell's pid could have been reused as an unrelated group leader (it cannot
// be reused while any member of the group is alive, and a wrong kill would need the group
// to be empty AND the pid to wrap around).
type processTree struct {
	cmd *exec.Cmd
}

var (
	liveTreesMu sync.Mutex
	liveTrees   = map[*processTree]struct{}{}
	signalsOnce sync.Once
)

// configureProcessTree must be called before cmd.Start.
func configureProcessTree(cmd *exec.Cmd) {
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
}

// attachProcessTree must be called right after cmd.Start succeeds.
func attachProcessTree(cmd *exec.Cmd) *processTree {
	t := &processTree{cmd: cmd}
	liveTreesMu.Lock()
	liveTrees[t] = struct{}{}
	liveTreesMu.Unlock()
	signalsOnce.Do(installInterruptCleanup)
	return t
}

// Kill kills the service and every process in its group.
func (t *processTree) Kill() error {
	if t == nil || t.cmd == nil || t.cmd.Process == nil {
		return nil
	}
	// Never signal pid 0 (our own group) or 1 (init) by accident: a negative pid
	// addresses a whole process group, so the pid must be a real child.
	pid := t.cmd.Process.Pid
	if pid <= 1 {
		return t.cmd.Process.Kill()
	}
	err := syscall.Kill(-pid, syscall.SIGKILL)
	if err == nil || err == syscall.ESRCH {
		return nil
	}
	return t.cmd.Process.Kill()
}

// Close forgets the tree. It is called once the service has been stopped.
func (t *processTree) Close() {
	if t == nil {
		return
	}
	liveTreesMu.Lock()
	delete(liveTrees, t)
	liveTreesMu.Unlock()
}

// killLiveTrees kills every service that is still running.
func killLiveTrees() {
	liveTreesMu.Lock()
	defer liveTreesMu.Unlock()
	for t := range liveTrees {
		_ = t.Kill()
	}
}

// installInterruptCleanup runs once, when the first service starts. A service in its own
// process group no longer receives the terminal's Ctrl-C, so without this an interrupted
// benchmark would leave its server running, holding its port and its log open. On
// SIGINT/SIGTERM/SIGHUP it kills the live services and then re-raises the signal with the
// default action, so the tool still exits the way it would have.
func installInterruptCleanup() {
	ch := make(chan os.Signal, 1)
	signal.Notify(ch, syscall.SIGINT, syscall.SIGTERM, syscall.SIGHUP)
	go func() {
		sig := <-ch
		killLiveTrees()
		signal.Reset(sig)
		if s, ok := sig.(syscall.Signal); ok {
			_ = syscall.Kill(os.Getpid(), s)
		}
	}()
}

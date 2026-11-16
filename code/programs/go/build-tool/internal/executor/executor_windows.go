//go:build windows

package executor

import (
	"os/exec"
	"strconv"
	"syscall"
)

// setRawWindowsCmdLine overrides cmd's native Windows command line so it is
// passed through to cmd.exe verbatim, bypassing Go's default argv-joining.
//
// Go's os/exec builds the actual CreateProcess command line by re-escaping
// each element of Args as if it were a single literal argument value (using
// MSVCRT-compatible backslash-escaping for embedded quotes). That's correct
// when each Args element is really one argument, but here command is already
// a full, pre-quoted shell command line meant to be reinterpreted by cmd.exe
// itself. The re-escaping corrupts any embedded double quotes -- for example
// `uv pip install -e ".[dev]"` arrives at uv as the literal text
// `-e \".[dev]\"`, which uv's own argument parser then rejects with
// `error: Failed to parse: ".[dev]"`. Setting SysProcAttr.CmdLine passes the
// command line through unescaped, matching what a real cmd.exe invocation
// would do, while leaving cmd.Args untouched for callers/tests that inspect
// the command's shape.
func setRawWindowsCmdLine(cmd *exec.Cmd, command string) {
	cmd.SysProcAttr = &syscall.SysProcAttr{CmdLine: "/C " + command}
}

// prepareProcessTree is a no-op on Windows: there is no process group to join
// before the start, and the tree is found at kill time by taskkill instead.
func prepareProcessTree(cmd *exec.Cmd) {}

// killProcessTree is the Cancel hook for a timed-out command.
//
// `taskkill /T /F /PID <pid>` kills the cmd.exe shell AND every process it
// started (/T walks the tree, /F forces it). Plain Process.Kill would end only
// cmd.exe and orphan the build it launched, which is exactly the hang this
// exists to stop. If taskkill itself cannot run, fall back to killing the
// shell so the command at least does not outlive the timeout by much. In that
// fallback (taskkill missing from PATH, which is unusual on Windows) the shell's
// descendants are NOT killed; they may linger, and WaitDelay still bounds the
// build's wait for their output pipes.
func killProcessTree(cmd *exec.Cmd) error {
	if cmd.Process == nil {
		return nil
	}
	kill := exec.Command("taskkill", "/T", "/F", "/PID", strconv.Itoa(cmd.Process.Pid))
	if err := kill.Run(); err != nil {
		return cmd.Process.Kill()
	}
	return nil
}

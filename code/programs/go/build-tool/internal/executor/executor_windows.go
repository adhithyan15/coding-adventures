//go:build windows

package executor

import (
	"os/exec"
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

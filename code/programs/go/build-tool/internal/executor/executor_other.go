//go:build !windows

package executor

import "os/exec"

// setRawWindowsCmdLine is a no-op on non-Windows platforms. See
// executor_windows.go for why this exists: shellCommandForOS can be called
// with goos="windows" from any host (to test Windows behavior without
// running on Windows), but the actual raw-command-line override only makes
// sense -- and only compiles, since syscall.SysProcAttr's fields are
// platform-specific -- when truly built for Windows.
func setRawWindowsCmdLine(cmd *exec.Cmd, command string) {}

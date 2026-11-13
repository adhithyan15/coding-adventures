//go:build windows

package legacy

import "os/exec"

// isolate is a no-op on Windows: the platform has no process groups to signal,
// and the per-probe and overall timeouts still apply.
func isolate(cmd *exec.Cmd) {}

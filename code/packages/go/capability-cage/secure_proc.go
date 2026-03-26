// Secure process wrappers.
//
// These functions wrap process management operations — launching subprocesses
// and sending signals — with capability checks.
//
// Target formats by action:
//   - exec:   command name (e.g., "git", "python3")
//   - signal: process ID as string (e.g., "1234"), or "*" for any PID
//
// Security note: exec capabilities are checked against the command name only,
// not the full path. A manifest declaring proc:exec:git permits running any
// "git" binary found on PATH. Tightening to an absolute path is possible
// via the target field if needed (e.g., "/usr/bin/git").
package capabilitycage

import (
	"os"
	"os/exec"
	"strconv"
)

// Exec checks proc:exec:{name} against m, then returns a configured Cmd.
//
// The returned *exec.Cmd is not started — the caller must call Start, Run,
// or Output. This matches exec.Command semantics.
//
// Returns CapabilityViolationError if the manifest does not declare
// proc:exec for the given command name.
func Exec(m *Manifest, name string, args ...string) (*exec.Cmd, error) {
	if err := m.Check(CategoryProc, ActionExec, name); err != nil {
		return nil, err
	}
	return defaultBackend.Command(name, args...), nil
}

// Signal checks proc:signal:{pid} against m, then sends sig to the process.
//
// pid is the target process ID. The target checked against the manifest is
// the decimal string representation of pid (e.g., "1234"), or "*" in the
// manifest to permit signaling any process.
//
// Returns CapabilityViolationError if the manifest does not declare
// proc:signal for the given PID.
func Signal(m *Manifest, pid int, sig os.Signal) error {
	pidStr := strconv.Itoa(pid)
	if err := m.Check(CategoryProc, ActionSignal, pidStr); err != nil {
		return err
	}
	return defaultBackend.Kill(pid, sig)
}

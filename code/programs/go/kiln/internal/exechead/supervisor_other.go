//go:build !(linux && amd64)

package exechead

import (
	"errors"
	"os"
)

// Supported says this build has a supervisor.  Only linux/amd64 does: the
// mechanism is seccomp user notification, and the filter it listens to is the
// sandbox's, which exists only there.
const Supported = false

// ErrUnsupported is New's answer on every other platform.
var ErrUnsupported = errors.New("exechead: the exec-head supervisor needs linux/amd64")

// Supervisor is a placeholder on platforms without the mechanism.
type Supervisor struct{}

// New always fails here.
func New(Config) (*Supervisor, error) { return nil, ErrUnsupported }

// ChildEnd, Start, Abort and Finish exist so callers compile; nothing calls them.
func (*Supervisor) ChildEnd() *os.File   { return nil }
func (*Supervisor) Start()               {}
func (*Supervisor) SetKillStrace(func()) {}
func (*Supervisor) Abort()               {}
func (*Supervisor) Finish() Result       { return Result{} }

package trace

// Error is the one error type Parse returns.  Every failure is a refusal to
// vouch for the trace, so every failure is `trace-incomplete:<detail>`; the
// detail strings are stable and are what KILN04 §6 reports.
//
// The Reason never contains path text.  A filename can come from a pull
// request, and reasons are printed in CI logs, so only syscall names (which are
// restricted to [a-z0-9_] and 64 bytes before they reach a reason) and fixed
// words appear in it.  Line is the 1-based line of the trace where the problem
// was detected (0 when it is not tied to one line).
type Error struct {
	Reason string // detail only, e.g. "unknown-syscall:foo"
	Line   int
	Err    error // underlying I/O error, if any
}

// Error returns "trace-incomplete:" followed by the reason.
func (e *Error) Error() string { return "trace-incomplete:" + e.Reason }

// Unwrap exposes the underlying I/O error for reads that failed.
func (e *Error) Unwrap() error { return e.Err }

func incomplete(line int, reason string) *Error { return &Error{Reason: reason, Line: line} }

// Stable reason details.  Reasons with a ":<name>" suffix are built by the
// code that knows the syscall name.
const (
	reasonSize           = "size"
	reasonAmbiguousPath  = "ambiguous-path"
	reasonTruncated      = "truncated"
	reasonNonUTF8        = "non-utf8-path"
	reasonDeleted        = "deleted-path"
	reasonUnparsable     = "unparsable-line"
	reasonPersonality    = "personality"
	reasonInterleaved    = "interleaved-call"
	reasonUnmatched      = "unmatched-resume"
	reasonUnresumed      = "unresumed-call"
	reasonCloneFlags     = "clone-flags"
	reasonArity          = "arity"
	reasonPathArg        = "path-arg"
	reasonDirfd          = "dirfd"
	reasonFDPath         = "fd-path"
	reasonSockaddr       = "sockaddr"
	reasonOrphanPid      = "orphan-pid"
	reasonPidReuse       = "pid-reuse"
	reasonProcessCycle   = "process-cycle"
	reasonMissingExit    = "missing-exit"
	reasonEventAfterExit = "event-after-exit"
	reasonDuplicateExit  = "duplicate-exit"
	reasonEmpty          = "empty"
	reasonInitialCwd     = "initial-cwd"
	reasonCwdTooLong     = "cwd-too-long"
	reasonResult         = "result"
	reasonRead           = "read"
)

package record

// strace's own stderr, reduced ON THE RUNNER to one code from a closed list.
//
// A step sidecar already says THAT strace wrote to stderr (strace_stderr_empty,
// the reason `unclean:strace-stderr-not-empty`).  The text itself never leaves the
// runner: it can hold host paths, so it stays in `<step>.strace-stderr`.  One
// census run had one step unclean for exactly this reason and nothing to say why.
// ClassifyStraceStderr gives the census the WHY without the text: a code from
// StraceStderrCodes, which is all a sidecar, a record or the census ever carries.
//
// The patterns are strace's own messages (strace.c, the ptrace code in
// trace_event.c and the xmalloc and error helpers).  The list is small on purpose:
// each code is something a human would act on differently, and anything else is
// `unknown`.  Matching is by fixed substring over at most MaxStraceStderrScan
// bytes, in the order of the switch below, so the result never depends on text
// beyond that.  The input is never copied into the result.

import (
	"bytes"
	"strconv"
)

// The codes (StepSidecar.StraceStderrCode).
const (
	StraceErrListenEIO  = "ptrace-listen-eio" // ptrace(PTRACE_LISTEN ...): Input/output error: the known race on a dying thread
	StraceErrAttach     = "attach-failed"     // attach: ptrace(PTRACE_SEIZE ...) or "Operation not permitted"
	StraceErrOOM        = "oom"               // "Out of memory", "memory exhausted", "Cannot allocate memory"
	StraceErrPtraceOth  = "ptrace-other"      // any other ptrace(...) failure
	StraceErrWait       = "wait-failed"       // wait4(...) or waitid(...) failed
	StraceErrKilled     = "exit-killed"       // strace was killed by a signal and nothing above matched
	StraceErrUnknown    = "unknown"           // anything else
	StraceErrUnrecorded = "unrecorded"        // census only: stderr text with no code (a sidecar written before the field)
)

// StraceStderrCodes is the closed list a sidecar may carry.
var StraceStderrCodes = []string{
	StraceErrListenEIO, StraceErrAttach, StraceErrOOM, StraceErrPtraceOth, StraceErrWait, StraceErrKilled, StraceErrUnknown,
}

// MaxStraceStderrScan bounds the bytes ClassifyStraceStderr looks at (the
// recorder's capture keeps 4 KiB; a longer input is cut, never scanned whole).
const MaxStraceStderrScan = 4096

// ClassifyStraceStderr returns the code for strace's stderr text, given the
// signal that killed strace (0 if none).  Empty text has no code: "".
func ClassifyStraceStderr(stderr []byte, signal int) string {
	if len(stderr) == 0 {
		return ""
	}
	if len(stderr) > MaxStraceStderrScan {
		stderr = stderr[:MaxStraceStderrScan]
	}
	has := func(s string) bool { return bytes.Contains(stderr, []byte(s)) }
	switch {
	case has("ptrace(PTRACE_LISTEN") && has("Input/output error"):
		return StraceErrListenEIO
	case has("attach: ptrace(PTRACE_SEIZE") || has("Operation not permitted"):
		return StraceErrAttach
	case has("Out of memory") || has("out of memory") || has("memory exhausted") || has("Cannot allocate memory"):
		return StraceErrOOM
	case has("ptrace("):
		return StraceErrPtraceOth
	case has("wait4(") || has("waitid("):
		return StraceErrWait
	case signal != 0:
		return StraceErrKilled
	}
	return StraceErrUnknown
}

// validStraceStderrCode says c is in the closed list.
func validStraceStderrCode(c string) bool {
	for _, k := range StraceStderrCodes {
		if c == k {
			return true
		}
	}
	return false
}

// signalNames are Linux's signals 1..31 (the ones strace can die of).
var signalNames = [...]string{
	1: "SIGHUP", 2: "SIGINT", 3: "SIGQUIT", 4: "SIGILL", 5: "SIGTRAP", 6: "SIGABRT", 7: "SIGBUS", 8: "SIGFPE",
	9: "SIGKILL", 10: "SIGUSR1", 11: "SIGSEGV", 12: "SIGUSR2", 13: "SIGPIPE", 14: "SIGALRM", 15: "SIGTERM",
	16: "SIGSTKFLT", 17: "SIGCHLD", 18: "SIGCONT", 19: "SIGSTOP", 20: "SIGTSTP", 21: "SIGTTIN", 22: "SIGTTOU",
	23: "SIGURG", 24: "SIGXCPU", 25: "SIGXFSZ", 26: "SIGVTALRM", 27: "SIGPROF", 28: "SIGWINCH", 29: "SIGIO",
	30: "SIGPWR", 31: "SIGSYS",
}

// StraceSignalLabel names strace's terminating signal for the census: "SIGKILL(9)",
// or "SIG(n)" for a number outside the table.  Built from a number only.
func StraceSignalLabel(n int) string {
	if n > 0 && n < len(signalNames) && signalNames[n] != "" {
		return signalNames[n] + "(" + strconv.Itoa(n) + ")"
	}
	return "SIG(" + strconv.Itoa(n) + ")"
}

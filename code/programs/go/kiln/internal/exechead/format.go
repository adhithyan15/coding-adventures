// Package exechead is the recorder's half of KILN04 D48, "exec heads recorded
// at exec time".
//
// The problem, in one paragraph
// -----------------------------
// `go test` builds a test binary under the step's fresh tmpfs, runs it, and the
// tmpfs is gone before `collect` could read the binary.  The classifier needs
// the first bytes of every executable the kernel loaded (a `#!` line, an ELF's
// PT_INTERP) to walk the loader chain, so it cannot judge such an action.  This
// package takes those bytes WHILE THE EXEC HAPPENS: a seccomp user-notification
// supervisor (supervisor_linux_amd64.go) is told about every execve and execveat
// of the step, pins the file the path names, lets the call go, and then watches
// `/proc/<tgid>/exe` to see that the process really became that file.  What it
// saw is written to a side file, `<step>.exechead`, in the format of this file.
//
// What lives where
// ----------------
//
//	format.go      the vocabulary (statuses, limits, header, line)   every OS
//	writer.go      the encoder: one line at a time, caps, a digest    every OS
//	decode.go      the strict decoder PR 5's classifier will use      every OS
//	supervisor_*   the kernel half                                    linux/amd64
//
// The format and its two ends are portable on purpose: the classifier reads the
// file on the machine that collects, which need not be Linux, and its tests run
// everywhere.
//
// The sidecar, in one picture
// ---------------------------
//
//	{"v":1,"kernel":"7.0.0-1014-aws","nonce":"<32 hex>","action_sha256":"<64 hex>","step":0}
//	{"seq":0,"tid":4889,"tgid":4889,"syscall":"execve","path":"/usr/bin/sh","status":"outside"}
//	{"seq":1,"tid":4891,"tgid":4891,"syscall":"execve","path":"/tmp/x/t.test","status":"confirmed",
//	 "dev":58,"ino":12,"head":"f0VMRg...","sha256":"<64 hex>"}
//
// One JSON object per line, header first, `seq` counting from 0 with no gap.
// `dev`, `ino`, `head` and `sha256` exist ONLY on `confirmed` lines.
package exechead

// Version is the format version of the header's "v".
const Version = 1

// The limits of the format.  The decoder refuses what exceeds them and the
// writer never produces it.
const (
	// HeadLimit is how many leading bytes of an exec'd file a confirmed line
	// carries.  It equals the classifier's own limit (traced.HeadLimit, checked
	// by a test in internal/record): a "#!" line is at most 255 bytes and an ELF
	// program-header table sits at the start of the file.
	HeadLimit = 64 << 10
	// PathMax is Linux's PATH_MAX: the most bytes of a path the supervisor reads
	// from the step (a string with no NUL within this many bytes is `unpinned`).
	PathMax = 4096
	// MaxLineBytes bounds one encoded line, newline excluded.  The head is at
	// most 87384 base64 characters; a path of PathMax-1 bytes may need six
	// characters each ("\u0001") once JSON-escaped.
	MaxLineBytes = 128 << 10
	// MaxLines bounds the notification lines of one file, and MaxFileBytes the
	// file.  Past either the supervisor stops writing and the step is unclean
	// (UncleanOverflow); the trace's own cap is the model.
	MaxLines     = 1 << 17
	MaxFileBytes = 64 << 20
	// MaxKernelBytes bounds the header's kernel release.
	MaxKernelBytes = 256
)

// Status is what the supervisor concluded about one notification.  The names
// are the spec's; the set is closed.
type Status string

const (
	// StatusConfirmed: the process was SEEN to become the pinned file (an
	// observed transition of /proc/<tgid>/exe), and the head was read from the
	// pinned descriptor.  The only status that carries dev, ino, head and sha256.
	StatusConfirmed Status = "confirmed"
	// StatusOutside: the file is not on the step's fresh tmpfs; nothing was
	// pinned.  A placeholder that keeps the count honest.
	StatusOutside Status = "outside"
	// StatusGone: the process exited before the confirm, so no head is vouched
	// for (a file no process holds could have been rewritten after the exit).
	StatusGone Status = "gone"
	// StatusMismatch: exe changed to something other than the pin (a `#!` script
	// under the tmpfs, a swapped path), or stopped being the pin after the head
	// was read.
	StatusMismatch Status = "mismatch"
	// StatusTimeout: the confirm deadline passed with the thread still in
	// execve and exe unchanged, and nothing settled it before the step ended or
	// another thread's exec forced a taint.
	StatusTimeout Status = "timeout"
	// StatusUnpinned: the path could not be read, resolved or pinned.
	StatusUnpinned Status = "unpinned"
	// StatusFexecve: execveat with AT_EMPTY_PATH and an empty string; there is no
	// path to pin (refused in v1).
	StatusFexecve Status = "fexecve"
	// StatusSameImage: the process already ran the very file it was about to
	// exec, so the exec is not observable as a transition (a self re-exec).
	StatusSameImage Status = "same-image"
	// StatusFailed: the thread left execve and exe is still the pre-exec image:
	// the exec failed and the process runs on.
	StatusFailed Status = "failed"
	// StatusOverload: a cap was exceeded; nothing was pinned and the group is
	// tainted.
	StatusOverload Status = "overload"
	// StatusTainted: the thread group is tainted; never confirmed.
	StatusTainted Status = "tainted"
)

// Valid says s is one of the eleven.
func (s Status) Valid() bool {
	switch s {
	case StatusConfirmed, StatusOutside, StatusGone, StatusMismatch, StatusTimeout,
		StatusUnpinned, StatusFexecve, StatusSameImage, StatusFailed, StatusOverload, StatusTainted:
		return true
	}
	return false
}

// The two system calls the filter notifies.
const (
	SyscallExecve   = "execve"
	SyscallExecveat = "execveat"
)

// Why a step is not clean because of the supervisor (entries of the step
// sidecar's Unclean).  Any of them refuses the action at `collect`.
const (
	// UncleanHandover: the listener never arrived, or arrived wrongly (not
	// exactly one descriptor and one data byte 'L'), within the deadline.
	UncleanHandover = "exechead-handover"
	// UncleanReceiver: the receiver stopped for a reason other than the listener
	// hanging up (an error, a panic).  The step is killed and the listener closed,
	// so no later exec can block for ever.
	UncleanReceiver = "exechead-receiver"
	// UncleanFailed: a supervisor-side error (an ioctl failure other than ENOENT
	// for a vanished notification, a sidecar write failure, a panic), or the
	// supervisor stopped before the step ended.
	UncleanFailed = "exechead-failed"
	// UncleanOverflow: the sidecar's line or byte cap, or the group table's, was
	// passed; the supervisor kept answering CONTINUE without pinning.
	UncleanOverflow = "exechead-overflow"
	// UncleanStuck: a pending exec outlived the absolute lifetime bound.
	UncleanStuck = "exechead-stuck"
	// UncleanStall: a notification went unanswered past the stall deadline.
	UncleanStall = "exechead-stall"
)

// Header is the first line of the file.
type Header struct {
	// V is Version.
	V int `json:"v"`
	// Kernel is `uname -r` of the recording machine.
	Kernel string `json:"kernel"`
	// Nonce is 16 random bytes in hex, drawn by `kiln record step` per step and
	// stored in the step sidecar too.
	Nonce string `json:"nonce"`
	// ActionSHA256 and Step name the step the file belongs to.
	ActionSHA256 string `json:"action_sha256"`
	Step         int    `json:"step"`
	// WaitKillable says the listener was installed with
	// SECCOMP_FILTER_FLAG_WAIT_KILLABLE_RECV (the init sent 'K'): a signal cannot
	// interrupt an unanswered notification.  False when the kernel refused the
	// flag (plain NEW_LISTENER, where a signal can interrupt and restart a call),
	// and when no listener ever arrived.
	WaitKillable bool `json:"wait_killable"`
}

// Line is one notification.
type Line struct {
	// Seq is set by the writer: 0, 1, 2, ... in arrival order.
	Seq int
	// Tid is the notified thread and Tgid its thread group, both in the
	// supervisor's pid namespace (strace's, so the numbers its trace prints).
	Tid, Tgid int
	// Syscall is SyscallExecve or SyscallExecveat.
	Syscall string
	// Path is the string the step asked to exec, as read from its memory:
	// evidence of what was asked, never trusted as what ran.  "" when it could
	// not be read or is not UTF-8.
	Path   string
	Status Status
	// The four below exist only when Status is StatusConfirmed.
	Dev, Ino uint64
	Head     []byte
	SHA256   string // hex of Head
}

// wireHeader and wireLine are the JSON shapes.  Every field is a pointer so
// "missing" can be told from "zero"; key order is the canonical order (the
// decoder re-encodes and compares bytes).
type wireHeader struct {
	V            *int    `json:"v"`
	Kernel       *string `json:"kernel"`
	Nonce        *string `json:"nonce"`
	ActionSHA256 *string `json:"action_sha256"`
	Step         *int    `json:"step"`
	WaitKillable *bool   `json:"wait_killable"`
}

type wireLine struct {
	Seq     *int    `json:"seq"`
	Tid     *int    `json:"tid"`
	Tgid    *int    `json:"tgid"`
	Syscall *string `json:"syscall"`
	Path    *string `json:"path"`
	Status  *string `json:"status"`
	Dev     *uint64 `json:"dev,omitempty"`
	Ino     *uint64 `json:"ino,omitempty"`
	Head    *string `json:"head,omitempty"`
	SHA256  *string `json:"sha256,omitempty"`
}

// hexN says s is exactly n lowercase hex digits.
func hexN(s string, n int) bool {
	if len(s) != n {
		return false
	}
	for i := 0; i < len(s); i++ {
		if c := s[i]; !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return false
		}
	}
	return true
}

// ValidNonce says s is a header nonce: 16 random bytes as 32 lowercase hex
// digits.  The step sidecar (internal/record) checks its copy with it.
func ValidNonce(s string) bool { return hexN(s, 32) }

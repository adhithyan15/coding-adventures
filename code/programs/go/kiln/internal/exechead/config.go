package exechead

import (
	"os"
	"time"
)

// Config configures a supervisor.
type Config struct {
	// Out is the sidecar, created exclusively by the caller and open for writing.
	// The supervisor writes the header at once, every line as it is decided, and
	// syncs the file in Finish.
	Out *os.File
	// Header names the step.  V and Kernel are filled in when zero.
	Header Header
	// Tmpfs are the paths, as the STEP sees them, of its fresh tmpfs: $HOME,
	// $TMPDIR and the scratch directories.  A file whose device is none of
	// theirs is `outside`.  The devices are measured through
	// /proc/<tgid>/root of the step's first process; one that cannot be measured
	// counts as "on the tmpfs", so nothing is skipped by mistake.
	Tmpfs []string
	// Timing and Caps override the defaults when set.
	Timing Timing
}

// Timing are the supervisor's deadlines and caps; a zero field is the default.
type Timing struct {
	// Handover bounds the wait for the listener.  strace holds its own copy of
	// the socket and never closes it, so end of file will NOT come: the deadline
	// is the only way to notice a lost hand-over.
	Handover time.Duration
	// Confirm is the confirm deadline: how long a pinned exec is polled before it
	// counts as past its deadline.  Chosen from the probe's 12 us to 1.2 ms with a
	// wide margin for a loaded runner.
	Confirm time.Duration
	// Stall is the longest a notification may go unanswered once the processor
	// has it (and the longest it may wait in the inbox).
	Stall time.Duration
	// Lifetime is the absolute bound on a pending exec, from its CONTINUE: an exec
	// stuck in the kernel that long is a stuck step.
	Lifetime time.Duration
	// FinishWait bounds each wait of Finish.
	FinishWait time.Duration

	MaxConfirms   int // pinned execs being confirmed at once, all groups together
	MaxGroupQueue int // held notifications of one group
	MaxGroups     int // thread groups remembered (the taint table)
	MaxInbox      int // notifications waiting for the processor

	File Limits // the sidecar's caps

	// The two below are TEST SEAMS, zero in production.  PollStart delays each
	// poller's first look, so a test can hold a pinned exec pending for as long as
	// it needs (the same-thread settle, the held queue, `gone`); ProcessDelay
	// delays the processor before it looks at each notification, so a test can
	// interrupt one that is still unanswered.
	PollStart    time.Duration
	ProcessDelay time.Duration
	// InjectReceiverError makes the receiver fail as if NOTIF_RECV had, after it
	// has taken that many notifications (0: never).
	InjectReceiverError int
}

func (t Timing) withDefaults() Timing {
	def := func(v *time.Duration, d time.Duration) {
		if *v <= 0 {
			*v = d
		}
	}
	defInt := func(v *int, d int) {
		if *v <= 0 {
			*v = d
		}
	}
	def(&t.Handover, 30*time.Second)
	def(&t.Confirm, 5*time.Second)
	def(&t.Stall, 10*time.Second)
	def(&t.Lifetime, 2*time.Minute)
	def(&t.FinishWait, 5*time.Second)
	defInt(&t.MaxConfirms, 64)
	defInt(&t.MaxGroupQueue, 32)
	defInt(&t.MaxGroups, 1024)
	defInt(&t.MaxInbox, 1<<16)
	return t
}

// Result is what Finish reports.
type Result struct {
	// Unclean lists why the step is not clean because of the supervisor, in the
	// order found, with no repeat.  Empty means the supervisor's half is clean.
	Unclean []string
	// Notes are diagnostics for a human (never stored in the sidecar).
	Notes []string
	// Lines, Bytes and SHA256 describe the sidecar as written.
	Lines  int
	Bytes  int64
	SHA256 string
	// Received says the listener arrived.
	Received bool
	// Dropped counts notifications that stopped being valid before they were
	// answered (a thread killed by another thread's exec, a call interrupted by a
	// signal and restarted): they never ran, no line was written, no taint.
	Dropped int
	// Notified counts the notifications the receiver took off the listener
	// (D48 step 6 item 2: the census sums it over a shard's steps, so a run that
	// "used exec heads" can be told from one whose supervisor never heard a
	// thing).  Kills counts the times the supervisor ended the step itself (a
	// stall, a stuck notification, a dead receiver, a died supervisor); a clean
	// step has none, and the census gate refuses any.
	Notified int
	Kills    int
}

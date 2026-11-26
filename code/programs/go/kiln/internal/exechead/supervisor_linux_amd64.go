//go:build linux && amd64

package exechead

// The supervisor (KILN04 D48, step 4)
// ===================================
//
// Who runs it, and where
// ----------------------
//
//	kiln record step                 <- this file runs here, as goroutines
//	  |  socketpair (AF_UNIX, SOCK_STREAM): our end is CLOEXEC, the other end is
//	  |  strace's descriptor 6, and `--exec-notify-fd 6` is on sandbox-exec's argv
//	  v
//	strace -f ...
//	  v
//	kiln sandbox-exec  ->  kiln sandbox-init  --- sends the seccomp LISTENER over
//	                          |                   the socket (SCM_RIGHTS, one
//	                          |                   descriptor, one data byte 'L'),
//	                          v                   then closes both
//	                       the step (execve is a notification the listener sees)
//
// The goroutines
// --------------
//
//	receiver   gets the listener (with a deadline), then loops poll + NOTIF_RECV
//	           and puts each notification in the INBOX.  It never decides anything.
//	processor  takes the inbox one item at a time and does what D48 calls "steps
//	           1 to 6": the bounded work of recording a notification and answering
//	           CONTINUE.  It never waits for a confirm.
//	pollers    one per pinned exec still being confirmed.  Each watches
//	           /proc/<tgid>/exe until the process has visibly BECOME the pinned
//	           file (or has not, for one of the reasons below) and writes the line.
//	watchdog   enforces the stall deadline, the pending-lifetime bound, and sweeps
//	           the thread groups that have exited.  It exists because a Go system
//	           call (process_vm_readv, openat2) cannot be cancelled: the receiver
//	           cannot time itself out, so something else must kill the step.
//
// What the answer means
// ---------------------
// The supervisor ALWAYS answers CONTINUE.  CONTINUE is explicitly not a security
// decision: the kernel re-reads the path from the step afterwards, so a thread of
// the step may have rewritten it.  The supervisor never blocks a build; it only
// declines to VOUCH.  What it writes in the sidecar, per notification, is a
// status that says how much the classifier may believe (format.go).
//
// The confirm, in one picture
// ---------------------------
//
//	notification for  execve("/tmp/x/t.test")  from thread T of group G
//	   pin the file                (open it; keep the descriptor: the inode cannot
//	                                be freed and its number reused while open)
//	   pre = identity of /proc/G/exe              (the image G runs BEFORE the exec)
//	   answer CONTINUE
//	   poll:  read /proc/T/syscall FIRST, /proc/G/exe AFTER
//	       exe changed from pre to the pin    -> read the head from the pinned
//	                                             descriptor, look at exe again:
//	                                             still the pin -> `confirmed`
//	       exe changed to anything else       -> `mismatch`
//	       T left execve, exe still pre       -> `failed`
//	       the process exited                 -> `gone`
//	       nothing yet                        -> keep polling; past the deadline
//	                                             this becomes a `timeout` only if
//	                                             another thread's exec forces a
//	                                             taint, or the step ends
//
// Why "exe changed" and not "exe equals the pin": a process that already runs
// binary F and execs "F" would pass the second test before the exec did
// anything, while another thread swapped the path to G.  A transition from A to B
// (A != B) can only be an exec that loaded B.
//
// One exec per thread group at a time, and the same-thread rule
// -------------------------------------------------------------
// /proc/<tgid>/exe belongs to the process, so two execs of one group in flight
// could be told apart only by luck.  A second notification for a group that has
// an exec pending is HELD (queued, not answered) until the first is decided,
// UNLESS it comes from the SAME thread as the group's last undecided exec: a
// thread that is making a notification cannot still be inside an earlier
// execve, so that earlier exec is over.  The same-thread notification settles the
// earlier exec on the spot and bypasses the queue.  A DIFFERENT thread's
// notification while the earlier exec is undecided and not pending-within-its-
// deadline TAINTS the group: every later line of it is `tainted`, never
// `confirmed`.  Taint is the backstop; the held queue is what keeps it rare.
//
// Locking
// -------
// One mutex (mu) guards every piece of shared state: the groups, the execs'
// states, the writer, the caps.  System calls on /proc or the step's memory are
// made WITHOUT it.  The processor is the only goroutine that creates execs and
// decides what to do with a new notification; the pollers and the processor
// meet only through an exec's three-state flag (pending, claimed, written): the
// first to move it from pending to claimed, under mu, owns the line, and the
// other does not write.  A claimed exec's line is written exactly once, in the
// order of the group, because a settle that finds the exec claimed WAITS for it
// to become written before it answers the new notification.

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"strconv"
	"sync"
	"sync/atomic"
	"syscall"
	"time"
	"unicode/utf8"
)

// Supported says this build has a supervisor.
const Supported = true

// Supervisor is the receiving end of the exec notifications of one step.
type Supervisor struct {
	cfg        Config
	tm         Timing
	killStrace func()

	parent int      // our end of the socket pair, close-on-exec
	child  *os.File // the other end, for strace's ExtraFiles; closed by Start

	// lfd is the listener, -1 until received.  lmu makes "load the number and use
	// it" atomic with "swap it out and close it": the users (answer, valid) hold
	// the read lock across their ioctl, closeListener the write lock, so a closed
	// descriptor number that something else has since reused is never ioctl'd.
	lfd atomic.Int32
	lmu sync.RWMutex
	// initBound is the one-shot flag of the init pidfd: only the FIRST notification
	// may bind it.
	initBound atomic.Bool
	// stale counts consecutive watchdog ticks that could not take mu.
	stale int

	// umu guards unclean and notes ONLY.  It is separate from mu because the kill
	// paths (the stall watchdog, the receiver's death) mark the step unclean, and
	// mu can be held across a blocking write of the sidecar: a stalled disk must not
	// be able to stop the step from being killed.
	umu     sync.Mutex
	unclean []string
	notes   []string

	mu     sync.Mutex
	cond   *sync.Cond // on mu: an exec became written
	w      *Writer    // created when the hand-over has been decided (the header says whether the listener is killable)
	hdr    Header
	groups map[int]*group
	active map[*exec]struct{} // pinned execs with a poller
	slots  int                // global confirm slots in use
	// overloadAll is the table-overflow state: every later line is `tainted`.
	overloadAll bool
	// writing is false once the sidecar cannot take more lines (cap or error).
	writing bool
	// degraded: the processor failed; every notification is answered CONTINUE and
	// nothing else is done.
	degraded bool
	// devs are the devices of the step's tmpfs mounts, measured once.
	devs       map[uint64]bool
	devsDone   bool
	unmeasured bool
	// initPidfd is a DUP of the pidfd the first notification's group was validated
	// with (the sandbox's init), or -1.  Atomic: the kill reads it without mu.
	initPidfd atomic.Int32
	received  bool
	pStarted  atomic.Bool
	pOnce     sync.Once
	rCount    int // notifications the receiver has taken (the injection seam)

	inb inbox

	ending  atomic.Bool
	aborted atomic.Bool
	// rExit is how the receiver ended.
	rExit atomic.Int32

	procStart atomic.Int64 // UnixNano when the processor took its current item, 0 idle
	dropped   atomic.Int64
	// notified and kills feed Result.Notified and Result.Kills.  Atomic because
	// Finish may read them while a goroutine that did not stop in time still runs.
	notified atomic.Int64
	kills    atomic.Int64

	started bool
	rDone   chan struct{}
	pDone   chan struct{}
	wDone   chan struct{}
	stopW   chan struct{}
	wg      sync.WaitGroup // pollers
}

// How the receiver ended.
const (
	rRunning    int32 = iota
	rHangup           // the listener hung up: every task of the filter is gone (the normal end)
	rStopped          // Finish asked it to stop
	rAborted          // Abort
	rError            // an error: the step is unclean
	rNoListener       // the hand-over failed
)

type execKind int

const (
	// kUndecided: released without a definite outcome, or still being confirmed.
	kUndecided execKind = iota
	// kFailed: decided `failed`, remembered so a wrong "failed" is caught by the
	// same-thread settle.
	kFailed
)

type execState int

const (
	statePending execState = iota
	stateClaimed
	stateWritten
)

// exec is one notification's record.
type exec struct {
	g         *group
	kind      execKind
	tid, tgid int
	nr        int32
	path      string
	// pin is the pinned descriptor, or -1; pinID its identity; pre the identity
	// of /proc/<tgid>/exe before the exec.
	pin   int
	pinID ident
	pre   ident
	// poller says this exec has (or had) a poller; state is the three-state flag.
	poller   bool
	state    execState
	cont     time.Time // when CONTINUE was sent
	deadline time.Time
	past     bool // past the confirm deadline; the poller lingers outside the global cap
	slot     bool // holds a global confirm slot
}

// group is a thread group the supervisor has seen exec.
type group struct {
	tgid  int
	pidfd int // -1 when it could not be opened (overload): never dropped, always tainted
	// tainted: every pending and later line is `tainted`, never `confirmed`.
	tainted bool
	// last is the group's most recent exec that has no definite outcome (or a
	// remembered `failed`).
	last *exec
	// queue holds notifications that arrived while an exec of another thread was
	// being confirmed.  They hold no descriptor and no poller, only memory.
	queue []seccompNotif
}

// New creates the socket pair and the sidecar's writer (the header is written
// at once).  The caller passes ChildEnd() to strace as ExtraFiles and calls
// Start once strace has started.
func New(cfg Config) (*Supervisor, error) {
	if cfg.Out == nil {
		return nil, errors.New("exechead: no sidecar file")
	}
	if cfg.Header.Kernel == "" {
		k, err := kernelRelease()
		if err != nil {
			return nil, err
		}
		cfg.Header.Kernel = k
	}
	// The header is checked now and written when the hand-over is decided (it says
	// whether the listener is killable); a file that could not be written is found
	// out then, and makes the step unclean.
	probe := cfg.Header
	probe.V = Version
	if err := checkHeader(probe); err != nil {
		return nil, err
	}
	// The kernel's notification structures must be the ones this file hard-codes:
	// a kernel that reports other sizes would read our buffers wrongly.
	if err := checkNotifSizes(); err != nil {
		return nil, err
	}
	pair, err := syscall.Socketpair(syscall.AF_UNIX, syscall.SOCK_STREAM|syscall.SOCK_CLOEXEC, 0)
	if err != nil {
		return nil, fmt.Errorf("exechead: socketpair: %w", err)
	}
	s := &Supervisor{
		cfg: cfg, tm: cfg.Timing.withDefaults(), parent: pair[0],
		child:  os.NewFile(uintptr(pair[1]), "exec-notify-child-end"),
		groups: map[int]*group{}, active: map[*exec]struct{}{}, hdr: cfg.Header, writing: true,
		rDone: make(chan struct{}), pDone: make(chan struct{}), wDone: make(chan struct{}), stopW: make(chan struct{}),
	}
	s.initPidfd.Store(-1)
	s.lfd.Store(-1)
	s.cond = sync.NewCond(&s.mu)
	s.inb.init()
	// os.NewFile does not set close-on-exec; the child end is meant to be
	// inherited by exactly one exec (strace's, through ExtraFiles, which dups it)
	// and our copy is closed by Start.  Mark it anyway so nothing else the
	// recorder starts in between can carry it.
	syscall.CloseOnExec(pair[1])
	return s, nil
}

// kernelRelease is `uname -r`.
func kernelRelease() (string, error) {
	var u syscall.Utsname
	if err := syscall.Uname(&u); err != nil {
		return "", fmt.Errorf("exechead: uname: %w", err)
	}
	b := make([]byte, 0, len(u.Release))
	for _, c := range u.Release {
		if c == 0 {
			break
		}
		b = append(b, byte(c))
	}
	return string(b), nil
}

// ChildEnd is the end of the socket pair strace inherits (as descriptor 6, the
// fourth ExtraFiles entry).  Start closes the recorder's own copy.
func (s *Supervisor) ChildEnd() *os.File { return s.child }

// SetKillStrace sets the second line of the kill (the first is the pidfd of the
// sandbox's init): a function that kills strace, whose --kill-on-exit takes every
// tracee with it.  Call it before Start.
func (s *Supervisor) SetKillStrace(f func()) { s.killStrace = f }

// Start begins receiving.  Call it right after strace has started; the child
// end is closed here (strace and, after it, the sandbox hold the only copies).
func (s *Supervisor) Start() {
	s.child.Close()
	s.started = true
	go s.runReceiver()
	go s.runWatchdog()
}

// Abort simulates the supervisor dying in the middle of a step (a test seam):
// the listener is closed, so every later exec of the step fails with ENOSYS, and
// the step is unclean.
func (s *Supervisor) Abort() {
	s.aborted.Store(true)
	s.closeListener()
}

// ---------------------------------------------------------------- the receiver

// startWriter creates the sidecar's writer (and so writes the header) once, with
// the hand-over's outcome.  Called by the receiver, and by Finish if the receiver
// never got that far.
func (s *Supervisor) startWriter(killable bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.w != nil {
		return
	}
	h := s.hdr
	h.V = Version
	h.WaitKillable = killable
	w, err := NewWriter(s.cfg.Out, h, s.cfg.Timing.File)
	if err != nil {
		s.writing = false
		s.markLocked(UncleanFailed, "the sidecar's header: "+err.Error())
		// A writer that writes nothing, so the counters stay valid.
		w, _ = NewWriter(io.Discard, h, s.cfg.Timing.File)
	}
	s.w = w
}

// closeP closes pDone once.
func (s *Supervisor) closeP() { s.pOnce.Do(func() { close(s.pDone) }) }

// receiverDied is what happens when the receiver stops for any reason other than
// the listener hanging up, Finish asking, or Abort: the step is marked unclean
// (exechead-receiver) and KILLED, and the listener is closed.  Without that the
// listener would stay open with nobody to answer, and every later exec of the step
// would block for ever (and the watchdog, which watches the processor, would not
// see it).  Closing the listener makes any exec still to come fail with ENOSYS
// instead of blocking.  The inbox is closed so the processor stops.
func (s *Supervisor) receiverDied(why string) {
	s.rExit.Store(rError)
	s.markLocked(UncleanReceiver, why) // umu only: the kill must not wait on mu
	if !s.ending.Load() {
		s.killStep()
	}
	s.closeListener()
	s.inb.close()
	if !s.pStarted.Load() {
		s.closeP()
	}
}

// runReceiver gets the listener, starts the processor and loops on the listener.
func (s *Supervisor) runReceiver() {
	defer close(s.rDone)
	defer func() {
		if r := recover(); r != nil {
			s.degrade(fmt.Sprintf("the receiver panicked: %v", r))
			s.receiverDied(fmt.Sprintf("the receiver panicked: %v", r))
		}
	}()
	lfd, killable, err := s.receiveListener()
	if err != nil {
		s.rExit.Store(rNoListener)
		s.markLocked(UncleanHandover, err.Error()) // umu only
		// Nobody can answer the init's exec: kill the step rather than let it hang.
		// There is no init pidfd yet (it is taken from the first notification), so
		// the kill is strace's, and --kill-on-exit takes every tracee with it.  The
		// kill comes BEFORE anything that takes mu (the header write).
		if !s.ending.Load() {
			s.killStep()
		}
		s.startWriter(false)
		s.inb.close()
		s.closeP()
		return
	}
	s.startWriter(killable)
	s.lfd.Store(int32(lfd))
	s.mu.Lock()
	s.received = true
	s.mu.Unlock()
	s.pStarted.Store(true)
	go s.runProcessor()
	code := s.receiveLoop(lfd)
	if code == rError {
		s.receiverDied("the receive loop failed")
		return
	}
	s.rExit.Store(code)
	s.inb.close()
}

// receiveListener receives the one message of the hand-over: exactly one
// descriptor (SCM_RIGHTS) and exactly one data byte, 'L' (a plain listener) or 'K' (one installed with
// SECCOMP_FILTER_FLAG_WAIT_KILLABLE_RECV).  Anything else fails.
// The wait is bounded by Timing.Handover.
func (s *Supervisor) receiveListener() (int, bool, error) {
	deadline := time.Now().Add(s.tm.Handover)
	data := make([]byte, 8) // room for more than the contract allows, so extra is SEEN
	oob := make([]byte, syscall.CmsgSpace(4*4))
	for {
		left := time.Until(deadline)
		if left <= 0 {
			return -1, false, fmt.Errorf("no listener within %s", s.tm.Handover)
		}
		tv := syscall.NsecToTimeval(left.Nanoseconds())
		if err := syscall.SetsockoptTimeval(s.parent, syscall.SOL_SOCKET, syscall.SO_RCVTIMEO, &tv); err != nil {
			return -1, false, fmt.Errorf("SO_RCVTIMEO: %w", err)
		}
		n, oobn, flags, _, err := syscall.Recvmsg(s.parent, data, oob, 0x40000000 /* MSG_CMSG_CLOEXEC */)
		if err == syscall.EINTR || err == syscall.EAGAIN {
			continue
		}
		if err != nil {
			return -1, false, fmt.Errorf("receiving the listener: %w", err)
		}
		if n == 0 && oobn == 0 {
			return -1, false, errors.New("the socket closed without the listener (the sandbox did not get as far as the hand-over)")
		}
		fds, perr := parseRights(oob[:oobn])
		closeAll := func() {
			for _, fd := range fds {
				syscall.Close(fd)
			}
		}
		switch {
		case perr != nil:
			closeAll()
			return -1, false, perr
		case flags&0x8 != 0: // MSG_CTRUNC
			closeAll()
			return -1, false, errors.New("the control message was truncated: more descriptors than one")
		case len(fds) != 1:
			closeAll()
			return -1, false, fmt.Errorf("%d descriptors arrived, want exactly 1", len(fds))
		case n != 1 || (data[0] != 'L' && data[0] != 'K'):
			closeAll()
			return -1, false, fmt.Errorf("%d data bytes arrived (first %#x), want exactly one 'L' or 'K'", n, data[0])
		}
		// Nothing else may be queued behind it: a second message is a protocol
		// violation.  Non-blocking; EAGAIN (nothing) and end of file are fine.
		var extra [8]byte
		eoob := make([]byte, syscall.CmsgSpace(4*4))
		en, eoobn, _, _, eerr := syscall.Recvmsg(s.parent, extra[:], eoob, syscall.MSG_DONTWAIT|0x40000000)
		if eerr == nil && (en > 0 || eoobn > 0) {
			if more, _ := parseRights(eoob[:eoobn]); len(more) > 0 {
				for _, fd := range more {
					syscall.Close(fd)
				}
			}
			syscall.Close(fds[0])
			return -1, false, errors.New("more than the one message arrived")
		}
		return fds[0], data[0] == 'K', nil
	}
}

// parseRights returns every descriptor in the control messages.
func parseRights(oob []byte) ([]int, error) {
	if len(oob) == 0 {
		return nil, nil
	}
	msgs, err := syscall.ParseSocketControlMessage(oob)
	if err != nil {
		return nil, fmt.Errorf("control message: %w", err)
	}
	var fds []int
	for i := range msgs {
		r, err := syscall.ParseUnixRights(&msgs[i])
		if err != nil {
			continue
		}
		fds = append(fds, r...)
	}
	return fds, nil
}

// receiveLoop is the loop of SECCOMP_IOCTL_NOTIF_RECV.  It returns how it ended.
func (s *Supervisor) receiveLoop(lfd int) int32 {
	for {
		if s.aborted.Load() {
			return rAborted
		}
		if s.ending.Load() {
			return rStopped
		}
		rev, err := pollOne(lfd, 50)
		if err != nil {
			if s.aborted.Load() {
				return rAborted
			}
			s.degrade("polling the listener: " + err.Error())
			return rError
		}
		if rev&pollIn == 0 {
			if rev&(pollErr|pollHup|pollNval) != 0 {
				if s.aborted.Load() {
					return rAborted
				}
				// POLLHUP without POLLIN: every task that had the filter is gone.
				return rHangup
			}
			continue
		}
		var n seccompNotif
		if err := notifRecv(lfd, &n); err != nil {
			switch err {
			case syscall.ENOENT, syscall.EAGAIN:
				continue // the notification vanished between the poll and the receive
			case syscall.ENOTTY:
				return rHangup // the filter has no user left
			}
			if s.aborted.Load() {
				return rAborted
			}
			s.degrade("NOTIF_RECV: " + err.Error())
			return rError
		}
		s.rCount++
		s.notified.Add(1)
		if k := s.tm.InjectReceiverError; k > 0 && s.rCount == k {
			// A test seam: the receiver fails as if NOTIF_RECV had.
			s.answer(n.ID)
			s.degrade("injected receiver error")
			return rError
		}
		if !s.inb.push(event{kind: evNotif, n: n, at: time.Now()}, s.tm.MaxInbox) {
			// Never stop answering: past the inbox cap the notification is answered
			// without being looked at, and the step is unclean.
			s.mu.Lock()
			s.overloadAll = true
			s.markLocked(UncleanOverflow, "the inbox is full")
			s.mu.Unlock()
			s.answer(n.ID)
		}
	}
}

// ---------------------------------------------------------------- the inbox

type eventKind int

const (
	evNotif    eventKind = iota // a notification from the receiver
	evReleased                  // a held notification whose turn has come
	evTaint                     // taint this group now (a held queue reached the deadline)
)

type event struct {
	kind eventKind
	n    seccompNotif
	g    *group
	at   time.Time
}

// inbox is an unbounded-until-capped FIFO with push-to-front, for the processor.
type inbox struct {
	mu   sync.Mutex
	cond *sync.Cond
	q    []event
	// cur is the event the processor has taken and not finished with (pop sets it
	// under mu, done clears it), so "waiting or being processed" is ONE fact read
	// under one lock: there is no instant at which a notification is in neither.
	cur    *event
	closed bool
}

func (b *inbox) init() { b.cond = sync.NewCond(&b.mu) }

// push appends; false when the cap is reached (the caller answers it itself).
func (b *inbox) push(e event, max int) bool {
	b.mu.Lock()
	defer b.mu.Unlock()
	if len(b.q) >= max {
		return false
	}
	b.q = append(b.q, e)
	b.cond.Signal()
	return true
}

// pushFront puts evs, in order, ahead of everything waiting: a held
// notification is older than anything that arrived after it.
func (b *inbox) pushFront(evs []event) {
	if len(evs) == 0 {
		return
	}
	b.mu.Lock()
	b.q = append(append([]event(nil), evs...), b.q...)
	b.cond.Signal()
	b.mu.Unlock()
}

// pop blocks for the next event; false once closed and empty.
func (b *inbox) pop() (event, bool) {
	b.mu.Lock()
	defer b.mu.Unlock()
	for len(b.q) == 0 {
		if b.closed {
			return event{}, false
		}
		b.cond.Wait()
	}
	e := b.q[0]
	b.q = b.q[1:]
	b.cur = &e
	return e, true
}

// done says the processor has finished the event pop gave it.
func (b *inbox) done() {
	b.mu.Lock()
	b.cur = nil
	b.mu.Unlock()
}

// holds says a notification from thread tid is waiting in the inbox or is being
// processed right now (a taint event carries no notification).  The pending-
// lifetime bound asks it: such a notification is about to SETTLE the thread's
// pending exec (the same-thread rule of D48 step 1), so the exec is not stuck.
func (b *inbox) holds(tid uint32) bool {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.cur != nil && b.cur.kind != evTaint && b.cur.n.Pid == tid {
		return true
	}
	for _, e := range b.q {
		if e.kind != evTaint && e.n.Pid == tid {
			return true
		}
	}
	return false
}

func (b *inbox) close() {
	b.mu.Lock()
	b.closed = true
	b.cond.Broadcast()
	b.mu.Unlock()
}

// oldest is when the oldest waiting event arrived (zero: none waiting).
func (b *inbox) oldest() time.Time {
	b.mu.Lock()
	defer b.mu.Unlock()
	for _, e := range b.q {
		if e.kind == evNotif {
			return e.at
		}
	}
	return time.Time{}
}

// ---------------------------------------------------------------- the processor

// runProcessor takes the inbox one item at a time.
func (s *Supervisor) runProcessor() {
	defer s.closeP()
	for {
		ev, ok := s.inb.pop()
		if !ok {
			return
		}
		s.procStart.Store(time.Now().UnixNano())
		s.handleEvent(ev)
		s.procStart.Store(0)
		s.inb.done()
	}
}

// handleEvent runs one event, turning a panic into a degraded supervisor: the
// step goes on being answered, and is unclean.
func (s *Supervisor) handleEvent(ev event) {
	defer func() {
		if r := recover(); r != nil {
			s.degrade(fmt.Sprintf("the processor panicked: %v", r))
			if ev.kind != evTaint {
				s.answer(ev.n.ID)
			}
		}
	}()
	if d := s.tm.ProcessDelay; d > 0 && ev.kind != evTaint {
		time.Sleep(d) // a test seam
	}
	s.mu.Lock()
	degraded := s.degraded
	s.mu.Unlock()
	if degraded {
		if ev.kind != evTaint {
			s.answer(ev.n.ID)
		}
		return
	}
	switch ev.kind {
	case evNotif, evReleased:
		s.process(ev.n)
	case evTaint:
		s.forceTaint(ev.g)
	}
}

// answer sends CONTINUE for id.  ENOENT is fine (the notification vanished);
// any other error makes the supervisor failed.
func (s *Supervisor) answer(id uint64) {
	s.lmu.RLock()
	lfd := int(s.lfd.Load())
	var err error
	if lfd >= 0 {
		err = notifContinue(lfd, id)
	}
	s.lmu.RUnlock()
	if lfd >= 0 && err != nil && err != syscall.ENOENT {
		s.fail("NOTIF_SEND: " + err.Error())
	}
}

// closeListener closes the listener once, excluding every user.
func (s *Supervisor) closeListener() {
	s.lmu.Lock()
	if fd := int(s.lfd.Swap(-1)); fd >= 0 {
		syscall.Close(fd)
	}
	s.lmu.Unlock()
}

// valid is notifValid with the failure handling: an error other than ENOENT
// marks the supervisor failed and counts as "valid" (answer it).
func (s *Supervisor) valid(id uint64) bool {
	s.lmu.RLock()
	lfd := int(s.lfd.Load())
	if lfd < 0 {
		s.lmu.RUnlock()
		return false
	}
	ok, err := notifValid(lfd, id)
	s.lmu.RUnlock()
	if err != nil {
		s.fail("NOTIF_ID_VALID: " + err.Error())
		return true
	}
	return ok
}

// fail records a supervisor-side error: the step is unclean.
func (s *Supervisor) fail(msg string) {
	s.mu.Lock()
	s.markLocked(UncleanFailed, msg)
	s.mu.Unlock()
}

// markLocked adds an unclean reason once, and keeps the first note per reason.
func (s *Supervisor) markLocked(reason, note string) {
	s.umu.Lock()
	defer s.umu.Unlock()
	for _, r := range s.unclean {
		if r == reason {
			if note != "" && len(s.notes) < 16 {
				s.notes = append(s.notes, reason+": "+note)
			}
			return
		}
	}
	s.unclean = append(s.unclean, reason)
	if note != "" && len(s.notes) < 16 {
		s.notes = append(s.notes, reason+": "+note)
	}
}

// degrade makes the supervisor failed for good: the step is unclean, nothing more
// is recorded, every notification (held ones included) is answered CONTINUE so
// the build is not hung, and the step carries on.
func (s *Supervisor) degrade(msg string) {
	s.mu.Lock()
	s.markLocked(UncleanFailed, msg)
	s.degraded = true
	s.writing = false
	var held []seccompNotif
	for _, g := range s.groups {
		held = append(held, g.queue...)
		g.queue = nil
	}
	s.cond.Broadcast()
	s.mu.Unlock()
	for _, n := range held {
		s.answer(n.ID)
	}
}

// supervisorDied is the response to a panic outside the receiver (a poller, the
// watchdog): the supervisor degrades to answer-only, the step is unclean and it is
// killed, because something that was meant to watch it has stopped.
func (s *Supervisor) supervisorDied(why string) {
	if !s.ending.Load() {
		s.killStep() // first: degrade takes mu
	}
	s.degrade(why)
}

// killStep ends the step: the pidfd of the sandbox's init when we have one (its
// death kills every process of the pid namespace), then strace (whose
// --kill-on-exit takes every tracee).  Never a pid number.
func (s *Supervisor) killStep() {
	// No s.mu: it can be held across a blocking write of the sidecar.
	s.kills.Add(1)
	if fd := int(s.initPidfd.Load()); fd >= 0 {
		pidfdKill(fd)
	}
	if s.killStrace != nil {
		s.killStrace()
	}
}

// writeLocked writes one line.  A cap ends the writing (the step is unclean,
// notifications are still answered); a write error does too.
func (s *Supervisor) writeLocked(l Line) {
	if !s.writing || s.w == nil {
		return
	}
	err := s.w.Append(l)
	switch {
	case err == nil:
	case errors.Is(err, ErrOverflow):
		s.writing = false
		s.markLocked(UncleanOverflow, "the sidecar's line or byte cap")
	default:
		s.writing = false
		s.markLocked(UncleanFailed, err.Error())
	}
}

// ---------------------------------------------------------------- one notification

// process is "steps 1 to 6" of D48 for one notification.
func (s *Supervisor) process(n seccompNotif) {
	tid := int(n.Pid)
	// Step 1: the thread-group id, read now (the thread may be gone later), and
	// NOTIF_ID_VALID immediately after, before anything else uses the number.
	tgid, err := readTgid(tid)
	if err != nil {
		if !s.valid(n.ID) {
			s.dropped.Add(1)
		} else {
			// The thread is alive but its status cannot be read: not a vanished
			// notification, so a supervisor-side failure.  Answer it.
			s.fail(fmt.Sprintf("reading the thread group of %d: %v", tid, err))
			s.answer(n.ID)
		}
		return
	}
	if !s.valid(n.ID) {
		s.dropped.Add(1)
		return // killed or interrupted since the RECV: it never ran, no taint
	}
	first := s.initBound.CompareAndSwap(false, true)
	g, ok := s.groupFor(tgid, n.ID, first, tid)
	if !ok {
		return
	}
	s.decide(n, g, tid, tgid)
}

// groupFor finds or creates the thread group's record.  A group whose pidfd is
// readable has exited and its number may be reused: that record is dropped.  The
// pidfd is taken only while the notification is still valid, so a tgid read from
// a recycled pid can never be used.  false: the notification was dropped, or
// already answered.
func (s *Supervisor) groupFor(tgid int, id uint64, first bool, tid int) (*group, bool) {
	s.mu.Lock()
	g := s.groups[tgid]
	s.mu.Unlock()
	if g != nil {
		if has, exited := s.pidfdState(g); !has || !exited {
			return g, true
		}
	}
	if g != nil {
		// The process of that number has exited, so this is a new one.  If the old
		// one still has a poller it is about to say `gone` and drop the entry
		// itself; wait for that rather than close a pidfd the poller is using.
		s.mu.Lock()
		for g.last != nil && g.last.poller && g.last.state != stateWritten && !s.degraded {
			s.cond.Wait()
		}
		if s.groups[tgid] == g {
			s.dropGroupLocked(g)
		}
		s.mu.Unlock()
	}
	pfd, perr := pidfdOpen(tgid)
	if perr == nil {
		// Valid AFTER the pidfd too: a notification valid now means its thread
		// has been alive throughout, so the pidfd names that very process.
		if !s.valid(id) {
			syscall.Close(pfd)
			return nil, false
		}
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if len(s.groups) >= s.tm.MaxGroups {
		s.sweepLocked()
		if len(s.groups) >= s.tm.MaxGroups {
			// The table is full.  An evicted entry would un-taint its group, so
			// instead the overload-all taint is set: every later line of every
			// group is `tainted`.  The notification is answered without a pin.
			s.overloadAll = true
			s.markLocked(UncleanOverflow, "the thread-group table is full")
			if perr == nil {
				syscall.Close(pfd)
			}
			s.mu.Unlock()
			s.answer(id)
			s.mu.Lock()
			return nil, false
		}
	}
	if perr != nil {
		// No pidfd (descriptor exhaustion, or the process is gone).  If the
		// notification is still valid this is an overload: record the group
		// pidfd-less and tainted, before any answer.
		s.mu.Unlock()
		stillValid := s.valid(id)
		s.mu.Lock()
		if !stillValid {
			return nil, false
		}
		g = &group{tgid: tgid, pidfd: -1, tainted: true}
		s.groups[tgid] = g
		return g, true
	}
	g = &group{tgid: tgid, pidfd: pfd}
	s.groups[tgid] = g
	if first {
		// The FIRST notification is the init's own execve of the step: a DUP of the
		// pidfd it was just validated with (not a second pidfd_open of the number) is
		// the kill target, bound only if the process really is its pid namespace's
		// init (the last NSpid entry is 1).  Any other process is never bound.
		if lastNSpid(tid) == 1 {
			if ipfd := dupCloexec(pfd); ipfd >= 0 {
				s.initPidfd.Store(int32(ipfd))
			}
		} else {
			s.noteLocked("the first notification is not from a pid-namespace init: no init pidfd, the kill is strace's")
		}
	}
	return g, true
}

// pidfdState says whether the group has a pidfd and whether it has exited.  The
// descriptor is DUPLICATED under mu and polled outside it, so a concurrent
// dropGroupLocked (which closes the original) can never make this poll a closed
// or reused descriptor number.
func (s *Supervisor) pidfdState(g *group) (has, exited bool) {
	s.mu.Lock()
	fd := -1
	if g.pidfd >= 0 {
		fd = dupCloexec(g.pidfd)
	}
	s.mu.Unlock()
	if fd < 0 {
		return false, false
	}
	defer syscall.Close(fd)
	return true, pidfdExited(fd)
}

// groupExited says the group's process has exited (false when it has no pidfd).
func (s *Supervisor) groupExited(g *group) bool {
	_, e := s.pidfdState(g)
	return e
}

// dropGroupLocked forgets a group (it has exited): its pidfd is closed, and any
// held notification is handed back to the processor, which will find it invalid.
func (s *Supervisor) dropGroupLocked(g *group) {
	if s.groups[g.tgid] == g {
		delete(s.groups, g.tgid)
	}
	if g.pidfd >= 0 {
		syscall.Close(g.pidfd)
	}
	g.pidfd = -1
	s.flushQueueLocked(g)
}

// sweepLocked drops groups that have exited and have nothing pending.
func (s *Supervisor) sweepLocked() {
	for _, g := range s.groups {
		if g.pidfd < 0 || !pidfdExited(g.pidfd) {
			continue
		}
		if g.last != nil && g.last.poller && g.last.state != stateWritten {
			continue // its poller will see `gone` and drop it
		}
		s.dropGroupLocked(g)
	}
}

// flushQueueLocked hands the group's held notifications back to the processor,
// ahead of anything newer, in their order.
func (s *Supervisor) flushQueueLocked(g *group) {
	if len(g.queue) == 0 {
		return
	}
	evs := make([]event, len(g.queue))
	for i, n := range g.queue {
		evs[i] = event{kind: evReleased, n: n, at: time.Now()}
	}
	g.queue = nil
	s.inb.pushFront(evs)
}

// mode is what decide concluded about how to treat a new notification.
type mode int

const (
	modeNormal mode = iota
	modeTainted
	modeOverload
)

// decide applies the per-group ordering rules (D48 step 1) and then handles the
// notification.
func (s *Supervisor) decide(n seccompNotif, g *group, tid, tgid int) {
	s.mu.Lock()
	m := modeNormal
	var settle *exec    // a pending pinned exec of the same thread to settle now
	var failedChk *exec // a remembered `failed` of the same thread to re-check
	var drained []seccompNotif
	switch last := g.last; {
	case s.overloadAll || g.tainted:
		m = modeTainted
	case last != nil && last.tid == tid:
		// SAME thread: the earlier exec is over (this thread cannot be in two
		// execve calls).  Never taints.  Bypasses the queue.
		switch {
		case last.kind == kFailed:
			failedChk = last
		case last.poller && last.state == statePending:
			last.state = stateClaimed
			settle = last
		}
	case last != nil && last.kind == kUndecided && last.tid != tid:
		// DIFFERENT thread: an overlap is possible.
		if last.poller && (last.state == stateClaimed || (last.state == statePending && !last.past)) {
			// The pending exec is still within its deadline (or its poller is deciding
			// it this very moment): hold this one.
			if len(g.queue) >= s.tm.MaxGroupQueue {
				m = modeOverload
				g.tainted = true // set BEFORE the CONTINUE of the overload
			} else {
				g.queue = append(g.queue, n)
				s.mu.Unlock()
				return
			}
		} else {
			// A pollerless exec, or a pinned one past its deadline: taint.
			g.tainted = true
			m = modeTainted
			drained = s.finalizeTimeoutLocked(g)
		}
	}
	s.mu.Unlock()

	// Anything that was held behind the exec that the taint just finalised is
	// answered `tainted` BEFORE this notification, which is younger.
	for _, q := range drained {
		s.decideTainted(q, g)
	}

	if settle != nil {
		st, head := s.settle(settle)
		s.finish(settle, st, head)
	} else if failedChk != nil {
		// A `failed` is never believed: if exe changed in between, the first exec
		// was still in flight after all, and this line is `tainted`.
		cur, err := statExe(tgid)
		if err == nil && cur != failedChk.pre {
			s.mu.Lock()
			g.tainted = true
			s.mu.Unlock()
			m = modeTainted
		}
	}
	// A poller that had claimed the exec (state claimed) when we looked: wait for
	// it to be written so this notification is never answered ahead of the line.
	s.mu.Lock()
	if g.last != nil && g.last.tid == tid && g.last.poller {
		for g.last != nil && g.last.state == stateClaimed && !s.degraded {
			s.cond.Wait()
		}
	}
	if m == modeNormal && (s.overloadAll || g.tainted) {
		m = modeTainted
	}
	s.mu.Unlock()

	switch m {
	case modeTainted:
		s.handleReleased(n, g, tid, tgid, StatusTainted)
	case modeOverload:
		s.handleReleased(n, g, tid, tgid, StatusOverload)
	default:
		s.handleNormal(n, g, tid, tgid)
	}
}

// decideTainted answers a held notification of a group that has just been
// tainted: a `tainted` line, CONTINUE, never confirmed.
func (s *Supervisor) decideTainted(n seccompNotif, g *group) {
	tid := int(n.Pid)
	if !s.valid(n.ID) {
		s.dropped.Add(1)
		return // its thread died in de_thread, say: it never ran, no taint
	}
	s.handleReleased(n, g, tid, g.tgid, StatusTainted)
}

// finalizeTimeoutLocked is the taint's finalisation: the group's pending pinned
// exec is written `timeout` (never confirmed), and the held queue is returned to
// be answered `tainted`.  A claimed exec is left to its claimant.
func (s *Supervisor) finalizeTimeoutLocked(g *group) []seccompNotif {
	if ex := g.last; ex != nil && ex.poller && ex.state == statePending {
		ex.state = stateClaimed
		s.finishLocked(ex, StatusTimeout, nil, true)
	}
	q := g.queue
	g.queue = nil
	return q
}

// forceTaint is the event a poller raises when the confirm deadline passes with
// notifications held: the group is tainted, the pending exec is finalised
// `timeout`, and every held notification is answered `tainted`.  The queue is
// not left holding.
func (s *Supervisor) forceTaint(g *group) {
	s.mu.Lock()
	// Only if the exec is still pending with notifications held: if it was decided
	// meanwhile its poller released the queue and there is nothing to taint, and
	// if its poller is deciding it right now that poller will release the queue.
	last := g.last
	if len(g.queue) == 0 || last == nil || !last.poller || last.state != statePending {
		s.mu.Unlock()
		return
	}
	g.tainted = true
	q := s.finalizeTimeoutLocked(g)
	s.mu.Unlock()
	for _, n := range q {
		s.decideTainted(n, g)
	}
}

// ---------------------------------------------------------------- reading the request

// request is what the notification asked for.
type request struct {
	path     string
	status   Status // "" or Unpinned or Fexecve: a refusal decided from the request itself
	nofollow bool
	dirfd    int32 // execveat's, as the kernel's int
	execveat bool
}

func (r request) syscallName() string {
	if r.execveat {
		return SyscallExecveat
	}
	return SyscallExecve
}

// readRequest reads the path (step 2).  For execveat the dirfd is compared as a
// 32-bit int (the kernel ignores the high bits), AT_FDCWD means the cwd, and
// AT_SYMLINK_NOFOLLOW is noted.
func readRequest(n seccompNotif, tid int) request {
	r := request{}
	addr := n.Data.Args[0]
	if n.Data.Nr == 322 {
		r.execveat = true
		r.dirfd = int32(n.Data.Args[0])
		addr = n.Data.Args[1]
		flags := int32(n.Data.Args[4])
		r.nofollow = flags&atSymlinkNofollow != 0
		path, res := readCString(tid, addr)
		if res == pathOK && path == "" && flags&atEmptyPath != 0 {
			r.status = StatusFexecve
			return r
		}
		return finishRequest(r, path, res)
	}
	path, res := readCString(tid, addr)
	return finishRequest(r, path, res)
}

func finishRequest(r request, path string, res pathReadResult) request {
	switch {
	case res != pathOK:
		r.status = StatusUnpinned
	case path == "" || !utf8.ValidString(path):
		// An empty path is ENOENT; bytes that are not UTF-8 would be mangled by
		// JSON.  Both are `unpinned`, with the path left empty.
		r.status = StatusUnpinned
	default:
		r.path = path
	}
	return r
}

// handleReleased records a notification with a status that has no pin and no
// poller (outside, unpinned, fexecve, same-image, overload, tainted) and answers
// it.  `tainted` and `overload` taint the group BEFORE the answer.
func (s *Supervisor) handleReleased(n seccompNotif, g *group, tid, tgid int, st Status) {
	req := readRequest(n, tid)
	if st == StatusTainted || st == StatusOverload {
		s.mu.Lock()
		g.tainted = true
		s.mu.Unlock()
	}
	s.release(n, g, tid, tgid, req, st)
}

// release writes the line of a notification that has no poller, remembers its
// thread as the group's last undecided exec, answers CONTINUE and lets the group's
// held notifications go.  If the notification has stopped being valid since the
// start of the processing, nothing is written.
func (s *Supervisor) release(n seccompNotif, g *group, tid, tgid int, req request, st Status) {
	if !s.valid(n.ID) {
		s.dropped.Add(1)
		return
	}
	line := Line{Tid: tid, Tgid: tgid, Syscall: req.syscallName(), Path: req.path, Status: st}
	s.mu.Lock()
	s.writeLocked(line)
	g.last = &exec{g: g, kind: kUndecided, tid: tid, tgid: tgid, nr: n.Data.Nr, path: req.path, pin: -1, state: stateWritten}
	s.flushQueueLocked(g)
	s.mu.Unlock()
	s.answer(n.ID)
}

// handleNormal is steps 2 to 6 for an untainted group.
func (s *Supervisor) handleNormal(n seccompNotif, g *group, tid, tgid int) {
	req := readRequest(n, tid)
	if req.status != "" {
		s.release(n, g, tid, tgid, req, req.status)
		return
	}
	pin, st := s.resolveAndPin(tid, tgid, req)
	if st != "" {
		s.release(n, g, tid, tgid, req, st)
		return
	}
	// pin is a regular file on the step's tmpfs.
	pre, err := statExe(tgid)
	if err != nil {
		syscall.Close(pin.fd)
		if s.groupExited(g) {
			s.release(n, g, tid, tgid, req, StatusGone)
		} else {
			s.release(n, g, tid, tgid, req, StatusUnpinned)
		}
		return
	}
	if pre == pin.id {
		// The process already runs the very file it is about to exec: not
		// observable as a transition.
		syscall.Close(pin.fd)
		s.release(n, g, tid, tgid, req, StatusSameImage)
		return
	}
	now := time.Now()
	ex := &exec{
		g: g, kind: kUndecided, tid: tid, tgid: tgid, nr: n.Data.Nr, path: req.path,
		pin: pin.fd, pinID: pin.id, pre: pre, poller: true, state: statePending,
		cont: now, deadline: now.Add(s.tm.Confirm), slot: true,
	}
	s.mu.Lock()
	if s.slots >= s.tm.MaxConfirms || g.pidfd < 0 {
		// The cap on pending confirms: nothing is pinned, the group is tainted
		// (the answered exec overlaps whatever is pending), and the taint is set
		// BEFORE the CONTINUE.
		g.tainted = true
		s.mu.Unlock()
		syscall.Close(pin.fd)
		s.release(n, g, tid, tgid, req, StatusOverload)
		return
	}
	s.mu.Unlock()
	if !s.valid(n.ID) {
		s.dropped.Add(1)
		syscall.Close(pin.fd)
		return
	}
	s.mu.Lock()
	s.slots++
	s.active[ex] = struct{}{}
	g.last = ex
	s.wg.Add(1)
	s.mu.Unlock()
	// CONTINUE first, then the confirm starts: a confirm's wait never counts
	// against the stall deadline.
	s.answer(n.ID)
	go s.poll(ex)
}

// pinned is a file held open by the supervisor.
type pinned struct {
	fd int
	id ident
}

// resolveAndPin is steps 3 to 5: resolve the path inside the step's root, decide
// whether it is on the step's tmpfs, and pin it.  A non-empty status means no pin
// was made (outside or unpinned); the descriptor is closed.
func (s *Supervisor) resolveAndPin(tid, tgid int, req request) (pinned, Status) {
	flags := uint64(oPath | syscall.O_CLOEXEC)
	if req.nofollow {
		flags |= syscall.O_NOFOLLOW
	}
	var fd int
	var err error
	if req.path[0] == '/' {
		// ABSOLUTE: resolved against the STEP's root, symlinks included.  A plain
		// "/proc/<pid>/root/<path>" string would resolve an absolute symlink in the
		// SUPERVISOR's root.
		var root int
		root, err = openPathDir("/proc/" + strconv.Itoa(tgid) + "/root")
		if err != nil {
			return pinned{}, StatusUnpinned
		}
		fd, err = openat2(root, req.path, flags, resolveInRoot|resolveNoMagiclinks)
		syscall.Close(root)
	} else {
		// RELATIVE: the base is the cwd or, for execveat, the directory descriptor,
		// opened as the link's real target; then RESOLVE_BENEATH refuses a ".." that
		// climbs above it and an absolute symlink.  A relative path that leaves its
		// base is `unpinned`, conservative and never resolved with a wrong root.
		base := "/proc/" + strconv.Itoa(tid) + "/cwd"
		if req.execveat && req.dirfd != atFdcwd {
			if req.dirfd < 0 {
				return pinned{}, StatusUnpinned
			}
			base = "/proc/" + strconv.Itoa(tid) + "/fd/" + strconv.Itoa(int(req.dirfd))
		}
		var bfd int
		bfd, err = openPathDir(base)
		if err != nil {
			return pinned{}, StatusUnpinned
		}
		fd, err = openat2(bfd, req.path, flags, resolveBeneath|resolveNoMagiclinks)
		syscall.Close(bfd)
	}
	if err != nil {
		return pinned{}, StatusUnpinned
	}
	var st syscall.Stat_t
	if err := syscall.Fstat(fd, &st); err != nil {
		syscall.Close(fd)
		return pinned{}, StatusUnpinned
	}
	// Step 4: is it on the step's fresh tmpfs?  A device that could not be
	// measured counts as "on the tmpfs".
	if !s.onTmpfs(tgid, uint64(st.Dev)) {
		syscall.Close(fd)
		return pinned{}, StatusOutside
	}
	if st.Mode&syscall.S_IFMT != syscall.S_IFREG {
		syscall.Close(fd) // a symlink under AT_SYMLINK_NOFOLLOW, a directory, a FIFO or a device
		return pinned{}, StatusUnpinned
	}
	// Step 5: PIN.  Re-open the very inode the O_PATH handle names, for reading;
	// O_NONBLOCK so a FIFO or a device node could not stall the supervisor (it is
	// a regular file here, but the pin takes no chances).
	rfd, err := syscall.Open("/proc/self/fd/"+strconv.Itoa(fd), syscall.O_RDONLY|syscall.O_NONBLOCK|syscall.O_CLOEXEC, 0)
	syscall.Close(fd)
	if err != nil {
		return pinned{}, StatusUnpinned
	}
	var rst syscall.Stat_t
	if err := syscall.Fstat(rfd, &rst); err != nil || rst.Mode&syscall.S_IFMT != syscall.S_IFREG || rst.Dev != st.Dev || rst.Ino != st.Ino {
		syscall.Close(rfd)
		return pinned{}, StatusUnpinned
	}
	return pinned{fd: rfd, id: ident{dev: uint64(rst.Dev), ino: rst.Ino}}, ""
}

// onTmpfs measures the step's tmpfs devices once (through /proc/<tgid>/root of
// the first process, whose root is the step's) and says whether dev is one.
func (s *Supervisor) onTmpfs(tgid int, dev uint64) bool {
	s.mu.Lock()
	done, unmeasured, hit := s.devsDone, s.unmeasured, s.devs[dev]
	s.mu.Unlock()
	if !done {
		devs, ok := measureDevices(tgid, s.cfg.Tmpfs)
		s.mu.Lock()
		if !s.devsDone {
			s.devs, s.unmeasured, s.devsDone = devs, !ok, true
		}
		unmeasured, hit = s.unmeasured, s.devs[dev]
		s.mu.Unlock()
	}
	// A device that could not be measured means the step's tmpfs is not known:
	// nothing is on it as far as the supervisor can say, so every file is
	// `outside` and none is opened for reading (fail closed; the classifier then
	// refuses an exec under the tmpfs as it does today).
	if unmeasured {
		s.mu.Lock()
		s.noteLocked("the step's tmpfs devices could not be measured: every exec is recorded `outside`")
		s.mu.Unlock()
		return false
	}
	return hit
}

// noteLocked keeps a diagnostic note (once).
func (s *Supervisor) noteLocked(note string) {
	s.umu.Lock()
	defer s.umu.Unlock()
	for _, n := range s.notes {
		if n == note {
			return
		}
	}
	if len(s.notes) < 16 {
		s.notes = append(s.notes, note)
	}
}

// measureDevices stats each of the step's tmpfs roots through the process's root.
// ok is false when any could not be measured.
func measureDevices(tgid int, roots []string) (map[uint64]bool, bool) {
	devs := map[uint64]bool{}
	ok := true
	root, err := openPathDir("/proc/" + strconv.Itoa(tgid) + "/root")
	if err != nil {
		return devs, false
	}
	defer syscall.Close(root)
	for _, r := range roots {
		fd, err := openat2(root, r, oPath|syscall.O_CLOEXEC|syscall.O_DIRECTORY, resolveInRoot|resolveNoMagiclinks)
		if err != nil {
			ok = false
			continue
		}
		var st syscall.Stat_t
		if err := syscall.Fstat(fd, &st); err != nil {
			ok = false
		} else {
			devs[uint64(st.Dev)] = true
		}
		syscall.Close(fd)
	}
	return devs, ok
}

// ---------------------------------------------------------------- the confirm

// obs is what one look at a pending exec showed.
type obs int

const (
	obsNone        obs = iota // nothing decided: keep polling
	obsGone                   // the process exited
	obsConfirmable            // exe changed from the pre-exec image to the pin
	obsMismatch               // exe changed to something else
	obsFailed                 // the thread left execve, exe is still the pre-exec image
	obsVanished               // the notified thread is gone and exe has not (yet) changed
)

// observe looks once.  The order is the point: /proc/<tid>/syscall FIRST, the exe
// link AFTER, the pidfd last.  exe changes before a successful exec returns to
// user mode, so a thread seen outside execve whose exe is still the old image has
// failed for good; the other order could read "old image" and then "outside" from
// an exec that finished in between.
func (s *Supervisor) observe(ex *exec) obs {
	sc := readSyscallState(ex.tid)
	cur, err := statExe(ex.tgid)
	if s.groupExited(ex.g) {
		return obsGone
	}
	if err != nil {
		return obsNone // not decided: with a live pidfd, poll again
	}
	switch {
	case cur == ex.pre:
		if sc == syscallOutsideExec {
			return obsFailed
		}
		if ex.tid != ex.tgid && tidGone(ex.tid) {
			// The notified thread has vanished (it is not the leader, so it is not
			// merely a zombie) and exe has not changed: a sibling's exec did de_thread
			// on it, or its own exec is about to change exe.  The poller re-checks.
			return obsVanished
		}
		return obsNone
	case cur == ex.pinID:
		return obsConfirmable
	default:
		return obsMismatch
	}
}

// vanishRecheck bounds the wait before a vanished thread with exe unchanged taints.
const vanishRecheck = 100 * time.Millisecond

// tidGone says /proc/<tid> no longer exists.
func tidGone(tid int) bool {
	_, err := os.Stat("/proc/" + strconv.Itoa(tid))
	return err != nil && os.IsNotExist(err)
}

// poll is the poller of one pinned exec: it watches until the exec is decided, or
// another goroutine settles it.
func (s *Supervisor) poll(ex *exec) {
	defer s.wg.Done()
	defer func() {
		if r := recover(); r != nil {
			s.supervisorDied(fmt.Sprintf("a poller panicked: %v", r))
		}
	}()
	g := ex.g
	delay := 50 * time.Microsecond
	if d := s.tm.PollStart; d > 0 {
		time.Sleep(d) // a test seam
	}
	for {
		s.mu.Lock()
		state := ex.state
		s.mu.Unlock()
		if state != statePending {
			return // settled or finalised by the processor
		}
		ending := s.ending.Load()
		o := s.observe(ex)
		if o == obsNone && ending {
			// The step is over and the exec never resolved: the supervisor writes
			// `timeout`, never a guess.
			if s.claim(ex) {
				s.finish(ex, StatusTimeout, nil)
			}
			return
		}
		if o == obsVanished {
			// D48 step 1: a vanished thread with exe unchanged is a taint, UNLESS it
			// was the thread's own exec (a non-leader's tid vanishes briefly before
			// exe changes).  Re-check for a short bounded time; only if exe is still
			// the pre-exec image does the group taint.
			changed := false
			for end := time.Now().Add(vanishRecheck); time.Now().Before(end); {
				time.Sleep(2 * time.Millisecond)
				if cur, err := statExe(ex.tgid); err == nil && cur != ex.pre {
					changed = true
					break
				}
				if s.groupExited(g) {
					changed = true // the group is gone: the next look says so
					break
				}
			}
			if changed {
				continue
			}
			s.mu.Lock()
			g.tainted = true
			s.mu.Unlock()
			if s.claim(ex) {
				s.finish(ex, StatusTainted, nil)
			}
			return
		}
		if o != obsNone {
			if !s.claim(ex) {
				return
			}
			switch o {
			case obsGone:
				s.finish(ex, StatusGone, nil)
			case obsMismatch:
				s.finish(ex, StatusMismatch, nil)
			case obsFailed:
				s.finish(ex, StatusFailed, nil)
			case obsConfirmable:
				st, head := s.confirmHead(ex)
				s.finish(ex, st, head)
			}
			return
		}
		if now := time.Now(); !ex.past && now.After(ex.deadline) {
			s.mu.Lock()
			if ex.state == statePending {
				ex.past = true
				if ex.slot {
					ex.slot = false
					s.slots--
				}
				if len(g.queue) > 0 {
					// Held notifications must not wait forever behind an exec that is
					// past its deadline: the group is tainted and they are answered.
					s.inb.pushFront([]event{{kind: evTaint, g: g, at: now}})
				}
			}
			s.mu.Unlock()
		}
		time.Sleep(delay)
		switch {
		case ex.past:
			delay = 50 * time.Millisecond
		case delay < 5*time.Millisecond:
			delay *= 2
		}
	}
}

// claim moves the exec from pending to claimed.  Whoever does it owns the line.
func (s *Supervisor) claim(ex *exec) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	if ex.state != statePending {
		return false
	}
	ex.state = stateClaimed
	return true
}

// confirmHead is step 8: read the head from the PINNED descriptor, then look at
// exe again.  The head counts only if the process still has that image.
func (s *Supervisor) confirmHead(ex *exec) (Status, []byte) {
	head, err := readLimited(ex.pin, make([]byte, HeadLimit))
	if err != nil {
		return StatusUnpinned, nil
	}
	cur, err := statExe(ex.tgid)
	if err != nil {
		if s.groupExited(ex.g) {
			return StatusGone, nil
		}
		return StatusMismatch, nil
	}
	if cur != ex.pinID {
		return StatusMismatch, nil
	}
	return StatusConfirmed, head
}

// settle concludes a claimed pending exec at the next notification of the same
// thread.  The thread is held in that notification, so the image cannot change
// under these reads.
func (s *Supervisor) settle(ex *exec) (Status, []byte) {
	cur, err := statExe(ex.tgid)
	switch {
	case err != nil:
		if s.groupExited(ex.g) {
			return StatusGone, nil
		}
		return StatusTimeout, nil
	case cur == ex.pre:
		return StatusFailed, nil
	case cur == ex.pinID:
		return s.confirmHead(ex)
	default:
		return StatusMismatch, nil
	}
}

// finish writes the claimed exec's line and does what the status asks of the
// group.
func (s *Supervisor) finish(ex *exec, st Status, head []byte) {
	s.mu.Lock()
	s.finishLocked(ex, st, head, false)
	s.mu.Unlock()
}

// finishLocked writes the line of an exec the caller has claimed, under mu: the
// taint is read HERE, after the final exe re-check and in the same critical
// section as the write, so a line is never written `confirmed` after a taint that
// an overlapping exec could have caused.
func (s *Supervisor) finishLocked(ex *exec, st Status, head []byte, byTaint bool) {
	g := ex.g
	if st == StatusConfirmed && (g.tainted || s.overloadAll) {
		st, head = StatusTainted, nil
	}
	line := Line{Tid: ex.tid, Tgid: ex.tgid, Syscall: SyscallExecve, Path: ex.path, Status: st}
	if ex.nr == 322 {
		line.Syscall = SyscallExecveat
	}
	if st == StatusConfirmed {
		sum := sha256.Sum256(head)
		line.Dev, line.Ino, line.Head, line.SHA256 = ex.pinID.dev, ex.pinID.ino, head, hex.EncodeToString(sum[:])
		if line.Head == nil {
			line.Head = []byte{}
		}
	}
	s.writeLocked(line)
	ex.state = stateWritten
	if ex.pin >= 0 {
		syscall.Close(ex.pin)
		ex.pin = -1
	}
	if ex.slot {
		ex.slot = false
		s.slots--
	}
	delete(s.active, ex)
	switch st {
	case StatusConfirmed, StatusMismatch:
		if g.last == ex {
			g.last = nil
		}
	case StatusFailed:
		if g.last == ex {
			g.last = &exec{g: g, kind: kFailed, tid: ex.tid, tgid: ex.tgid, pin: -1, pre: ex.pre, state: stateWritten}
		}
	case StatusGone:
		s.dropGroupLocked(g)
	}
	if !byTaint {
		s.flushQueueLocked(g)
	}
	s.cond.Broadcast()
}

// ---------------------------------------------------------------- the watchdog

// runWatchdog enforces the stall deadline and the pending-lifetime bound, and
// sweeps groups that have exited.
func (s *Supervisor) runWatchdog() {
	defer close(s.wDone)
	defer func() {
		if r := recover(); r != nil {
			s.supervisorDied(fmt.Sprintf("the watchdog panicked: %v", r))
		}
	}()
	tick := time.NewTicker(25 * time.Millisecond)
	defer tick.Stop()
	sweep := 0
	for {
		select {
		case <-s.stopW:
			return
		case <-tick.C:
		}
		if s.ending.Load() {
			continue
		}
		now := time.Now()
		// The stall deadline: the processor has had one notification too long, or
		// one has waited in the inbox too long.  A held notification (in a group
		// queue) is not a stall: it is held by design and by the lifetime bound.
		// Neither check, nor the kill, takes mu.
		if ps := s.procStart.Load(); ps != 0 && now.Sub(time.Unix(0, ps)) > s.tm.Stall {
			s.stalled("the processor has held one notification for over " + s.tm.Stall.String())
			return
		}
		if old := s.inb.oldest(); !old.IsZero() && now.Sub(old) > s.tm.Stall {
			s.stalled("a notification waited in the inbox for over " + s.tm.Stall.String())
			return
		}
		// The absolute bound on a pending exec's life, from its CONTINUE.  mu is only
		// TRIED: if a blocked write holds it, the stall check above still works.  If
		// the lock stays unavailable for longer than the bound itself, the check is
		// being starved: that is a stuck step.
		if s.mu.TryLock() {
			s.stale = 0
			var cand *exec
			for ex := range s.active {
				if ex.state == statePending && now.Sub(ex.cont) > s.tm.Lifetime {
					cand = ex
					break
				}
			}
			s.mu.Unlock()
			if cand != nil && s.sameTidArriving(cand.tid) {
				// The false kill D48's step-4 review found: the pending exec's thread
				// has made a NEW execve, and that notification is in the inbox or being
				// processed.  The thread reads as blocked in execve because it IS (in
				// the new call), but the same-thread rule settles the old exec at
				// receipt, so nothing is stuck.  Skip the candidate this tick (neither
				// kill nor taint); if the processor really is stuck the stall
				// deadline says so.
				cand = nil
			}
			if cand != nil {
				// Kill only a thread seen BLOCKED in execve/execveat: that is a stuck
				// step.  Anything else (a failed in-place exec can leave its thread
				// computing, which reads `running`) is a healthy step whose exec nobody
				// observed: the group is tainted and the line is `timeout`.  Either way
				// the candidate is re-checked under the lock first: it may have been
				// decided since, and a stale candidate must not kill anything.
				inExec := readSyscallState(cand.tid) == syscallInExec
				if inExec {
					// The new notification may be between the kernel and the inbox
					// (received, not yet pushed): give it a moment, then ask again,
					// before a kill that cannot be taken back.
					time.Sleep(sameTidGrace)
					if s.sameTidArriving(cand.tid) {
						continue
					}
					inExec = readSyscallState(cand.tid) == syscallInExec
				}
				if s.mu.TryLock() {
					if cand.state == statePending {
						if inExec {
							s.mu.Unlock()
							s.markLocked(UncleanStuck, fmt.Sprintf("exec by thread %d blocked in execve for over %s", cand.tid, s.tm.Lifetime))
							s.killStep()
							return
						}
						cand.state = stateClaimed
						cand.g.tainted = true
						s.finishLocked(cand, StatusTimeout, nil, false)
					}
					s.mu.Unlock()
				}
			}
		} else if s.stale++; time.Duration(s.stale)*25*time.Millisecond > s.tm.Lifetime {
			s.markLocked(UncleanStuck, "the supervisor's lock was unavailable for longer than the lifetime bound")
			s.killStep()
			return
		}
		if sweep++; sweep%8 == 0 && s.mu.TryLock() {
			s.sweepLocked()
			s.mu.Unlock()
		}
	}
}

// sameTidGrace is how long the lifetime bound waits, before a kill, for a
// notification of the candidate's own thread to appear in the inbox.
const sameTidGrace = 50 * time.Millisecond

// sameTidArriving says a notification from thread tid is in the inbox or being
// processed.
func (s *Supervisor) sameTidArriving(tid int) bool { return s.inb.holds(uint32(tid)) }

func (s *Supervisor) stalled(note string) {
	s.markLocked(UncleanStall, note) // umu only: never mu
	s.killStep()
}

// ---------------------------------------------------------------- the end

// Finish ends the supervision once strace has exited: it stops the receiver,
// lets the processor and the pollers finish (a pinned exec that never resolved is
// written `timeout`), syncs the sidecar and reports.  Call it exactly once.
func (s *Supervisor) Finish() Result {
	if !s.started {
		// Never started (strace did not start): close both ends, nothing to wait for.
		s.child.Close()
		syscall.Close(s.parent)
		s.startWriter(false)
		return Result{Lines: s.w.Lines(), Bytes: s.w.Bytes(), SHA256: s.w.SHA256()}
	}
	s.ending.Store(true)
	wait := func(ch <-chan struct{}, what string) {
		select {
		case <-ch:
		case <-time.After(s.tm.FinishWait):
			s.mu.Lock()
			s.markLocked(UncleanFailed, what+" did not stop")
			s.mu.Unlock()
		}
	}
	// The receiver may still be waiting for a listener that will not come:
	// strace is gone, so nothing can send it, and a shutdown wakes the receive.
	select {
	case <-s.rDone:
	case <-time.After(s.tm.FinishWait):
		syscall.Shutdown(s.parent, syscall.SHUT_RDWR)
		wait(s.rDone, "the receiver")
	}
	wait(s.pDone, "the processor")
	// The pollers see `ending` and write what they have.
	done := make(chan struct{})
	go func() { s.wg.Wait(); close(done) }()
	select {
	case <-done:
	case <-time.After(s.tm.FinishWait):
		// Whatever is still pending is written `timeout` here, never guessed.
		s.mu.Lock()
		for ex := range s.active {
			if ex.state == statePending {
				ex.state = stateClaimed
				s.finishLocked(ex, StatusTimeout, nil, true)
			}
		}
		s.mu.Unlock()
		wait(done, "the pollers")
	}
	close(s.stopW)
	<-s.wDone
	s.startWriter(false)

	s.mu.Lock()
	defer s.mu.Unlock()
	switch s.rExit.Load() {
	case rAborted:
		s.markLocked(UncleanFailed, "the supervisor stopped before the step ended")
	case rError:
		s.markLocked(UncleanFailed, "the receiver failed")
	case rRunning:
		s.markLocked(UncleanFailed, "the receiver did not end")
	}
	// Held notifications of groups that never got released belong to threads that
	// are gone with the step; nothing remains to answer.
	for _, g := range s.groups {
		for _, ex := range []*exec{g.last} {
			if ex != nil && ex.pin >= 0 {
				syscall.Close(ex.pin)
				ex.pin = -1
			}
		}
		if g.pidfd >= 0 {
			syscall.Close(g.pidfd)
			g.pidfd = -1
		}
	}
	if fd := int(s.initPidfd.Swap(-1)); fd >= 0 {
		syscall.Close(fd)
	}
	s.closeListener()
	syscall.Close(s.parent)
	if err := s.cfg.Out.Sync(); err != nil {
		s.markLocked(UncleanFailed, "syncing the sidecar: "+err.Error())
	}
	s.umu.Lock()
	unclean, notes := append([]string{}, s.unclean...), append([]string{}, s.notes...)
	s.umu.Unlock()
	return Result{
		Unclean: unclean, Notes: notes,
		Lines: s.w.Lines(), Bytes: s.w.Bytes(), SHA256: s.w.SHA256(), Received: s.received,
		Dropped: int(s.dropped.Load()), Notified: int(s.notified.Load()), Kills: int(s.kills.Load()),
	}
}

package trace

import (
	"bufio"
	"errors"
	"io"
	"math"
	"strconv"
	"strings"

	straceparser "github.com/adhithyan15/coding-adventures/code/packages/go/strace-parser"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/syscallceil"
)

// This file is pass 1: turn the byte stream into an ordered list of COMPLETED
// records (a system call with its result, or a process exit).  It is the part
// that knows strace's line shapes.
//
// # What strace -f --quiet=attach -y -s 4096 prints
//
// With -o <file> and -f every line starts with the pid of the process (strictly
// speaking the thread) that produced it, followed by two spaces.  Five shapes
// matter:
//
//	1234  openat(AT_FDCWD, "/etc/passwd", O_RDONLY|O_CLOEXEC) = 3</etc/passwd>
//	1234  openat(AT_FDCWD, "/nope", O_RDONLY) = -1 ENOENT (No such file or directory)
//	1234  +++ exited with 0 +++
//	1234  +++ killed by SIGKILL +++
//	1234  --- SIGCHLD {si_signo=SIGCHLD, si_code=CLD_EXITED, si_pid=1235} ---
//
// and, when two processes make calls at the same time, a call is split:
//
//	1234  wait4(-1,  <unfinished ...>
//	1235  openat(AT_FDCWD, "/x", O_RDONLY) = 3</x>
//	1234  <... wait4 resumed>[{WIFEXITED(s) && WEXITSTATUS(s) == 0}], 0, NULL) = 1235
//
// The state machine below keeps, per pid, the text printed before
// "<unfinished ...>", and when the matching "<... NAME resumed>" arrives it
// glues the two halves into one ordinary line and parses that.  A resume with
// nothing pending, a resume with the wrong name, a second call while one is
// pending, or a call still pending at end of input are all errors.
//
// Two further lines are special.  A personality change is printed without a
// pid, as "[ Process PID=1234 runs in 32 bit mode. ]": the process switched
// system-call ABI and the syscall names we classify can no longer be trusted,
// so it is an error.  Signal lines ("--- ... ---") carry no path and are
// skipped.
//
// # A thread killed at syscall entry: "???("
//
// One more shape is accepted, and only these three spellings of it.  When a
// thread reaches a syscall-ENTRY stop with a fatal signal already pending (a
// sibling's exit_group, a SIGKILL, or an exec by another thread), strace cannot
// read the syscall number, decodes the call as the stub name "???" and prints
//
//	3486  ???( <unfinished ...>                       (S1, S2)
//	3486  <... ??? resumed>)          = ?            (S2 only)
//	3486  ???()                       = ?            (S3, in one line)
//	3486  +++ exited with 0 +++
//
// Which of the three appears depends on a stale per-thread "filtered" flag
// (TCB_FILTERED) left by the thread's PREVIOUS syscall, so all three are
// legitimate output of the same event: S1 when the previous syscall was
// untraced, S2 when it was traced (or this is the thread's first), S3 as S2
// when nothing was printed in between.  The call is not an observation of
// anything, because the kernel never ran it: ptrace_report_syscall() returns
// fatal_signal_pending(), and syscall_trace_enter() then returns -1 BEFORE
// seccomp and before dispatch.  There is no later stop for that task; it goes
// straight to do_group_exit.  So the line is a "this thread died" marker,
// carries no path, and is dropped; the exit line that follows is what the model
// sees.  A "???" call can stand for ANY syscall (it is printed before -e
// trace= filtering), which is exactly why it is dropped rather than
// classified.  Anything that is not one of the three exact spellings, or that
// is followed by anything but the thread's exit, stays refused: the one way the
// pid lives on (a non-leader exec while the leader sits at the stop) is refused
// by its "superseded by execve" line and the mismatched resume.
//
// # A phantom execve (D33)
//
// The same race has a fourth spelling.  Thread T was at a syscall-entry stop
// when another thread L's execve made de_thread kill it; strace still
// dispatched T's stale stop, PTRACE_GET_SYSCALL_INFO returned op=NONE with only
// its header written, and strace (which tests only op == EXIT) reused its
// static entry buffer, still holding L's execve.  So it prints a byte-identical
// line under T:
//
//	4288  execve("/usr/bin/sh", ["sh", "-c", "x"], 0x24e1 /* 5 vars */ <unfinished ...>
//	4289  execve("/usr/bin/sh", ["sh", "-c", "x"], 0x24e1 /* 5 vars */ <unfinished ...>
//	4289  +++ exited with 0 +++
//	4288  <... execve resumed>) = 0
//
// exitInsideCall accepts the unfinished execve at T's exit, and drops it, only
// when: it is execve (not execveat) with an absolute path; T's next line is
// its own exit with code 0 (no signal line, not killed); another pid has an
// EARLIER pending execve with the same text before "<unfinished ...>" (the
// earliest wins); and that owner later resumes with "= 0" (withGhosts; a failed
// owner refuses at the ghost's line, a never-resumed one at its own).  Pass 2 (model.apply)
// also requires T and L in one thread group.  It is sound because L's
// identical execve succeeded: T died before its image changed, and its only
// possible effect is reading the path L's exec reads.
//
// # A task that dies inside a call (D35)
//
// When a thread group exits (Go's exit_group, a fatal signal), the kernel kills
// the group's other threads wherever they are, and a task that dies inside a
// system call is printed in three shapes.  Every threaded runtime makes them.
//
//	2  tgkill(1, 1, SIGURG <unfinished ...>
//	2  <... tgkill resumed>) = ? <unavailable>              (shape 1)
//
//	2  clone(child_stack=0x7f.., flags=CLONE_VM|... <unfinished ...>
//	2  <... clone resumed> <unfinished ...>) = ?            (shape 2)
//
//	2  openat(AT_FDCWD</w>, 0x7f3a6c001230, O_RDONLY|O_CLOEXEC <unfinished ...>
//	2  +++ exited with 0 +++                                (shape 3)
//
// In shape 1 strace could not fetch the result because the tracee was gone; in
// shape 2 it printed at PTRACE_EVENT_EXIT with the call still open; in shape 3
// the task was killed out of its syscall-ENTRY stop, so strace could no longer
// read its memory and the path is an address.  All three mean the call's result
// is unknown.  Both readers accept exactly "= ? <unavailable>" (parseResult) and
// exactly " <unfinished ...>)" then "= ?" after a resume (dyingResumeTail); the
// grammar (strace.grammar) is looser, and these two functions pin the spelling,
// as parseResult pins every result.
//
// The first two shapes leave the pid as a DYING call (pending.dying, built on
// the dead machinery above): only its own exit line may follow, a signal line,
// a second resume or a new call is refused.  Shape 3 is simply an unfinished call
// at the pid's exit.  What the exit does to it is dyingPolicy, in one table
// shared by both readers:
//
//   - an ignored-class call (tgkill) is dropped, as it always was;
//   - fork and vfork are dropped and no spawn is recorded.  A clone is dropped
//     only when its flags are readable and allowed (flags= present, every token
//     a known name, no CLONE_UNTRACED or CLONE_NEW*).  If the child did start,
//     its first line has no spawn record: orphan-pid, which fails closed;
//   - everything else stays unresumed-call: a write, a network call, chdir, exec,
//     and every call that READS (stat, listing, open).  The model infers reads
//     only from the open and stat records, so a dropped one would remove an input
//     from the read set, and a live sibling thread (or a sibling's fork or exec)
//     can still use the descriptor or the buffer.
//
// Events are recorded in the order calls COMPLETE (when the return value is
// known), which for a split call is the position of its "resumed" line.

// Limits caps how much input Parse accepts.  A zero field takes the default.
type Limits struct {
	MaxLineBytes  int   // longest accepted line, including its newline
	MaxTotalBytes int64 // total input bytes
	MaxEvents     int   // number of events produced
}

// DefaultLimits returns the caps used for zero fields.
//
// They are a RESIDENT-MEMORY bound for the parser, not the recorder's 2 GiB
// disk cap.  Pass 1 keeps every non-ignored record until pass 2, and each kept
// record retains its whole source line.  So memory is roughly
// MaxTotalBytes (the lines) plus about 200 bytes and a few slice headers per
// record (MaxEvents of them): with the defaults below, a few hundred MiB at
// the very worst, which a CI runner can afford.  A caller that has measured a
// bigger trace and a bigger budget raises them explicitly.
func DefaultLimits() Limits {
	return Limits{MaxLineBytes: 1 << 20, MaxTotalBytes: 64 << 20, MaxEvents: 1_000_000}
}

func (l Limits) withDefaults() Limits {
	d := DefaultLimits()
	if l.MaxLineBytes <= 0 {
		l.MaxLineBytes = d.MaxLineBytes
	}
	if l.MaxTotalBytes <= 0 {
		l.MaxTotalBytes = d.MaxTotalBytes
	}
	if l.MaxEvents <= 0 {
		l.MaxEvents = d.MaxEvents
	}
	return l
}

// result is a parsed "= ..." tail.
type result struct {
	value   int64
	hasVal  bool // a number was printed (as opposed to "?")
	errno   string
	ann     []byte // -y annotation on a returned descriptor
	hasAnn  bool
	unknown bool // "= ?"
	// unavailable: the exact text "= ? <unavailable>".  strace could not fetch
	// the call's result because the tracee was already gone (the dying-call
	// rule, "A task that dies inside a call" in the file head).
	unavailable bool
	// minusOne says the value was the exact decimal text "-1", as strace prints
	// a failed call.  The hex spelling of the same number and "-01" are not
	// what strace prints, so the sandbox filter's answer (filterAnswered)
	// requires this, not just value == -1.
	minusOne bool
	// hex says the value was printed as 0x... (pointer-style).  A descriptor
	// is never printed that way, so an open result with no annotation is only
	// believed (D50, bareReadOnlyOpen) when it is a plain decimal.
	hex bool
	// spaced says an unknown result `?` was followed by a space, as strace
	// prints it (`= ? ERESTARTNOINTR`).  Only restartedSpawn reads it.
	spaced bool
	// host is the pid in strace's own PID namespace, from the annotation
	// `= 2 /* 5030 in strace's PID NS */` that --pidns-translation adds to a
	// pid returned inside another PID namespace; hasHost says it was printed.
	host    int64
	hasHost bool
}

// pid is the pid a spawn returned, in the namespace the trace's line prefixes
// use: the translated host pid when there is one, else the printed value.
func (r result) pid() int {
	v := r.value
	if r.hasHost {
		v = r.host
	}
	if v < math.MinInt || v > math.MaxInt {
		return 0
	}
	return int(v)
}

// parsePidnsNote reads ` /* N in strace's PID NS */` at the start of s (leading
// spaces allowed) and returns N and the rest.  It is exact: any other comment
// is not recognised, and then fails the result parse like any stray text.
func parsePidnsNote(s string) (n int64, rest string, ok bool) {
	t := strings.TrimLeft(s, " ")
	const open, tail = "/* ", " in strace's PID NS */"
	if !strings.HasPrefix(t, open) {
		return 0, s, false
	}
	t = t[len(open):]
	k := 0
	for k < len(t) && t[k] >= '0' && t[k] <= '9' {
		k++
	}
	// At most 9 digits (below 1<<30, the bound local pids have), and no leading
	// zero: "007" is not how strace prints a pid.
	if k == 0 || k > 9 || (k > 1 && t[0] == '0') || !strings.HasPrefix(t[k:], tail) {
		return 0, s, false
	}
	v, err := strconv.ParseInt(t[:k], 10, 64)
	if err != nil || v <= 0 {
		return 0, s, false
	}
	return v, t[k+len(tail):], true
}

func (r result) ok() bool { return !r.unknown && r.errno == "" }

// rawCall is a completed call: name, argument texts, result.
type rawCall struct {
	name string
	args []string
	ret  result
}

// record is one completed observation, in trace order.
type record struct {
	line  int
	pid   int
	call  *rawCall // nil for exit lines and start markers
	exit  *Exit
	start bool // a handled call began (its "<unfinished ...>" line); call is nil
	split bool // this completed call was split: its cwd was sampled at the start record
	// startLine is the line of a split call's "<unfinished ...>" half, set on
	// every split record.  Handled calls also get a start marker record and
	// the model reads their start line from it; a refused-class call (unshare)
	// gets no marker, so this is the only place its interval's start survives
	// (D46: the unshare(CLONE_FS) of the sandbox helper).
	startLine int
	// exitCall marks the START of an exit_group or exit call (call is nil;
	// rawExit says which).  The call
	// is otherwise ignored; the model needs only WHEN it began, because strace
	// prints it at the thread group's exit, before the "+++ exited +++" line
	// (see Trace.InitExitLine).  Only the start line is emitted, and only for
	// a call that did NOT return: "= ?", or the unfinished form (whose
	// resumed half may still show a real result, see retract).
	exitCall bool
	rawExit  bool // the call is exit (one thread), not exit_group
	// retract, with exitCall, says the resumed half of an exit_group or exit that
	// began at line `line` returned a real result (it FAILED, e.g. EPERM from
	// a seccomp filter the step installed): the thread group is not dying, so
	// the marker taken at the start line is withdrawn.
	retract bool
	// ghosts are the phantom execve lines (D33) that were accepted against this
	// completed execve; pass 2 checks they share its thread group.
	ghosts []ghost
	// spawnDropped, on an exit record, says whether the dying call this exit
	// ended was a fork, vfork or clone that dyingPolicy dropped, and if so of
	// what kind (D44, D44a).  The model counts them to decide how many orphan
	// exits it may accept, and whether they are all threads.
	spawnDropped droppedSpawn
}

// ghost is a phantom "execve(" line accepted by ghostOwner: the pid that
// printed it and the line it was printed on.
type ghost struct{ pid, line int }

// isExitName: the two calls that end a thread (exit) or a thread group
// (exit_group), whose START lines the model keeps (Trace.InitExitLine).
func isExitName(n string) bool { return n == "exit_group" || n == "exit" }

// lineState is what the line handler carries between lines.
type lineState struct {
	pend map[int]*pending
	// execs indexes the pending execve calls (see setPending).
	execs map[string]*execQueue
	root  int // pid of the first line
	// helper is HelperRoot: name classification (unknown, refused) is DEFERRED
	// to pass 2, because whether a name is refused depends on which process
	// made the call and whether it has exec'd, and that is only known once the
	// whole spawn structure has been read.
	helper bool
	// maxLine is the byte limit of one line, for the grammar path (zero means
	// the default).  The old path enforces it in readLine.
	maxLine int
	// lp and argbuf are the grammar path's reusable parse state (P2 PR 5): one
	// LineParser per trace parse, owned by this lineState, never shared between
	// goroutines (a lineState is per-trace, so neither is).  The zero values are
	// ready.  A lineState is used by pointer and must not be copied.
	lp     straceparser.LineParser
	argbuf []string
}

type pending struct {
	name   string
	prefix string
	line   int
	// dead marks a "???(" entry: the thread was killed at syscall entry and
	// the call never ran.  Only its own "<... ??? resumed>) = ?" (resumed) and
	// its exit line are acceptable next; the pending ends at the exit line.
	dead    bool
	resumed bool
	// dying marks a call whose RESULT strace could not fetch because the task
	// died inside it: "= ? <unavailable>", or the resume "<... NAME resumed>
	// <unfinished ...>) = ?".  Like dead it accepts only the pid's own exit next
	// (dead and resumed are set with it, so a signal, a second resume and a new
	// call all refuse); unlike dead the call has a name, and the exit applies
	// dyingPolicy to it.
	dying bool
	// args are the top-level arguments printed before "<unfinished ...>" (or of
	// the whole dying call): what dyingPolicy reads the open flags from.
	args []string
	// sawSignal: a signal line from this pid arrived while the call was pending
	// (the D33 phantom-exec rule needs the ghost's next line to be its exit).
	sawSignal bool
	// ghosts: phantom execve lines (D33) accepted against this pending execve.
	ghosts []ghost
}

// setPending and clearPending are the only places a pending call is added or
// removed, in all three readers.  Besides the pend map they keep st.execs, a
// FIFO per text-before-"<unfinished ...>" of the pending execve calls, in line
// order, so ghostOwner finds its owner without scanning the whole map (D33).
// st.execs is derived state: it is not part of the reader-agreement comparison.
//
// The FIFO is lazy: removing a call from the middle costs nothing (the entry
// just goes stale), and only the head is trimmed, so N same-text calls
// resumed in any order cost O(N) in all, not O(N^2).  An entry is live only if
// the pid's pending call is still that very call (same line), so a reused pid
// cannot revive a stale entry.
type execQueue struct {
	ents []execEnt
	head int
}

type execEnt struct{ pid, line int }

func (st *lineState) execLive(e execEnt) bool {
	p := st.pend[e.pid]
	return p != nil && p.line == e.line && p.name == "execve"
}

func (st *lineState) setPending(pid int, p *pending) {
	st.pend[pid] = p
	if p.name == "execve" {
		if st.execs == nil {
			st.execs = map[string]*execQueue{}
		}
		q := st.execs[p.prefix]
		if q == nil {
			q = &execQueue{}
			st.execs[p.prefix] = q
		}
		q.ents = append(q.ents, execEnt{pid, p.line})
	}
}

func (st *lineState) clearPending(pid int) {
	p := st.pend[pid]
	if p == nil {
		return
	}
	delete(st.pend, pid)
	if p.name == "execve" {
		if q := st.execs[p.prefix]; q != nil {
			for q.head < len(q.ents) && !st.execLive(q.ents[q.head]) {
				q.head++
			}
			if q.head == len(q.ents) {
				delete(st.execs, p.prefix)
			}
		}
	}
}

// exitInsideCall is what an exit line does to the exiting pid's pending call,
// shared by all readers so they cannot drift.  An ignored-class call or a
// killed-at-entry entry just ends (D18); a phantom execve (D33) is recorded on
// its owner and ends; a call the task died inside (D35) ends under
// dyingPolicy; anything else is an unresumed call, refused at the call's own
// line.
func exitInsideCall(st *lineState, pid int, ex *Exit) (spawnDropped droppedSpawn, e *Error) {
	p := st.pend[pid]
	if p == nil {
		return dropNone, nil
	}
	if p.dying {
		// strace already said the result is unknown because the task died: the
		// policy alone decides, with no other way out.
		if !dyingPolicy(p) {
			return dropNone, unresumedError(p)
		}
		st.clearPending(pid)
		return droppedSpawnOf(p), nil
	}
	if lookup(p.name).class == classIgnored || p.dead {
		st.clearPending(pid)
		return dropNone, nil
	}
	if l := ghostOwner(st, pid, p, ex); l != nil {
		l.ghosts = append(l.ghosts, ghost{pid, p.line})
		st.clearPending(pid)
		return dropNone, nil
	}
	// A task killed out of its syscall-ENTRY stop prints "<unfinished ...>" and
	// then only its exit: the same dying-call rule applies (shape 3).
	if dyingPolicy(p) {
		st.clearPending(pid)
		return droppedSpawnOf(p), nil
	}
	return dropNone, unresumedError(p)
}

// droppedSpawn says what a dropped dying call could have started (D44, D44a).
//
//	dropNone     not a spawn: nothing was dropped that could have a child
//	dropThread    a clone whose flags argument names CLONE_THREAD: the child is
//	              a thread of the caller's group and cannot outlive it
//	dropProcess  anything else that could have started a child: fork, vfork, a
//	              clone without CLONE_THREAD, or a clone whose flags cannot be
//	              read.  Fails closed: that child could be a real daemon.
//
// clone3 is refused-class and never dropped, so it never appears here.
type droppedSpawn uint8

const (
	dropNone droppedSpawn = iota
	dropThread
	dropProcess
)

// droppedSpawnOf classifies the dying call dyingPolicy just dropped.  The flags are
// read with analyzeClone, the same reader that vetted them (cloneArgsAllowed);
// it returns the text after "flags=", which hasFlag splits on "|".  If it cannot
// read them the answer is dropProcess.
func droppedSpawnOf(p *pending) droppedSpawn {
	ent := lookup(p.name)
	if ent.class != classHandled || (ent.role != roleFork && ent.role != roleClone) {
		return dropNone
	}
	if ent.role == roleClone {
		if flags, _, reason := analyzeClone(p.args); reason == "" && hasFlag(flags, "CLONE_THREAD") {
			return dropThread
		}
	}
	return dropProcess
}

// unresumedError is the refusal of a call that never completed.  For a name
// outside the closed table it also carries the census's unknown-number code
// (D44); the name itself never leaves this function.
func unresumedError(p *pending) *Error {
	e := incomplete(p.line, reasonUnresumed)
	if lookup(p.name).class == classUnknown {
		e.UnknownNumber = "unknown-number:no"
		if isUnknownNumberName(p.name) {
			e.UnknownNumber = "unknown-number:yes"
		}
	}
	return e
}

// isUnknownNumberName reports whether name is exactly how strace spells a
// syscall number it has no table entry for.  Source: strace's src/syscall.c
// (xsprintf(s->buf, "syscall_%#" PRI_klx, scno)): "syscall_" followed by the
// number in C's "%#lx": lowercase hex with a 0x prefix and no leading zeros,
// and plain "0" for zero.  At most 16 hex digits (64 bits).
func isUnknownNumberName(name string) bool {
	rest, ok := strings.CutPrefix(name, "syscall_")
	if !ok {
		return false
	}
	if rest == "0" {
		return true
	}
	h, ok := strings.CutPrefix(rest, "0x")
	if !ok || h == "" || len(h) > 16 || h[0] == '0' {
		return false
	}
	for i := 0; i < len(h); i++ {
		if c := h[i]; !((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')) {
			return false
		}
	}
	return true
}

// dyingPolicy is the table of the dying-call rule (KILN04 D35): whether the call
// a task died inside, whose result is therefore unknown, may be discarded.
//
//	ignored class (tgkill, wait4, ...)      drop    no path, no effect
//	fork, vfork                             drop    no spawn is recorded; a child
//	                                                 that did start is an
//	                                                 orphan-pid, which fails closed
//	clone with readable, allowed flags      drop    as above.  flags= must be
//	                                                 present, every token a known
//	                                                 name, and none CLONE_UNTRACED
//	                                                 or CLONE_NEW* (analyzeClone)
//	unknown number, above the filter's      drop    D47: the filter answered ENOSYS,
//	ceiling or in the path-free set                  or the call has no path
//	everything else                         refuse  see below
//
// A call that READS (a stat, a listing, an open) is deliberately NOT dropped.
// The model infers what a step read only from the open and stat records (read
// is not traced), so dropping one removes an input from the read set.  Its
// result is lost, not absent: a live sibling thread may use the descriptor or
// the buffer, a sibling's fork in the window copies it, a sibling's exec keeps
// the descriptor open without CLOEXEC.  No guard short of "shares nothing with
// any live task" is sound, and that excludes every thread.  Writes, network
// calls, chdir, exec and unknown or refused names stay refused for the same
// reason.
func dyingPolicy(p *pending) bool {
	ent := lookup(p.name)
	switch ent.class {
	case classIgnored:
		return true
	case classUnknown:
		return unknownNumberDying(p)
	case classHandled:
		if nullPathDying(p) {
			return true
		}
	default:
		return false
	}
	switch ent.role {
	case roleFork:
		return true
	case roleClone:
		return cloneArgsAllowed(p.args)
	}
	return false
}

// nullPathDying (D44): a dying stat, lstat, access or readlink whose path
// argument was printed as NULL.  The kernel copies the path in before it looks
// anything up, so a NULL pointer fails with EFAULT having read nothing, and no
// sibling can use a result that does not exist.  newfstatat and statx are NOT
// here: with AT_EMPTY_PATH a NULL or "" path means "the descriptor itself".
func nullPathDying(p *pending) bool {
	switch p.name {
	case "stat", "lstat", "access", "readlink":
		return len(p.args) > 0 && p.args[0] == "NULL"
	}
	return false
}

// pathFreeUnknown is the closed set of system call numbers at or below the
// filter's ceiling that dyingPolicy may drop when strace could not name them
// (D47, branch b).  The sandbox's filter is a DENYLIST at or below the ceiling,
// so such a number reaches the kernel; it may be dropped only because it is
// known to take no path and so to read nothing the model infers inputs from.
//
//	456  futex_requeue(struct futex_waitv *waiters, unsigned int flags,
//	                   int nr_wake, int nr_requeue)
//
// Source: torvalds/linux, arch/x86/entry/syscalls/syscall_64.tbl,
// "456 common futex_requeue sys_futex_requeue" (the pinned strace 6.8 predates
// it, which is why it prints syscall_0x1c8; the number appeared as a dying call
// in a recorded Go action trace; which code issued it is not established).
//
// ANY ADDITION NEEDS a citation of that table and a signature with no path (and
// no descriptor whose result a sibling could use).  When in doubt it stays out:
// a number that is not here is refused, as before D47.
var pathFreeUnknown = map[uint64]bool{456: true}

// maxFilterNr is the largest number the filter can see at all: seccomp_data.nr
// is 32 bits, so the kernel (and the filter) judge only the low 32 bits of a
// 64-bit number strace might print.  A larger printed number is not covered by
// the filter's jgt (syscall_0x1000001c8 is nr 456 to the filter), so branch (a)
// does not apply to it.
const maxFilterNr = 1<<32 - 1

// unknownNumberDying (D47): a dying call whose name is strace's spelling of a
// number it cannot name (isUnknownNumberName, exact) and whose number N is
//
//	(a) above syscallceil.Max and representable in 32 bits: the seccomp filter
//	    (internal/sandbox BuildFilter) answers ENOSYS to it with no effect; sound
//	    by construction.  The filter is a denylist at or below the ceiling, so the
//	    spelling alone proves nothing there; or
//	(b) in pathFreeUnknown: a number the kernel table names with a no-path
//	    signature.
//
// Every other unknown number stays refused.  The arguments are not read: neither
// branch depends on them.  Not a spawn (droppedSpawnOf says dropNone for the
// unknown class), so it accounts for no orphan pid.
func unknownNumberDying(p *pending) bool {
	n, ok := unknownNumber(p.name)
	if !ok {
		return false
	}
	return (n > uint64(syscallceil.Max) && n <= maxFilterNr) || pathFreeUnknown[n]
}

// unknownNumber parses the number out of a name isUnknownNumberName accepts.
func unknownNumber(name string) (uint64, bool) {
	if !isUnknownNumberName(name) {
		return 0, false
	}
	rest := strings.TrimPrefix(name, "syscall_")
	if rest == "0" {
		return 0, true
	}
	n, err := strconv.ParseUint(strings.TrimPrefix(rest, "0x"), 16, 64)
	return n, err == nil
}

// cloneArgsAllowed reports whether a clone's printed arguments are readable and
// allowed: flags= is there (analyzeClone), every token is a name strace prints
// (knownCloneToken), and none is CLONE_UNTRACED or CLONE_NEW* (analyzeClone's
// own refusal).  A clone with any other flags may have escaped the tracer or the
// sandbox's namespaces, so its child could be anything.
func cloneArgsAllowed(args []string) bool {
	flags, _, reason := analyzeClone(args)
	if reason != "" {
		return false
	}
	for _, tok := range strings.Split(flags, "|") {
		if !knownCloneToken[strings.TrimSpace(tok)] {
			return false
		}
	}
	return true
}

// diedInCall records that the pid's call (name, its entry arguments) has an
// unknown result because the task died inside it.  The pending call stays, as
// a dying one: only the pid's own exit may follow.  startLine is where the call
// began, which is where an unresumed refusal is reported.
func diedInCall(st *lineState, pid, startLine int, name, prefix string, args []string) {
	st.setPending(pid, &pending{name: name, prefix: prefix, line: startLine, dead: true, resumed: true, dying: true, args: args})
}

// dyingResumeTail is the text strace prints after "<... NAME resumed>" when the
// task died inside a call that had printed "<unfinished ...>": exactly that
// marker, the ")" and then "= ?" (the dying-call rule, shape 2).  It reports
// whether after is that, and nothing else.
func dyingResumeTail(after string) bool {
	const marker = " <unfinished ...>)"
	rest, ok := strings.CutPrefix(after, marker)
	return ok && isUnknownResult(rest)
}

// ghostOwner finds the pending execve a phantom "execve(" line copies (D33), or
// nil when the line is not a phantom.  Every condition is in the file head
// ("A phantom execve").  st.execs queues the candidates (same text) in line
// order, so the first live entry is the earliest: the answer is deterministic,
// and it must also be EARLIER than the ghost's own line.
func ghostOwner(st *lineState, pid int, p *pending, ex *Exit) *pending {
	if p.name != "execve" || p.sawSignal || len(p.ghosts) != 0 || ex.Killed || ex.Code != 0 ||
		!strings.HasPrefix(p.prefix, `execve("/`) {
		return nil
	}
	// The head of the queue is the earliest live execve of this text (clearPending
	// trims dead entries off the head).  If it is the ghost itself, nothing
	// earlier exists; if an entry is stale anyway, skip it rather than trust it.
	q := st.execs[p.prefix]
	if q == nil {
		return nil
	}
	for _, e := range q.ents[q.head:] {
		if !st.execLive(e) {
			continue
		}
		if e.pid == pid || e.line >= p.line {
			return nil
		}
		return st.pend[e.pid]
	}
	return nil
}

// withGhosts finishes the owner's resumed execve (D33): a failed exec with
// ghosts means the ghost line was not a copy of a successful call, so the first
// ghost's line is refused; otherwise the ghosts ride on the record to pass 2.
func withGhosts(p *pending, call rawCall, rec *record, e *Error) (*record, *Error) {
	if len(p.ghosts) == 0 {
		return rec, e
	}
	if e == nil && !call.ret.ok() {
		return nil, incomplete(p.ghosts[0].line, reasonUnresumed)
	}
	if rec != nil {
		rec.ghosts = p.ghosts
	}
	return rec, e
}

const (
	deadOpen    = "???( <unfinished ...>"
	deadResumed = "<... ??? resumed>)"
	deadWhole   = "???()"
)

// isUnknownResult reports whether s is spaces then exactly "= ?" (strace pads
// the '=' to a column), with at least one space.
func isUnknownResult(s string) bool {
	t := strings.TrimLeft(s, " ")
	return len(t) < len(s) && t == "= ?"
}

// maxUnknownItems caps the arguments of a name outside the handled table.
const maxUnknownItems = 16

func validName(n string) bool {
	if n == "" || len(n) > 64 {
		return false
	}
	for i := 0; i < len(n); i++ {
		c := n[i]
		if !((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '_') {
			return false
		}
	}
	return true
}

// parseCall parses "name(args) = result" or "name(args <unfinished ...>".
func parseCall(s string) (call rawCall, unfinished bool, prefix string, reason string) {
	j := strings.IndexByte(s, '(')
	if j < 0 || !validName(s[:j]) {
		return call, false, "", reasonUnparsable
	}
	call.name = s[:j]
	// A handled call has a known arity; anything else gets a generous global
	// cap.  Either way the bound is enforced WHILE scanning.
	maxItems := maxUnknownItems
	if ent := lookup(call.name); ent.class == classHandled {
		maxItems = ent.max
	}
	res, r := scanItems(s, j+1, true, maxItems)
	if r != "" {
		return call, false, "", r
	}
	if res.unfinished {
		p := s[:res.end]
		p = strings.TrimSuffix(p, " ")
		call.args = res.items // what dyingPolicy reads an open's flags from
		return call, true, p, ""
	}
	call.args = res.items
	ret, r := parseResult(s[res.end+1:])
	if r != "" {
		return call, false, "", r
	}
	call.ret = ret
	return call, false, "", ""
}

func isErrnoName(s string) bool {
	if s == "" || s[0] < 'A' || s[0] > 'Z' {
		return false
	}
	for i := 0; i < len(s); i++ {
		c := s[i]
		if !((c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_') {
			return false
		}
	}
	return true
}

// parseResult parses what follows the closing parenthesis:
//
//	= 3</etc/passwd>
//	= 0
//	= -1 ENOENT (No such file or directory)
//	= 0x7f1234567000
//	= ?
//	= ? ERESTARTSYS (To be restarted if SA_RESTART is set)
//
// strace pads the '=' to a column, so any run of spaces may precede it.  After
// the value only an errno name and a parenthesised message (free of quotes,
// brackets and '<' '>') may follow; anything else could be a path pretending
// to be the end of the call, so it is an error.
func parseResult(s string) (result, string) {
	var r result
	s = strings.TrimLeft(s, " ")
	if !strings.HasPrefix(s, "=") {
		return r, reasonResult
	}
	s = strings.TrimLeft(s[1:], " ")
	switch {
	case strings.HasPrefix(s, "?"):
		r.unknown = true
		s = s[1:]
		r.spaced = strings.HasPrefix(s, " ")
		// "= ? <unavailable>", exactly: strace could not fetch the result (the
		// dying-call rule).  One space, nothing after it, and nothing else may
		// carry it (not an errno, not a value).
		if s == " <unavailable>" {
			r.unavailable = true
			s = ""
		}
	case strings.HasPrefix(s, "0x"):
		k := 2
		for k < len(s) {
			if _, ok := hexVal(s[k]); !ok {
				break
			}
			k++
		}
		if k == 2 {
			return r, reasonResult
		}
		v, err := strconv.ParseUint(s[2:k], 16, 64)
		if err != nil {
			return r, reasonResult
		}
		if v > math.MaxInt64 {
			if v == math.MaxUint64 {
				// 0xffffffffffffffff is strace's 64-bit hex spelling of -1 (e.g. failed syscall).
				// It is still rejected below unless accompanied by an errno, and minusOne stays false.
				r.value, r.hasVal = -1, true
			} else {
				return r, reasonResult
			}
		} else {
			r.value, r.hasVal = int64(v), true
		}
		r.hex = true
		s = s[k:]
	default:
		k := 0
		if k < len(s) && s[k] == '-' {
			k++
		}
		d := k
		for k < len(s) && s[k] >= '0' && s[k] <= '9' {
			k++
		}
		if k == d || k-d > 18 {
			return r, reasonResult
		}
		v, err := strconv.ParseInt(s[:k], 10, 64)
		if err != nil {
			return r, reasonResult
		}
		r.value, r.hasVal = v, true
		r.minusOne = s[:k] == "-1"
		s = s[k:]
		if h, rest, ok := parsePidnsNote(s); ok && v > 0 {
			r.host, r.hasHost = h, true
			s = rest
		}
		if strings.HasPrefix(s, "<") {
			if v < 0 {
				return r, reasonResult
			}
			body, end, rs := scanAnnotation(s, 0)
			if rs != "" {
				return r, rs
			}
			if strings.TrimSpace(s[end:]) != "" {
				return r, reasonAmbiguousPath
			}
			r.ann, r.hasAnn = body, true
			s = s[end:]
		}
	}
	s = strings.TrimLeft(s, " ")
	if s != "" && s[0] != '(' {
		k := strings.IndexByte(s, ' ')
		name := s
		if k >= 0 {
			name = s[:k]
		}
		if !isErrnoName(name) {
			return r, reasonResult
		}
		r.errno = name
		s = strings.TrimLeft(s[len(name):], " ")
	}
	if s != "" {
		if s[0] != '(' || s[len(s)-1] != ')' || strings.ContainsAny(s[1:len(s)-1], "()<>\"[]{}\\") {
			return r, reasonResult
		}
	}
	if r.hasVal && r.value < 0 {
		if r.value != -1 || r.errno == "" {
			return r, reasonResult
		}
	}
	if r.unknown && r.errno == "" {
		r.errno = "?"
	}
	return r, ""
}

// parseExit parses the inside of "+++ ... +++".
func parseExit(body string, pid int) (*Exit, bool) {
	body = strings.TrimSpace(body)
	if rest, ok := strings.CutPrefix(body, "exited with "); ok {
		if rest == "" || len(rest) > 9 {
			return nil, false
		}
		n, err := strconv.Atoi(rest)
		if err != nil || n < 0 {
			return nil, false
		}
		return &Exit{Pid: pid, Code: n}, true
	}
	if rest, ok := strings.CutPrefix(body, "killed by "); ok {
		core := false
		if r2, ok2 := strings.CutSuffix(rest, " (core dumped)"); ok2 {
			rest, core = r2, true
		}
		if !strings.HasPrefix(rest, "SIG") || !isErrnoName(rest) {
			return nil, false
		}
		return &Exit{Pid: pid, Killed: true, Signal: rest, CoreDumped: core}, true
	}
	return nil, false
}

// parsePid strips the pid prefix: "1234  rest" or "[pid 1234] rest".
func parsePid(line string) (int, string, bool) {
	if rest, ok := strings.CutPrefix(line, "[pid "); ok {
		k := strings.IndexByte(rest, ']')
		if k < 0 {
			return 0, "", false
		}
		pid, ok := atoiPid(rest[:k])
		if !ok {
			return 0, "", false
		}
		return pid, strings.TrimLeft(rest[k+1:], " "), true
	}
	k := 0
	for k < len(line) && line[k] >= '0' && line[k] <= '9' {
		k++
	}
	if k == 0 || k >= len(line) || line[k] != ' ' {
		return 0, "", false
	}
	pid, ok := atoiPid(line[:k])
	if !ok {
		return 0, "", false
	}
	return pid, strings.TrimLeft(line[k:], " "), true
}

func atoiPid(s string) (int, bool) {
	if s == "" || len(s) > 9 || (len(s) > 1 && s[0] == '0') {
		return 0, false
	}
	n, err := strconv.Atoi(s)
	if err != nil || n <= 0 {
		return 0, false
	}
	return n, true
}

// readRecords is pass 1.  It also returns the pid of the first line, which is
// the root of the process tree.  Every record (calls, start markers and exit
// lines alike) counts against MaxEvents.
func readRecords(r io.Reader, lim Limits, helper, grammar bool) ([]record, int, *Error) {
	recs, root, e, _ := readRecordsAt(r, lim, helper, grammar)
	return recs, root, e
}

// readRecordsAt is readRecords that also says where pass 1 stopped: the 1-based
// number of the line being processed when it refused (the line AFTER the last
// one for a refusal at the end of the input).  An Error's own Line can be an
// earlier line (an unresumed call is reported at its start), so the
// cross-check uses this to tell how far each reader got.
func readRecordsAt(r io.Reader, lim Limits, helper, grammar bool) ([]record, int, *Error, int) {
	br := bufio.NewReaderSize(r, 64<<10)
	var recs []record
	st := &lineState{pend: map[int]*pending{}, helper: helper, maxLine: lim.MaxLineBytes}
	handle := handleLine
	if grammar {
		handle = handleLineG
	}
	var total int64
	lineNo := 0
	for {
		line, err := readLine(br, lim.MaxLineBytes)
		if err != nil && !errors.Is(err, io.EOF) {
			var e *Error
			if errors.As(err, &e) {
				e.Line = lineNo + 1
				return nil, 0, e, lineNo + 1
			}
			return nil, 0, &Error{Reason: reasonRead, Line: lineNo + 1, Err: err}, lineNo + 1
		}
		if errors.Is(err, io.EOF) {
			if line == "" {
				break
			}
			// The last line has no newline: strace was cut off (or the file
			// was truncated) and the line may be a prefix of a different one.
			return nil, 0, incomplete(lineNo+1, reasonUnparsable), lineNo + 1
		}
		lineNo++
		total += int64(len(line))
		if total > lim.MaxTotalBytes {
			return nil, 0, incomplete(lineNo, reasonSize), lineNo
		}
		text := strings.TrimSuffix(line, "\n")
		if st.root == 0 {
			if pid, _, ok := parsePid(text); ok {
				st.root = pid
			}
		}
		rec, e := handle(text, lineNo, st)
		if e != nil {
			return nil, 0, e, lineNo
		}
		if rec != nil {
			recs = append(recs, *rec)
			if len(recs) > lim.MaxEvents {
				return nil, 0, incomplete(lineNo, reasonSize), lineNo
			}
		}
	}
	var first *pending
	for _, p := range st.pend {
		if first == nil || p.line < first.line {
			first = p
		}
	}
	if first != nil {
		return nil, 0, incomplete(first.line, reasonUnresumed), lineNo + 1
	}
	return recs, st.root, nil, lineNo + 1
}

// isWaitName: the ignored-class calls whose result is a pid.
func isWaitName(n string) bool { return n == "wait4" || n == "waitid" || n == "waitpid" }

func isExecName(n string) bool { return n == "execve" || n == "execveat" }

// readLine reads one line (including its '\n') of at most max bytes.
func readLine(br *bufio.Reader, max int) (string, error) {
	var buf []byte
	for {
		chunk, err := br.ReadSlice('\n')
		if len(buf)+len(chunk) > max {
			return "", incomplete(0, reasonSize)
		}
		buf = append(buf, chunk...)
		switch {
		case err == nil:
			return string(buf), nil
		case errors.Is(err, bufio.ErrBufferFull):
			continue
		default:
			return string(buf), err
		}
	}
}

// handleLine classifies one line.  It returns the completed record (nil when
// the line completes nothing, e.g. an unfinished call or a signal line).
func handleLine(text string, lineNo int, st *lineState) (*record, *Error) {
	pend := st.pend
	if text == "" {
		return nil, incomplete(lineNo, reasonUnparsable)
	}
	for i := 0; i < len(text); i++ {
		if c := text[i]; c < 0x20 || c == 0x7f {
			// strace escapes every control character; a raw one means the line
			// is not strace's own output.
			return nil, incomplete(lineNo, reasonUnparsable)
		}
	}
	if strings.HasPrefix(text, "[ Process PID=") {
		return nil, incomplete(lineNo, reasonPersonality)
	}
	pid, body, ok := parsePid(text)
	if !ok || body == "" {
		return nil, incomplete(lineNo, reasonUnparsable)
	}
	switch {
	case strings.HasPrefix(body, "+++ "):
		if !strings.HasSuffix(body, " +++") || len(body) < 8 {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		ex, ok := parseExit(body[4:len(body)-4], pid)
		if !ok {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		// A process killed or exiting inside a call that has no effect on paths
		// is normal (wait4 interrupted by SIGKILL).  A path call that never
		// returned is not: its result is unknown (but see D33).
		dropped, e := exitInsideCall(st, pid, ex)
		if e != nil {
			return nil, e
		}
		return &record{line: lineNo, pid: pid, exit: ex, spawnDropped: dropped}, nil
	case strings.HasPrefix(body, "--- "):
		if !strings.HasSuffix(body, " ---") || len(body) < 8 {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		if p := pend[pid]; p != nil {
			if p.dead {
				return nil, incomplete(lineNo, reasonInterleaved)
			}
			p.sawSignal = true
		}
		return nil, nil
	case strings.HasPrefix(body, "<... "):
		rest := body[len("<... "):]
		k := strings.Index(rest, " resumed>")
		if k < 0 {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		name := rest[:k]
		p := pend[pid]
		if p == nil || p.name != name {
			return nil, incomplete(lineNo, reasonUnmatched)
		}
		if p.dead {
			// The only thing a killed-at-entry call may resume to is "= ?",
			// once.  "= 0", an errno, "<unavailable>" or arguments would mean
			// the call actually ran.
			if p.resumed {
				return nil, incomplete(lineNo, reasonUnmatched)
			}
			if !strings.HasPrefix(body, deadResumed) || !isUnknownResult(body[len(deadResumed):]) {
				return nil, incomplete(lineNo, reasonUnparsable)
			}
			p.resumed = true
			return nil, nil
		}
		st.clearPending(pid)
		after := rest[k+len(" resumed>"):]
		if dyingResumeTail(after) {
			// The task died inside the call: only its exit may follow (D35).
			diedInCall(st, pid, p.line, p.name, p.prefix, p.args)
			return nil, nil
		}
		call, unfinished, _, r := parseCall(p.prefix + after)
		if r != "" {
			return nil, incomplete(lineNo, r)
		}
		if unfinished {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		if call.ret.unavailable {
			return diedWithResult(st, pid, lineNo, p.line, call, true, st.helper)
		}
		if isExitName(call.name) && !call.ret.unknown {
			// The call began (the marker was taken at its start line) but
			// returned: it failed, and the group is not dying.
			return &record{line: p.line, pid: pid, exitCall: true, rawExit: call.name == "exit", retract: true}, nil
		}
		rec, e := finishCall(pid, lineNo, call, true, st.helper)
		if rec != nil {
			rec.startLine = p.line
		}
		return withGhosts(p, call, rec, e)
	}
	if body == deadOpen || (strings.HasPrefix(body, deadWhole) && isUnknownResult(body[len(deadWhole):])) {
		// See "A thread killed at syscall entry" in the file head.  Something
		// already pending for the pid is the usual interleaving error.
		if pend[pid] != nil {
			return nil, incomplete(lineNo, reasonInterleaved)
		}
		st.setPending(pid, &pending{name: "???", line: lineNo, dead: true, resumed: body != deadOpen})
		return nil, nil
	}
	call, unfinished, prefix, r := parseCall(body)
	if r != "" {
		return nil, incomplete(lineNo, r)
	}
	if pend[pid] != nil {
		return nil, incomplete(lineNo, reasonInterleaved)
	}
	if unfinished {
		// A refused name is judged by its result, at the resumed half.
		if !st.helper && lookup(call.name).class != classRefused {
			if e := classifyName(call.name, lineNo); e != nil {
				return nil, e
			}
		}
		st.setPending(pid, &pending{name: call.name, prefix: prefix, line: lineNo, args: call.args})
		if isExitName(call.name) {
			return &record{line: lineNo, pid: pid, exitCall: true, rawExit: call.name == "exit"}, nil
		}
		if lookup(call.name).class == classHandled {
			// The working directory a split call runs in is the one at its
			// START; pass 2 samples it when it meets this marker.
			return &record{line: lineNo, pid: pid, start: true}, nil
		}
		return nil, nil
	}
	if call.ret.unavailable {
		return diedWithResult(st, pid, lineNo, lineNo, call, false, st.helper)
	}
	return finishCall(pid, lineNo, call, false, st.helper)
}

// diedWithResult handles a completed call whose result is "= ? <unavailable>":
// strace could not fetch it because the task was already gone (the dying-call
// rule, D35).  Nothing is recorded for the call (it has no result to judge).
// The pid goes on as a dying call and only its own exit may follow.  An
// ignored-class call still gets what finishCall would give it, which is the
// start marker of a whole-line exit_group.
func diedWithResult(st *lineState, pid, lineNo, startLine int, call rawCall, split, deferClass bool) (*record, *Error) {
	var rec *record
	if lookup(call.name).class == classIgnored {
		var e *Error
		if rec, e = finishCall(pid, lineNo, call, split, deferClass); e != nil {
			return nil, e
		}
	}
	diedInCall(st, pid, startLine, call.name, "", call.args)
	return rec, nil
}

// finishCall checks the closed table and keeps the call unless it is ignored.
//
// With HelperRoot, unknown and refused names are kept and judged in pass 2
// (deferClass), where the sandbox helper's setup calls can be told apart.
func finishCall(pid, lineNo int, call rawCall, split, deferClass bool) (*record, *Error) {
	// The PID-namespace note belongs on the calls that return a pid: the
	// spawns, and the waits that hand one back.  On any other call it means the
	// line is not what we think it is.
	if call.ret.hasHost && !isSpawn(call.name) && !isWaitName(call.name) {
		return nil, incomplete(lineNo, reasonResult)
	}
	// A refused name the sandbox filter answered (clone3 = -1 ENOSYS, mount =
	// -1 EPERM) is the filter working: drop it, in both modes.
	if tolerated, e := classifyCall(call.name, call.ret, lineNo); tolerated {
		return nil, nil
	} else if e != nil && !deferClass {
		return nil, e
	}
	if lookup(call.name).class == classIgnored {
		if isExitName(call.name) && !split && call.ret.unknown {
			// A whole-line exit_group that did not return (the group is
			// dying): its one line is its start line.  One that returned
			// (a failure) is nothing.
			return &record{line: lineNo, pid: pid, exitCall: true, rawExit: call.name == "exit"}, nil
		}
		return nil, nil
	}
	return &record{line: lineNo, pid: pid, call: &call, split: split}, nil
}

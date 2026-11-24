package traced

import (
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Exec heads recorded at exec time (KILN04 D48, step 5: consumption)
// ===================================================================
//
// The problem this answers
// ------------------------
// `go test` builds its test binary under the step's fresh tmpfs, runs it and
// deletes it.  loadChain (exec.go) must read the first bytes of every exec'd
// file to learn what the KERNEL loaded for it (a `#!` interpreter, an ELF's
// PT_INTERP), and the tmpfs is gone by the time anything can read it, so such an
// exec is refused as `exec-unreadable:under-tmp`.
//
// internal/exechead's supervisor took the bytes WHILE the exec happened and left
// them in `<step>.exechead`, one JSON line per notified execve/execveat.  This
// file decides when a classifier may BELIEVE such a line.  Believing a wrong head
// is exactly the unsoundness this package exists to prevent (a cached run valid
// after its loader changed), so every rule below fails closed: anything the
// pairing cannot vouch for keeps today's refusal, with a sub-code that names why
// (the census then says what is left to win).
//
// What a line is, and what it is not
// ----------------------------------
// A `confirmed` line says: the supervisor SAW the process's /proc/<tgid>/exe
// become the file it had pinned, and read the head from the pinned inode while
// the kernel held it write-denied.  It does not say WHICH of the trace's exec
// events it belongs to; that is the pairing's job.
//
// The pairing
// -----------
//	trace  (strace, per pid, in order)       sidecar (supervisor, per tgid, in order)
//	  pid 11  execve /usr/bin/sh -> ENOENT      seq 0 tgid 11 execve /usr/bin/sh   failed
//	  pid 11  execve /bin/sh     -> 0           seq 1 tgid 11 execve /bin/sh       outside
//	  pid 14  execve /tmp/x/t.test -> 0         seq 2 tgid 14 execve /tmp/x/t.test confirmed
//
// The nth exec EVENT of a pid pairs with the nth LINE of that tgid (strace prints
// the leader's pid for a process, which is the tgid).  Failed execs and `outside`
// placeholders count on both sides; that is what keeps the ordinals honest.  The
// counts of every pid must be EQUAL, the init's included: the listener is only
// installed just before the init's `execve` of the step, and the parser has
// already dropped the init's setup execs of the Kiln binary (windowFilter in
// internal/trace/model.go keeps a failed exec of anything else and the step's
// own exec), so from the first line the init pid pairs one for one too.  There is
// no "leading unmatched event" allowance and nothing is tolerated: a slip fails
// closed.  (isHelperExe is the only thing windowFilter drops, and the parser is
// given the RECORDED Kiln binary, so "the setup execs were of the recorded
// helper" holds by construction; the G7 digest checks in internal/record read
// the same two execs' argv.)
//
// A pid is refused as a whole (`exechead-pair`) when
//
//   - its counts differ, or the init pid is unknown (a trace that is not a
//     helper trace cannot be anchored);
//   - any of its lines has tid != tgid.  A non-leader's exec takes over the
//     leader's pid, and mapping a tid to the pid strace prints would trust an
//     attribution nothing here verifies;
//   - any of its exec events has no result (strace's `= ?`).  The trace parser
//     refuses such a trace outright today (unfinished, killed and result-less
//     execs are `trace-incomplete` there), so this is a second wall, not the
//     first: if the parser ever learned to keep one, the pairing still could not
//     say whether that exec reached the loader;
//   - a line that is `overload`, `timeout`, `gone` or `tainted` is followed by a
//     later `confirmed` line whose trace event did NOT return normally.  The
//     earlier exec's outcome is unknown, and an exec in flight is what a false
//     confirm needs.
//
// A refused pid costs only the execs under the tmpfs: everything else is judged
// as before.  An integrity failure of the file itself (malformed, a digest that
// is not the step sidecar's) is not decided here: internal/record refuses the
// WHOLE action before Classify is called.
//
// Using a line
// ------------
// Only for a successful exec of a target under the fresh tmpfs, and only when,
// in this order,
//
//	the pid is not refused                                      else exechead-pair
//	the line's syscall is the event's                           else exechead-pair
//	the header's kernel series is on the table (7.0)            else exechead-kernel
//	the status is `confirmed`                                   else exechead-<status>
//	the line's path is non-empty and equals the trace's         else exechead-pair
//	  decoded path byte for byte
//
// A non-confirmed line carries no evidence, whatever else it holds.  An `outside`
// line for a path the classifier thinks is under the tmpfs refuses
// (`exechead-outside`): the two disagree about where the file is.
//
// What the classifier does with a believed head is exactly what it does with a
// head read from disk, minus checkStability: the loader comes from the head
// through the same loadChain walk and is observed like any read, and the head
// itself was read from the pinned inode, so there is nothing to compare later.

// trustedKernelSeries is the table of kernel series (major.minor) on which the
// supervisor's two kernel facts (ETXTBSY write-deny, SECCOMP CONTINUE) were run
// by the real-kernel tests of D48 step 4.  A series nobody ran them on is not
// evidence.  Matched EXACTLY.
var trustedKernelSeries = []string{"7.0"}

// ExecHeadSubCodes are the `exechead-*` sub-codes of the exec-unreadable census
// kind, from the classifier's own vocabulary (censusSubCodes), for a consumer
// that keeps a copy and must be held equal to it.
func ExecHeadSubCodes() []string {
	var out []string
	for _, c := range censusSubCodes["trace-incomplete:exec-unreadable"] {
		if len(c) > 9 && c[:9] == "exechead-" {
			out = append(out, c)
		}
	}
	return out
}

// KernelTrusted is kernelTrusted for the census, which reports whether a shard's
// kernel (the binding's `uname -r`) was on the table, so a gate can insist that
// `exechead-kernel` never fires on a trusted series.
func KernelTrusted(release string) bool { return kernelTrusted(release) }

// kernelTrusted says the header's `uname -r` is of a series on the table.  The
// release is "major.minor" then end of text or one of ".", "-", "+", "~", "_"
// (7.0.0-1014-aws, 7.0.1, 7.0-rc2); digits are compared as written, so "7.00",
// "7.01" and "7.0x" are not "7.0", and "7.10" is the series 7.10.  An empty or
// unparseable release is refused.
func kernelTrusted(release string) bool {
	i := 0
	digits := func() string {
		s := i
		for i < len(release) && release[i] >= '0' && release[i] <= '9' {
			i++
		}
		return release[s:i]
	}
	major := digits()
	if major == "" || i >= len(release) || release[i] != '.' {
		return false
	}
	i++
	minor := digits()
	if minor == "" {
		return false
	}
	if i < len(release) {
		switch release[i] {
		case '.', '-', '+', '~', '_':
		default:
			return false
		}
	}
	series := major + "." + minor
	for _, ok := range trustedKernelSeries {
		if series == ok {
			return true
		}
	}
	return false
}

// execVerdict is what the pairing concluded about ONE exec event of a trace.
type execVerdict struct {
	// believed says head is the bytes the kernel loaded.  When false, sub is the
	// census sub-code (exechead-*) the exec is refused under.
	believed bool
	head     []byte
	sub      string
}

// execEvent says ev is a call whose exec the supervisor is notified of.
func execEvent(ev *trace.Event) bool {
	return ev.Kind == trace.KindProcess && (ev.Syscall == "execve" || ev.Syscall == "execveat")
}

// The sub-codes.  Each is a string literal returned from this one function, so
// the vocabulary is closed (the census, like the record, never carries text taken
// from a file or a trace).
func statusSub(s exechead.Status) string {
	switch s {
	case exechead.StatusOutside:
		return "exechead-outside"
	case exechead.StatusGone:
		return "exechead-gone"
	case exechead.StatusMismatch:
		return "exechead-mismatch"
	case exechead.StatusTimeout:
		return "exechead-timeout"
	case exechead.StatusUnpinned:
		return "exechead-unpinned"
	case exechead.StatusFexecve:
		return "exechead-fexecve"
	case exechead.StatusSameImage:
		return "exechead-same-image"
	case exechead.StatusFailed:
		return "exechead-failed"
	case exechead.StatusOverload:
		return "exechead-overload"
	case exechead.StatusTainted:
		return "exechead-tainted"
	}
	// A status outside the enum cannot reach here (Decode refuses it); a
	// confirmed one is never a refusal.  The pairing code is the safe answer.
	return "exechead-pair"
}

// execHeadSubCodes is every exec-unreadable sub-code the exec head can raise, for
// the census's closed vocabulary (censusSubCodes).
var execHeadSubCodes = []string{
	"exechead-pair", "exechead-kernel", "exechead-outside", "exechead-gone", "exechead-mismatch",
	"exechead-timeout", "exechead-unpinned", "exechead-fexecve", "exechead-same-image",
	"exechead-failed", "exechead-overload", "exechead-tainted", "exechead-notelf",
}

// undecided says a line's status leaves the exec's outcome unknown: the process
// may have been running another exec at the time.
func undecided(s exechead.Status) bool {
	switch s {
	case exechead.StatusOverload, exechead.StatusTimeout, exechead.StatusGone, exechead.StatusTainted:
		return true
	}
	return false
}

// pairExecHeads pairs every exec event of t with its sidecar line and returns
// the verdict of each, by event index.  Only a pid that has an exec event gets
// anything: a line of a pid the trace never saw exec is simply never consulted.
func pairExecHeads(t *trace.Trace, f *exechead.File) map[int]execVerdict {
	events := map[int][]int{} // pid -> indexes of its exec events, in trace order
	for i := range t.Events {
		if ev := &t.Events[i]; execEvent(ev) {
			events[ev.Pid] = append(events[ev.Pid], i)
		}
	}
	lines := map[int][]*exechead.Line{} // tgid -> its lines, in sidecar order
	nonLeader := map[int]bool{}
	for k := range f.Lines {
		l := &f.Lines[k]
		lines[l.Tgid] = append(lines[l.Tgid], l)
		if l.Tid != l.Tgid {
			nonLeader[l.Tgid] = true
		}
	}
	kernelOK := kernelTrusted(f.Header.Kernel)
	out := make(map[int]execVerdict, len(t.Events))
	for pid, idxs := range events {
		ls := lines[pid]
		// The pid-level refusals (see the header comment).
		refused := t.InitPid == 0 || len(ls) != len(idxs) || nonLeader[pid]
		for _, i := range idxs {
			if t.Events[i].Result.Errno == "?" {
				refused = true
			}
		}
		if !refused {
			sawUndecided := false
			for n, i := range idxs {
				if undecided(ls[n].Status) {
					sawUndecided = true
				}
				// A status that contradicts the trace's result is slip evidence.
				// `confirmed` (exe became the file) and `mismatch` (exe changed)
				// both say an exec SUCCEEDED, so paired with a FAILED event the
				// ordinals have slipped: refuse the pid, wherever it occurs.
				// The reverse is NOT a contradiction: a `failed` line (exe
				// unchanged) beside a successful exec is legitimate, e.g. `sh`
				// running a tmpfs script whose interpreter is sh itself.
				if (ls[n].Status == exechead.StatusConfirmed || ls[n].Status == exechead.StatusMismatch) && !t.Events[i].Result.OK {
					refused = true
				}
				if ls[n].Status == exechead.StatusConfirmed && sawUndecided && !t.Events[i].Result.OK {
					refused = true
				}
			}
		}
		for n, i := range idxs {
			ev := &t.Events[i]
			if refused {
				out[i] = execVerdict{sub: "exechead-pair"}
				continue
			}
			l := ls[n]
			switch {
			case l.Syscall != ev.Syscall:
				out[i] = execVerdict{sub: "exechead-pair"}
			case !kernelOK:
				out[i] = execVerdict{sub: "exechead-kernel"}
			case l.Status != exechead.StatusConfirmed:
				out[i] = execVerdict{sub: statusSub(l.Status)}
			case l.Path == "" || l.Path != ev.Path:
				out[i] = execVerdict{sub: "exechead-pair"}
			default:
				out[i] = execVerdict{believed: true, head: l.Head}
			}
		}
	}
	return out
}

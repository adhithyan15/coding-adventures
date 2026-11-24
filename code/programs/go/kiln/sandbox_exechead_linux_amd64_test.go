//go:build linux && amd64

package main

// KILN04 D48 step 4, the supervisor against a real kernel.  Every test starts
// the step under real strace in the real sandbox with `--exec-notify-fd`, with
// an exechead.Supervisor on the other end (sandbox_exechead_harness_linux_amd64_test.go),
// and judges the `<step>.exechead` file it wrote, decoded strictly.
//
// What each test pins, against the decision's step-4 list:
//
//	Confirmed                 a first exec of a tmpfs binary from a different image
//	SameImage                 a self re-exec is `same-image`
//	Gone                      a fast exit before the confirm is `gone`
//	Script                    a `#!` script under the tmpfs is `mismatch` or `gone`
//	UnpinnedAndFexecve        `unpinned` (NULL, missing, non-UTF-8) and `fexecve`
//	Resolution                relative, `..`, symlink and execveat dirfd paths
//	OutsideThenTmpfsChain     an in-place chain in one pid confirms (same-thread rule)
//	SettledAtNextSameThread   a pending pin is settled by the next same-thread notification
//	PollerAndSettleRace       the poller and the settle never both write a line
//	Failed                    an exec that returns EACCES is `failed`
//	SameThreadBypass          a same-thread notification bypasses the held queue
//	HeldUntilDecided          a different thread's notification is held, not tainted
//	HeldNotificationOfADead   a held notification whose thread dies is dropped, no taint
//	SignalInterrupted         an interrupted-and-restarted notification is dropped
//	TwoThreadOverlapTaints    two threads, one group: taint, never confirmed
//	NonLeaderExec             a non-leader exec is recorded with tid != tgid
//	Overload                  the confirm cap: `overload`, and the step still runs
//	CapExceeded               the sidecar's line cap: unclean, still answered
//	HeaderAndSeq              the header and the sequence numbers
//	LostHandover              no listener: the receive deadline, then unclean
//	SupervisorKilledMidStep   a listener that dies mid-step: unclean
//	StallKillsTheStep         the stall deadline kills the step
//	PendingLifetimeKills      the pending-lifetime bound kills the step
//	ThreadSwapNeverConfirms   a path flipped by another thread never falsely confirms

import (
	"bytes"
	"os"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
)

func TestSandboxExecHeadConfirmed(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `cp {bin} {tmp}/s && stat -c 'ino=%i' {tmp}/s && stat -c 'dev=%d' {tmp}/s && {tmp}/s `+execHeadProbeCommand+` hold; true`, ehOpt{})
	r.clean()
	if r.exit != 0 {
		t.Fatalf("exit %d", r.exit)
	}
	l := r.one(f.tmp + "/s")
	if l.Status != exechead.StatusConfirmed || l.Syscall != exechead.SyscallExecve || l.Tid != l.Tgid {
		t.Fatalf("%s", formatLine(l))
	}
	ino, _ := strconv.ParseUint(r.printed("ino")[0], 10, 64)
	dev, _ := strconv.ParseUint(r.printed("dev")[0], 10, 64)
	if l.Ino != ino || l.Dev != dev || ino == 0 {
		t.Errorf("the line says dev %d ino %d, the file is dev %d ino %d", l.Dev, l.Ino, dev, ino)
	}
	host, err := os.ReadFile(r.bin)
	if err != nil {
		t.Fatal(err)
	}
	want := host[:min(len(host), exechead.HeadLimit)]
	if !bytes.Equal(l.Head, want) || !bytes.HasPrefix(l.Head, []byte("\x7fELF")) {
		t.Errorf("the head (%d bytes) is not the first bytes of the file", len(l.Head))
	}
	// A host binary is only a placeholder.
	var sawCp bool
	for _, x := range r.file.Lines {
		if strings.HasSuffix(x.Path, "/cp") {
			sawCp = true
			if x.Status != exechead.StatusOutside {
				t.Errorf("cp: %s", formatLine(x))
			}
		}
	}
	if !sawCp {
		t.Error("no line for cp")
	}
}

func TestSandboxExecHeadSameImage(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `cp {bin} {tmp}/p && {tmp}/p `+execHeadProbeCommand+` selfexec; true`, ehOpt{})
	r.clean()
	ls := r.byPath(f.tmp + "/p")
	if len(ls) != 2 || ls[0].Status != exechead.StatusConfirmed || ls[1].Status != exechead.StatusSameImage {
		t.Fatalf("lines for the tmpfs binary: %v", ls)
	}
	if ls[0].Tgid != ls[1].Tgid || ls[0].Tid != ls[1].Tid {
		t.Errorf("the two execs are of one thread")
	}
}

func TestSandboxExecHeadGone(t *testing.T) {
	f := newSandboxFixture(t)
	// The poller's first look is 600 ms late: the process is long gone.
	r := f.ehScript(t, `cp /bin/true {tmp}/t && {tmp}/t; true`, ehOpt{timing: exechead.Timing{PollStart: 600 * time.Millisecond}})
	r.clean()
	if l := r.one(f.tmp + "/t"); l.Status != exechead.StatusGone {
		t.Fatalf("%s", formatLine(l))
	}
}

func TestSandboxExecHeadScript(t *testing.T) {
	f := newSandboxFixture(t)
	// The interpreter is not the forked shell's image, so exe becomes /bin/cat:
	// not the pin.  (If the process is gone before the first look it is `gone`;
	// both refuse.)
	r := f.ehScript(t, `printf '#!/bin/cat\n' > {tmp}/sc && chmod +x {tmp}/sc && {tmp}/sc; true`, ehOpt{})
	r.clean()
	l := r.one(f.tmp + "/sc")
	if l.Status != exechead.StatusMismatch && l.Status != exechead.StatusGone {
		t.Fatalf("a script under the tmpfs: %s", formatLine(l))
	}
	r.none(exechead.StatusConfirmed)
}

func TestSandboxExecHeadUnpinnedAndFexecve(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `{bin} `+execHeadProbeCommand+` misc`, ehOpt{})
	r.clean()
	if r.exit != 0 {
		t.Fatalf("exit %d: fexecve of true should have run it", r.exit)
	}
	// In order: NULL (path ""), missing, non-UTF-8 (path ""), fexecve.
	var seq []exechead.Line
	for _, l := range r.file.Lines {
		if l.Path == "" || l.Path == "/nonexistent/x" {
			seq = append(seq, l)
		}
	}
	want := []struct {
		path string
		st   exechead.Status
	}{{"", exechead.StatusUnpinned}, {"/nonexistent/x", exechead.StatusUnpinned}, {"", exechead.StatusUnpinned}, {"", exechead.StatusFexecve}}
	if len(seq) != len(want) {
		t.Fatalf("%d lines of interest, want %d", len(seq), len(want))
	}
	for i, w := range want {
		if seq[i].Path != w.path || seq[i].Status != w.st {
			t.Errorf("line %d: %s, want %s %s", i, formatLine(seq[i]), w.path, w.st)
		}
	}
	if seq[3].Syscall != exechead.SyscallExecveat {
		t.Errorf("fexecve is an execveat: %s", formatLine(seq[3]))
	}
	for _, l := range seq {
		if l.Tid != l.Tgid {
			t.Errorf("the leader makes these calls: %s", formatLine(l))
		}
	}
}

func TestSandboxExecHeadResolution(t *testing.T) {
	f := newSandboxFixture(t)
	t.Run("relative to the cwd", func(t *testing.T) {
		r := f.ehScript(t, `cd {tmp} && cp {bin} rel && ./rel `+execHeadProbeCommand+` hold; true`, ehOpt{})
		r.clean()
		if l := r.one("./rel"); l.Status != exechead.StatusConfirmed {
			t.Fatalf("%s", formatLine(l))
		}
	})
	t.Run("a relative path that climbs out of its base", func(t *testing.T) {
		r := f.ehScript(t, `mkdir {tmp}/sub && cp {bin} {tmp}/up && cd {tmp}/sub && ../up `+execHeadProbeCommand+` hold; true`, ehOpt{})
		r.clean()
		if l := r.one("../up"); l.Status != exechead.StatusUnpinned {
			t.Fatalf("RESOLVE_BENEATH refuses a '..' above the base: %s", formatLine(l))
		}
	})
	t.Run("through an absolute symlink", func(t *testing.T) {
		r := f.ehScript(t, `cp {bin} {tmp}/real && ln -s {tmp}/real {tmp}/lnk && stat -c 'ino=%i' {tmp}/real && {tmp}/lnk `+execHeadProbeCommand+` hold; true`, ehOpt{})
		r.clean()
		l := r.one(f.tmp + "/lnk")
		ino, _ := strconv.ParseUint(r.printed("ino")[0], 10, 64)
		if l.Status != exechead.StatusConfirmed || l.Ino != ino {
			t.Fatalf("%s, the target's inode is %d", formatLine(l), ino)
		}
	})
	for _, mode := range []string{"dir", "high", "cwd"} {
		t.Run("execveat dirfd "+mode, func(t *testing.T) {
			r := f.ehScript(t, `cp {bin} {tmp}/ex && {bin} `+execHeadProbeCommand+` execveat `+mode+` {tmp} ex`, ehOpt{})
			r.clean()
			l := r.one("ex")
			if l.Status != exechead.StatusConfirmed || l.Syscall != exechead.SyscallExecveat {
				t.Fatalf("%s", formatLine(l))
			}
		})
	}
}

func TestSandboxExecHeadOutsideThenTmpfsChain(t *testing.T) {
	f := newSandboxFixture(t)
	// The shell is a host binary (an `outside` line); it then execs a tmpfs binary
	// IN PLACE, the same pid and thread.  The same-thread rule: no taint.
	r := f.ehScript(t, `cp {bin} {tmp}/s && exec {tmp}/s `+execHeadProbeCommand+` hold`, ehOpt{})
	r.clean()
	l := r.one(f.tmp + "/s")
	if l.Status != exechead.StatusConfirmed {
		t.Fatalf("%s", formatLine(l))
	}
	r.none(exechead.StatusTainted)
	// The previous line of the same process is the host shell, `outside`: the
	// chain is outside, then tmpfs, in one pid.
	var prev *exechead.Line
	for i := range r.file.Lines {
		x := &r.file.Lines[i]
		if x.Seq < l.Seq && x.Tgid == l.Tgid {
			prev = x
		}
	}
	if prev == nil || prev.Tid != l.Tid || prev.Status != exechead.StatusOutside {
		t.Errorf("the previous exec of the same thread is not the host shell: %v", prev)
	}
}

func TestSandboxExecHeadSettledAtNextSameThreadNotification(t *testing.T) {
	f := newSandboxFixture(t)
	// The poller's first look is 2 s late, so the ONLY thing that can write the
	// first line before the step ends is the next notification of the same thread
	// (the probe, once it is a, execs b).  A poller that looked late would find
	// exe already b and say mismatch or gone.
	r := f.ehScript(t, `cp {bin} {tmp}/a && cp /bin/true {tmp}/b && exec {tmp}/a `+execHeadProbeCommand+` chain {tmp}/b`,
		ehOpt{timing: exechead.Timing{PollStart: 2 * time.Second}})
	r.clean()
	if a := r.one(f.tmp + "/a"); a.Status != exechead.StatusConfirmed {
		t.Fatalf("the first exec was not settled by the next one: %s", formatLine(a))
	}
	if b := r.one(f.tmp + "/b"); b.Status != exechead.StatusGone && b.Status != exechead.StatusConfirmed {
		t.Fatalf("%s", formatLine(b))
	}
	ia := r.index(f.tmp+"/a", exechead.StatusConfirmed)
	for i, l := range r.file.Lines {
		if l.Path == f.tmp+"/b" && i < ia {
			t.Error("the lines of one thread are out of order: b before a")
		}
	}
}

func TestSandboxExecHeadPollerAndSettleRaceWritesOneLine(t *testing.T) {
	f := newSandboxFixture(t)
	var loop strings.Builder
	for i := 0; i < 25; i++ {
		loop.WriteString(` sh -c 'exec {tmp}/a ` + execHeadProbeCommand + ` chain {tmp}/b';`)
	}
	// Each child: a (pinned, its poller racing) then, in place, b (the same
	// thread's next notification, which settles a if the poller has not).
	r := f.ehScript(t, `cp {bin} {tmp}/a && cp /bin/true {tmp}/b;`+loop.String()+` true`, ehOpt{})
	r.clean()
	as, bs := r.byPath(f.tmp+"/a"), r.byPath(f.tmp+"/b")
	if len(as) != 25 || len(bs) != 25 {
		t.Fatalf("%d lines for a and %d for b, want 25 each: one line per exec, never two", len(as), len(bs))
	}
	for _, l := range as {
		if l.Status != exechead.StatusConfirmed {
			t.Errorf("a: %s", formatLine(l))
		}
	}
	for _, l := range bs {
		if l.Status != exechead.StatusConfirmed && l.Status != exechead.StatusGone {
			t.Errorf("b: %s", formatLine(l))
		}
	}
	seen := map[int]int{}
	for _, l := range as {
		seen[l.Tgid]++
	}
	for tgid, n := range seen {
		if n != 1 {
			t.Errorf("process %d has %d lines for a", tgid, n)
		}
	}
}

func TestSandboxExecHeadFailed(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `cp /bin/true {tmp}/nx && chmod 644 {tmp}/nx && {bin} `+execHeadProbeCommand+` execfail {tmp}/nx`, ehOpt{})
	r.clean()
	if l := r.one(f.tmp + "/nx"); l.Status != exechead.StatusFailed {
		t.Fatalf("an exec that returned EACCES: %s", formatLine(l))
	}
	if got := r.printed("execfail"); len(got) != 1 || !strings.Contains(got[0], "permission denied") {
		t.Errorf("execfail=%v", got)
	}
}

func TestSandboxExecHeadSameThreadBypass(t *testing.T) {
	f := newSandboxFixture(t)
	// a (thread 1): pending, its poller asleep for 1.5 s.  c (thread 2, +50 ms): held
	// behind a.  b (thread 1 again, +100 ms): the SAME thread as a, so a is over;
	// b settles a at receipt and bypasses the queue.  c is released after b and,
	// because b left no outcome and c is another thread, is tainted.
	r := f.ehScript(t, `cp /bin/true {tmp}/nx && chmod 644 {tmp}/nx && {bin} `+execHeadProbeCommand+` bypass {tmp}/nx`,
		ehOpt{timing: exechead.Timing{PollStart: 1500 * time.Millisecond}})
	r.clean()
	ia, ib, ic := r.index(f.tmp+"/nx", exechead.StatusFailed), r.index("/nonexistent/b", exechead.StatusUnpinned), r.index("/nonexistent/c", exechead.StatusTainted)
	if ia < 0 || ib < 0 || ic < 0 || !(ia < ib && ib < ic) {
		t.Fatalf("a failed at %d, b unpinned at %d, c tainted at %d; want that order", ia, ib, ic)
	}
	ms, err := strconv.Atoi(r.printed("b-returned-ms")[0])
	if err != nil || ms >= 1000 {
		t.Fatalf("b returned after %d ms (%v): it waited for the slow poller instead of bypassing the queue", ms, err)
	}
}

func TestSandboxExecHeadHeldUntilDecided(t *testing.T) {
	f := newSandboxFixture(t)
	// a is pending; another thread's exec (+50 ms) is held; at 300 ms the poller
	// finds a failed, which releases the queue: the held exec is then handled
	// normally, not tainted.
	r := f.ehScript(t, `cp /bin/true {tmp}/nx && chmod 644 {tmp}/nx && {bin} `+execHeadProbeCommand+` held {tmp}/nx`,
		ehOpt{timing: exechead.Timing{PollStart: 300 * time.Millisecond}})
	r.clean()
	ia, ih := r.index(f.tmp+"/nx", exechead.StatusFailed), r.index("/nonexistent/h", exechead.StatusUnpinned)
	if ia < 0 || ih < 0 || ia > ih {
		t.Fatalf("a failed at %d, the held exec unpinned at %d", ia, ih)
	}
	r.none(exechead.StatusTainted)
}

func TestSandboxExecHeadHeldNotificationOfADeadThreadIsDropped(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `cp /bin/true {tmp}/nx && chmod 644 {tmp}/nx && {bin} `+execHeadProbeCommand+` dethread {tmp}/nx`,
		ehOpt{timing: exechead.Timing{PollStart: 600 * time.Millisecond}})
	r.clean()
	if got := r.byPath("/nonexistent/c"); len(got) != 0 {
		t.Fatalf("the held exec of a thread that died left a line: %v", got)
	}
	r.none(exechead.StatusTainted)
	if l := r.one(f.tmp + "/nx"); l.Status != exechead.StatusGone {
		t.Fatalf("%s", formatLine(l))
	}
}

func TestSandboxExecHeadSignalInterrupted(t *testing.T) {
	f := newSandboxFixture(t)
	// The processor is slow (300 ms per item), so the first notification of the
	// probe's exec is still unanswered when a signal interrupts it: the kernel
	// restarts the call (ERESTARTNOINTR) with a NEW notification.  The old one is
	// no longer valid, is dropped and leaves no line.
	r := f.ehScript(t, `{bin} `+execHeadProbeCommand+` signal`, ehOpt{timing: exechead.Timing{ProcessDelay: 300 * time.Millisecond}})
	r.clean()
	if n := len(r.byPath("/nonexistent/sig")); n != 1 {
		t.Fatalf("%d lines for the restarted exec, want 1", n)
	}
	r.none(exechead.StatusTainted)
	if r.file.Header.WaitKillable {
		// SECCOMP_FILTER_FLAG_WAIT_KILLABLE_RECV (Linux 5.19): a signal cannot
		// interrupt an unanswered notification at all, so there is no restart and
		// nothing to drop.  The restart path (ERESTARTNOINTR, a second notification,
		// the first no longer valid) exists only on a kernel that refuses the flag;
		// it is exercised there, and this branch pins that the flag removes it.
		if r.res.Dropped != 0 {
			t.Errorf("a killable listener dropped %d notifications", r.res.Dropped)
		}
		return
	}
	if r.res.Dropped < 1 {
		t.Errorf("no notification was dropped as invalid (Dropped = %d)", r.res.Dropped)
	}
}

func TestSandboxExecHeadTwoThreadOverlapTaints(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `cp /bin/true {tmp}/ok && {bin} `+execHeadProbeCommand+` overlap {tmp}/ok`, ehOpt{})
	r.clean()
	// Both threads are not the leader whose exec of the probe came first and was
	// released without an outcome, so the FIRST second-thread notification already
	// taints the group: a1, b1 and the leader's later exec are all `tainted`.
	ia, ib := r.index("/nonexistent/a1", exechead.StatusTainted), r.index("/nonexistent/b1", exechead.StatusTainted)
	if ia < 0 || ib < 0 || ia > ib {
		t.Fatalf("a1 tainted at %d, b1 tainted at %d", ia, ib)
	}
	if a, b := r.one("/nonexistent/a1"), r.one("/nonexistent/b1"); a.Tid == b.Tid || a.Tgid != b.Tgid {
		t.Fatalf("a1 and b1 must be two threads of one group: %s, %s", formatLine(a), formatLine(b))
	}
	if l := r.one(f.tmp + "/ok"); l.Status != exechead.StatusTainted {
		t.Fatalf("a later exec of a tainted group: %s", formatLine(l))
	}
	r.none(exechead.StatusConfirmed)
}

func TestSandboxExecHeadNonLeaderExec(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `cp /bin/sh {tmp}/sh2 && {bin} `+execHeadProbeCommand+` nonleader {tmp}/sh2 -c 'exec /bin/true'`, ehOpt{})
	r.clean()
	l := r.one(f.tmp + "/sh2")
	if l.Tid == l.Tgid {
		t.Fatalf("the exec was made by a non-leader thread, its line must say so: %s", formatLine(l))
	}
	// By design (D48 step 1) the group is tainted: the leader's exec of the probe
	// was released without a definite outcome, and a notification from a DIFFERENT
	// tid follows, which is an overlap the supervisor cannot rule out.  So the
	// non-leader's own line and every later line of the group are `tainted`, never
	// `confirmed`, and the next exec (by the image that took over the leader's pid)
	// is tainted too.  PR 5's pairing refuses the group twice over (tid != tgid).
	if l.Status != exechead.StatusTainted {
		t.Errorf("the non-leader's line: %s", formatLine(l))
	}
	n := r.one("/bin/true")
	if n.Status != exechead.StatusTainted || n.Tid != n.Tgid || n.Tgid != l.Tgid {
		t.Fatalf("the next exec of the same group: %s (after %s)", formatLine(n), formatLine(l))
	}
	r.none(exechead.StatusConfirmed)
}

func TestSandboxExecHeadOverload(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `cp {bin} {tmp}/s1 && cp {bin} {tmp}/s2 && ({tmp}/s1 `+execHeadProbeCommand+` hold & {tmp}/s2 `+execHeadProbeCommand+` hold & wait); true`,
		ehOpt{timing: exechead.Timing{MaxConfirms: 1, PollStart: 1200 * time.Millisecond}})
	r.clean()
	if r.exit != 0 {
		t.Fatalf("exit %d: the supervisor must keep answering", r.exit)
	}
	var over, other int
	for _, p := range []string{f.tmp + "/s1", f.tmp + "/s2"} {
		l := r.one(p)
		if l.Status == exechead.StatusOverload {
			over++
		} else {
			other++
		}
	}
	if over != 1 || other != 1 {
		t.Fatalf("%d overload and %d other lines for two concurrent execs under a cap of one", over, other)
	}
}

func TestSandboxExecHeadCapExceeded(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `echo a; /bin/true; echo b; /bin/true; echo c`, ehOpt{timing: exechead.Timing{File: exechead.Limits{MaxLines: 2}}})
	r.unclean(exechead.UncleanOverflow)
	if r.exit != 0 || r.stdout != "a\nb\nc\n" {
		t.Fatalf("exit %d, stdout %q: the supervisor must go on answering past its cap", r.exit, r.stdout)
	}
	if len(r.file.Lines) != 2 {
		t.Fatalf("%d lines written, the cap is 2", len(r.file.Lines))
	}
}

func TestSandboxExecHeadHeaderAndSeq(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `/bin/true; /bin/true`, ehOpt{})
	r.clean()
	if len(r.file.Lines) < 3 {
		t.Fatalf("%d lines", len(r.file.Lines))
	}
	h := r.file.Header
	if h.V != exechead.Version || h.Nonce != r.hdr.Nonce || h.ActionSHA256 != r.hdr.ActionSHA256 || h.Step != 0 {
		t.Fatalf("%+v", h)
	}
	first := r.file.Lines[0]
	if first.Seq != 0 || first.Tid != first.Tgid || first.Status != exechead.StatusOutside || first.Syscall != exechead.SyscallExecve {
		t.Errorf("the first line is the init's own exec of the step: %s", formatLine(first))
	}
	// A file whose header names another nonce (another step's) does not bind.
	other := r.hdr
	other.Nonce = strings.Repeat("f", 32)
	var raw bytes.Buffer
	if _, err := exechead.NewWriter(&raw, other, exechead.Limits{}); err != nil {
		t.Fatal(err)
	}
	if _, err := exechead.Decode(raw.Bytes(), exechead.Limits{}, exechead.Expect{Nonce: r.hdr.Nonce, ActionSHA256: r.hdr.ActionSHA256, Step: 0}); err == nil {
		t.Error("a file with another nonce was accepted")
	}
}

func TestSandboxExecHeadLostHandover(t *testing.T) {
	f := newSandboxFixture(t)
	// The sandbox is given no --exec-notify-fd, so the listener never comes, and
	// strace holds its own copy of the socket for as long as it lives: end of file
	// would never arrive.  The receive deadline is the only way out.
	r := f.ehScript(t, `sleep 60`, ehOpt{noOption: true, timing: exechead.Timing{Handover: 1500 * time.Millisecond, FinishWait: 2 * time.Second}})
	r.unclean(exechead.UncleanHandover)
	if r.res.Received {
		t.Error("a listener out of nothing")
	}
	if r.elapsed > 30*time.Second {
		t.Fatalf("the step took %s: the supervisor did not kill it at the deadline", r.elapsed)
	}
	if len(r.file.Lines) != 0 {
		t.Errorf("%d lines", len(r.file.Lines))
	}
}

func TestSandboxExecHeadSupervisorKilledMidStep(t *testing.T) {
	f := newSandboxFixture(t)
	// The listener closes at 700 ms: the exec of /bin/true, later, fails (a dead
	// listener makes notified calls fail with ENOSYS), and the step is unclean.
	r := f.ehScript(t, `echo before; sleep 1.5; /bin/true; echo after=$?`, ehOpt{abortAt: 700 * time.Millisecond})
	r.unclean(exechead.UncleanFailed)
	if !strings.Contains(r.stdout, "before\n") {
		t.Errorf("stdout %q", r.stdout)
	}
	if strings.Contains(r.stdout, "after=0") {
		t.Errorf("an exec after the listener died succeeded: %q", r.stdout)
	}
}

func TestSandboxExecHeadStallKillsTheStep(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.ehScript(t, `true`, ehOpt{timing: exechead.Timing{Stall: 300 * time.Millisecond, ProcessDelay: 5 * time.Second, FinishWait: 2 * time.Second}})
	r.unclean(exechead.UncleanStall)
	if r.elapsed > 30*time.Second {
		t.Fatalf("the step ran for %s", r.elapsed)
	}
}

func TestSandboxExecHeadPendingLifetimeTaintsAHealthyStep(t *testing.T) {
	f := newSandboxFixture(t)
	// The poller does not look for 3 s and the lifetime bound is 800 ms.  The
	// process stays alive for 1.5 s (`hold`, a copy of the probe: NOT a copy of
	// /bin/sleep, which on Ubuntu 26.04 is a multi-call binary that exits at once
	// under another name, which made this test racy there) so it is alive when the
	// bound is checked.  It is NOT blocked in execve (it is asleep in its new image), so the bound
	// does not kill the step: the group is tainted and the line is `timeout`.  (A
	// thread seen blocked in execve past the bound is killed as `exechead-stuck`;
	// no test can hold a real thread there, so that branch is by inspection.)
	r := f.ehScript(t, `cp {bin} {tmp}/s && {tmp}/s `+execHeadProbeCommand+` hold 1500; true`,
		ehOpt{timing: exechead.Timing{Lifetime: 800 * time.Millisecond, PollStart: 3 * time.Second, FinishWait: 6 * time.Second}})
	r.clean()
	if r.exit != 0 {
		t.Fatalf("exit %d: a healthy step was killed", r.exit)
	}
	if l := r.one(f.tmp + "/s"); l.Status != exechead.StatusTimeout {
		t.Fatalf("%s", formatLine(l))
	}
	r.none(exechead.StatusConfirmed)
}

func TestSandboxExecHeadReceiverDeathKillsTheStep(t *testing.T) {
	f := newSandboxFixture(t)
	// The receiver fails after its second notification (an injected error, zero in
	// production).  Without the fix the listener would stay open with nobody
	// answering, and the next exec would block for ever.
	r := f.ehScript(t, `/bin/true; /bin/true; sleep 30`, ehOpt{timing: exechead.Timing{InjectReceiverError: 2, FinishWait: 3 * time.Second}})
	r.unclean(exechead.UncleanReceiver)
	if r.elapsed > 25*time.Second {
		t.Fatalf("the step ran for %s: it was not killed", r.elapsed)
	}
}

func TestSandboxExecHeadRenameByAnotherProcessNeverFalselyConfirms(t *testing.T) {
	f := newSandboxFixture(t)
	var loop strings.Builder
	for i := 0; i < 30; i++ {
		loop.WriteString(` {bin} ` + execHeadProbeCommand + ` racepath {tmp}/p ` + strconv.Itoa(i) + `;`)
	}
	// A SEPARATE PROCESS (a background shell loop) renames one file and then the
	// other over {tmp}/p, atomically, while the step execs {tmp}/p: the kernel's
	// lookup may find a different inode than the one pinned.  Each try prints
	// try=<k> before the exec and the new image prints ino=<inode of its exe>
	// after it, so a try whose exec printed nothing (it failed, or the image did
	// not run) is told apart from the rest and never shifts the others.  Whatever
	// happens, a `confirmed` line must name the inode that really ran, and its head
	// must be the head of that file (both files are copies of the probe binary).
	flip := `( while [ ! -f {tmp}/stop ]; do ln -f {tmp}/fa {tmp}/t1 && mv -f {tmp}/t1 {tmp}/p; ln -f {tmp}/fb {tmp}/t2 && mv -f {tmp}/t2 {tmp}/p; done ) &`
	r := f.ehScript(t, `cp {bin} {tmp}/fa && cp {bin} {tmp}/fb && ln {tmp}/fa {tmp}/p && `+flip+loop.String()+` touch {tmp}/stop; wait; true`, ehOpt{})
	r.clean()
	var raced []exechead.Line
	for _, l := range r.file.Lines {
		if l.Path == f.tmp+"/p" {
			raced = append(raced, l)
		}
	}
	if len(raced) != 30 {
		t.Fatalf("%d lines for the raced exec, want 30", len(raced))
	}
	// reported[k] is the inode the k-th try's new image printed, if it did.
	reported := map[int]string{}
	k := -1
	for _, line := range strings.Split(r.stdout, "\n") {
		switch {
		case strings.HasPrefix(line, "try="):
			k, _ = strconv.Atoi(strings.TrimPrefix(line, "try="))
		case strings.HasPrefix(line, "ino=") && k >= 0:
			reported[k] = strings.TrimSpace(strings.TrimPrefix(line, "ino="))
		}
	}
	host, err := os.ReadFile(r.bin)
	if err != nil {
		t.Fatal(err)
	}
	want := host[:min(len(host), exechead.HeadLimit)]
	counts := map[exechead.Status]int{}
	verified := 0
	for i, l := range raced {
		counts[l.Status]++
		if l.Status != exechead.StatusConfirmed {
			continue
		}
		if !bytes.Equal(l.Head, want) {
			t.Errorf("try %d: the confirmed head is not the head of the file that ran", i)
		}
		got, ok := reported[i]
		if !ok {
			t.Logf("try %d: confirmed as inode %d, no report from the process to check it against", i, l.Ino)
			continue
		}
		verified++
		if strconv.FormatUint(l.Ino, 10) != got {
			t.Errorf("try %d: confirmed as inode %d, but the process that ran is inode %s", i, l.Ino, got)
		}
	}
	t.Logf("statuses of the 30 raced execs: %v; %d reports, %d confirmations checked against one", counts, len(reported), verified)
	if verified < 10 {
		t.Errorf("only %d confirmations could be checked against a report", verified)
	}
}

func TestSandboxExecHeadThreadSwapNeverFalselyConfirms(t *testing.T) {
	f := newSandboxFixture(t)
	var loop strings.Builder
	for i := 0; i < 30; i++ {
		loop.WriteString(` {bin} ` + execHeadProbeCommand + ` race {tmp}/ra {tmp}/rb;`)
	}
	// Each child execs a path whose last byte another thread flips between ra and
	// rb, so the kernel may run a file other than the one the supervisor pinned.
	// Whatever happens, a `confirmed` line must name the file that really ran:
	// the step prints the inode of /proc/self/exe after each exec.
	r := f.ehScript(t, `cp {bin} {tmp}/ra && cp {bin} {tmp}/rb;`+loop.String()+` true`, ehOpt{})
	r.clean()
	inos := r.printed("ino")
	var raced []exechead.Line
	for _, l := range r.file.Lines {
		if l.Path == f.tmp+"/ra" || l.Path == f.tmp+"/rb" {
			raced = append(raced, l)
		}
	}
	if len(raced) != 30 || len(inos) != 30 {
		t.Fatalf("%d lines for the raced exec and %d reports, want 30 each", len(raced), len(inos))
	}
	counts := map[exechead.Status]int{}
	for i, l := range raced {
		counts[l.Status]++
		if l.Status != exechead.StatusConfirmed {
			continue
		}
		if got := strconv.FormatUint(l.Ino, 10); got != inos[i] {
			t.Errorf("exec %d: confirmed as inode %s, but the process that ran is inode %s", i, got, inos[i])
		}
	}
	t.Logf("statuses of the 30 raced execs: %v", counts)
}

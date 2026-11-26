package traced

import (
	"debug/elf"
	"encoding/binary"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// KILN04 D48 step 5: the classifier believes an exec head recorded at exec time,
// but only when the supervisor's line PAIRS with the trace's exec event.  These
// tests pin every rule of the pairing and every refusal; the sub-code is read
// from the census (the record only ever says `exec-unreadable`).

const (
	// ehBin is a Go test binary under the step's fresh $TMPDIR: built, run and
	// deleted, so nothing can read it after the run.
	ehBin  = "/tmp/kiln-1/go-build/b001/x.test"
	ehBin2 = "/tmp/kiln-1/go-build/b002/y.test"
	// ehUnknownLoader is a PT_INTERP the classifier would otherwise never meet:
	// the host root list does not cover it, so observing it is a `host-read`.
	// Its appearance in the record is the proof the loader came from the HEAD.
	ehUnknownLoader = "/opt/unknown/ld.so"
	ehKernel        = "7.0.0-1014-aws"
)

func ehELF(interp string) []byte { return buildELF(interp, elf.ELFCLASS64, binary.LittleEndian) }

func ehConfirmed(tid, tgid int, path string, head []byte) exechead.Line {
	return exechead.Line{Tid: tid, Tgid: tgid, Syscall: exechead.SyscallExecve, Path: path, Status: exechead.StatusConfirmed,
		Dev: 58, Ino: 12, Head: head}
}

func ehPlain(pid int, path string, st exechead.Status) exechead.Line {
	return exechead.Line{Tid: pid, Tgid: pid, Syscall: exechead.SyscallExecve, Path: path, Status: st}
}

// ehFile is a decoded sidecar of these lines (Decode itself is tested in
// internal/exechead; here the lines are handed over as it would return them).
func ehFile(kernel string, lines ...exechead.Line) *exechead.File {
	f := &exechead.File{Header: exechead.Header{V: exechead.Version, Kernel: kernel}}
	for i, l := range lines {
		l.Seq = i
		f.Lines = append(f.Lines, l)
	}
	return f
}

// ehTrace is a helper-mode-shaped trace: the init is pid 1 and the events carry
// the pids they name (0 means the init).
func ehTrace(initPid int, evs ...trace.Event) *trace.Trace {
	pids := map[int]bool{initPid: true}
	for i := range evs {
		if evs[i].Pid == 0 {
			evs[i].Pid = initPid
		}
		pids[evs[i].Pid] = true
		evs[i].Line, evs[i].StartLine = i+1, i+1
	}
	tr := &trace.Trace{RootPid: initPid, InitPid: initPid, Events: evs, Procs: map[int]*trace.Process{}}
	for p := range pids {
		tr.Procs[p] = &trace.Process{Pid: p, Exit: &trace.Exit{Pid: p}}
		tr.Order = append(tr.Order, p)
		tr.Exits = append(tr.Exits, trace.Exit{Pid: p})
	}
	return tr
}

func execAs(pid int, path string) trace.Event {
	e := execve(path, "x")
	e.Pid = pid
	return e
}

func failExecAs(pid int, path, errno string) trace.Event { return fail(execAs(pid, path), errno) }

// ehRun classifies the trace with the given sidecar (nil: no supervisor) and
// returns the record and the census.
func (h *harness) ehRun(file *exechead.File, tr *trace.Trace) (*RawRecord, *Census) {
	h.t.Helper()
	h.fs.bytes("/opt/unknown/ld.so", []byte("loader"))
	h.image("/opt/unknown/ld.so")
	h.settle([]*trace.Trace{tr})
	c := NewCensus()
	in := Input{Package: "code/p/a", Traces: []*trace.Trace{tr}, Recipe: []string{"step"}, Census: c}
	if file != nil {
		in.ExecHeads = []*exechead.File{file}
	}
	rec, err := Classify(h.cfg, in)
	if err != nil {
		h.t.Fatal(err)
	}
	return rec, c
}

// subs are the exec-unreadable sub-codes the census holds.
func subs(c *Census) []string {
	var out []string
	for k := range censusKinds(c) {
		if s, ok := strings.CutPrefix(k, "trace-incomplete:exec-unreadable:"); ok {
			out = append(out, s)
		}
	}
	return out
}

// shell is the init's own exec: a host binary, a placeholder line.
func shellEvent() trace.Event {
	e := execve("/usr/bin/sh", "sh", "-c", "step") // the recipe line CheckRecipe expects
	e.Pid = 1
	return e
}

func mkdirAs(pid int, path string) trace.Event {
	e := mkdir(path)
	e.Pid = pid
	return e
}

// madeBy is the action building the test binary (the compiler writes it).
func madeBy(pid int, path string) trace.Event {
	e := open(path, "O_WRONLY|O_CREAT|O_TRUNC", path)
	e.Pid = pid
	return e
}
func shellLine() exechead.Line {
	return ehPlain(1, "/usr/bin/sh", exechead.StatusOutside)
}

// TestExecHeadIsBelievedAndTheLoaderComesFromIt is the positive case: the init's
// sh (host), then a child (pid 2) that execs the tmpfs test binary.  With a
// confirmed line whose path equals the trace's, the exec is not refused, and the
// loader the HEAD names is observed like any read.
func TestExecHeadIsBelievedAndTheLoaderComesFromIt(t *testing.T) {
	h := newHarness(t)
	tr := ehTrace(1, shellEvent(), execAs(2, ehBin))
	rec, c := h.ehRun(ehFile(ehKernel, shellLine(), ehConfirmed(2, 2, ehBin, ehELF(ehUnknownLoader))), tr)
	if hasReason(rec, rExecUnreadable) {
		t.Fatalf("a confirmed head must be believed: %q census %v", rec.Reasons, subs(c))
	}
	if !hasReason(rec, "host-read:"+ehUnknownLoader) {
		t.Fatalf("the loader named by the head must be observed (a host read of %s), got %q", ehUnknownLoader, rec.Reasons)
	}
	// A static binary: no loader to read, no reason at all.
	h = newHarness(t)
	h.noSettle = true // the tmpfs is gone after the run: the disk must not pretend the binary is there
	rec, c = h.ehRun(ehFile(ehKernel, shellLine(), ehConfirmed(2, 2, ehBin, buildELFStatic())), ehTrace(1, shellEvent(), mkdirAs(2, "/tmp/kiln-1/go-build"), mkdirAs(2, "/tmp/kiln-1/go-build/b001"), madeBy(2, ehBin), execAs(2, ehBin)))
	if len(rec.Reasons) != 0 {
		t.Fatalf("a believed static head leaves no reason: %q census %v", rec.Reasons, censusKinds(c))
	}
}

// Without a sidecar the behaviour is exactly today's.
func TestExecUnderTmpWithoutASupervisorIsRefusedAsToday(t *testing.T) {
	h := newHarness(t)
	rec, c := h.ehRun(nil, ehTrace(1, shellEvent(), execAs(2, ehBin)))
	if !hasReason(rec, rExecUnreadable) || strings.Join(subs(c), ",") != "under-tmp" {
		t.Fatalf("reasons %q, sub-codes %v", rec.Reasons, subs(c))
	}
}

// A refusal: the record carries exec-unreadable, the census the sub-code, and
// the loader the head names was NOT read (no head was used).
func (h *harness) wantRefused(rec *RawRecord, c *Census, sub string) {
	h.t.Helper()
	if !hasReason(rec, rExecUnreadable) {
		h.t.Fatalf("want exec-unreadable, got %q (census %v)", rec.Reasons, subs(c))
	}
	if got := strings.Join(subs(c), ","); got != sub {
		h.t.Fatalf("sub-codes %q, want %q", got, sub)
	}
	if hasReason(rec, "host-read:"+ehUnknownLoader) {
		h.t.Fatalf("a refused head must not be used, but its loader was read: %q", rec.Reasons)
	}
}

func TestExecHeadPathMustEqualTheTracesPathByteForByte(t *testing.T) {
	head := ehELF(ehUnknownLoader)
	for name, linePath := range map[string]string{
		"other file":   ehBin2,
		"prefix":       ehBin + "x",
		"trailing dot": ehBin + ".",
		"relative":     "go-build/b001/x.test",
		"double slash": "/tmp/kiln-1/go-build//b001/x.test",
		"case":         strings.ToUpper(ehBin),
		"empty path":   "",
	} {
		t.Run(name, func(t *testing.T) {
			h := newHarness(t)
			rec, c := h.ehRun(ehFile(ehKernel, shellLine(), ehConfirmed(2, 2, linePath, head)), ehTrace(1, shellEvent(), execAs(2, ehBin)))
			h.wantRefused(rec, c, "exechead-pair")
		})
	}
}

// Every status but `confirmed` carries no evidence, even if a head is attached.
func TestExecHeadEveryOtherStatusRefusesWithItsOwnSubCode(t *testing.T) {
	for st, want := range map[exechead.Status]string{
		exechead.StatusOutside: "exechead-outside", exechead.StatusGone: "exechead-gone", exechead.StatusMismatch: "exechead-mismatch",
		exechead.StatusTimeout: "exechead-timeout", exechead.StatusUnpinned: "exechead-unpinned", exechead.StatusFexecve: "exechead-fexecve",
		exechead.StatusSameImage: "exechead-same-image", exechead.StatusFailed: "exechead-failed",
		exechead.StatusOverload: "exechead-overload", exechead.StatusTainted: "exechead-tainted",
	} {
		t.Run(string(st), func(t *testing.T) {
			h := newHarness(t)
			l := ehPlain(2, ehBin, st)
			l.Head = ehELF(ehUnknownLoader) // never read: only a confirmed line is evidence
			rec, c := h.ehRun(ehFile(ehKernel, shellLine(), l), ehTrace(1, shellEvent(), execAs(2, ehBin)))
			h.wantRefused(rec, c, want)
		})
	}
}

func TestExecHeadKernelSeriesIsMatchedExactly(t *testing.T) {
	for kernel, ok := range map[string]bool{
		"7.0.0-1014-aws": true, "7.0.0-1020-azure": true, "7.0": true, "7.0.1": true, "7.0-rc2": true, "7.0+": true,
		"6.8.0-test": false, "7.1.0": false, "7.10.1": false, "7.00.1": false, "7.01": false, "7.0x": false, "70.0.0": false,
		"17.0.0": false, "7": false, "7.": false, ".0": false, "": false, "seven.zero": false, " 7.0.0": false, "7.0\n": false,
	} {
		if got := kernelTrusted(kernel); got != ok {
			t.Errorf("kernelTrusted(%q) = %v, want %v", kernel, got, ok)
		}
	}
	for _, kernel := range []string{"6.8.0-test", "7.1.0", ""} {
		h := newHarness(t)
		rec, c := h.ehRun(ehFile(kernel, shellLine(), ehConfirmed(2, 2, ehBin, ehELF(ehUnknownLoader))), ehTrace(1, shellEvent(), execAs(2, ehBin)))
		h.wantRefused(rec, c, "exechead-kernel")
	}
}

// The nth exec EVENT of a pid pairs with the nth LINE of its tgid, failed execs
// included: a PATH search is several failed execs, each notified.
func TestExecHeadFailedExecsCountOnBothSides(t *testing.T) {
	head := ehELF(ehUnknownLoader)
	// pid 2 searches PATH (two ENOENTs), then execs the tmpfs binary.
	evs := func() []trace.Event {
		return []trace.Event{shellEvent(), failExecAs(2, "/usr/local/bin/x.test", "ENOENT"), failExecAs(2, "/usr/bin/x.test", "ENOENT"), execAs(2, ehBin)}
	}
	h := newHarness(t)
	rec, c := h.ehRun(ehFile(ehKernel, shellLine(),
		ehPlain(2, "/usr/local/bin/x.test", exechead.StatusFailed), ehPlain(2, "/usr/bin/x.test", exechead.StatusFailed),
		ehConfirmed(2, 2, ehBin, head)), ehTrace(1, evs()...))
	if hasReason(rec, rExecUnreadable) {
		t.Fatalf("failed execs with their placeholder lines keep the ordinals honest: %q %v", rec.Reasons, subs(c))
	}

	// A slipped count (a missing line) shifts every later ordinal: refuse.
	h = newHarness(t)
	rec, c = h.ehRun(ehFile(ehKernel, shellLine(), ehPlain(2, "/usr/bin/x.test", exechead.StatusFailed), ehConfirmed(2, 2, ehBin, head)), ehTrace(1, evs()...))
	h.wantRefused(rec, c, "exechead-pair")

	// The right number of lines in the WRONG order: the exec of the tmpfs binary
	// is paired with the line of a failed PATH probe (status `failed`: no
	// evidence), never with the confirmed line that sits one place early.
	h = newHarness(t)
	rec, c = h.ehRun(ehFile(ehKernel, shellLine(),
		ehPlain(2, "/usr/local/bin/x.test", exechead.StatusFailed), ehConfirmed(2, 2, ehBin, head), ehPlain(2, "/usr/bin/x.test", exechead.StatusFailed)), ehTrace(1, evs()...))
	// (The confirmed line sits beside a FAILED exec, a contradiction, so the pid
	// is refused as a slip.)
	h.wantRefused(rec, c, "exechead-pair")

	// The same slip with two CONFIRMED lines swapped: the paths differ, so the
	// ordinals cannot hand a head to the wrong exec.
	h = newHarness(t)
	rec, c = h.ehRun(ehFile(ehKernel, shellLine(), ehConfirmed(2, 2, ehBin2, head), ehConfirmed(2, 2, ehBin, head)),
		ehTrace(1, shellEvent(), execAs(2, ehBin), execAs(2, ehBin2)))
	h.wantRefused(rec, c, "exechead-pair")
}

func TestExecHeadCountsMustBeEqual(t *testing.T) {
	head := ehELF(ehUnknownLoader)
	conf := ehConfirmed(2, 2, ehBin, head)
	for name, tc := range map[string]struct {
		lines []exechead.Line
		sub   string
	}{
		"no line for the pid":       {[]exechead.Line{shellLine()}, "exechead-pair"},
		"an extra line":             {[]exechead.Line{shellLine(), conf, ehPlain(2, "/usr/bin/true", exechead.StatusOutside)}, "exechead-pair"},
		"the line is another pid's": {[]exechead.Line{shellLine(), ehConfirmed(3, 3, ehBin, head)}, "exechead-pair"},
		"the init has no line":      {[]exechead.Line{conf}, "exechead-pair"}, // pid 2 is fine; see below
	} {
		t.Run(name, func(t *testing.T) {
			h := newHarness(t)
			rec, c := h.ehRun(ehFile(ehKernel, tc.lines...), ehTrace(1, shellEvent(), execAs(2, ehBin)))
			if name == "the init has no line" {
				// The init's missing line refuses the INIT pid only (its sh is not under
				// the tmpfs, so nothing is refused here), and pid 2 is believed.
				if hasReason(rec, rExecUnreadable) {
					t.Fatalf("pid 2 pairs on its own: %q %v", rec.Reasons, subs(c))
				}
				return
			}
			h.wantRefused(rec, c, tc.sub)
		})
	}
}

// The init pid is anchored the same way: its events pair one for one with its
// lines from the first (the parser has already dropped the setup execs of the
// Kiln binary), and a step whose own exec is under the tmpfs is refused when the
// counts differ.  A trace that is not a helper trace has no anchor at all.
func TestExecHeadInitAnchor(t *testing.T) {
	head := ehELF(ehUnknownLoader)
	// The step itself is the tmpfs binary (init pid 1 execs it).
	step := execAs(1, ehBin)
	h := newHarness(t)
	rec, c := h.ehRun(ehFile(ehKernel, ehConfirmed(1, 1, ehBin, head)), ehTrace(1, step))
	if hasReason(rec, rExecUnreadable) || !hasReason(rec, "host-read:"+ehUnknownLoader) {
		t.Fatalf("the init's own exec pairs from the first line: %q %v", rec.Reasons, subs(c))
	}
	// A failed PATH probe the parser kept (a real observation) has a line too: with
	// the line missing the counts differ and the init is refused.
	h = newHarness(t)
	rec, c = h.ehRun(ehFile(ehKernel, ehConfirmed(1, 1, ehBin, head)), ehTrace(1, failExecAs(1, "/usr/local/bin/x.test", "ENOENT"), step))
	h.wantRefused(rec, c, "exechead-pair")
	// An extra leading line (an exec the trace never shows, e.g. one the parser
	// dropped) is not skipped: nothing is tolerated.
	h = newHarness(t)
	rec, c = h.ehRun(ehFile(ehKernel, ehPlain(1, "/usr/bin/kiln", exechead.StatusOutside), ehConfirmed(1, 1, ehBin, head)), ehTrace(1, step))
	h.wantRefused(rec, c, "exechead-pair")
	// Not a helper trace (no init pid): no anchor, so no head is believed.
	h = newHarness(t)
	tr := ehTrace(1, step)
	tr.InitPid = 0
	rec, c = h.ehRun(ehFile(ehKernel, ehConfirmed(1, 1, ehBin, head)), tr)
	h.wantRefused(rec, c, "exechead-pair")
}

func TestExecHeadNonLeaderLineRefusesTheWholePid(t *testing.T) {
	head := ehELF(ehUnknownLoader)
	// pid 2 has a line whose tid is another thread's: even a confirmed line of the
	// same tgid is refused, because the attribution is not verified.
	for name, lines := range map[string][]exechead.Line{
		"the tmpfs exec is from a non-leader": {shellLine(), ehConfirmed(9, 2, ehBin, head)},
		"an earlier line is from a non-leader": {shellLine(),
			{Tid: 9, Tgid: 2, Syscall: exechead.SyscallExecve, Path: "/usr/bin/true", Status: exechead.StatusTainted}, ehConfirmed(2, 2, ehBin, head)},
	} {
		t.Run(name, func(t *testing.T) {
			h := newHarness(t)
			evs := []trace.Event{shellEvent(), execAs(2, ehBin)}
			if len(lines) == 3 {
				evs = []trace.Event{shellEvent(), execAs(2, "/usr/bin/true"), execAs(2, ehBin)}
			}
			rec, c := h.ehRun(ehFile(ehKernel, lines...), ehTrace(1, evs...))
			h.wantRefused(rec, c, "exechead-pair")
		})
	}
}

// An exec event with no result (strace's `= ?`) is refused for the pid.  The
// trace parser already refuses such a trace wholesale (see the test in
// internal/trace), so this is the second wall.
func TestExecHeadResultlessExecRefusesThePid(t *testing.T) {
	head := ehELF(ehUnknownLoader)
	h := newHarness(t)
	lost := execAs(2, "/usr/bin/true")
	lost.Result = trace.Result{Errno: "?"}
	rec, c := h.ehRun(ehFile(ehKernel, shellLine(), ehPlain(2, "/usr/bin/true", exechead.StatusOutside), ehConfirmed(2, 2, ehBin, head)),
		ehTrace(1, shellEvent(), lost, execAs(2, ehBin)))
	h.wantRefused(rec, c, "exechead-pair")
}

// A non-confirmed overload, timeout, gone or tainted line followed by a later
// confirmed line refuses the pid, unless that later trace event returned
// normally.  The earlier exec's outcome is unknown: it may have been in flight.
func TestExecHeadUndecidedThenConfirmed(t *testing.T) {
	head := ehELF(ehUnknownLoader)
	for _, st := range []exechead.Status{exechead.StatusOverload, exechead.StatusTimeout, exechead.StatusGone, exechead.StatusTainted} {
		t.Run(string(st), func(t *testing.T) {
			// E0 confirmed (would be believed), E1 undecided, E2 a confirmed line whose
			// exec FAILED: the pid is refused, E0 included.
			lines := []exechead.Line{shellLine(), ehConfirmed(2, 2, ehBin, head), ehPlain(2, ehBin2, st), ehConfirmed(2, 2, "/usr/bin/false", head)}
			evs := func(last trace.Event) []trace.Event {
				return []trace.Event{shellEvent(), execAs(2, ehBin), execAs(2, ehBin2), last}
			}
			h := newHarness(t)
			rec, c := h.ehRun(ehFile(ehKernel, lines...), ehTrace(1, evs(failExecAs(2, "/usr/bin/false", "EACCES"))...))
			if !hasReason(rec, rExecUnreadable) {
				t.Fatalf("the pid must be refused: %q", rec.Reasons)
			}
			got := map[string]bool{}
			for _, s := range subs(c) {
				got[s] = true
			}
			if !got["exechead-pair"] {
				t.Fatalf("sub-codes %v", subs(c))
			}
			// The same lines, but the later confirmed line's event returned normally:
			// E0 is believed, E1 keeps its own status.
			h = newHarness(t)
			rec, c = h.ehRun(ehFile(ehKernel, lines...), ehTrace(1, evs(execAs(2, "/usr/bin/false"))...))
			want := map[string]bool{"exechead-" + string(st): true}
			for _, s := range subs(c) {
				if !want[s] {
					t.Fatalf("sub-codes %v, want only %v (E0 believed)", subs(c), want)
				}
			}
			if !hasReason(rec, "host-read:"+ehUnknownLoader) {
				t.Fatalf("E0's head must have been used: %q", rec.Reasons)
			}
		})
	}
}

// A line is for the exec the trace shows: the syscall must agree.
func TestExecHeadSyscallMustAgree(t *testing.T) {
	h := newHarness(t)
	l := ehConfirmed(2, 2, ehBin, ehELF(ehUnknownLoader))
	l.Syscall = exechead.SyscallExecveat
	rec, c := h.ehRun(ehFile(ehKernel, shellLine(), l), ehTrace(1, shellEvent(), execAs(2, ehBin)))
	h.wantRefused(rec, c, "exechead-pair")
}

// A status that contradicts the trace's result is slip evidence: `confirmed` and
// `mismatch` say an exec SUCCEEDED, so beside a FAILED event the ordinals have
// slipped and the pid is refused (exechead-pair), wherever it occurs, with no
// undecided line before it.  A failed exec itself reaches no loader, and the
// head of the contradicting line is never used.
func TestExecHeadStatusContradictingAFailedEventRefusesThePid(t *testing.T) {
	head := ehELF(ehUnknownLoader)
	for _, st := range []exechead.Status{exechead.StatusConfirmed, exechead.StatusMismatch} {
		t.Run(string(st), func(t *testing.T) {
			l := ehPlain(2, ehBin2, st)
			if st == exechead.StatusConfirmed {
				l = ehConfirmed(2, 2, ehBin2, head)
			}
			// E0 is a good confirmed exec that would be believed; E1 failed but its
			// line says it succeeded: the whole pid is refused, E0 included.
			h := newHarness(t)
			rec, c := h.ehRun(ehFile(ehKernel, shellLine(), ehConfirmed(2, 2, ehBin, head), l),
				ehTrace(1, shellEvent(), execAs(2, ehBin), failExecAs(2, ehBin2, "ENOEXEC")))
			h.wantRefused(rec, c, "exechead-pair")
		})
	}
}

// The reverse is legitimate and stays as it was: a `failed` line (exe unchanged)
// beside a SUCCESSFUL exec is e.g. `sh` running a tmpfs script whose interpreter
// is sh itself.  It is refused as exechead-failed, for that exec only: another
// confirmed exec of the same pid is still believed.
func TestExecHeadFailedLineBesideASuccessfulExecIsNotSlipEvidence(t *testing.T) {
	head := ehELF(ehUnknownLoader)
	h := newHarness(t)
	rec, c := h.ehRun(ehFile(ehKernel, shellLine(), ehPlain(2, ehBin2, exechead.StatusFailed), ehConfirmed(2, 2, ehBin, head)),
		ehTrace(1, shellEvent(), execAs(2, ehBin2), execAs(2, ehBin)))
	if got := strings.Join(subs(c), ","); got != "exechead-failed" {
		t.Fatalf("sub-codes %q, want only exechead-failed: %q", got, rec.Reasons)
	}
	if !hasReason(rec, "host-read:"+ehUnknownLoader) {
		t.Fatalf("the confirmed exec of the same pid must still be believed: %q", rec.Reasons)
	}
}

// A confirmed line means exe became the pinned file, which is never a `#!`
// script, so a believed head that starts with "#!" contradicts itself and is
// refused rather than followed as a shebang.
func TestExecHeadConfirmedShebangHeadIsRefused(t *testing.T) {
	h := newHarness(t)
	rec, c := h.ehRun(ehFile(ehKernel, shellLine(), ehConfirmed(2, 2, ehBin, []byte("#!"+ehUnknownLoader+"\n"))),
		ehTrace(1, shellEvent(), execAs(2, ehBin)))
	h.wantRefused(rec, c, "exechead-notelf")
}

func TestExecHeadsMustBeEmptyOrOnePerTrace(t *testing.T) {
	h := newHarness(t)
	tr := ehTrace(1, shellEvent())
	_, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{tr}, Recipe: []string{"step"},
		ExecHeads: []*exechead.File{nil, nil}})
	if err == nil {
		t.Fatal("two sidecars for one trace must be refused")
	}
	// A nil entry is a step with no supervisor: classified as before.
	h.settle([]*trace.Trace{tr})
	if _, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{tr}, Recipe: []string{"step"}, ExecHeads: []*exechead.File{nil}}); err != nil {
		t.Fatal(err)
	}
}

// Two steps: the first has a supervisor, the second does not.  Each is judged by
// its own file.
func TestExecHeadIsPerStep(t *testing.T) {
	head := ehELF(ehUnknownLoader)
	h := newHarness(t)
	t1 := ehTrace(1, shellEvent(), execAs(2, ehBin))
	t2 := ehTrace(1, shellEvent(), execAs(2, ehBin))
	h.settle([]*trace.Trace{t1, t2})
	c := NewCensus()
	rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{t1, t2}, Recipe: []string{"step", "step"}, Census: c,
		ExecHeads: []*exechead.File{ehFile(ehKernel, shellLine(), ehConfirmed(2, 2, ehBin, head)), nil}})
	if err != nil {
		t.Fatal(err)
	}
	if strings.Join(subs(c), ",") != "under-tmp" || !hasReason(rec, rExecUnreadable) {
		t.Fatalf("the second step has no supervisor and is refused as today: %q %v", rec.Reasons, subs(c))
	}
}

// Every sub-code the classifier can raise from the pairing is on the census's
// closed list.
func TestExecHeadSubCodesAreOnTheCensusList(t *testing.T) {
	known := map[string]bool{}
	for _, s := range censusSubCodes["trace-incomplete:exec-unreadable"] {
		known[s] = true
	}
	for _, s := range execHeadSubCodes {
		if !known[s] {
			t.Errorf("%q is not in censusSubCodes", s)
		}
	}
	for st := range map[exechead.Status]bool{
		exechead.StatusOutside: true, exechead.StatusGone: true, exechead.StatusMismatch: true, exechead.StatusTimeout: true,
		exechead.StatusUnpinned: true, exechead.StatusFexecve: true, exechead.StatusSameImage: true, exechead.StatusFailed: true,
		exechead.StatusOverload: true, exechead.StatusTainted: true,
	} {
		if s := statusSub(st); !known[s] || s == "exechead-pair" {
			t.Errorf("statusSub(%s) = %q", st, s)
		}
	}
	if len(execHeadSubCodes) != 13 {
		t.Errorf("%d sub-codes", len(execHeadSubCodes))
	}
}

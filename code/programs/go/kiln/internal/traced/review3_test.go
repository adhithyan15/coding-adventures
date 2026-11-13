package traced

import (
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Regression tests for the round-2 review.

// timed places an event: process, the line it started on and the line it
// completed on (a split call has start < line).
func timed(e trace.Event, pid, start, line int) trace.Event {
	e.Pid, e.StartLine, e.Line = pid, start, line
	return e
}

// mtrace builds a two-process trace from events that already carry their pids
// and lines.  Line 1 is the shell's exec.
func mtrace(events ...trace.Event) *trace.Trace {
	sh := execve("/usr/bin/sh", "sh", "-c", "step")
	sh.Pid, sh.Line, sh.StartLine = 1, 1, 1
	return &trace.Trace{
		RootPid: 1,
		Events:  append([]trace.Event{sh}, events...),
		Procs:   map[int]*trace.Process{1: {Pid: 1, Exit: &trace.Exit{Pid: 1}}, 2: {Pid: 2, Parent: 1, Exit: &trace.Exit{Pid: 2}}},
		Exits:   []trace.Exit{{Pid: 2}, {Pid: 1}},
	}
}

func TestOverlappingCreationDefeatsAbsentStop(t *testing.T) {
	// P2 makes a link k (lines 9-11) while P1 resolves through it (lines 10-12);
	// k is removed afterwards, so the disk shows nothing there.
	sym := timed(symlink("/home/runner/evil", "code/p/a/k"), 2, 9, 11)
	unlink := timed(write("unlink", "code/p/a/k"), 2, 13, 14)

	h := newHarness(t)
	h.noSettle = true
	ok := timed(stat("code/p/a/k/x"), 1, 10, 12)
	rec := h.runSteps(mtrace(sym, ok, unlink))
	wantReason(t, rec, "trace-incomplete:unstable-path")

	// The same lookup failing with ENOENT: no contradiction on its own, but the
	// creation overlapped it.
	h = newHarness(t)
	h.noSettle = true
	bad := timed(fail(stat("code/p/a/k/x"), "ENOENT"), 1, 10, 12)
	rec = h.runSteps(mtrace(sym, bad, unlink))
	wantReason(t, rec, "trace-incomplete:unstable-path")

	// A creation that completed AND was removed before the lookup began is
	// settled: the lookup could only have seen it absent.
	h = newHarness(t)
	h.noSettle = true
	sym2 := timed(symlink("/home/runner/evil", "code/p/a/k"), 2, 2, 3)
	rm2 := timed(write("unlink", "code/p/a/k"), 2, 4, 5)
	late := timed(fail(stat("code/p/a/k/x"), "ENOENT"), 1, 8, 9)
	rec = h.runSteps(mtrace(sym2, rm2, late))
	if hasReason(rec, "trace-incomplete:unstable-path") {
		t.Fatalf("settled creation flagged: %q", rec.Reasons)
	}
	// A removal that overlaps the creation does not settle it.
	h = newHarness(t)
	h.noSettle = true
	rm3 := timed(write("unlink", "code/p/a/k"), 2, 3, 5)
	rec = h.runSteps(mtrace(sym2, rm3, late))
	wantReason(t, rec, "trace-incomplete:unstable-path")
}

func TestChdirIntoAnUnstableDirectoryPoisonsRelativeCalls(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	sym := timed(symlink("/home/runner/evil", "code/p/a/k"), 2, 9, 11)
	cd := timed(base("chdir", trace.KindDirectory, "code/p/a/k"), 1, 10, 12)
	rel := timed(fail(stat("x"), "ENOENT"), 1, 14, 14)
	rel.Cwd = abs("code/p/a/k")
	unlink := timed(write("unlink", "code/p/a/k"), 2, 15, 16)
	rec := h.runSteps(mtrace(sym, cd, rel, unlink))
	wantReason(t, rec, "trace-incomplete:unstable-path")
}

func TestSuccessfulCallThroughAMissingComponentContradictsTheDisk(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	// Nothing at all was changed, yet a call succeeded through a name the disk
	// does not have: another action (or the disk) changed it.
	wantReason(t, h.run(stat("code/p/a/gone/x")), "trace-incomplete:unstable-path")
	wantReason(t, h.run(stat("go.work/x")), "trace-incomplete:unstable-path")
	// A failing call through the same name is an ordinary absence.
	wantNoReasons(t, h.run(fail(stat("code/p/a/gone/x"), "ENOENT")))
	// The final name missing on a successful create is fine.
	wantNoReasons(t, h.run(open("code/p/a/new", "O_WRONLY|O_CREAT|O_EXCL", abs("code/p/a/new"))))
}

func TestRootUnderHomeIsNotFreshTmpfs(t *testing.T) {
	const root = "/home/runner/work/repo"
	h := newHarness(t)
	h.cfg.Root = root
	h.fs.root = root
	for _, d := range []string{"/home/runner", "/home/runner/work"} {
		h.fs.dir(d)
	}
	at := func(e trace.Event) trace.Event { e.Cwd = root; return e }
	// A tracked link code/p/a/tob -> ../b read through to a missing name.
	h.tree.link("code/p/a/tob", "../b")
	rec := h.run(at(fail(stat("code/p/a/tob/zz"), "ENOENT")))
	eq(t, "entries", rec.Entries, "pkg:code/p/a", "pkg:code/p/b")
	wantNoReasons(t, rec)
	eq(t, "traversed", rec.Traversed, "code", "code/p", "code/p/a", "code/p/a/tob", "code/p/b")
	// The checkout's ancestors are real too.
	rec = h.run(at(stat("../../..")))
	for _, r := range rec.Reasons {
		if strings.HasPrefix(r, "trace-incomplete:unstable") {
			t.Fatalf("%q", rec.Reasons)
		}
	}
	// A persistent state root under HOME is not fresh, nor are its parents.
	h = newHarness(t)
	h.fs.dir("/home/runner").dir("/home/runner/.cargo").dir("/home/runner/.cargo/registry")
	h.fs.file("/home/runner/.cargo/registry/index")
	rec = h.run(openAbs("/home/runner/.cargo/registry/index"))
	wantReason(t, rec, "host-read:$HOME/.cargo/registry/index")
	if hasReason(rec, "trace-incomplete:unstable-path") {
		t.Fatalf("%q", rec.Reasons)
	}

	// Misconfigurations are refused.
	good := Input{Package: "code/p/a", Traces: []*trace.Trace{step(stat("go.work"))}, Recipe: []string{"step"}}
	for name, mut := range map[string]func(*Config){
		"root is home":   func(c *Config) { c.Home = testRoot },
		"root is tmpdir": func(c *Config) { c.Tmpdir = testRoot },
		"home in root":   func(c *Config) { c.Home = testRoot + "/home" },
		"tmpdir in root": func(c *Config) { c.Tmpdir = testRoot + "/tmp" },
	} {
		h := newHarness(t)
		cfg := h.cfg
		mut(&cfg)
		if _, err := Classify(cfg, good); err == nil {
			t.Errorf("%s: no error", name)
		}
	}
}

func TestExecThenRewriteMeasuredFromTheFirstExec(t *testing.T) {
	tool := abs("code/p/a/tool")
	setup := func() *harness {
		h := newHarness(t)
		h.fs.bytes(tool, buildELFStatic())
		h.noSettle = true
		return h
	}
	exec1 := timed(execve("code/p/a/tool", "t"), 1, 2, 2)
	write := timed(open("code/p/a/tool", "O_WRONLY|O_TRUNC", tool), 2, 3, 3)
	exec2 := timed(execve("code/p/a/tool", "t"), 1, 4, 4)
	// exec, rewrite, exec again: the second exec read the NEW bytes, the first
	// the old, and the disk holds the new ones.
	rec := setup().runSteps(mtrace(exec1, write, exec2))
	wantReason(t, rec, "trace-incomplete:exec-unreadable")
	// The write was already running when the exec began (it completes later).
	w2 := timed(open("code/p/a/tool", "O_WRONLY|O_TRUNC", tool), 2, 2, 5)
	e2 := timed(execve("code/p/a/tool", "t"), 1, 3, 4)
	rec = setup().runSteps(mtrace(w2, e2))
	wantReason(t, rec, "trace-incomplete:exec-unreadable")
	// A write that completed before the exec started is fine.
	w3 := timed(open("code/p/a/tool", "O_WRONLY|O_TRUNC", tool), 2, 2, 3)
	e3 := timed(execve("code/p/a/tool", "t"), 1, 4, 5)
	rec = setup().runSteps(mtrace(w3, e3))
	if hasReason(rec, "trace-incomplete:exec-unreadable") {
		t.Fatalf("%q", rec.Reasons)
	}
}

func TestTruncWithoutCreatObservesWhatWasThere(t *testing.T) {
	h := newHarness(t)
	rec := h.run(open("code/p/a/old.txt", "O_WRONLY|O_TRUNC", abs("code/p/a/old.txt")))
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	eq(t, "untracked", rec.UntrackedObservations, "code/p/a/old.txt")
	// But not for a file the action made.
	rec = h.run(open("code/p/a/new.txt", "O_WRONLY|O_CREAT|O_EXCL", abs("code/p/a/new.txt")),
		open("code/p/a/new.txt", "O_WRONLY|O_TRUNC", abs("code/p/a/new.txt")))
	eq(t, "untracked", rec.UntrackedObservations)
}

func TestRenameExchangeForgetsWhatWasCreated(t *testing.T) {
	h := newHarness(t)
	ex := rename("code/p/a/d1", "code/p/a/d2")
	ex.Syscall, ex.Flags = "renameat2", "RENAME_EXCHANGE"
	h.fs.dir(abs("code/p/a/d1")).dir(abs("code/p/a/d2")).file(abs("code/p/a/d2/f"))
	rec := h.run(
		mkdir("code/p/a/d1"),
		open("code/p/a/d1/f", "O_WRONLY|O_CREAT|O_EXCL", abs("code/p/a/d1/f")),
		mkdir("code/p/a/d2"),
		ex,
		// After the exchange d2/f is what used to be d1/f, but d1 and d2 are no
		// longer provably the action's own, so a read observes them.
		stat("code/p/a/d2/f"),
	)
	if len(rec.UntrackedObservations) == 0 {
		t.Fatalf("nothing observed after the exchange: %q", rec.Entries)
	}
}

func TestBindsAreRecordedForTheMergeJob(t *testing.T) {
	h := newHarness(t)
	rec := h.runNet(inet("bind", "127.0.0.1", 8080), inet("bind", "127.0.0.1", 0), inet("bind", "0.0.0.0", 9000))
	eq(t, "binds", rec.Binds, "SOCK_STREAM/0.0.0.0:9000", "SOCK_STREAM/127.0.0.1:8080")
}

// 1M events must classify in seconds, not minutes: the stability scans are
// logarithmic, not quadratic.
func TestClassifyScalesLinearly(t *testing.T) {
	if testing.Short() {
		t.Skip("scaling test")
	}
	h := newHarness(t)
	h.noSettle = true
	h.fs.dir(abs("code/p/a/hot"))
	var evs []trace.Event
	evs = append(evs, mkdir("code/p/a/hot"))
	for i := 0; i < 1_000_000; i++ {
		name := "code/p/a/hot/d" + string(rune('a'+i%26)) + string(rune('a'+(i/26)%26)) + string(rune('a'+(i/676)%26)) + string(rune('a'+(i/17576)%26))
		switch i % 10 {
		case 0:
			evs = append(evs, mkdir(name))
		case 1:
			evs = append(evs, write("unlink", name))
		default:
			evs = append(evs, fail(stat(name+"/x"), "ENOENT"))
		}
	}
	start := time.Now()
	rec := h.run(evs...)
	if d := time.Since(start); d > 60*time.Second {
		t.Fatalf("1M events took %v", d)
	}
	if len(rec.Entries) == 0 {
		t.Fatal("no entries")
	}
}

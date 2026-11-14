package traced

import (
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Tests of RawRecord.WritesComplete (KILN04 section 5.5, triggers W1..W10).
//
// Every trigger gets a pair: the case that must say "unknown" (false), and the
// nearest case that must NOT (true), so a trigger that is too eager is as
// visible as one that is missing.  The differential fuzzer (model_test.go)
// covers the data-dependent triggers (W8, W10) at scale.

// wc runs events as one step and returns the record.
func (h *harness) wc(events ...trace.Event) *RawRecord {
	h.t.Helper()
	return h.run(events...)
}

// noUnknown drops the writes-unknown reason, which the classifier adds when an
// unknown write set would otherwise leave no trace-incomplete reason.
func noUnknown(rs []string) []string {
	var out []string
	for _, r := range rs {
		if r != rWritesUnknown {
			out = append(out, r)
		}
	}
	return out
}

func wantComplete(t *testing.T, what string, rec *RawRecord, want bool) {
	t.Helper()
	if rec.WritesComplete != want {
		t.Errorf("%s: WritesComplete = %v, want %v (reasons %q, writes %q)", what, rec.WritesComplete, want, rec.Reasons, rec.Writes)
	}
}

func openW(path, ret string) trace.Event {
	return open(path, "O_WRONLY|O_CREAT|O_TRUNC", ret)
}

func TestWritesCompleteIsTheDefaultForAnOrdinaryAction(t *testing.T) {
	h := newHarness(t)
	h.fs.dir(abs("code/p/a/out")).file(abs("code/p/a/out/x"))
	rec := h.wc(
		mkdir(abs("code/p/a/out")),
		openW(abs("code/p/a/out/x"), abs("code/p/a/out/x")),
	)
	wantNoReasons(t, rec)
	wantComplete(t, "ordinary action", rec, true)
	eq(t, "writes", rec.Writes, "code/p/a/out")
	// An action that wrote nothing is complete too.
	wantComplete(t, "read-only action", h.wc(stat(abs("go.work"))), true)
	// The zero RawRecord is fail-closed.
	if (RawRecord{}).WritesComplete {
		t.Error("the zero value must be false")
	}
}

// A read-side trace-incomplete reason is exactly what writes_complete exists
// to excuse: it must not turn the write set unknown.
func TestReadSideReasonsLeaveWritesComplete(t *testing.T) {
	h := newHarness(t)
	h.fs.errs[abs("code/p/b/bad")] = errors.New("EIO")
	rec := h.wc(
		stat(abs("code/p/b/bad/q")), // an unreadable component, only looked at
		mkdir(abs("code/p/a/out")),
	)
	wantReason(t, rec, rUnresolvable)
	wantComplete(t, "read-only unresolvable", rec, true)

	// A failed step (nonzero exit) is not a truncated trace.
	st := step(mkdir(abs("code/p/a/out")))
	st.Procs[1].Exit.Code = 2
	rec = h.runSteps(st)
	wantReason(t, rec, "failed")
	wantComplete(t, "nonzero exit", rec, true)

	// Read-only host and network reasons cannot hide a write.
	rec = h.wc(openAbs("/opt/somewhere/else"), inet("connect", "8.8.8.8", 443), mkdir(abs("code/p/a/out")))
	if len(rec.Reasons) == 0 {
		t.Fatal("expected read-side reasons")
	}
	wantComplete(t, "host-read and network", rec, true)
}

// W1: a Write-kind call that cannot be placed.
func TestWritesCompleteW1UnresolvedWrite(t *testing.T) {
	h := newHarness(t)
	// ".." after a component that does not exist: where it leads is unknown.
	rec := h.wc(mkdir("code/p/a/nodir/../x"))
	wantComplete(t, "mkdir past a missing component", rec, false)
	// Whatever its result: a failed call that could not be placed is not
	// proved to have changed nothing.
	wantComplete(t, "failed unresolved mkdir", h.wc(fail(mkdir("code/p/a/nodir/../x"), "ENOENT")), false)
	wantComplete(t, "chmod of an empty path", h.wc(write("chmod", "")), false)
	// A symlink loop.
	h.fs.link(abs("L1"), "L2")
	h.fs.link(abs("L2"), "L1")
	wantComplete(t, "loop", h.wc(write("chmod", abs("L1/x"))), false)
	// The same names, only looked at, are a read-side reason.
	rec = h.wc(stat(abs("L1/x")))
	wantReason(t, rec, rLoop)
	wantComplete(t, "looked at a loop", rec, true)
	rec = h.wc(stat("code/p/a/nodir/../x"))
	wantComplete(t, "looked past a missing component", rec, true)
}

// W2: a writing open that cannot be placed; a read-only one is a read-side matter.
func TestWritesCompleteW2UnresolvedOpen(t *testing.T) {
	h := newHarness(t)
	wantComplete(t, "creating open past ..", h.wc(openW("code/p/a/nodir/../x", abs("code/p/a/x"))), false)
	wantComplete(t, "O_WRONLY past ..", h.wc(open("code/p/a/nodir/../x", "O_WRONLY", abs("code/p/a/x"))), false)
	wantComplete(t, "failed creating open past ..", h.wc(fail(openW("code/p/a/nodir/../x", ""), "ENOENT")), false)
	wantComplete(t, "read-only open past ..", h.wc(open("code/p/a/nodir/../x", "O_RDONLY", abs("code/p/a/x"))), true)

	// openat2 with a resolve field changes what a path means.
	o := openW(abs("code/p/a/out"), abs("code/p/a/out"))
	o.Syscall, o.Flags = "openat2", "{flags=O_WRONLY|O_CREAT, resolve=RESOLVE_NO_SYMLINKS}"
	rec := h.wc(o)
	wantReason(t, rec, rResolveFlags)
	wantComplete(t, "writing openat2 with resolve flags", rec, false)
	r := open(abs("go.work"), "{flags=O_RDONLY, resolve=RESOLVE_NO_SYMLINKS}", abs("go.work"))
	r.Syscall = "openat2"
	rec = h.wc(r)
	wantReason(t, rec, rResolveFlags)
	wantComplete(t, "reading openat2 with resolve flags", rec, true)
	// resolve=0 is ordinary.
	o.Flags = "{flags=O_WRONLY|O_CREAT, resolve=0}"
	wantComplete(t, "openat2, resolve 0", h.wc(o), true)
}

// W3: either endpoint of a rename or link.
func TestWritesCompleteW3Endpoints(t *testing.T) {
	h := newHarness(t)
	for _, sys := range []func(a, b string) trace.Event{rename, link} {
		wantComplete(t, "bad source", h.wc(sys("code/p/a/nodir/../x", abs("code/p/a/y"))), false)
		wantComplete(t, "bad destination", h.wc(sys(abs("code/p/a/README"), "code/p/a/nodir/../y")), false)
		wantComplete(t, "failed, bad source", h.wc(fail(sys("code/p/a/nodir/../x", abs("code/p/a/y")), "ENOENT")), false)
	}
	rec := h.wc(rename(abs("code/p/a/README"), abs("code/p/a/R2")))
	wantComplete(t, "plain rename", rec, true)
	eq(t, "tracked writes", rec.TrackedWrites, "code/p/a/README")
}

// W4: a Unix pathname bind.
func TestWritesCompleteW4UnixBind(t *testing.T) {
	h := newHarness(t)
	wantComplete(t, "bind past ..", h.wc(unix("bind", "code/p/a/nodir/../s")), false)
	nb := base("bind", trace.KindNetwork, "")
	wantComplete(t, "bind with no address", h.wc(nb), false)
	wantComplete(t, "plain bind", h.wc(unix("bind", abs("code/p/a/s"))), true)
	ab := unix("bind", "name")
	ab.Addr.Abstract = true
	wantComplete(t, "abstract bind", h.wc(ab), true)
	wantComplete(t, "inet bind", h.wc(inet("bind", "127.0.0.1", 9100)), true)
	// A bind through /proc.
	wantComplete(t, "bind through /proc", h.wc(unix("bind", "/proc/self/cwd/s")), false)
	// A failed bind that could be placed made nothing.
	wantComplete(t, "failed bind", h.wc(fail(unix("bind", abs("code/p/a/s")), "EADDRINUSE")), true)
}

// W5: virtual names say nothing about the object written.
func TestWritesCompleteW5Virtual(t *testing.T) {
	h := newHarness(t)
	wantComplete(t, "chmod via /proc/self/cwd", h.wc(write("chmod", "/proc/self/cwd/code/p/a/README")), false)
	wantComplete(t, "mkdir via /proc/self/root", h.wc(mkdir("/proc/self/root"+abs("code/p/a/o"))), false)
	wantComplete(t, "creating open via /proc/self/cwd", h.wc(openW("/proc/self/cwd/code/p/a/o", abs("code/p/a/o"))), false)
	wantComplete(t, "rename into /proc", h.wc(rename(abs("code/p/a/README"), "/proc/self/cwd/R")), false)
	wantComplete(t, "link from /dev/fd", h.wc(link("/dev/fd/3", abs("code/p/a/l"))), false)
	// Reading through /proc is a read-side reason.
	rec := h.wc(open("/proc/self/cwd/go.work", "O_RDONLY", abs("go.work")))
	wantComplete(t, "read through /proc/self/cwd", rec, true)
	// A write open of /proc/self/fd/N that returns a real -y path is classified by it.
	rec = h.wc(open("/proc/self/fd/3", "O_WRONLY", abs("code/p/a/out")))
	wantComplete(t, "write via an fd link with a path", rec, true)
	h.has(rec, rec.Writes, "code/p/a/out")
	// ... but not one that returns no path.
	wantComplete(t, "write via an fd link without a path", h.wc(open("/proc/self/fd/3", "O_WRONLY", "weird:[1]")), false)
	// A descriptor link whose -y path is itself virtual names nothing real.
	wantComplete(t, "write via an fd link to a virtual path", h.wc(open("/proc/self/fd/3", "O_WRONLY", "/dev/fd/9")), false)
	// O_TMPFILE is unnamed until linked, and linking is a /proc write.
	tf := open(abs("code/p/a"), "O_RDWR|O_TMPFILE", "")
	wantComplete(t, "O_TMPFILE alone", h.wc(tf), true)
}

// W6: paths the record cannot hold.
func TestWritesCompleteW6BadPaths(t *testing.T) {
	h := newHarness(t)
	rec := h.wc(mkdir(abs(".git/hooks")))
	wantReason(t, rec, rVCSWrite)
	wantComplete(t, "write under .git", rec, false)
	rec = h.wc(mkdir(abs("code/p/a/x\xff")))
	wantReason(t, rec, rBadPath)
	wantComplete(t, "unrepresentable name", rec, false)
	// Reading them is only a reason.
	rec = h.wc(stat(abs(".git/config")))
	wantReason(t, rec, rVCSRead)
	wantComplete(t, "read of .git", rec, true)
}

// W7: the root itself.
func TestWritesCompleteW7Root(t *testing.T) {
	h := newHarness(t)
	wantComplete(t, "chmod root", h.wc(write("chmod", testRoot)), false)
	wantComplete(t, "chmod .", h.wc(write("chmod", ".")), false)
	wantComplete(t, "chmod a child", h.wc(write("chmod", abs("go.work"))), true)
	wantComplete(t, "stat root", h.wc(stat(testRoot)), true)
}

// W8: instability under a write, and only under a write.
func TestWritesCompleteW8Unstable(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	h.fs.dir(abs("code/p/a/d"))
	h.fs.link(abs("code/p/a/lk"), "d2")
	flip := []trace.Event{
		mkdir(abs("code/p/a/d")),
		symlink("d", abs("code/p/a/lk")),
	}
	tail := []trace.Event{
		write("unlink", abs("code/p/a/lk")),
		symlink("d2", abs("code/p/a/lk")),
	}
	// A write through a link the action later replaced.
	rec := h.wc(append(append(append([]trace.Event{}, flip...), mkdir(abs("code/p/a/lk/new"))), tail...)...)
	wantReason(t, rec, rUnstable)
	wantComplete(t, "write through a replaced link", rec, false)
	// The same replacement, only read through: a read-side reason.
	rec = h.wc(append(append(append([]trace.Event{}, flip...), stat(abs("code/p/a/lk/new"))), tail...)...)
	wantReason(t, rec, rUnstable)
	wantComplete(t, "read through a replaced link", rec, true)

	// An unreadable component: raised while a write is placed, and remembered
	// (the lookup is cached) for a later write.
	h2 := newHarness(t)
	h2.fs.errs[abs("code/p/b/bad")] = errors.New("EIO")
	rec = h2.wc(stat(abs("code/p/b/bad/q")), mkdir(abs("code/p/b/bad/new")))
	wantReason(t, rec, rUnresolvable)
	wantComplete(t, "write through a cached unreadable component", rec, false)
	rec = h2.wc(mkdir(abs("code/p/b/bad/new")))
	wantComplete(t, "write through an unreadable component", rec, false)
}

// W9: caps and budgets.
func TestWritesCompleteW9Caps(t *testing.T) {
	h := newHarness(t)
	var evs []trace.Event
	for i := 0; i <= maxListed; i++ {
		evs = append(evs, mkdir(abs(fmt.Sprintf("code/p/a/o%d", i))))
	}
	h.noSettle = true
	rec := h.wc(evs...)
	wantReason(t, rec, rSize)
	wantComplete(t, "more than maxListed writes", rec, false)
	rec = h.wc(evs[:100]...)
	wantComplete(t, "few writes", rec, true)

	// The walk budget: every path refused is unresolved.
	old := walkBudget
	defer func() { walkBudget = old }()
	walkBudget = 3
	rec = h.wc(mkdir(abs("code/p/a/deep/er/than/budget")))
	wantReason(t, rec, rSize)
	wantComplete(t, "walk budget", rec, false)

	// The stability pass stops early when its budget runs out.
	walkBudget = old
	p, err := prepare(h.cfg)
	if err != nil {
		t.Fatal(err)
	}
	c := newClassifier(p)
	c.seenAbsent["/x"] = span{}
	c.mods["/x"] = []mod{{made: true}}
	c.steps = walkBudget
	c.checkStability()
	if !c.incomplete || !c.reasons[rSize] {
		t.Errorf("an exhausted stability budget must say incomplete (reasons %v)", c.reasons)
	}
}

// W10: whole-trace problems.
func TestWritesCompleteW10WholeTrace(t *testing.T) {
	h := newHarness(t)
	evs := []trace.Event{mkdir(abs("code/p/a/out"))}

	none := step(evs...)
	none.Procs[1].Exit = nil
	wantComplete(t, "no exit line", h.runSteps(none), false)

	killed := step(evs...)
	killed.Procs[1].Exit = &trace.Exit{Pid: 1, Killed: true, Signal: "SIGKILL"}
	wantComplete(t, "killed step", h.runSteps(killed), false)

	orphan := step(evs...)
	orphan.Procs[9] = &trace.Process{Pid: 9, Parent: 1}
	wantComplete(t, "a process with no exit line", h.runSteps(orphan), false)

	// One bad step among several poisons the action.
	good := step(evs...)
	wantComplete(t, "two good steps", h.runSteps(good, step(evs...)), true)
	wantComplete(t, "one bad step of two", h.runSteps(step(evs...), none), false)

	// A daemon: a survivor killed after the init child exited.
	d := step(evs...)
	d.InitPid = 1
	d.Procs[7] = &trace.Process{Pid: 7, Parent: 1, Exit: &trace.Exit{Pid: 7, Killed: true, Signal: "SIGKILL"}}
	d.Exits = []trace.Exit{{Pid: 1}, {Pid: 7, Killed: true, Signal: "SIGKILL"}}
	d.ExitLines, d.InitExitLine = []int{20, 21}, 19 // init's exit_group at 19, the survivor after it
	rec := h.runSteps(d)
	wantReason(t, rec, rDaemonLeft)
	wantComplete(t, "daemon-left", rec, false)

	// A recipe mismatch: the traced steps are not the recipe's.
	rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{step(evs...)}, Recipe: []string{"other"}})
	if err != nil {
		t.Fatal(err)
	}
	wantReason(t, rec, rRecipe)
	wantComplete(t, "recipe mismatch", rec, false)
}

// The flag is not part of the record's JSON: the recorder carries it in the
// shard record's own writes_complete key.
func TestWritesCompleteIsNotInTheRecordJSON(t *testing.T) {
	h := newHarness(t)
	rec := h.wc(mkdir(abs("code/p/a/out")))
	if !rec.WritesComplete {
		t.Fatal("setup")
	}
	b, err := json.Marshal(rec)
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(string(b), "writes_complete") {
		t.Errorf("the record JSON must not carry the flag: %s", b)
	}
}

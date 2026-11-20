//go:build linux && amd64

package main

// When a thread group exits, the kernel kills its other threads wherever they
// are, and strace prints a thread that dies inside a system call in shapes no
// canned trace can vouch for (KILN04 D35, the dying-call rule):
//
//	<... tgkill resumed>) = ? <unavailable>
//	<... clone resumed> <unfinished ...>) = ?
//	openat(AT_FDCWD</w>, 0x7f.., O_RDONLY|O_CLOEXEC <unfinished ...>   (then only the exit)
//
// Go's os.Exit is exit_group, so a Go program whose workers are inside file
// system calls when main returns produces them.  This test runs such a program
// under the recorder's own strace argv, many times so the race is hit often,
// and requires that every trace parses with no refusal and with both readers
// agreeing (a cross-check divergence is a failure, an allowed stricter-only one
// too).
//
// Whether a given run shows a dying call is the kernel's timing, so the test
// does NOT insist on one: it insists that the loop really ran (every child
// exited through exit_group) and logs how many dying shapes it saw.  A run that
// saw none is still a real Go trace through both readers; a run that sees a
// shape the parser refuses fails, with the lines around it.

import (
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

const (
	dyingProbeCommand  = "sandbox-dying"
	dyingChildCommand  = "sandbox-dying-child"
	dyingIterations    = 20
	dyingWorkersPerRun = 12
)

// dyingProbe runs INSIDE the sandbox, as the step.  It runs dyingIterations
// children one after another and prints how many exited cleanly.
func dyingProbe(out *os.File) int {
	exe, err := os.Executable()
	if err != nil {
		out.WriteString("error=" + err.Error() + "\n")
		return 1
	}
	ok := 0
	for i := 0; i < dyingIterations; i++ {
		cmd := exec.Command(exe, dyingChildCommand)
		if err := cmd.Run(); err == nil {
			ok++
		}
	}
	out.WriteString("iterations=" + strconv.Itoa(dyingIterations) + " ok=" + strconv.Itoa(ok) + "\n")
	return 0
}

// dyingChild starts workers that make file system calls in a tight loop, lets
// them run for a moment, and calls os.Exit (exit_group) while they are inside a
// call.  The calls are read-only ones the parser must drop: stat, open, close,
// a directory listing.
func dyingChild() int {
	for w := 0; w < dyingWorkersPerRun; w++ {
		go func() {
			for {
				_, _ = os.Stat("data.txt")
				if f, err := os.Open("data.txt"); err == nil {
					_ = f.Close()
				}
				if d, err := os.Open("."); err == nil {
					_, _ = d.ReadDir(-1)
					_ = d.Close()
				}
			}
		}()
	}
	time.Sleep(20 * time.Millisecond)
	os.Exit(0)
	return 0
}

var (
	dyingExitGroup   = regexp.MustCompile(`(?m)^\d+ +exit_group\(`)
	dyingUnavailable = regexp.MustCompile(`(?m)<unavailable>$`)
	dyingCloneResume = regexp.MustCompile(`(?m)<\.\.\. clone resumed> <unfinished \.\.\.>\)`)
	// An unfinished call whose pid never prints a resume: counted per line in
	// dyingUnresumed below (a regexp cannot see "never").
	dyingOpenLine = regexp.MustCompile(`(?m)^(\d+) +\w+\(.*<unfinished \.\.\.>$`)
)

// dyingUnresumed counts the unfinished lines whose pid does not resume them: the
// shape 3 lines (killed out of the syscall-entry stop).
func dyingUnresumed(raw []byte) int {
	pending := map[string]bool{}
	n := 0
	for _, l := range strings.Split(string(raw), "\n") {
		if m := dyingOpenLine.FindStringSubmatch(l); m != nil {
			pending[m[1]] = true
			continue
		}
		pid, rest, ok := strings.Cut(l, " ")
		if !ok {
			continue
		}
		rest = strings.TrimLeft(rest, " ")
		switch {
		case strings.HasPrefix(rest, "<... "):
			delete(pending, pid)
		case strings.HasPrefix(rest, "+++ "):
			if pending[pid] {
				n++
				delete(pending, pid)
			}
		}
	}
	return n
}

// TestSandboxStraceDyingThreads: a Go step whose workers are killed mid-call by
// os.Exit, twenty times over, produces traces that parse with no refusal and no
// disagreement between the two readers.
func TestSandboxStraceDyingThreads(t *testing.T) {
	r := runStracedWith(t, "{pkg}/probe-bin "+dyingProbeCommand, func(f *sandboxFixture) {
		// The test binary lives under /tmp, which the step does not see: run a
		// copy from inside the checkout.
		exe, err := os.Executable()
		if err != nil {
			t.Fatal(err)
		}
		b, err := os.ReadFile(exe)
		if err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(f.pkg, "probe-bin"), b, 0o755); err != nil {
			t.Fatal(err)
		}
	})
	// runStracedWith already failed the test if the trace was refused.  The loop
	// must really have run, or the parse above proved nothing.
	want := "iterations=" + strconv.Itoa(dyingIterations) + " ok=" + strconv.Itoa(dyingIterations)
	if !strings.Contains(r.stdout, want) {
		t.Fatalf("the step's loop did not finish: %q, want %q (stderr %q)\n%s", r.stdout, want, r.stderr, r.tail(30))
	}
	raw, err := os.ReadFile(r.trf)
	if err != nil {
		t.Fatal(err)
	}
	if n := len(dyingExitGroup.FindAll(raw, -1)); n < dyingIterations {
		t.Fatalf("%d exit_group lines in the trace, want at least %d (one per child):\n%s", n, dyingIterations, r.tail(30))
	}
	// Both readers agree on the whole trace (the production path is the grammar
	// path alone; the old scanner is the oracle).
	_, perr, mismatch, allowed := trace.ParseCrossChecked(strings.NewReader(string(raw)), r.f.pkg, trace.Options{HelperRoot: true, HelperExe: r.f.exe})
	if perr != nil {
		t.Fatalf("the cross-checked parse refused the trace: %v\n%s", perr, r.tail(30))
	}
	if mismatch != nil {
		t.Fatalf("the readers disagree: %v", mismatch)
	}
	if allowed != 0 {
		t.Errorf("%d allowed stricter-only divergences on a real trace", allowed)
	}
	t.Logf("%d children; dying shapes seen: %d unavailable, %d clone resumed unfinished, %d unfinished-then-exit",
		dyingIterations, len(dyingUnavailable.FindAll(raw, -1)), len(dyingCloneResume.FindAll(raw, -1)), dyingUnresumed(raw))
}

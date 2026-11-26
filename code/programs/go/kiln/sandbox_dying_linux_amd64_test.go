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
// system calls when main returns produces them.  The workers here really do die
// inside openat and newfstatat, and the rule REFUSES those (a call that reads is
// never dropped: its result is lost, not absent), so this test must not insist
// that the trace parses.  It runs such a program under the recorder's own strace
// argv, many times so the race is hit often, and requires:
//
//   - the loop ran: every child exited (iterations=20 ok=20) through exit_group;
//   - the two readers agree on the whole trace, with no allowed stricter-only
//     divergence;
//   - the trace either parses, or is refused with exactly unresumed-call,
//     unparsable-line or result, on a line of a dying shape (an unfinished call
//     left pending at the pid's exit, a resumed half whose result is
//     `= ? <unavailable>` or `<unfinished ...>) = ?`, or a resumed half whose
//     result is `-1` with an errno strace could not name, `(errno N)`, followed
//     by that pid's own exit): any other refusal is a shape the parser does not
//     know and fails the test with the lines around it.
//     unparsable-line happens when a resumed newfstatat or getdents64 glues to a
//     pending prefix that ends in a comma.  The `(errno N)` form was first seen
//     on ubuntu-24.04 (PR #19750's kiln-sandbox run): a worker killed between
//     its openat's kernel return and strace's exit stop resumed with
//     `= -1 (errno 18446744073709551544)` (-72 read as unsigned) and then
//     exited.  Both readers refuse it as `result`, which fails closed; the rule
//     for dying calls (D35) is not widened for it.
//
// How many dying shapes a run shows is the kernel's timing, so it is logged, not
// asserted.

import (
	"fmt"
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
	// A dying shape, on a line the parser refused: the call left unfinished, or
	// its resumed half with an unavailable or unfinished result.
	dyingShapeUnfinished = regexp.MustCompile(`^\d+ +[a-z0-9_]+\(.*<unfinished \.\.\.>$`)
	dyingShapeResumed    = regexp.MustCompile(`^\d+ +<\.\.\. [a-z0-9_]+ resumed>.*(= \? <unavailable>|<unfinished \.\.\.>\) = \?)$`)
	// strace 6.8 (ubuntu-24.04) spells a thread killed mid-call as ONE line: the
	// whole call, with its "<unfinished ...>" marker still inside it, then
	// `) = ?` (no separate resumed half).  The parser refuses it as
	// unparsable-line, which fails closed; this only lets the test recognise it
	// as a dying shape.  Same narrowness as the others: the marker must be the
	// last thing before `) = ?`.
	dyingShapeOneLine = regexp.MustCompile(`^\d+ +[a-z0-9_]+\(.*<unfinished \.\.\.>\) = \?$`)
	// A resumed half whose result is -1 with an errno strace could not name.
	dyingShapeBadErrno = regexp.MustCompile(`^(\d+) +<\.\.\. [a-z0-9_]+ resumed>.*= -1 \(errno \d+\)$`)
	dyingExitGroup     = regexp.MustCompile(`(?m)^\d+ +exit_group\(`)
	dyingUnavailable   = regexp.MustCompile(`(?m)<unavailable>$`)
	dyingCloneResume   = regexp.MustCompile(`(?m)<\.\.\. clone resumed> <unfinished \.\.\.>\)`)
	// An unfinished call whose pid never prints a resume: counted per line in
	// dyingUnresumed below (a regexp cannot see "never").
	dyingOpenLine = regexp.MustCompile(`(?m)^(\d+) +\w+\(.*<unfinished \.\.\.>$`)
)

// nextOwnLineIsExit reports whether the first line after line (1-based) printed
// by pid is that pid's own exit (`+++ exited` or `+++ killed`).
func nextOwnLineIsExit(lines []string, line int, pid string) bool {
	for _, l := range lines[line:] {
		f := strings.Fields(l)
		if len(f) < 2 || f[0] != pid {
			continue
		}
		return f[1] == "+++"
	}
	return false
}

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
// os.Exit, twenty times over, gives traces the two readers agree on, which parse
// or are refused only as unresumed-call or unparsable-line on a dying shape.
func TestSandboxStraceDyingThreads(t *testing.T) {
	r := runStracedOr(t, "{pkg}/probe-bin "+dyingProbeCommand, func(f *sandboxFixture) {
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
	}, func(e *trace.Error) bool {
		return e.Reason == "unresumed-call" || e.Reason == "unparsable-line" || e.Reason == "result"
	})
	// The loop must really have run, or nothing below proved anything.
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
	// A refusal that runStracedOr tolerated must be on a line of a dying shape.
	// The line is our own synthetic Go fixture's, so it is safe to print.
	if r.refusal != nil {
		lines := strings.Split(string(raw), "\n")
		if r.refusal.Line < 1 || r.refusal.Line > len(lines) {
			t.Fatalf("the refusal %v names no line of the trace", r.refusal)
		}
		text := lines[r.refusal.Line-1]
		var shape string
		switch {
		case dyingShapeUnfinished.MatchString(text):
			shape = "an unfinished call left pending at the pid's exit"
		case dyingShapeOneLine.MatchString(text):
			shape = "a whole call with its unfinished marker inside it, result unknown (strace 6.8)"
		case dyingShapeResumed.MatchString(text):
			shape = "a resumed half whose result is unavailable or unfinished"
		case dyingShapeBadErrno.MatchString(text) && nextOwnLineIsExit(lines, r.refusal.Line, dyingShapeBadErrno.FindStringSubmatch(text)[1]):
			shape = "a resumed half with an unnamed errno, then the pid's exit"
		case bareOpenatUnknownResume.MatchString(text):
			terminalLine := laterBareOpenatExit(raw, r.refusal)
			if terminalLine == 0 {
				pid, _ := strconv.Atoi(bareOpenatUnknownResume.FindStringSubmatch(text)[1])
				t.Fatalf("refused as %s at line %d: bare resumed openat result has no unambiguous later same-pid terminal exit\n--- refusal and terminal candidate ---\n%s",
					r.refusal.Reason, r.refusal.Line, dyingRefusalDiagnostic(raw, r.refusal.Line, pid))
			}
			shape = fmt.Sprintf("bare resumed openat result, later same-pid terminal exit at line %d", terminalLine)
		default:
			t.Fatalf("refused as %s at line %d, which is not a dying shape: %s\n%s", r.refusal.Reason, r.refusal.Line, text, r.tail(30))
		}
		t.Logf("refused as %s at line %d: %s (%s)", r.refusal.Reason, r.refusal.Line, text, shape)
	}
	// Both readers agree on the whole trace (the production path is the grammar
	// path alone; the old scanner is the oracle).
	_, _, mismatch, allowed := trace.ParseCrossChecked(strings.NewReader(string(raw)), r.f.pkg, trace.Options{HelperRoot: true, HelperExe: r.f.exe})
	if mismatch != nil {
		t.Fatalf("the readers disagree: %v", mismatch)
	}
	if allowed != 0 {
		t.Errorf("%d allowed stricter-only divergences on a real trace", allowed)
	}
	t.Logf("%d children; dying shapes seen: %d unavailable, %d clone resumed unfinished, %d unfinished-then-exit",
		dyingIterations, len(dyingUnavailable.FindAll(raw, -1)), len(dyingCloneResume.FindAll(raw, -1)), dyingUnresumed(raw))
}

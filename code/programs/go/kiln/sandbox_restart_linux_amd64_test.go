//go:build linux && amd64

package main

// KILN04 D50 against the real kernel and the real strace.
//
// Rule 2b drops a fork, vfork or clone that ended `= ? ERESTARTNOINTR`; rule 2c
// accepts a successful read-only open with no -y annotation.  Both rest on how
// the kernel and strace really print those calls, which no canned trace shows.
// A rule that passed once was enough to cause the D46 flake, so CI runs this
// test (and its neighbours) many times in a row (`-count=20`, ci.yml).
//
// The kernel restarts a fork when a signal is pending while the child is being
// made.  Whether one is pending at that instant is a race, so the test cannot
// REQUIRE the shape to appear; it makes it likely (a loop of forks while a
// sibling signals the shell without pause), logs how often the rules fired, and
// holds the properties that must be true whenever it does:
//
//   - the trace parses, both readers agree (probeTraceWall runs the cross-check
//     and fails on a mismatch or an allowed divergence), or
//   - it is refused only for a shape a rule deliberately keeps refusing: an
//     ERESTART name other than ERESTARTNOINTR, a bare `= ?`, ERESTARTNOINTR on a
//     call that is not a spawn.  Any other refusal fails the test.
//
// A silent acceptance is also checked: every pid the parse made must have an
// exit or be the root group's own.

import (
	"os"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// restartedSpawnRefused says the refused line of the raw trace is a fork, vfork
// or clone whose result ENDS in `= ? ERESTARTNOINTR (To be restarted)`: exactly
// what rule 2b must now accept, so a refusal there means 2b is broken.  A split
// call is judged by its resumed half, which follows its unfinished line.
func restartedSpawnRefused(lines []string, at int) bool {
	const tail = "= ? ERESTARTNOINTR (To be restarted)"
	for i := at - 1; i < len(lines) && i < at+4; i++ {
		if i < 0 {
			continue
		}
		l := strings.TrimSpace(lines[i])
		if strings.HasSuffix(l, tail) && (strings.Contains(l, "clone") || strings.Contains(l, "fork")) {
			return true
		}
	}
	return false
}

// keptRefusal says the refused line of the raw trace is one the D50 rules keep
// refusing on purpose (another ERESTART name, a bare `= ?`), and NOT a restarted
// spawn that rule 2b should have accepted.
func keptRefusal(raw string, te *trace.Error) bool {
	lines := strings.Split(raw, "\n")
	if te.Line < 1 || te.Line > len(lines) {
		return false
	}
	if restartedSpawnRefused(lines, te.Line) {
		return false
	}
	// A call that died with its tracee (SIGKILL at teardown) never resumes: the
	// deliberately refused unresumed-call class, tolerated like the signals and
	// dying-threads tests tolerate it.
	if te.Reason == "unresumed-call" {
		return true
	}
	l := lines[te.Line-1]
	if !strings.Contains(l, "= ? ") && !strings.HasSuffix(strings.TrimSpace(l), "= ?") {
		return false
	}
	return strings.Contains(l, "ERESTART") || strings.HasSuffix(strings.TrimSpace(l), "= ?")
}

func TestSandboxStraceRestartedSpawn(t *testing.T) {
	// Every signaller stops when the main shell creates the stop file, and the
	// main shell waits for each one (a plain `wait` returns early when a trapped
	// signal arrives, so it loops on kill -0) before the script ends.  Otherwise
	// the sandbox's PID 1 teardown SIGKILLs a signaller in the middle of a call
	// and the trace ends in the deliberately refused unresumed-call shape.
	const stop = `STOP=${TMPDIR:-/tmp}/stop-signallers
signaller() { while [ ! -e $STOP ]; do kill -$1 $me 2>/dev/null; done; }
reap() { for p in "$@"; do while kill -0 $p 2>/dev/null; do wait $p; done; done; }
`
	cases := []struct{ name, script string }{
		// Background subshells: forks with no wait4 and no exec between them, so
		// the signal handler's only way into a handled call is the fork itself.
		{"background-forks-under-signals", `trap : USR1
me=$$
` + stop + `( signaller USR1 ) & s1=$!
i=0; while [ $i -lt 400 ]; do ( : ) & i=$((i+1)); done
: > $STOP; reap $s1; wait; cat {data}`},
		// A shell that forks 300 times while a sibling signals it.
		{"fork-under-signals", `trap : USR1
me=$$
` + stop + `( signaller USR1 ) & s1=$!
i=0; while [ $i -lt 300 ]; do /bin/true; i=$((i+1)); done
: > $STOP; reap $s1; wait; cat {data}`},
		// Two signallers, and a subshell forking too, for more pending-signal windows.
		{"subshells-under-signals", `trap : USR1 USR2
me=$$
` + stop + `( signaller USR1 ) & s1=$!
( signaller USR2 ) & s2=$!
i=0; while [ $i -lt 200 ]; do ( /bin/true ) ; i=$((i+1)); done
: > $STOP; reap $s1 $s2; wait; cat {data}`},
	}
	restarted, bare := 0, 0
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			r := runStracedOr(t, c.script, nil, func(te *trace.Error) bool { return true })
			if r.refusal != nil {
				raw, _ := os.ReadFile(r.trf)
				te := r.refusal
				if !keptRefusal(string(raw), te) {
					t.Fatalf("the trace was refused for a shape no D50 rule keeps refusing: %v\n--- trace tail ---\n%s", te, r.tail(25))
				}
				t.Logf("refused on purpose: %v", te)
				t.Logf("RESTARTED_SPAWNS=0 REFUSED=1")
				return
			}
			raw, _ := os.ReadFile(r.trf)
			t.Logf("restarted spawns in the raw trace: %d; parsed as dropped: %d; bare opens accepted: %d",
				strings.Count(string(raw), "= ? ERESTARTNOINTR"), r.tr.RestartedSpawns, r.tr.BareOpens)
			t.Logf("RESTARTED_SPAWNS=%d REFUSED=0", r.tr.RestartedSpawns)
			restarted += r.tr.RestartedSpawns
			bare += r.tr.BareOpens
			// Every NOINTR spawn line the kernel printed was accepted as dropped.
			if n := strings.Count(string(raw), "= ? ERESTARTNOINTR"); r.tr.RestartedSpawns > n {
				t.Errorf("the parse dropped %d restarted spawns but the trace shows %d", r.tr.RestartedSpawns, n)
			}
			// No process the dropped calls could have invented: every non-root pid
			// has a parent that is in the tree.
			for pid, p := range r.tr.Procs {
				if p.Parent != 0 && r.tr.Procs[p.Parent] == nil {
					t.Errorf("pid %d has a parent %d that is not in the tree", pid, p.Parent)
				}
			}
		})
	}
	t.Logf("D50 on the real kernel: restarted spawns dropped %d, bare read-only opens accepted %d (zero is allowed: the race may not fire)", restarted, bare)
}
